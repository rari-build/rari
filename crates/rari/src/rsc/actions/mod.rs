use std::{fmt::Write, sync::Arc};

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use cow_utils::CowUtils;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::error;

use crate::server::ServerState;

const MAX_BOUND_ARGS: usize = 1000;
const MAX_BIGINT_DIGITS: usize = 300;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ValidationConfig {
    pub max_depth: usize,
    pub max_string_length: usize,
    pub max_array_length: usize,
    pub max_object_keys: usize,
    pub allow_special_numbers: bool,
    pub max_total_elements: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_string_length: 10_000,
            max_array_length: 1_000,
            max_object_keys: 100,
            allow_special_numbers: false,
            max_total_elements: 1_000_000,
        }
    }
}

impl ValidationConfig {
    pub fn development() -> Self {
        Self {
            max_depth: 20,
            max_string_length: 50_000,
            max_array_length: 5_000,
            max_object_keys: 500,
            allow_special_numbers: false,
            max_total_elements: 5_000_000,
        }
    }

    pub fn production() -> Self {
        Self::default()
    }
}

#[derive(Debug)]
struct ValidationContext {
    total_elements: usize,
    has_fork: bool,
}

impl ValidationContext {
    fn new() -> Self {
        Self { total_elements: 0, has_fork: false }
    }

