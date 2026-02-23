use axum::http::HeaderValue;
use cow_utils::CowUtils;
use rustc_hash::{FxHashMap, FxHashSet};
use sysinfo::{Pid, System};

pub fn extract_search_params(
    query_params: FxHashMap<String, String>,
) -> FxHashMap<String, Vec<String>> {
    query_params.into_iter().map(|(k, v)| (k, vec![v])).collect()
}

pub fn extract_headers(headers: &axum::http::HeaderMap) -> FxHashMap<String, String> {
    let mut header_map = FxHashMap::default();
    for (name, value) in headers {
        if let Ok(value_str) = value.to_str() {
            header_map.insert(name.to_string(), value_str.to_string());
        }
    }
    header_map
}

pub fn merge_vary_with_accept(existing_vary: Option<&HeaderValue>) -> String {
    let mut seen = FxHashSet::default();
    let mut vary_values = Vec::new();

    seen.insert("accept".to_string());
    vary_values.push("Accept".to_string());

    if let Some(vary_header) = existing_vary
        && let Ok(vary_str) = vary_header.to_str()
    {
        for value in vary_str.split(',') {
            let trimmed = value.trim();
            if trimmed == "*" {
                return "*".to_string();
            }
            if !trimmed.is_empty() {
                let normalized = trimmed.cow_to_ascii_lowercase().into_owned();
                if seen.insert(normalized) {
                    vary_values.push(trimmed.to_string());
                }
            }
        }
    }

    vary_values.sort_by_cached_key(|a| a.cow_to_ascii_lowercase().into_owned());

    vary_values.join(", ")
}

pub fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

pub fn get_memory_usage() -> Option<u64> {
    let mut sys = System::new_all();
    sys.refresh_memory();

    let pid = Pid::from(std::process::id() as usize);

    if let Some(process) = sys.process(pid) {
        return Some(process.memory());
    }

    Some(sys.used_memory() * 1024)
}

pub fn is_origin_allowed(origin: &str, allowed_origins: &[String]) -> bool {
    allowed_origins.iter().any(|allowed| {
        if allowed == origin {
            return true;
        }

        if let Some(domain) = allowed.strip_prefix("*.")
            && let Ok(origin_url) = url::Url::parse(origin)
            && let Some(host) = origin_url.host_str()
        {
            return host.ends_with(domain) || host == domain;
        }

        false
    })
}

pub fn add_api_cors_headers(
    headers: &mut axum::http::HeaderMap,
    request_origin: Option<&str>,
    allowed_origins: &[String],
    allow_credentials: bool,
    max_age: u32,
) {
    if let Some(origin) = request_origin
        && is_origin_allowed(origin, allowed_origins)
    {
        if !headers.contains_key("Access-Control-Allow-Origin")
            && let Ok(value) = HeaderValue::from_str(origin)
        {
            headers.insert("Access-Control-Allow-Origin", value);
        }

        if allow_credentials && !headers.contains_key("Access-Control-Allow-Credentials") {
            headers.insert("Access-Control-Allow-Credentials", HeaderValue::from_static("true"));
        }
    }

    if !headers.contains_key("Access-Control-Allow-Methods") {
        headers.insert(
            "Access-Control-Allow-Methods",
            HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"),
        );
    }

    if !headers.contains_key("Access-Control-Allow-Headers") {
        headers.insert(
            "Access-Control-Allow-Headers",
            HeaderValue::from_static(
                "Content-Type, Authorization, Accept, Origin, X-Requested-With, Cache-Control, X-RSC-Streaming, X-CSRF-Token",
            ),
        );
    }

    if !headers.contains_key("Access-Control-Max-Age")
        && let Ok(value) = HeaderValue::from_str(&max_age.to_string())
    {
        headers.insert("Access-Control-Max-Age", value);
    }

    if !headers.contains_key("Vary") {
        headers.insert("Vary", HeaderValue::from_static("Origin"));
    }
}

