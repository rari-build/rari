#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{fmt::Write, str, sync::Arc};

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use cow_utils::CowUtils;
use rari_error::RariError;
use serde::Serialize;
use serde_json::Value;

use crate::{
    rendering::base::constants::ACTION_HANDLER_SCRIPT,
    server::{
        ServerState,
        cache::revalidate::invalidate_route_caches,
        config::RedirectConfig,
        core::utils::http::{extract_headers, is_origin_allowed},
        middleware::request_context::{PendingCookie, PendingCookieKey, RequestContext},
    },
};

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct ServerActionResponse {
    pub success: bool,
    pub result: Option<Value>,
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
            tracing::error!("Missing host header in server action request");
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
            tracing::error!("Failed to parse server origin: {}", e);
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
            tracing::error!(
                "Origin mismatch: origin={}, server_origin={}",
                origin,
                server_origin_str
            );
            return Err(StatusCode::FORBIDDEN);
        }

        if let Some(referer) = headers.get("referer").and_then(|v| v.to_str().ok()) {
            if let Ok(referer_url) = url::Url::parse(referer) {
                let referer_tuple = normalize_origin(&referer_url);

                if referer_tuple == server_origin_tuple {
                    return Ok(());
                }
                tracing::error!(
                    "Referer mismatch: referer_origin={}://{}:{}, server_origin={}",
                    referer_tuple.0,
                    referer_tuple.1,
                    referer_tuple.2,
                    server_origin_str
                );
            } else {
                tracing::error!("Invalid referer header: failed to parse");
            }
            return Err(StatusCode::FORBIDDEN);
        }

        tracing::error!("Missing origin and referer headers in server action request");
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) {
        if !is_origin_allowed(origin, allowed_origins) {
            tracing::error!("Invalid origin: {}", origin);
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
            if is_origin_allowed(&referer_origin, allowed_origins) {
                return Ok(());
            }
            tracing::error!("Invalid referer origin: {}", referer_origin);
        } else {
            tracing::error!("Invalid referer header: failed to parse origin");
        }
        return Err(StatusCode::FORBIDDEN);
    }

    tracing::error!("Missing Origin and Referer headers with non-empty allowed_origins");
    Err(StatusCode::FORBIDDEN)
}

fn build_official_action_script(action_id: &str, body_text: &str) -> Result<String, RariError> {
    let action_id_json = serde_json::to_string(action_id)
        .map_err(|e| RariError::serialization(format!("Failed to serialize action id: {e}")))?;
    let body_text_json = serde_json::to_string(body_text)
        .map_err(|e| RariError::serialization(format!("Failed to serialize action body: {e}")))?;

    Ok(ACTION_HANDLER_SCRIPT
        .cow_replace("__RARI_ACTION_MODE__", "\"reply\"")
        .cow_replace("__RARI_ACTION_ID__", &action_id_json)
        .cow_replace("__RARI_ACTION_BODY__", &body_text_json)
        .cow_replace("__RARI_ACTION_BODY_B64__", "\"\"")
        .cow_replace("__RARI_ACTION_CONTENT_TYPE__", "\"\"")
        .cow_replace("__RARI_ACTION_FORM_ENTRIES__", "[]")
        .into_owned())
}

fn build_multipart_action_script(
    action_id: &str,
    body: &[u8],
    content_type: &str,
) -> Result<String, RariError> {
    let action_id_json = serde_json::to_string(action_id)
        .map_err(|e| RariError::serialization(format!("Failed to serialize action id: {e}")))?;
    let body_b64_json = serde_json::to_string(&BASE64_STANDARD.encode(body))
        .map_err(|e| RariError::serialization(format!("Failed to serialize action body: {e}")))?;
    let content_type_json = serde_json::to_string(content_type)
        .map_err(|e| RariError::serialization(format!("Failed to serialize content type: {e}")))?;

    Ok(ACTION_HANDLER_SCRIPT
        .cow_replace("__RARI_ACTION_MODE__", "\"reply-multipart\"")
        .cow_replace("__RARI_ACTION_ID__", &action_id_json)
        .cow_replace("__RARI_ACTION_BODY__", "\"\"")
        .cow_replace("__RARI_ACTION_BODY_B64__", &body_b64_json)
        .cow_replace("__RARI_ACTION_CONTENT_TYPE__", &content_type_json)
        .cow_replace("__RARI_ACTION_FORM_ENTRIES__", "[]")
        .into_owned())
}