    fn bump_count(&mut self, count: usize, config: &ValidationConfig) -> Result<(), RariError> {
        self.total_elements = self.total_elements.saturating_add(count);

        if self.total_elements > config.max_total_elements && self.has_fork {
            return Err(RariError::bad_request(format!(
                "Maximum array nesting exceeded: {} > {}. Large nested arrays can be dangerous. Try adding intermediate objects.",
                self.total_elements, config.max_total_elements
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ServerActionRequest {
    pub id: String,
    pub export_name: String,
    pub args: Vec<JsonValue>,
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct ServerActionResponse {
    pub success: bool,
    pub result: Option<JsonValue>,
    pub error: Option<String>,
    pub redirect: Option<String>,
}

fn effective_port(url: &url::Url) -> u16 {
    url.port().unwrap_or_else(|| match url.scheme() {
        "https" => 443,
        "http" => 80,
        _ => 0,
    })
}

fn normalize_origin(url: &url::Url) -> (String, String, u16) {
    (url.scheme().to_string(), url.host_str().unwrap_or("").to_string(), effective_port(url))
}

fn check_origin(headers: &HeaderMap, allowed_origins: &[String]) -> Result<(), StatusCode> {
    if allowed_origins.is_empty() {
        let host = headers.get("host").and_then(|v| v.to_str().ok()).ok_or_else(|| {
            error!("Missing host header in server action request");
            StatusCode::BAD_REQUEST
        })?;

        let scheme = headers
            .get("x-forwarded-proto")
            .or_else(|| headers.get("x-forwarded-protocol"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or_else(|| {
                tracing::debug!(
                    "No x-forwarded-proto header; defaulting to http for origin validation"
                );
                "http"
            });

        let server_origin_str = format!("{scheme}://{host}");
        let server_origin_url = url::Url::parse(&server_origin_str).map_err(|e| {
            error!("Failed to parse server origin: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        let server_origin_tuple = normalize_origin(&server_origin_url);

        if let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) {
            if let Ok(origin_url) = url::Url::parse(origin) {
                let origin_tuple = normalize_origin(&origin_url);

                if origin_tuple == server_origin_tuple {
                    return Ok(());
                }
            }
            error!("Origin mismatch: origin={}, server_origin={}", origin, server_origin_str);
            return Err(StatusCode::FORBIDDEN);
        }

        if let Some(referer) = headers.get("referer").and_then(|v| v.to_str().ok()) {
            if let Ok(referer_url) = url::Url::parse(referer) {
                let referer_tuple = normalize_origin(&referer_url);

                if referer_tuple == server_origin_tuple {
                    return Ok(());
                }
                error!(
                    "Referer mismatch: referer_origin={}://{}:{}, server_origin={}",
                    referer_tuple.0, referer_tuple.1, referer_tuple.2, server_origin_str
                );
            } else {
                error!("Invalid referer header: failed to parse");
            }
            return Err(StatusCode::FORBIDDEN);
        }

        error!("Missing origin and referer headers in server action request");
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) {
        if !crate::server::core::utils::http::is_origin_allowed(origin, allowed_origins) {
            error!("Invalid origin: {}", origin);
            return Err(StatusCode::FORBIDDEN);
        }
        return Ok(());
    }

    if let Some(referer) = headers.get("referer").and_then(|v| v.to_str().ok()) {
        if let Ok(referer_url) = url::Url::parse(referer) {
            let (scheme, host, port) = normalize_origin(&referer_url);
            let referer_origin =
                if (scheme == "http" && port == 80) || (scheme == "https" && port == 443) {
                    format!("{scheme}://{host}")
                } else {
                    format!("{scheme}://{host}:{port}")
                };
            if crate::server::core::utils::http::is_origin_allowed(&referer_origin, allowed_origins)
            {
                return Ok(());
            }
            error!("Invalid referer origin: {}", referer_origin);
        } else {
            error!("Invalid referer header: failed to parse origin");
        }
        return Err(StatusCode::FORBIDDEN);
    }

    error!("Missing Origin and Referer headers with non-empty allowed_origins");
    Err(StatusCode::FORBIDDEN)
}

async fn clear_layout_html_cache(state: &ServerState) {
    if let Err(e) = state.layout_html_cache.clear().await {
        tracing::warn!(
            error = %e,
            "layout_html_cache.clear failed during action; stale layout entries may persist until next revalidate"
        );
    }
}

pub async fn handle_server_action(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, StatusCode> {
    let allowed_origins = state.config.action_origins();
    check_origin(&headers, &allowed_origins)?;

    let request: ServerActionRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse server action request: {}", e);
            let mut response = Json(ServerActionResponse {
                success: false,
                result: None,
                error: Some("Invalid request format".to_string()),
                redirect: None,
            })
            .into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
                "no-store, no-cache, must-revalidate, private"
                    .parse()
                    .expect("Valid cache-control header"),
            );
            return Ok(response);
        }
    };

    if request.args.len() > MAX_BOUND_ARGS {
        error!("Too many server function arguments: {} > {}", request.args.len(), MAX_BOUND_ARGS);
        let mut response = Json(ServerActionResponse {
            success: false,
            result: None,
            error: Some(format!(
                "Server Function has too many bound arguments. Received {} but the limit is {}.",
                request.args.len(),
                MAX_BOUND_ARGS
            )),
            redirect: None,
        })
        .into_response();
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
            "no-store, no-cache, must-revalidate, private"
                .parse()
                .expect("Valid cache-control header"),
        );
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return Ok(response);
    }

    if is_reserved_export_name(&request.export_name) {
        error!("Attempted to call reserved export name: {}", request.export_name);
        let mut response = Json(ServerActionResponse {
            success: false,
            result: None,
            error: Some(format!(
                "Invalid export name '{}': reserved for internal use",
                request.export_name
            )),
            redirect: None,
        })
        .into_response();
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
            "no-store, no-cache, must-revalidate, private"
                .parse()
                .expect("Valid cache-control header"),
        );
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return Ok(response);
    }

    let validation_config = if state.config.is_development() {
        ValidationConfig::development()
    } else {
        ValidationConfig::production()
    };

    let sanitized_args = match validate_and_sanitize_args(&request.args, &validation_config) {
        Ok(args) => args,
        Err(e) => {
            error!("Input validation failed: {}", e);
            let mut response = Json(ServerActionResponse {
                success: false,
                result: None,
                error: Some(format!("Input validation failed: {e}")),
                redirect: None,
            })
            .into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
                "no-store, no-cache, must-revalidate, private"
                    .parse()
                    .expect("Valid cache-control header"),
            );
            *response.status_mut() = StatusCode::BAD_REQUEST;
            return Ok(response);
        }
    };

    let cookie_header =
        headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).map(ToString::to_string);

    let request_context = std::sync::Arc::new(
        crate::server::middleware::request_context::RequestContext::new(
            "/_rari/action".to_string(),
        )
        .with_cookies(cookie_header),
    );

    let renderer = state.renderer.lock().await;

    let result = renderer
        .runtime
        .execute_with_request_context(Arc::clone(&request_context), async {
            renderer
                .execute_server_function(&request.id, &request.export_name, &sanitized_args)
                .await
        })
        .await;

    match result {
        Ok(value) => {
            let redirect_config = state.config.redirect_config();
            let redirect = extract_redirect_from_result(&value, &redirect_config);

            if let Some(ref redirect_url) = redirect {
                let redirect_path = if let Ok(parsed) = url::Url::parse(redirect_url) {
                    parsed.path().to_string()
                } else if redirect_url.starts_with('/') {
                    redirect_url.split('?').next().unwrap_or(redirect_url).to_string()
                } else {
                    redirect_url.clone()
                };

                state.response_cache.invalidate_by_tag(&redirect_path).await;
                state.html_cache.remove(&redirect_path);
                clear_layout_html_cache(&state).await;
            }

            let response =
                ServerActionResponse { success: true, result: Some(value), error: None, redirect };

            let mut response = Json(response).into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
                "no-store, no-cache, must-revalidate, private"
                    .parse()
                    .expect("Valid cache-control header"),
            );

            append_pending_cookies(response.headers_mut(), &request_context.pending_cookies);

            Ok(response)
        }
        Err(e) => {
            error!("Server action execution failed: {}", e);
            let mut response = Json(ServerActionResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
                redirect: None,
            })
            .into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
                "no-store, no-cache, must-revalidate, private"
                    .parse()
                    .expect("Valid cache-control header"),
            );
            append_pending_cookies(response.headers_mut(), &request_context.pending_cookies);
            Ok(response)
        }
    }
}

pub async fn handle_form_action(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, StatusCode> {
    let allowed_origins = state.config.action_origins();
    check_origin(&headers, &allowed_origins)?;

    let form_data = match parse_form_data(&body) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse form data: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let action_id = form_data.get("__action_id").ok_or(StatusCode::BAD_REQUEST)?;
    let export_name = form_data.get("__export_name").ok_or(StatusCode::BAD_REQUEST)?;

    if is_reserved_export_name(export_name) {
        error!("Attempted to call reserved export name in form action: {}", export_name);
        return Err(StatusCode::BAD_REQUEST);
    }

    let args = convert_form_data_to_args(&form_data);

    let validation_config = if state.config.is_development() {
        ValidationConfig::development()
    } else {
        ValidationConfig::production()
    };

    let sanitized_args = match validate_and_sanitize_args(&args, &validation_config) {
        Ok(args) => args,
        Err(e) => {
            error!("Form action input validation failed: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let cookie_header =
        headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).map(ToString::to_string);

    let request_context = std::sync::Arc::new(
        crate::server::middleware::request_context::RequestContext::new(
            "/_rari/action".to_string(),
        )
        .with_cookies(cookie_header),
    );

    let renderer = state.renderer.lock().await;

    let result = renderer
        .runtime
        .execute_with_request_context(Arc::clone(&request_context), async {
            renderer.execute_server_function(action_id, export_name, &sanitized_args).await
        })
        .await;

    match result {
        Ok(value) => {
            let redirect_config = state.config.redirect_config();
            if let Some(redirect_url) = extract_redirect_from_result(&value, &redirect_config) {
                let redirect_path = if let Ok(parsed) = url::Url::parse(&redirect_url) {
                    parsed.path().to_string()
                } else if redirect_url.starts_with('/') {
                    redirect_url.split('?').next().unwrap_or(&redirect_url).to_string()
                } else {
                    redirect_url.clone()
                };

                state.response_cache.invalidate_by_tag(&redirect_path).await;
                state.html_cache.remove(&redirect_path);
                clear_layout_html_cache(&state).await;

                let mut redirect_response = Response::builder()
                    .status(StatusCode::SEE_OTHER)
                    .header("Location", redirect_url)
                    .header("Cache-Control", "no-store, no-cache, must-revalidate")
                    .body("".into())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                append_pending_cookies(
                    redirect_response.headers_mut(),
                    &request_context.pending_cookies,
                );

                return Ok(redirect_response);
            }

            let (redirect_url, redirect_path_opt) = if let Some(referer) =
                headers.get("referer").and_then(|h| h.to_str().ok())
            {
                if let Ok(parsed) = url::Url::parse(referer) {
                    let referer_tuple = normalize_origin(&parsed);

                    let server_origin_tuple_opt = {
                        let host = headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or("");
                        let scheme = headers
                            .get("x-forwarded-proto")
                            .or_else(|| headers.get("x-forwarded-protocol"))
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("http");
                        let server_origin_str = format!("{scheme}://{host}");

                        url::Url::parse(&server_origin_str).ok().map(|u| normalize_origin(&u))
                    };

                    let (is_same_origin, is_allowed) = if allowed_origins.is_empty() {
                        if let Some(server_origin_tuple) = &server_origin_tuple_opt {
                            let is_same = referer_tuple == *server_origin_tuple;
                            (is_same, is_same)
                        } else {
                            (false, false)
                        }
                    } else {
                        let is_same =
                            server_origin_tuple_opt.as_ref().is_some_and(|t| referer_tuple == *t);

                        let referer_origin = format!(
                            "{}://{}:{}",
                            referer_tuple.0, referer_tuple.1, referer_tuple.2
                        );
                        let allowed = crate::server::core::utils::http::is_origin_allowed(
                            &referer_origin,
                            &allowed_origins,
                        );
                        (is_same, allowed)
                    };

                    if is_allowed {
                        if is_same_origin {
                            let path_and_query = if let Some(query) = parsed.query() {
                                format!("{}?{}", parsed.path(), query)
                            } else {
                                parsed.path().to_string()
                            };
                            (path_and_query, Some(parsed.path().to_string()))
                        } else {
                            (referer.to_string(), None)
                        }
                    } else {
                        ("/".to_string(), None)
                    }
                } else {
                    ("/".to_string(), None)
                }
            } else {
                ("/".to_string(), None)
            };

            if let Some(redirect_path) = redirect_path_opt {
                state.response_cache.invalidate_by_tag(&redirect_path).await;
                state.html_cache.remove(&redirect_path);
                clear_layout_html_cache(&state).await;
            }

            let mut redirect_response = Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header("Location", redirect_url)
                .header("Cache-Control", "no-store, no-cache, must-revalidate")
                .body("".into())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            append_pending_cookies(
                redirect_response.headers_mut(),
                &request_context.pending_cookies,
            );

            Ok(redirect_response)
        }
        Err(e) => {
            error!("Form action execution failed: {}", e);
            let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
            append_pending_cookies(response.headers_mut(), &request_context.pending_cookies);
            Ok(response)
        }
    }
}

pub fn validate_redirect_url(
    url: &str,
    config: &crate::server::config::RedirectConfig,
) -> Result<String, RariError> {
    if config.allow_relative && url.starts_with('/') && !url.starts_with("//") {
        return Ok(url.to_string());
    }

    let parsed =
        url::Url::parse(url).map_err(|_| RariError::bad_request("Invalid redirect URL format"))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(RariError::bad_request("Invalid redirect scheme: only http/https allowed"));
    }

    if let Some(host) = parsed.host_str() {
        let is_allowed = config.allowed_hosts.iter().any(|allowed| {
            if config.allow_subdomains {
                host == allowed || host.ends_with(&format!(".{allowed}"))
            } else {
                host == allowed
            }
        });

        if !is_allowed {
            return Err(RariError::bad_request("Redirect to untrusted host not allowed"));
        }
    } else {
        return Err(RariError::bad_request("Invalid redirect URL: missing host"));
    }

    Ok(url.to_string())
}

fn extract_redirect_from_result(
    result: &JsonValue,
    config: &crate::server::config::RedirectConfig,
) -> Option<String> {
    if let Some(redirect) = result.get("redirect") {
        if let Some(url) = redirect.as_str() {
            return validate_redirect_url(url, config).ok();
        }
        if let Some(obj) = redirect.as_object()
            && let Some(destination) = obj.get("destination").and_then(|d| d.as_str())
        {
            return validate_redirect_url(destination, config).ok();
        }
    }
    None
}

fn parse_form_data(body: &Bytes) -> Result<FxHashMap<String, String>, RariError> {
    let body_str = std::str::from_utf8(body)
        .map_err(|_| RariError::bad_request("Invalid UTF-8 in form data"))?;

    let mut form_data = FxHashMap::default();

    for pair in body_str.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = percent_decode(key)
                .map_err(|_| RariError::bad_request("Invalid URL encoding in form key"))?;
            let value = percent_decode(value)
                .map_err(|_| RariError::bad_request("Invalid URL encoding in form value"))?;
            form_data.insert(key, value);
        }
    }

    Ok(form_data)
}

fn convert_form_data_to_args(form_data: &FxHashMap<String, String>) -> Vec<JsonValue> {
    let mut form_entries = serde_json::Map::new();

    for (key, value) in form_data {
        if key.starts_with("__") {
            continue;
        }
        form_entries.insert(key.clone(), JsonValue::String(value.clone()));
    }

    let form_data_object = JsonValue::Object(form_entries);

    vec![JsonValue::Null, form_data_object]
}

fn percent_decode(input: &str) -> Result<String, RariError> {
    let mut bytes = Vec::new();
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex1 =
                chars.next().ok_or_else(|| RariError::bad_request("Invalid percent encoding"))?;
            let hex2 =
                chars.next().ok_or_else(|| RariError::bad_request("Invalid percent encoding"))?;

            let hex_str = format!("{hex1}{hex2}");
            let byte = u8::from_str_radix(&hex_str, 16)
                .map_err(|_| RariError::bad_request("Invalid hex in percent encoding"))?;

            bytes.push(byte);
        } else if ch == '+' {
            bytes.push(b' ');
        } else {
            let mut buf = [0u8; 4];
            for b in ch.encode_utf8(&mut buf).bytes() {
                bytes.push(b);
            }
        }
    }

