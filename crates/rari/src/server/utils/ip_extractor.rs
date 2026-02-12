use axum::http::HeaderMap;
use std::net::SocketAddr;

pub fn extract_client_ip(headers: &HeaderMap, socket_addr: &SocketAddr) -> String {
    if let Some(forwarded_for) = headers.get("x-forwarded-for")
        && let Ok(value) = forwarded_for.to_str()
        && let Some(client_ip) = value.split(',').next()
    {
        let trimmed = client_ip.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(fly_ip) = headers.get("fly-client-ip")
        && let Ok(value) = fly_ip.to_str()
    {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip")
        && let Ok(value) = real_ip.to_str()
    {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(cf_ip) = headers.get("cf-connecting-ip")
        && let Ok(value) = cf_ip.to_str()
    {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    socket_addr.ip().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn create_socket_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8080)
    }

    #[test]
    fn test_extract_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "203.0.113.1, 198.51.100.1".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_extract_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.5".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.5");
    }

    #[test]
    fn test_extract_from_cf_connecting_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "203.0.113.10".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.10");
    }

    #[test]
    fn test_fallback_to_socket_addr() {
        let headers = HeaderMap::new();
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_x_forwarded_for_priority() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        headers.insert("x-real-ip", "203.0.113.2".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }
}
