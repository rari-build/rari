use axum::http::HeaderValue;
use rustc_hash::FxHashMap;
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

pub fn add_api_cors_headers(headers: &mut axum::http::HeaderMap) {
    if !headers.contains_key("Access-Control-Allow-Origin") {
        headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
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
                "Content-Type, Authorization, Accept, Origin, X-Requested-With, Cache-Control, X-RSC-Streaming",
            ),
        );
    }

    if !headers.contains_key("Access-Control-Allow-Credentials") {
        headers.insert("Access-Control-Allow-Credentials", HeaderValue::from_static("true"));
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