    String::from_utf8(bytes)
        .map_err(|_| RariError::bad_request("Invalid UTF-8 in percent-decoded data"))
}

pub fn validate_and_sanitize_args(
    args: &[JsonValue],
    config: &ValidationConfig,
) -> Result<Vec<JsonValue>, RariError> {
    let mut context = ValidationContext::new();
    args.iter().map(|arg| validate_and_sanitize_value(arg, config, 0, &mut context)).collect()
}

fn validate_and_sanitize_value(
    value: &JsonValue,
    config: &ValidationConfig,
    depth: usize,
    context: &mut ValidationContext,
) -> Result<JsonValue, RariError> {
    if depth > config.max_depth {
        return Err(RariError::bad_request(format!(
            "Maximum nesting depth exceeded: {} > {}",
            depth, config.max_depth
        )));
    }

    match value {
        JsonValue::String(s) => {
            if s.len() > config.max_string_length {
                return Err(RariError::bad_request(format!(
                    "String too long: {} > {}",
                    s.len(),
                    config.max_string_length
                )));
            }

            context.bump_count(s.len(), config)?;

            Ok(value.clone())
        }
        JsonValue::Number(n) => {
            if let Some(f) = n.as_f64()
                && !config.allow_special_numbers
                && !f.is_finite()
            {
                return Err(RariError::bad_request(
                    "Invalid number: Infinity or NaN not allowed".to_string(),
                ));
            }

            if let Some(f) = n.as_f64() {
                let abs_f = f.abs();
                if abs_f > 1e100 {
                    let estimated_digits =
                        if abs_f == 0.0 { 1 } else { (abs_f.log10().floor() as usize) + 1 };

                    if estimated_digits > MAX_BIGINT_DIGITS {
                        return Err(RariError::bad_request(format!(
                            "Number too large. Estimated {estimated_digits} digits but the limit is {MAX_BIGINT_DIGITS}."
                        )));
                    }
                }
            }

            Ok(value.clone())
        }
        JsonValue::Array(arr) => {
            if arr.len() > config.max_array_length {
                return Err(RariError::bad_request(format!(
                    "Array too large: {} > {}",
                    arr.len(),
                    config.max_array_length
                )));
            }

            if arr.len() > 1 {
                context.has_fork = true;
            }

            context.bump_count(arr.len() + 1, config)?;

            let validated: Result<Vec<_>, _> = arr
                .iter()
                .map(|v| validate_and_sanitize_value(v, config, depth + 1, context))
                .collect();

            Ok(JsonValue::Array(validated?))
        }
        JsonValue::Object(obj) => {
            if obj.len() > config.max_object_keys {
                return Err(RariError::bad_request(format!(
                    "Too many object keys: {} > {}",
                    obj.len(),
                    config.max_object_keys
                )));
            }

            let mut sanitized = serde_json::Map::new();
            for (key, val) in obj {
                if is_dangerous_property(key) {
                    continue;
                }

                let validated_val = validate_and_sanitize_value(val, config, depth + 1, context)?;
                sanitized.insert(key.clone(), validated_val);
            }

            Ok(JsonValue::Object(sanitized))
        }
        JsonValue::Bool(_) | JsonValue::Null => Ok(value.clone()),
    }
}

