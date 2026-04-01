use crate::error::RariError;
use crate::server::ServerState;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use cow_utils::CowUtils;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::error;

#[cfg(test)]
mod tests;

const MAX_BOUND_ARGS: usize = 1000;
const MAX_BIGINT_DIGITS: usize = 300;

#[derive(Debug, Clone)]
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
pub struct ServerActionRequest {
    pub id: String,
    pub export_name: String,
    pub args: Vec<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct ServerActionResponse {
    pub success: bool,
    pub result: Option<JsonValue>,
    pub error: Option<String>,
    pub redirect: Option<String>,
}

fn check_origin(headers: &HeaderMap, allowed_origins: &[String]) -> Result<(), StatusCode> {
    if allowed_origins.is_empty() {
        let host = headers.get("host").and_then(|v| v.to_str().ok()).ok_or_else(|| {
            error!("Missing host header in server action request");
            StatusCode::BAD_REQUEST
        })?;

        if let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) {
            if let Ok(origin_url) = url::Url::parse(origin) {
                let origin_host = if let Some(port) = origin_url.port() {
                    format!("{}:{}", origin_url.host_str().unwrap_or(""), port)
                } else {
                    origin_url.host_str().unwrap_or("").to_string()
                };

                if origin_host == host {
                    return Ok(());
                }
            }
            error!("Origin mismatch: origin={}, host={}", origin, host);
            return Err(StatusCode::FORBIDDEN);
        }

        if let Some(referer) = headers.get("referer").and_then(|v| v.to_str().ok()) {
            if let Ok(referer_url) = url::Url::parse(referer) {
                let referer_host = if let Some(port) = referer_url.port() {
                    format!("{}:{}", referer_url.host_str().unwrap_or(""), port)
                } else {
                    referer_url.host_str().unwrap_or("").to_string()
                };

                if referer_host == host {
                    return Ok(());
                }
                error!("Referer mismatch: referer={}, host={}", referer, host);
            } else {
                error!("Invalid referer header: failed to parse");
            }
            return Err(StatusCode::FORBIDDEN);
        }

        error!("Missing origin and referer headers in server action request");
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) {
        if !crate::server::utils::http_utils::is_origin_allowed(origin, allowed_origins) {
            error!("Invalid origin: {}", origin);
            return Err(StatusCode::FORBIDDEN);
        }
        return Ok(());
    }