fn build_form_action_script(body: &[u8], content_type: &str) -> Result<String, RariError> {
    let body_b64_json = serde_json::to_string(&BASE64_STANDARD.encode(body))
        .map_err(|e| RariError::serialization(format!("Failed to serialize action body: {e}")))?;
    let content_type_json = serde_json::to_string(content_type)
        .map_err(|e| RariError::serialization(format!("Failed to serialize content type: {e}")))?;

    Ok(ACTION_HANDLER_SCRIPT
        .cow_replace("__RARI_ACTION_MODE__", "\"form\"")
        .cow_replace("__RARI_ACTION_ID__", "\"\"")
        .cow_replace("__RARI_ACTION_BODY__", "\"\"")
        .cow_replace("__RARI_ACTION_BODY_B64__", &body_b64_json)
        .cow_replace("__RARI_ACTION_CONTENT_TYPE__", &content_type_json)
        .cow_replace("__RARI_ACTION_FORM_ENTRIES__", "[]")
        .into_owned())
}

fn is_form_content_type(content_type: &str) -> bool {
    content_type.starts_with("multipart/form-data")
        || content_type.starts_with("application/x-www-form-urlencoded")
}

fn action_script_name(action_id: Option<&str>) -> String {
    match action_id {
        Some(action_id) => {
            format!("official_action_{}.ts", action_id.cow_replace('/', "_").cow_replace('#', "_"))
        }
        None => "official_action_form.ts".to_string(),
    }
}

fn build_action_script(
    action_id: Option<&str>,
    content_type: &str,
    body: &[u8],
) -> Result<String, RariError> {
    match action_id {
        Some(action_id) if content_type.starts_with("multipart/form-data") => {
            build_multipart_action_script(action_id, body, content_type)
        }
        Some(action_id) => {
            let body_text = str::from_utf8(body).map_err(|_| {
                RariError::bad_request("Server action body is not valid UTF-8".to_string())
            })?;
            build_official_action_script(action_id, body_text)
        }
        None if is_form_content_type(content_type) => build_form_action_script(body, content_type),
        None => Err(RariError::bad_request(
            "Missing rsc-action-id header for non-form server action request".to_string(),
        )),
    }
}

fn redirect_target_path(redirect_url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(redirect_url) {
        parsed.path().to_string()
    } else if redirect_url.starts_with('/') {
        redirect_url.split('?').next().unwrap_or(redirect_url).to_string()
    } else {
        redirect_url.to_string()
    }
}

async fn invalidate_redirect_target_caches(state: &ServerState, redirect_url: &str) {
    let redirect_path = redirect_target_path(redirect_url);
    if let Err(e) = invalidate_route_caches(state, &redirect_path).await {
        tracing::warn!(
            error = %e,
            path = %redirect_path,
            "route cache invalidation failed after server action redirect"
        );
    }
}

fn document_form_redirect_response(
    redirect_url: &str,
    pending_cookies: &dashmap::DashMap<PendingCookieKey, PendingCookie>,
) -> Response {
    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    let mut response = Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, redirect_url)
        .header(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, private")
        .body(Body::empty())
        .expect("Valid redirect response");
    append_pending_cookies(response.headers_mut(), pending_cookies);
    response
}

fn action_export_name(action_id: &str) -> &str {
    action_id.rsplit_once('#').map_or("default", |(_, export_name)| export_name)
}