pub fn is_dangerous_property(key: &str) -> bool {
    matches!(
        key,
        "__proto__"
            | "constructor"
            | "prototype"
            | "__defineGetter__"
            | "__defineSetter__"
            | "__lookupGetter__"
            | "__lookupSetter__"
    )
}

pub fn is_reserved_export_name(name: &str) -> bool {
    matches!(
        name,
        "then"
            | "catch"
            | "finally"
            | "toString"
            | "valueOf"
            | "toLocaleString"
            | "constructor"
            | "Symbol"
            | "@@iterator"
            | "@@toStringTag"
    )
}

fn append_pending_cookies(
    headers: &mut axum::http::HeaderMap,
    pending_cookies: &dashmap::DashMap<
        crate::server::middleware::request_context::PendingCookieKey,
        crate::server::middleware::request_context::PendingCookie,
    >,
) {
    for entry in pending_cookies {
        let cookie = entry.value();
        match build_set_cookie_header(cookie) {
            Ok(set_cookie_value) => match set_cookie_value.parse() {
                Ok(header_value) => {
                    headers.append(header::SET_COOKIE, header_value);
                }
                Err(_) => {
                    tracing::warn!(
                        "Failed to parse Set-Cookie header for '{}': invalid header value",
                        cookie.name
                    );
                }
            },
            Err(e) => {
                tracing::warn!("Skipped invalid cookie '{}': {}", cookie.name, e);
            }
        }
    }
}

pub fn is_valid_cookie_name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b > 32 && b < 127 && !b"()<>@,;:\\\"/[]?={} \t".contains(&b))
}

pub fn is_valid_cookie_value(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, 0x21 | 0x23..=0x2B | 0x2D..=0x3A | 0x3C..=0x5B | 0x5D..=0x7E))
}

pub fn is_valid_attr_value(s: &str) -> bool {
    !s.is_empty() && s.is_ascii() && s.bytes().all(|b| b >= 32 && b != b';' && b != 127)
}

pub fn build_set_cookie_header(
    cookie: &crate::server::middleware::request_context::PendingCookie,
) -> Result<String, String> {
    if !is_valid_cookie_name(&cookie.name) {
        return Err(format!("invalid cookie name: {}", cookie.name));
    }
    if !is_valid_cookie_value(&cookie.value) {
        return Err(format!("invalid cookie value for: {}", cookie.name));
    }

    let path = cookie.path.as_deref().unwrap_or("/");
    if !is_valid_attr_value(path) {
        return Err(format!("invalid cookie path: {path}"));
    }

    let mut header = format!("{}={}", cookie.name, cookie.value);
    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
    write!(&mut header, "; Path={path}").unwrap();

    if let Some(domain) = &cookie.domain {
        if !is_valid_attr_value(domain) {
            return Err(format!("invalid cookie domain: {domain}"));
        }
        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
        write!(&mut header, "; Domain={domain}").unwrap();
    }
    if let Some(expires) = &cookie.expires {
        if !is_valid_attr_value(expires) {
            return Err(format!("invalid cookie expires: {expires}"));
        }
        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
        write!(&mut header, "; Expires={expires}").unwrap();
    }
    if let Some(max_age) = cookie.max_age {
        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
        write!(&mut header, "; Max-Age={max_age}").unwrap();
    }
    let normalized_same_site =
        cookie.same_site.as_deref().map(cow_utils::CowUtils::cow_to_ascii_lowercase);
    if normalized_same_site.as_deref() == Some("none") && !cookie.secure {
        return Err("SameSite=None requires Secure".to_string());
    }
    if cookie.partitioned && !cookie.secure {
        return Err("Partitioned requires Secure".to_string());
    }
    if cookie.http_only {
        header.push_str("; HttpOnly");
    }
    if cookie.secure {
        header.push_str("; Secure");
    }
    if let Some(same_site) = normalized_same_site.as_deref() {
        let serialized_same_site = match same_site {
            "strict" => "Strict",
            "lax" => "Lax",
            "none" => "None",
            _ => return Err(format!("invalid SameSite value: {same_site}")),
        };
        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
        write!(&mut header, "; SameSite={serialized_same_site}").unwrap();
    }
    if let Some(priority) = &cookie.priority {
        match priority.cow_to_ascii_lowercase().as_ref() {
            "low" => header.push_str("; Priority=Low"),
            "medium" => header.push_str("; Priority=Medium"),
            "high" => header.push_str("; Priority=High"),
            _ => return Err(format!("invalid Priority value: {priority}")),
        }
    }
    if cookie.partitioned {
        header.push_str("; Partitioned");
    }
    Ok(header)
}

