use axum::{
    extract::MatchedPath,
    http::{HeaderValue, Request, Response},
    middleware::Next,
};
use std::time::Instant;
use tracing::{Instrument, info, warn};
use uuid::Uuid;

const ACCESS_CONTROL_ALLOW_ORIGIN: &str = "Access-Control-Allow-Origin";
const ACCESS_CONTROL_ALLOW_METHODS: &str = "Access-Control-Allow-Methods";
const ACCESS_CONTROL_ALLOW_HEADERS: &str = "Access-Control-Allow-Headers";
const ACCESS_CONTROL_MAX_AGE: &str = "Access-Control-Max-Age";
const X_CONTENT_TYPE_OPTIONS: &str = "X-Content-Type-Options";
const X_FRAME_OPTIONS: &str = "X-Frame-Options";
const X_XSS_PROTECTION: &str = "X-XSS-Protection";
const STRICT_TRANSPORT_SECURITY: &str = "Strict-Transport-Security";
const CONTENT_SECURITY_POLICY: &str = "Content-Security-Policy";
const ALLOW_ALL_ORIGINS: &str = "*";
const ALLOWED_METHODS: &str = "GET, POST, PUT, DELETE, OPTIONS";
const ALLOWED_HEADERS: &str =
    "Content-Type, Authorization, Accept, Origin, X-Requested-With, Cache-Control, X-RSC-Streaming";
const MAX_AGE_SECONDS: &str = "86400";
const NOSNIFF: &str = "nosniff";
const FRAME_DENY: &str = "DENY";
const XSS_PROTECTION: &str = "1; mode=block";
const HSTS_HEADER: &str = "max-age=31536000; includeSubDomains";
const CSP_POLICY: &str = "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' ws: wss:";

pub async fn request_logger(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4();

    let method = request.method().clone();
    let uri = request.uri().clone();

    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched_path| matched_path.as_str().to_owned())
        .unwrap_or_else(|| uri.path().to_owned());

    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        request_id = %request_id,
    );

    async move {
        info!(
            target: "rari::http",
            method = %method,
            path = %path,
            request_id = %request_id,
            "Request started"
        );

        let response = next.run(request).await;
        let duration = start_time.elapsed();

        let status = response.status();
        let status_code = status.as_u16();

        log_request_completion(method, path, status_code, duration, request_id);

        response
    }
    .instrument(span)
    .await
}

fn log_request_completion(
    method: axum::http::Method,
    path: String,
    status_code: u16,
    duration: std::time::Duration,
    request_id: Uuid,
) {
    let duration_ms = duration.as_millis();

    match status_code {
        200..=299 => {
            info!(
                target: "rari::http",
                method = %method,
                path = %path,
                status = status_code,
                duration_ms = duration_ms,
                request_id = %request_id,
                "Request completed successfully"
            );
        }
        300..=399 => {
            info!(
                target: "rari::http",
                method = %method,
                path = %path,
                status = status_code,
                duration_ms = duration_ms,
                request_id = %request_id,
                "Request completed with redirection"
            );
        }
        400..=499 => {
            warn!(
                target: "rari::http",
                method = %method,
                path = %path,
                status = status_code,
                duration_ms = duration_ms,
                request_id = %request_id,
                "Request completed with client error"
            );
        }
        500..=599 => {
            warn!(
                target: "rari::http",
                method = %method,
                path = %path,
                status = status_code,
                duration_ms = duration_ms,
                request_id = %request_id,
                "Request completed with server error"
            );
        }
        _ => {
            info!(
                target: "rari::http",
                method = %method,
                path = %path,
                status = status_code,
                duration_ms = duration_ms,
                request_id = %request_id,
                "Request completed"
            );
        }
    }
}

pub async fn minimal_request_logger(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let start_time = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    let response = next.run(request).await;
    let duration = start_time.elapsed();
    let status = response.status().as_u16();

    info!(
        target: "rari::http",
        "{} {} {} - {}ms",
        method,
        path,
        status,
        duration.as_millis()
    );

    response
}