pub fn add_api_security_headers(headers: &mut axum::http::HeaderMap) {
    if !headers.contains_key("X-Content-Type-Options") {
        headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    }

    if !headers.contains_key("X-Frame-Options") {
        headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    }

    if !headers.contains_key("X-XSS-Protection") {
        headers.insert("X-XSS-Protection", HeaderValue::from_static("1; mode=block"));
    }

    if !headers.contains_key("Strict-Transport-Security") {
        headers.insert(
            "Strict-Transport-Security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    if !headers.contains_key("Content-Security-Policy") {
        headers.insert(
            "Content-Security-Policy",
            HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
        );
    }

    if !headers.contains_key("Referrer-Policy") {
        headers.insert("Referrer-Policy", HeaderValue::from_static("no-referrer"));
    }

    if !headers.contains_key("Permissions-Policy") {
        headers.insert(
            "Permissions-Policy",
            HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
        );
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_is_origin_allowed_exact_match() {
        let allowed = vec!["https://example.com".to_string()];

        assert!(is_origin_allowed("https://example.com", &allowed));
        assert!(!is_origin_allowed("https://evil.com", &allowed));
        assert!(!is_origin_allowed("https://example.com.evil.com", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_wildcard_subdomain() {
        let allowed = vec!["*.example.com".to_string()];

        assert!(is_origin_allowed("https://app.example.com", &allowed));
        assert!(is_origin_allowed("https://api.example.com", &allowed));
        assert!(is_origin_allowed("https://example.com", &allowed));
        assert!(!is_origin_allowed("https://evil.com", &allowed));
        assert!(!is_origin_allowed("https://example.com.evil.com", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_multiple_origins() {
        let allowed = vec![
            "https://example.com".to_string(),
            "https://app.example.com".to_string(),
            "http://localhost:3000".to_string(),
        ];

        assert!(is_origin_allowed("https://example.com", &allowed));
        assert!(is_origin_allowed("https://app.example.com", &allowed));
        assert!(is_origin_allowed("http://localhost:3000", &allowed));
        assert!(!is_origin_allowed("https://evil.com", &allowed));
    }

    #[test]
    fn test_is_origin_allowed_empty_list() {
        let allowed: Vec<String> = vec![];

        assert!(!is_origin_allowed("https://example.com", &allowed));
        assert!(!is_origin_allowed("http://localhost:3000", &allowed));
    }

    #[test]
    fn test_add_api_cors_headers_valid_origin() {
        let mut headers = axum::http::HeaderMap::new();
        let allowed = vec!["https://example.com".to_string()];

        add_api_cors_headers(&mut headers, Some("https://example.com"), &allowed, true, 86400);

        assert_eq!(headers.get("Access-Control-Allow-Origin").unwrap(), "https://example.com");
        assert_eq!(headers.get("Access-Control-Allow-Credentials").unwrap(), "true");
        assert_eq!(headers.get("Access-Control-Max-Age").unwrap(), "86400");
        assert!(headers.contains_key("Access-Control-Allow-Methods"));
        assert!(headers.contains_key("Access-Control-Allow-Headers"));
        assert_eq!(headers.get("Vary").unwrap(), "Origin");
    }

    #[test]
    fn test_add_api_cors_headers_invalid_origin() {
        let mut headers = axum::http::HeaderMap::new();
        let allowed = vec!["https://example.com".to_string()];

        add_api_cors_headers(&mut headers, Some("https://evil.com"), &allowed, true, 86400);

        assert!(!headers.contains_key("Access-Control-Allow-Origin"));
        assert!(!headers.contains_key("Access-Control-Allow-Credentials"));

        assert!(headers.contains_key("Access-Control-Allow-Methods"));
        assert!(headers.contains_key("Access-Control-Allow-Headers"));
    }

    #[test]
    fn test_add_api_cors_headers_no_origin() {
        let mut headers = axum::http::HeaderMap::new();
        let allowed = vec!["https://example.com".to_string()];

        add_api_cors_headers(&mut headers, None, &allowed, true, 86400);

        assert!(!headers.contains_key("Access-Control-Allow-Origin"));
        assert!(!headers.contains_key("Access-Control-Allow-Credentials"));

        assert!(headers.contains_key("Access-Control-Allow-Methods"));
        assert!(headers.contains_key("Access-Control-Allow-Headers"));
    }

    #[test]
    fn test_add_api_cors_headers_without_credentials() {
        let mut headers = axum::http::HeaderMap::new();
        let allowed = vec!["https://example.com".to_string()];

        add_api_cors_headers(&mut headers, Some("https://example.com"), &allowed, false, 86400);

        assert_eq!(headers.get("Access-Control-Allow-Origin").unwrap(), "https://example.com");

        assert!(!headers.contains_key("Access-Control-Allow-Credentials"));
    }

    #[test]
    fn test_add_api_cors_headers_preserves_existing() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "Access-Control-Allow-Origin",
            HeaderValue::from_static("https://existing.com"),
        );

        let allowed = vec!["https://example.com".to_string()];

        add_api_cors_headers(&mut headers, Some("https://example.com"), &allowed, true, 86400);

        assert_eq!(headers.get("Access-Control-Allow-Origin").unwrap(), "https://existing.com");
    }

    #[test]
    fn test_add_api_cors_headers_includes_csrf_token() {
        let mut headers = axum::http::HeaderMap::new();
        let allowed = vec!["https://example.com".to_string()];

        add_api_cors_headers(&mut headers, Some("https://example.com"), &allowed, true, 86400);

        let allow_headers = headers.get("Access-Control-Allow-Headers").unwrap().to_str().unwrap();
        assert!(allow_headers.contains("X-CSRF-Token"));
    }

    #[test]
    fn test_wildcard_subdomain_with_port() {
        let allowed = vec!["*.example.com".to_string()];

        assert!(is_origin_allowed("https://app.example.com:8080", &allowed));
        assert!(is_origin_allowed("http://api.example.com:3000", &allowed));
    }

    #[test]
    fn test_localhost_variations() {
        let allowed =
            vec!["http://localhost:3000".to_string(), "http://127.0.0.1:3000".to_string()];

        assert!(is_origin_allowed("http://localhost:3000", &allowed));
        assert!(is_origin_allowed("http://127.0.0.1:3000", &allowed));
        assert!(!is_origin_allowed("http://localhost:8080", &allowed));
        assert!(!is_origin_allowed("http://127.0.0.1:8080", &allowed));
    }
}