    if let Some(referer) = headers.get("referer").and_then(|v| v.to_str().ok()) {
        if let Ok(referer_url) = url::Url::parse(referer) {
            let referer_origin =
                format!("{}://{}", referer_url.scheme(), referer_url.host_str().unwrap_or(""));
            let referer_origin = if let Some(port) = referer_url.port() {
                format!("{}:{}", referer_origin, port)
            } else {
                referer_origin
            };
            if crate::server::utils::http_utils::is_origin_allowed(&referer_origin, allowed_origins)
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
                error: Some(format!("Input validation failed: {}", e)),
                redirect: None,
            })
            .into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                "no-store, no-cache, must-revalidate, private"
                    .parse()
                    .expect("Valid cache-control header"),
            );
            *response.status_mut() = StatusCode::BAD_REQUEST;
            return Ok(response);
        }
    };

    let cookie_header =
        headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).map(|s| s.to_string());

    let request_context = std::sync::Arc::new(
        crate::server::middleware::request_context::RequestContext::new(
            "/_rari/action".to_string(),
        )
        .with_cookies(cookie_header),
    );

    let renderer = state.renderer.lock().await;

    if let Err(e) = renderer.runtime.set_request_context(request_context.clone()).await {
        error!("Failed to set request context for server action: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let result =
        renderer.execute_server_function(&request.id, &request.export_name, &sanitized_args).await;

    if let Err(e) = renderer.runtime.clear_request_context().await {
        error!("Failed to clear request context for server action: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

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
            }

            let response =
                ServerActionResponse { success: true, result: Some(value), error: None, redirect };

            let mut response = Json(response).into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
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
        headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).map(|s| s.to_string());

    let request_context = std::sync::Arc::new(
        crate::server::middleware::request_context::RequestContext::new(
            "/_rari/action".to_string(),
        )
        .with_cookies(cookie_header),
    );

    let renderer = state.renderer.lock().await;

    if let Err(e) = renderer.runtime.set_request_context(request_context.clone()).await {
        error!("Failed to set request context for form action: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let result = renderer.execute_server_function(action_id, export_name, &sanitized_args).await;

    if let Err(e) = renderer.runtime.clear_request_context().await {
        error!("Failed to clear request context for form action: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

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

            let allowed_origins = state.config.action_origins();
            let (redirect_url, redirect_path_opt) =
                if let Some(referer) = headers.get("referer").and_then(|h| h.to_str().ok()) {
                    if let Ok(parsed) = url::Url::parse(referer) {
                        let referer_origin =
                            format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
                        let referer_origin = if let Some(port) = parsed.port() {
                            format!("{}:{}", referer_origin, port)
                        } else {
                            referer_origin
                        };

                        if crate::server::utils::http_utils::is_origin_allowed(
                            &referer_origin,
                            &allowed_origins,
                        ) {
                            let path_and_query = if let Some(query) = parsed.query() {
                                format!("{}?{}", parsed.path(), query)
                            } else {
                                parsed.path().to_string()
                            };
                            (path_and_query.clone(), Some(parsed.path().to_string()))
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

pub(crate) fn validate_redirect_url(
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
                host == allowed || host.ends_with(&format!(".{}", allowed))
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
    let mut result = String::new();
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex1 =
                chars.next().ok_or_else(|| RariError::bad_request("Invalid percent encoding"))?;
            let hex2 =
                chars.next().ok_or_else(|| RariError::bad_request("Invalid percent encoding"))?;

            let hex_str = format!("{}{}", hex1, hex2);
            let byte = u8::from_str_radix(&hex_str, 16)
                .map_err(|_| RariError::bad_request("Invalid hex in percent encoding"))?;

            result.push(byte as char);
        } else if ch == '+' {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

pub(crate) fn validate_and_sanitize_args(
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
                            "Number too large. Estimated {} digits but the limit is {}.",
                            estimated_digits, MAX_BIGINT_DIGITS
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

pub(crate) fn is_dangerous_property(key: &str) -> bool {
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

pub(crate) fn is_reserved_export_name(name: &str) -> bool {
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
    for entry in pending_cookies.iter() {
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
            Err(()) => {
                tracing::warn!("Skipped invalid cookie '{}': failed validation", cookie.name);
            }
        }
    }
}

pub(crate) fn build_set_cookie_header(
    cookie: &crate::server::middleware::request_context::PendingCookie,
) -> Result<String, ()> {
    fn is_valid_cookie_name(s: &str) -> bool {
        !s.is_empty()
            && s.bytes().all(|b| b > 32 && b < 127 && !b"()<>@,;:\\\"/[]?={} \t".contains(&b))
    }

    fn is_valid_cookie_value(s: &str) -> bool {
        s.bytes().all(|b| matches!(b, 0x21 | 0x23..=0x2B | 0x2D..=0x3A | 0x3C..=0x5B | 0x5D..=0x7E))
    }

    fn is_valid_attr_value(s: &str) -> bool {
        !s.is_empty() && s.is_ascii() && s.bytes().all(|b| b >= 32 && b != b';' && b != 127)
    }

    if !is_valid_cookie_name(&cookie.name) {
        return Err(());
    }
    if !is_valid_cookie_value(&cookie.value) {
        return Err(());
    }

    let path = cookie.path.as_deref().unwrap_or("/");
    if !is_valid_attr_value(path) {
        return Err(());
    }

    let mut header = format!("{}={}", cookie.name, cookie.value);
    header.push_str(&format!("; Path={}", path));

    if let Some(domain) = &cookie.domain {
        if !is_valid_attr_value(domain) {
            return Err(());
        }
        header.push_str(&format!("; Domain={}", domain));
    }
    if let Some(expires) = &cookie.expires {
        if !is_valid_attr_value(expires) {
            return Err(());
        }
        header.push_str(&format!("; Expires={}", expires));
    }
    if let Some(max_age) = cookie.max_age {
        header.push_str(&format!("; Max-Age={}", max_age));
    }
    let normalized_same_site =
        cookie.same_site.as_deref().map(|value| value.cow_to_ascii_lowercase());
    if normalized_same_site.as_deref() == Some("none") && !cookie.secure {
        return Err(());
    }
    if cookie.partitioned && !cookie.secure {
        return Err(());
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
            _ => return Err(()),
        };
        header.push_str(&format!("; SameSite={}", serialized_same_site));
    }
    if let Some(priority) = &cookie.priority {
        match priority.cow_to_ascii_lowercase().as_ref() {
            "low" => header.push_str("; Priority=Low"),
            "medium" => header.push_str("; Priority=Medium"),
            "high" => header.push_str("; Priority=High"),
            _ => return Err(()),
        }
    }
    if cookie.partitioned {
        header.push_str("; Partitioned");
    }
    Ok(header)
}