pub async fn cors_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    add_cors_headers(headers);

    response
}

fn add_cors_headers(headers: &mut axum::http::HeaderMap) {
    let cors_headers = [
        (ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ALL_ORIGINS),
        (ACCESS_CONTROL_ALLOW_METHODS, ALLOWED_METHODS),
        (ACCESS_CONTROL_ALLOW_HEADERS, ALLOWED_HEADERS),
        (ACCESS_CONTROL_MAX_AGE, MAX_AGE_SECONDS),
    ];

    for (header_name, header_value) in cors_headers {
        if let Ok(value) = HeaderValue::from_str(header_value) {
            headers.insert(header_name, value);
        }
    }
}

pub async fn security_headers_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    add_security_headers(headers);

    response
}

fn add_security_headers(headers: &mut axum::http::HeaderMap) {
    let security_headers = [
        (X_CONTENT_TYPE_OPTIONS, NOSNIFF),
        (X_FRAME_OPTIONS, FRAME_DENY),
        (X_XSS_PROTECTION, XSS_PROTECTION),
        (STRICT_TRANSPORT_SECURITY, HSTS_HEADER),
        (CONTENT_SECURITY_POLICY, CSP_POLICY),
    ];

    for (header_name, header_value) in security_headers {
        if let Ok(value) = HeaderValue::from_str(header_value) {
            headers.insert(header_name, value);
        }
    }
}

pub async fn html_optimization_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let response = next.run(request).await;

    if let Some(content_type) = response.headers().get("content-type")
        && let Ok(content_type_str) = content_type.to_str()
        && content_type_str.contains("text/html")
    {
        return optimize_html_response(response).await;
    }

    response
}

async fn optimize_html_response(
    response: Response<axum::body::Body>,
) -> Response<axum::body::Body> {
    let (parts, body) = response.into_parts();

    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => return Response::from_parts(parts, axum::body::Body::empty()),
    };

    let html_str = match String::from_utf8(body_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return Response::from_parts(parts, axum::body::Body::from(body_bytes)),
    };

    let optimized_html = apply_html_optimizations(html_str);

    let mut response = Response::from_parts(parts, axum::body::Body::from(optimized_html));

    add_performance_headers(response.headers_mut());

    response
}

fn apply_html_optimizations(html: String) -> String {
    let mut optimized = html;

    optimized = optimize_css_loading(optimized);

    optimized = add_preconnect_hints(optimized);

    optimized
}

fn optimize_css_loading(html: String) -> String {
    if let Some(start) = html.find(r#"<link rel="stylesheet" crossorigin href="/assets/main-"#)
        && let Some(end) = html[start..].find(">")
    {
        let full_end = start + end + 1;
        let css_link = &html[start..full_end];

        if let Some(href_start) = css_link.find("href=\"")
            && let Some(href_end) = css_link[href_start + 6..].find("\"")
        {
            let href = &css_link[href_start + 6..href_start + 6 + href_end];

            let preload_link = format!(
                r#"<link rel="preload" as="style" crossorigin href="{href}" onload="this.onload=null;this.rel='stylesheet'">
  <noscript><link rel="stylesheet" crossorigin href="{href}"></noscript>"#
            );

            return html.replace(css_link, &preload_link);
        }
    }

    html
}

fn add_preconnect_hints(html: String) -> String {
    let mut result = html;

    if let Some(head_end) = result.find("</head>") {
        let preconnect_hints = r#"
  <link rel="dns-prefetch" href="//fonts.googleapis.com">
  <link rel="dns-prefetch" href="//fonts.gstatic.com">
  "#;

        result.insert_str(head_end, preconnect_hints);
    }

    result
}

fn add_performance_headers(headers: &mut axum::http::HeaderMap) {
    let performance_headers = [("X-DNS-Prefetch-Control", "on"), ("Server-Timing", "app;dur=0")];

    for (header_name, header_value) in performance_headers {
        if let Ok(value) = HeaderValue::from_str(header_value) {
            headers.insert(header_name, value);
        }
    }
}