pub async fn handle_server_action(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, StatusCode> {
    let allowed_origins = state.config.action_origins();
    check_origin(&headers, &allowed_origins)?;

    let action_id = headers
        .get("rsc-action-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty());

    let is_document_form_post = action_id.is_none();

    if let Some(action_id) = action_id {
        let export_name = action_export_name(action_id);
        if is_reserved_export_name(export_name) {
            tracing::error!("Attempted to call reserved export name: {}", export_name);
            let mut response = Json(ServerActionResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "Invalid export name '{export_name}': reserved for internal use"
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
    }

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("text/plain;charset=UTF-8");

    let request_context = Arc::new(
        RequestContext::new("/_rari/action".to_string())
            .with_http_headers(extract_headers(&headers)),
    );

    let runtime = {
        let renderer = state.renderer.lock().await;
        Arc::clone(&renderer.runtime)
    };

    let script = build_action_script(action_id, content_type, &body).map_err(|e| {
        tracing::error!("Failed to build action script: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let script_name = action_script_name(action_id);

    let value = match runtime
        .execute_script_with_request_context(Arc::clone(&request_context), script_name, script)
        .await
    {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Official server action execution failed: {}", e);
            if is_document_form_post {
                #[expect(
                    clippy::expect_used,
                    reason = "Response::builder() with valid components never fails"
                )]
                let mut response = Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, private")
                    .body(Body::from(e.to_string()))
                    .expect("Valid error response");
                append_pending_cookies(response.headers_mut(), &request_context.pending_cookies);
                return Ok(response);
            }

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
            return Ok(response);
        }
    };

    let redirect_config = state.config.redirect_config();
    let redirect = extract_redirect_from_result(&value, &redirect_config);

    if let Some(ref redirect_url) = redirect {
        invalidate_redirect_target_caches(&state, redirect_url).await;
    }

    if is_document_form_post {
        if is_failed_action_result(&value) {
            let error_message = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Server action failed")
                .to_owned();
            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            let mut response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, private")
                .body(Body::from(error_message))
                .expect("Valid error response");
            append_pending_cookies(response.headers_mut(), &request_context.pending_cookies);
            return Ok(response);
        }

        if let Some(redirect_url) = redirect {
            return Ok(document_form_redirect_response(
                &redirect_url,
                &request_context.pending_cookies,
            ));
        }

        if let Some(referer) = headers.get(header::REFERER).and_then(|value| value.to_str().ok()) {
            invalidate_redirect_target_caches(&state, referer).await;
            return Ok(document_form_redirect_response(referer, &request_context.pending_cookies));
        }

        #[expect(
            clippy::expect_used,
            reason = "Response::builder() with valid components never fails"
        )]
        let mut response = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, private")
            .body(Body::from("Missing Referer header for document form action"))
            .expect("Valid error response");
        append_pending_cookies(response.headers_mut(), &request_context.pending_cookies);
        return Ok(response);
    }

    let response =
        ServerActionResponse { success: true, result: Some(value), error: None, redirect };

    let mut response = Json(response).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
        "no-store, no-cache, must-revalidate, private".parse().expect("Valid cache-control header"),
    );

    append_pending_cookies(response.headers_mut(), &request_context.pending_cookies);

    Ok(response)
}

pub fn validate_redirect_url(url: &str, config: &RedirectConfig) -> Result<String, RariError> {
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

fn is_failed_action_result(result: &Value) -> bool {
    result.get("~promiseError").and_then(Value::as_bool) == Some(true)
        || result.get("~timeoutError").is_some()
        || result.get("success").and_then(Value::as_bool) == Some(false)
}

fn extract_redirect_from_result(result: &Value, config: &RedirectConfig) -> Option<String> {
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
    headers: &mut HeaderMap,
    pending_cookies: &dashmap::DashMap<PendingCookieKey, PendingCookie>,
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

pub fn build_set_cookie_header(cookie: &PendingCookie) -> Result<String, String> {
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
#[expect(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::server::{config::RedirectConfig, middleware::request_context::PendingCookie};

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
        use super::is_reserved_export_name;

        assert!(is_reserved_export_name("then"));
        assert!(is_reserved_export_name("catch"));
        assert!(is_reserved_export_name("finally"));
    }

    #[test]
    fn test_is_reserved_export_name_object_methods() {
        use super::is_reserved_export_name;

        assert!(is_reserved_export_name("toString"));
        assert!(is_reserved_export_name("valueOf"));
        assert!(is_reserved_export_name("toLocaleString"));
        assert!(is_reserved_export_name("constructor"));
    }

    #[test]
    fn test_is_reserved_export_name_symbols() {
        use super::is_reserved_export_name;

        assert!(is_reserved_export_name("Symbol"));
        assert!(is_reserved_export_name("@@iterator"));
        assert!(is_reserved_export_name("@@toStringTag"));
    }

    #[test]
    fn test_is_reserved_export_name_allows_valid_names() {
        use super::is_reserved_export_name;

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
        use super::is_reserved_export_name;

        assert!(is_reserved_export_name("then"));
        assert!(!is_reserved_export_name("Then"));
        assert!(!is_reserved_export_name("THEN"));

        assert!(is_reserved_export_name("catch"));
        assert!(!is_reserved_export_name("Catch"));
    }

    #[test]
    fn test_is_reserved_export_name_similar_names() {
        use super::is_reserved_export_name;

        assert!(!is_reserved_export_name("thenDo"));
        assert!(!is_reserved_export_name("catchError"));
        assert!(!is_reserved_export_name("finallyDone"));
        assert!(!is_reserved_export_name("myThen"));
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

        let result = super::build_set_cookie_header(&cookie);
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

        let result = super::build_set_cookie_header(&cookie);
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

        let result = super::build_set_cookie_header(&cookie);
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

        let result = super::build_set_cookie_header(&cookie);
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

        let result = super::build_set_cookie_header(&cookie);
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

        let result = super::build_set_cookie_header(&cookie);
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

        let result = super::build_set_cookie_header(&cookie);
        assert!(result.is_ok(), "Cookie value with exclamation mark (0x21) should be accepted");
    }

    #[test]
    fn test_origin_comparison_with_default_https_port() {
        use axum::http::HeaderMap;

        use super::check_origin;

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

        use super::check_origin;

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

        use super::check_origin;

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

        use super::check_origin;

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

        use super::check_origin;

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

    #[test]
    fn test_is_failed_action_result() {
        assert!(is_failed_action_result(&serde_json::json!({ "success": false })));
        assert!(is_failed_action_result(&serde_json::json!({ "~promiseError": true })));
        assert!(is_failed_action_result(&serde_json::json!({ "~timeoutError": "timed out" })));
        assert!(!is_failed_action_result(&serde_json::json!({ "success": true })));
        assert!(!is_failed_action_result(&serde_json::json!({ "result": "ok" })));
    }

    #[test]
    fn test_build_action_script_reply_mode() {
        use super::build_action_script;

        let script = build_action_script(
            Some("actions/foo#bar"),
            "text/plain;charset=UTF-8",
            b"[\"hello\"]",
        )
        .expect("script");

        assert!(script.contains("\"reply\""));
        assert!(script.contains("decodeReply"));
    }

    #[test]
    fn test_build_form_action_script_mode() {
        use super::build_form_action_script;

        let script = build_form_action_script(b"--test\r\n", "multipart/form-data; boundary=test")
            .expect("script");

        assert!(script.contains("\"form\""));
        assert!(script.contains("decodeAction"));
    }

    #[test]
    fn test_build_action_script_requires_form_without_action_id() {
        use super::build_action_script;

        let err = build_action_script(None, "text/plain", b"{}").expect_err("error");
        assert!(err.to_string().contains("rsc-action-id"));
    }
}