#[cfg(test)]
#[allow(
    clippy::allow_attributes,
    clippy::unreadable_literal,
    clippy::needless_raw_string_hashes,
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp,
    clippy::bool_assert_comparison,
    clippy::redundant_clone,
    clippy::redundant_closure_for_method_calls,
    clippy::single_char_pattern,
    clippy::approx_constant,
    clippy::uninlined_format_args,
    clippy::module_inception,
    clippy::return_self_not_must_use,
    clippy::disallowed_methods,
    clippy::clone_on_ref_ptr,
    clippy::get_unwrap
)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::server::{config::RedirectConfig, middleware::request_context::PendingCookie};

    #[test]
    fn test_sanitize_args_removes_proto() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "__proto__": {
                "isAdmin": true
            },
            "username": "test"
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        assert_eq!(sanitized.len(), 1);
        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("__proto__"));
        assert_eq!(obj.get("username").unwrap().as_str().unwrap(), "test");
    }

    #[test]
    fn test_sanitize_args_removes_constructor() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "constructor": {
                "prototype": {
                    "isAdmin": true
                }
            },
            "data": "safe"
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("constructor"));
        assert_eq!(obj.get("data").unwrap().as_str().unwrap(), "safe");
    }

    #[test]
    fn test_sanitize_args_removes_prototype() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "prototype": {
                "polluted": true
            },
            "normal": "value"
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let obj = sanitized[0].as_object().unwrap();
        assert!(!obj.contains_key("prototype"));
        assert_eq!(obj.get("normal").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_sanitize_args_nested_objects() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "user": {
                "__proto__": {
                    "isAdmin": true
                },
                "name": "John",
                "settings": {
                    "constructor": "bad",
                    "theme": "dark"
                }
            }
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let obj = sanitized[0].as_object().unwrap();
        let user = obj.get("user").unwrap().as_object().unwrap();
        assert!(!user.contains_key("__proto__"));
        assert_eq!(user.get("name").unwrap().as_str().unwrap(), "John");

        let settings = user.get("settings").unwrap().as_object().unwrap();
        assert!(!settings.contains_key("constructor"));
        assert_eq!(settings.get("theme").unwrap().as_str().unwrap(), "dark");
    }

    #[test]
    fn test_sanitize_args_arrays() {
        let config = ValidationConfig::default();
        let args = vec![json!([
            {
                "__proto__": "bad",
                "id": 1
            },
            {
                "constructor": "bad",
                "id": 2
            }
        ])];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let arr = sanitized[0].as_array().unwrap();
        assert_eq!(arr.len(), 2);

        let obj1 = arr[0].as_object().unwrap();
        assert!(!obj1.contains_key("__proto__"));
        assert_eq!(obj1.get("id").unwrap().as_i64().unwrap(), 1);

        let obj2 = arr[1].as_object().unwrap();
        assert!(!obj2.contains_key("constructor"));
        assert_eq!(obj2.get("id").unwrap().as_i64().unwrap(), 2);
    }

    #[test]
    fn test_sanitize_args_preserves_safe_data() {
        let config = ValidationConfig::default();
        let args = vec![
            json!("string value"),
            json!(42),
            json!(true),
            json!(null),
            json!({
                "name": "test",
                "count": 10,
                "active": true,
                "tags": ["a", "b", "c"]
            }),
        ];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        assert_eq!(sanitized.len(), 5);
        assert_eq!(sanitized[0].as_str().unwrap(), "string value");
        assert_eq!(sanitized[1].as_i64().unwrap(), 42);
        assert_eq!(sanitized[2].as_bool().unwrap(), true);
        assert!(sanitized[3].is_null());

        let obj = sanitized[4].as_object().unwrap();
        assert_eq!(obj.get("name").unwrap().as_str().unwrap(), "test");
        assert_eq!(obj.get("count").unwrap().as_i64().unwrap(), 10);
        assert_eq!(obj.get("active").unwrap().as_bool().unwrap(), true);
        assert_eq!(obj.get("tags").unwrap().as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_is_dangerous_property() {
        assert!(is_dangerous_property("__proto__"));
        assert!(is_dangerous_property("constructor"));
        assert!(is_dangerous_property("prototype"));
        assert!(is_dangerous_property("__defineGetter__"));
        assert!(is_dangerous_property("__defineSetter__"));
        assert!(is_dangerous_property("__lookupGetter__"));
        assert!(is_dangerous_property("__lookupSetter__"));

        assert!(!is_dangerous_property("name"));
        assert!(!is_dangerous_property("value"));
        assert!(!is_dangerous_property("data"));
        assert!(!is_dangerous_property("__typename"));
    }

    #[test]
    fn test_sanitize_deeply_nested() {
        let config = ValidationConfig::default();
        let args = vec![json!({
            "level1": {
                "level2": {
                    "level3": {
                        "__proto__": "bad",
                        "level4": {
                            "constructor": "bad",
                            "safe": "value"
                        }
                    }
                }
            }
        })];

        let sanitized = validate_and_sanitize_args(&args, &config).unwrap();

        let obj = sanitized[0].as_object().unwrap();
        let level1 = obj.get("level1").unwrap().as_object().unwrap();
        let level2 = level1.get("level2").unwrap().as_object().unwrap();
        let level3 = level2.get("level3").unwrap().as_object().unwrap();
        assert!(!level3.contains_key("__proto__"));

        let level4 = level3.get("level4").unwrap().as_object().unwrap();
        assert!(!level4.contains_key("constructor"));
        assert_eq!(level4.get("safe").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_validation_depth_limit() {
        let config = ValidationConfig { max_depth: 3, ..Default::default() };

        let valid = vec![json!({
            "level1": {
                "level2": {
                    "level3": "ok"
                }
            }
        })];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let invalid = vec![json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "too deep"
                    }
                }
            }
        })];
        let result = validate_and_sanitize_args(&invalid, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nesting depth"));
    }

    #[test]
    fn test_validation_string_length() {
        let config = ValidationConfig { max_string_length: 100, ..Default::default() };

        let valid = vec![json!({"text": "A".repeat(100)})];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let invalid = vec![json!({"text": "A".repeat(101)})];
        let result = validate_and_sanitize_args(&invalid, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("String too long"));
    }

    #[test]
    fn test_validation_array_length() {
        let config = ValidationConfig { max_array_length: 10, ..Default::default() };

        let valid = vec![json!({"items": vec![1; 10]})];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let invalid = vec![json!({"items": vec![1; 11]})];
        let result = validate_and_sanitize_args(&invalid, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Array too large"));
    }

    #[test]
    fn test_validation_object_keys() {
        let config = ValidationConfig { max_object_keys: 5, ..Default::default() };

        let mut valid_obj = serde_json::Map::new();
        for i in 0..5 {
            valid_obj.insert(format!("key{}", i), json!(i));
        }
        let valid = vec![json!(valid_obj)];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let mut invalid_obj = serde_json::Map::new();
        for i in 0..6 {
            invalid_obj.insert(format!("key{}", i), json!(i));
        }
        let invalid = vec![json!(invalid_obj)];
        let result = validate_and_sanitize_args(&invalid, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many object keys"));
    }

    #[test]
    fn test_validation_special_numbers() {
        let config = ValidationConfig { allow_special_numbers: false, ..Default::default() };

        let valid = vec![json!({"value": 42.5})];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let valid_negative = vec![json!({"value": -123.456})];
        assert!(validate_and_sanitize_args(&valid_negative, &config).is_ok());
    }

    #[test]
    fn test_validation_combined_limits() {
        let config = ValidationConfig {
            max_depth: 3,
            max_string_length: 50,
            max_array_length: 3,
            max_object_keys: 3,
            allow_special_numbers: false,
            max_total_elements: 100_000,
        };

        let valid = vec![json!({
            "user": {
                "name": "John",
                "tags": ["a", "b", "c"]
            }
        })];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let too_deep = vec![json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "fail"
                    }
                }
            }
        })];
        assert!(validate_and_sanitize_args(&too_deep, &config).is_err());

        let long_string = vec![json!({
            "text": "A".repeat(51)
        })];
        assert!(validate_and_sanitize_args(&long_string, &config).is_err());

        let large_array = vec![json!({
            "items": vec![1, 2, 3, 4]
        })];
        assert!(validate_and_sanitize_args(&large_array, &config).is_err());

        let many_keys = vec![json!({
            "key1": 1,
            "key2": 2,
            "key3": 3,
            "key4": 4
        })];
        assert!(validate_and_sanitize_args(&many_keys, &config).is_err());
    }

    #[test]
    fn test_validation_with_dangerous_properties() {
        let config = ValidationConfig::default();

        let args = vec![json!({
            "__proto__": {"isAdmin": true},
            "username": "test",
            "data": "A".repeat(100)
        })];

        let result = validate_and_sanitize_args(&args, &config).unwrap();
        let obj = result[0].as_object().unwrap();

        assert!(!obj.contains_key("__proto__"));
        assert_eq!(obj.get("username").unwrap().as_str().unwrap(), "test");
        assert_eq!(obj.get("data").unwrap().as_str().unwrap().len(), 100);
    }

    #[test]
    fn test_validation_nested_arrays() {
        let config = ValidationConfig { max_depth: 3, max_array_length: 2, ..Default::default() };

        let valid = vec![json!({
            "matrix": [
                [1, 2],
                [3, 4]
            ]
        })];
        assert!(validate_and_sanitize_args(&valid, &config).is_ok());

        let invalid = vec![json!({
            "matrix": [
                [1, 2, 3]
            ]
        })];
        assert!(validate_and_sanitize_args(&invalid, &config).is_err());
    }

    #[test]
    fn test_validation_preserves_types() {
        let config = ValidationConfig::default();

        let args = vec![
            json!(null),
            json!(true),
            json!(false),
            json!(42),
            json!(-123),
            json!(3.14),
            json!("string"),
            json!([1, 2, 3]),
            json!({"key": "value"}),
        ];

        let result = validate_and_sanitize_args(&args, &config).unwrap();

        assert!(result[0].is_null());
        assert_eq!(result[1].as_bool().unwrap(), true);
        assert_eq!(result[2].as_bool().unwrap(), false);
        assert_eq!(result[3].as_i64().unwrap(), 42);
        assert_eq!(result[4].as_i64().unwrap(), -123);
        assert_eq!(result[5].as_f64().unwrap(), 3.14);
        assert_eq!(result[6].as_str().unwrap(), "string");
        assert_eq!(result[7].as_array().unwrap().len(), 3);
        assert_eq!(result[8].as_object().unwrap().len(), 1);
    }

    #[test]
    fn test_validation_config_development() {
        let dev_config = ValidationConfig::development();

        assert_eq!(dev_config.max_depth, 20);
        assert_eq!(dev_config.max_string_length, 50_000);
        assert_eq!(dev_config.max_array_length, 5_000);
        assert_eq!(dev_config.max_object_keys, 500);
    }

    #[test]
    fn test_validation_config_production() {
        let prod_config = ValidationConfig::production();

        assert_eq!(prod_config.max_depth, 10);
        assert_eq!(prod_config.max_string_length, 10_000);
        assert_eq!(prod_config.max_array_length, 1_000);
        assert_eq!(prod_config.max_object_keys, 100);
    }

    #[test]
    fn test_validation_empty_structures() {
        let config = ValidationConfig::default();

        let empty_obj = vec![json!({})];
        assert!(validate_and_sanitize_args(&empty_obj, &config).is_ok());

        let empty_arr = vec![json!([])];
        assert!(validate_and_sanitize_args(&empty_arr, &config).is_ok());

        let empty_str = vec![json!({"text": ""})];
        assert!(validate_and_sanitize_args(&empty_str, &config).is_ok());
    }

    #[test]
    fn test_validation_realistic_payload() {
        let config = ValidationConfig::default();

        let args = vec![json!({
            "user": {
                "id": 123,
                "name": "John Doe",
                "email": "john@example.com",
                "roles": ["user", "admin"],
                "metadata": {
                    "lastLogin": "2025-12-09T14:00:00Z",
                    "preferences": {
                        "theme": "dark",
                        "notifications": true
                    }
                }
            },
            "action": "update",
            "timestamp": 1733756400
        })];

        let result = validate_and_sanitize_args(&args, &config);
        assert!(result.is_ok());

        let sanitized = result.unwrap();
        assert_eq!(sanitized.len(), 1);

        let obj = sanitized[0].as_object().unwrap();
        assert!(obj.contains_key("user"));
        assert!(obj.contains_key("action"));
        assert!(obj.contains_key("timestamp"));
    }

    #[test]
    fn test_redirect_relative_url_allowed() {
        let config =
            RedirectConfig { allowed_hosts: vec![], allow_relative: true, allow_subdomains: false };

        assert!(validate_redirect_url("/dashboard", &config).is_ok());
        assert!(validate_redirect_url("/users/123", &config).is_ok());
        assert!(validate_redirect_url("/", &config).is_ok());
    }

    #[test]
    fn test_redirect_relative_url_blocked_when_disabled() {
        let config = RedirectConfig {
            allowed_hosts: vec![],
            allow_relative: false,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("/dashboard", &config).is_err());
    }

    #[test]
    fn test_redirect_protocol_relative_blocked() {
        let config =
            RedirectConfig { allowed_hosts: vec![], allow_relative: true, allow_subdomains: false };

        assert!(validate_redirect_url("//evil.com/phishing", &config).is_err());
    }

    #[test]
    fn test_redirect_allowed_host() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("http://example.com/page", &config).is_ok());
    }

    #[test]
    fn test_redirect_blocked_host() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://evil.com/phishing", &config).is_err());
        assert!(validate_redirect_url("https://attacker.com", &config).is_err());
    }

    #[test]
    fn test_redirect_subdomain_when_allowed() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: true,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://www.example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://api.example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://sub.domain.example.com/page", &config).is_ok());
    }

    #[test]
    fn test_redirect_subdomain_when_blocked() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://www.example.com/page", &config).is_err());
        assert!(validate_redirect_url("https://api.example.com/page", &config).is_err());
    }

    #[test]
    fn test_redirect_invalid_scheme() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("javascript:alert(1)", &config).is_err());
        assert!(
            validate_redirect_url("data:text/html,<script>alert(1)</script>", &config).is_err()
        );
        assert!(validate_redirect_url("ftp://example.com/file", &config).is_err());
        assert!(validate_redirect_url("file:///etc/passwd", &config).is_err());
    }

    #[test]
    fn test_redirect_multiple_allowed_hosts() {
        let config = RedirectConfig {
            allowed_hosts: vec![
                "example.com".to_string(),
                "trusted.com".to_string(),
                "localhost".to_string(),
            ],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://trusted.com/page", &config).is_ok());
        assert!(validate_redirect_url("http://localhost:3000/page", &config).is_ok());
        assert!(validate_redirect_url("https://evil.com/page", &config).is_err());
    }

    #[test]
    fn test_redirect_invalid_url_format() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("not a url", &config).is_err());
        assert!(validate_redirect_url("ht!tp://example.com", &config).is_err());
    }

    #[test]
    fn test_redirect_with_port() {
        let config = RedirectConfig {
            allowed_hosts: vec!["localhost".to_string(), "example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("http://localhost:3000/page", &config).is_ok());
        assert!(validate_redirect_url("https://example.com:8443/page", &config).is_ok());
    }

    #[test]
    fn test_redirect_with_query_and_fragment() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page?foo=bar", &config).is_ok());
        assert!(validate_redirect_url("https://example.com/page#section", &config).is_ok());
        assert!(validate_redirect_url("https://example.com/page?foo=bar#section", &config).is_ok());
        assert!(validate_redirect_url("/page?foo=bar#section", &config).is_ok());
    }

    #[test]
    fn test_redirect_empty_allowed_hosts() {
        let config =
            RedirectConfig { allowed_hosts: vec![], allow_relative: true, allow_subdomains: false };

        assert!(validate_redirect_url("/page", &config).is_ok());

        assert!(validate_redirect_url("https://example.com/page", &config).is_err());
    }

    #[test]
    fn test_redirect_case_sensitivity() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://example.com/page", &config).is_ok());
        assert!(validate_redirect_url("https://EXAMPLE.COM/page", &config).is_ok());
        assert!(validate_redirect_url("https://Example.Com/page", &config).is_ok());

        assert!(validate_redirect_url("https://evil.com/page", &config).is_err());
    }

    #[test]
    fn test_redirect_homograph_attack_prevention() {
        let config = RedirectConfig {
            allowed_hosts: vec!["example.com".to_string()],
            allow_relative: true,
            allow_subdomains: false,
        };

        assert!(validate_redirect_url("https://examp1e.com/page", &config).is_err());
        assert!(validate_redirect_url("https://example.co/page", &config).is_err());
        assert!(validate_redirect_url("https://examplecom.com/page", &config).is_err());
    }

    #[test]
    fn test_is_reserved_export_name_then() {
        use crate::rsc::actions::is_reserved_export_name;

        assert!(is_reserved_export_name("then"));
        assert!(is_reserved_export_name("catch"));
        assert!(is_reserved_export_name("finally"));
    }

    #[test]
    fn test_is_reserved_export_name_object_methods() {
        use crate::rsc::actions::is_reserved_export_name;

        assert!(is_reserved_export_name("toString"));
        assert!(is_reserved_export_name("valueOf"));
        assert!(is_reserved_export_name("toLocaleString"));
        assert!(is_reserved_export_name("constructor"));
    }

    #[test]
    fn test_is_reserved_export_name_symbols() {
        use crate::rsc::actions::is_reserved_export_name;

        assert!(is_reserved_export_name("Symbol"));
        assert!(is_reserved_export_name("@@iterator"));
        assert!(is_reserved_export_name("@@toStringTag"));
    }

    #[test]
    fn test_is_reserved_export_name_allows_valid_names() {
        use crate::rsc::actions::is_reserved_export_name;

        assert!(!is_reserved_export_name("getData"));
        assert!(!is_reserved_export_name("submitForm"));
        assert!(!is_reserved_export_name("updateUser"));
        assert!(!is_reserved_export_name("deleteItem"));
        assert!(!is_reserved_export_name("GET"));
        assert!(!is_reserved_export_name("POST"));
        assert!(!is_reserved_export_name("myAction"));
    }

    #[test]
    fn test_is_reserved_export_name_case_sensitive() {
        use crate::rsc::actions::is_reserved_export_name;

        assert!(is_reserved_export_name("then"));
        assert!(!is_reserved_export_name("Then"));
        assert!(!is_reserved_export_name("THEN"));

        assert!(is_reserved_export_name("catch"));
        assert!(!is_reserved_export_name("Catch"));
    }

    #[test]
    fn test_is_reserved_export_name_similar_names() {
        use crate::rsc::actions::is_reserved_export_name;

        assert!(!is_reserved_export_name("thenDo"));
        assert!(!is_reserved_export_name("catchError"));
        assert!(!is_reserved_export_name("finallyDone"));
        assert!(!is_reserved_export_name("myThen"));
    }

    #[test]
    fn test_cve_2025_55182_wide_array_dos_attack() {
        let config = ValidationConfig {
            max_depth: 10,
            max_total_elements: 10_000,
            max_array_length: 1_000,
            ..Default::default()
        };

        let mut outer_array = Vec::new();
        for _ in 0..20 {
            outer_array.push(json!(vec![1; 600]));
        }
        let wide_nested = json!({ "data": outer_array });

        let result = validate_and_sanitize_args(&[wide_nested], &config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Maximum array nesting exceeded") || err_msg.contains("12000 > 10000"),
            "Expected array nesting error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_cve_2025_55182_string_accumulation_dos() {
        let config = ValidationConfig {
            max_depth: 10,
            max_total_elements: 50_000,
            max_string_length: 10_000,
            ..Default::default()
        };

        let strings: Vec<_> = (0..10).map(|_| json!("A".repeat(6_000))).collect();
        let many_strings = json!({ "strings": strings });

        let result = validate_and_sanitize_args(&[many_strings], &config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Maximum array nesting exceeded"),
            "Expected cumulative limit error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_cve_2025_55182_fork_detection() {
        let config = ValidationConfig {
            max_depth: 5,
            max_total_elements: 1_000,
            max_array_length: 500,
            ..Default::default()
        };

        let single_child = json!({ "data": [vec![1; 500]] });
        assert!(validate_and_sanitize_args(&[single_child], &config).is_ok());

        let forked = json!({ "data": [vec![1; 500], vec![2; 500]] });
        let result = validate_and_sanitize_args(&[forked], &config);

        assert!(result.is_err(), "Expected fork with >1000 elements to fail");
    }

    #[test]
    fn test_cve_2025_55182_production_limits() {
        let prod_config = ValidationConfig::production();

        assert_eq!(prod_config.max_total_elements, 1_000_000);
        assert_eq!(prod_config.max_depth, 10);
        assert_eq!(prod_config.max_array_length, 1_000);
        assert_eq!(prod_config.max_string_length, 10_000);
    }

    #[test]
    fn test_cve_2025_55182_development_limits() {
        let dev_config = ValidationConfig::development();

        assert_eq!(dev_config.max_total_elements, 5_000_000);
        assert_eq!(dev_config.max_depth, 20);
        assert_eq!(dev_config.max_array_length, 5_000);
        assert_eq!(dev_config.max_string_length, 50_000);
    }

    #[test]
    fn test_cookie_value_rejects_double_quote() {
        let cookie = PendingCookie {
            name: "session".to_string(),
            value: "value\"with\"quotes".to_string(),
            path: Some("/".to_string()),
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
            priority: None,
            partitioned: false,
        };

        let result = crate::rsc::actions::build_set_cookie_header(&cookie);
        assert!(result.is_err(), "Cookie value with double-quote should be rejected");
    }

    #[test]
    fn test_cookie_value_rejects_backslash() {
        let cookie = PendingCookie {
            name: "session".to_string(),
            value: "value\\with\\backslash".to_string(),
            path: Some("/".to_string()),
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
            priority: None,
            partitioned: false,
        };

        let result = crate::rsc::actions::build_set_cookie_header(&cookie);
        assert!(result.is_err(), "Cookie value with backslash should be rejected");
    }

    #[test]
    fn test_cookie_value_rejects_space() {
        let cookie = PendingCookie {
            name: "session".to_string(),
            value: "value with spaces".to_string(),
            path: Some("/".to_string()),
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
            priority: None,
            partitioned: false,
        };

        let result = crate::rsc::actions::build_set_cookie_header(&cookie);
        assert!(result.is_err(), "Cookie value with space should be rejected");
    }

    #[test]
    fn test_cookie_value_accepts_valid_characters() {
        let cookie = PendingCookie {
            name: "session".to_string(),
            value: "abc123-_~!#$%&'()*+./:<=>?@[]^`{|}".to_string(),
            path: Some("/".to_string()),
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
            priority: None,
            partitioned: false,
        };

        let result = crate::rsc::actions::build_set_cookie_header(&cookie);
        assert!(result.is_ok(), "Cookie value with valid RFC 6265 characters should be accepted");
    }

    #[test]
    fn test_cookie_value_rejects_control_characters() {
        let cookie = PendingCookie {
            name: "session".to_string(),
            value: "value\x00with\x1Fcontrol".to_string(),
            path: Some("/".to_string()),
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
            priority: None,
            partitioned: false,
        };

        let result = crate::rsc::actions::build_set_cookie_header(&cookie);
        assert!(result.is_err(), "Cookie value with control characters should be rejected");
    }

    #[test]
    fn test_cookie_value_rejects_del_character() {
        let cookie = PendingCookie {
            name: "session".to_string(),
            value: "value\x7Fwith_del".to_string(),
            path: Some("/".to_string()),
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
            priority: None,
            partitioned: false,
        };

        let result = crate::rsc::actions::build_set_cookie_header(&cookie);
        assert!(result.is_err(), "Cookie value with DEL character (0x7F) should be rejected");
    }

    #[test]
    fn test_cookie_value_accepts_exclamation_mark() {
        let cookie = PendingCookie {
            name: "session".to_string(),
            value: "value!".to_string(),
            path: Some("/".to_string()),
            domain: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
            priority: None,
            partitioned: false,
        };

        let result = crate::rsc::actions::build_set_cookie_header(&cookie);
        assert!(result.is_ok(), "Cookie value with exclamation mark (0x21) should be accepted");
    }

    #[test]
    fn test_origin_comparison_with_default_https_port() {
        use axum::http::HeaderMap;

        use crate::rsc::actions::check_origin;

        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("origin", "https://example.com:443".parse().unwrap());

        let result = check_origin(&headers, &[]);
        assert!(
            result.is_ok(),
            "Origin with explicit default HTTPS port (443) should match server origin without port"
        );
    }

    #[test]
    fn test_origin_comparison_with_default_http_port() {
        use axum::http::HeaderMap;

        use crate::rsc::actions::check_origin;

        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com".parse().unwrap());
        headers.insert("x-forwarded-proto", "http".parse().unwrap());
        headers.insert("origin", "http://example.com:80".parse().unwrap());

        let result = check_origin(&headers, &[]);
        assert!(
            result.is_ok(),
            "Origin with explicit default HTTP port (80) should match server origin without port"
        );
    }

    #[test]
    fn test_origin_comparison_with_explicit_port_in_host() {
        use axum::http::HeaderMap;

        use crate::rsc::actions::check_origin;

        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com:8080".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("origin", "https://example.com:8080".parse().unwrap());

        let result = check_origin(&headers, &[]);
        assert!(
            result.is_ok(),
            "Origin with explicit non-default port should match server origin with same port"
        );
    }

    #[test]
    fn test_origin_comparison_port_mismatch() {
        use axum::http::HeaderMap;

        use crate::rsc::actions::check_origin;

        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com:8080".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("origin", "https://example.com:9090".parse().unwrap());

        let result = check_origin(&headers, &[]);
        assert!(result.is_err(), "Origin with different port should not match");
    }

    #[test]
    fn test_referer_comparison_with_default_port() {
        use axum::http::HeaderMap;

        use crate::rsc::actions::check_origin;

        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers.insert("referer", "https://example.com:443/some/path".parse().unwrap());

        let result = check_origin(&headers, &[]);
        assert!(
            result.is_ok(),
            "Referer with explicit default HTTPS port (443) should match server origin without port"
        );
    }
}
