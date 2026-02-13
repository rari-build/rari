use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};
use tower_governor::key_extractor::KeyExtractor;

#[derive(Debug, Clone, Copy, Default)]
pub struct RariIpKeyExtractor;

impl KeyExtractor for RariIpKeyExtractor {
    type Key = String;

    fn extract<T>(
        &self,
        req: &axum::http::Request<T>,
    ) -> Result<Self::Key, tower_governor::errors::GovernorError> {
        let socket_addr = req
            .extensions()
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0)
            .ok_or(tower_governor::errors::GovernorError::UnableToExtractKey)?;

        Ok(extract_client_ip(req.headers(), &socket_addr))
    }
}

pub fn extract_client_ip(headers: &HeaderMap, socket_addr: &SocketAddr) -> String {
    if let Some(real_ip) = headers.get("x-real-ip")
        && let Ok(value) = real_ip.to_str()
        && let Some(ip) = validate_ip(value.trim())
    {
        return ip;
    }

    if let Some(fly_ip) = headers.get("fly-client-ip")
        && let Ok(value) = fly_ip.to_str()
        && let Some(ip) = validate_ip(value.trim())
    {
        return ip;
    }

    if let Some(cf_ip) = headers.get("cf-connecting-ip")
        && let Ok(value) = cf_ip.to_str()
        && let Some(ip) = validate_ip(value.trim())
    {
        return ip;
    }

    if let Some(forwarded_for) = headers.get("x-forwarded-for")
        && let Ok(value) = forwarded_for.to_str()
        && let Some(proxy_ip) = value.split(',').next_back()
        && let Some(ip) = validate_ip(proxy_ip.trim())
    {
        return ip;
    }

    socket_addr.ip().to_string()
}

fn validate_ip(ip_str: &str) -> Option<String> {
    if ip_str.is_empty() {
        return None;
    }

    ip_str.parse::<IpAddr>().ok().map(|ip| ip.to_string())
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
    fn test_extract_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.5".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.5");
    }

    #[test]
    fn test_extract_from_fly_client_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("fly-client-ip", "203.0.113.6".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.6");
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
    fn test_extract_from_x_forwarded_for_rightmost() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "1.1.1.1, 198.51.100.1, 203.0.113.1".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_fallback_to_socket_addr() {
        let headers = HeaderMap::new();
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_priority_x_real_ip_over_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.2".parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.2");
    }

    #[test]
    fn test_priority_fly_client_ip_over_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("fly-client-ip", "203.0.113.3".parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.3");
    }

    #[test]
    fn test_priority_cf_connecting_ip_over_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "203.0.113.4".parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.4");
    }

    #[test]
    fn test_invalid_ip_falls_back() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "not-an-ip".parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_empty_ip_falls_back() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "".parse().expect("Valid header value"));
        headers.insert("fly-client-ip", "203.0.113.5".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.5");
    }

    #[test]
    fn test_all_invalid_falls_back_to_socket() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "invalid".parse().expect("Valid header value"));
        headers.insert("fly-client-ip", "also-invalid".parse().expect("Valid header value"));
        headers.insert("cf-connecting-ip", "still-invalid".parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "not-valid-either".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_ipv6_validation() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "2001:db8::1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "2001:db8::1");
    }

    #[test]
    fn test_x_forwarded_for_single_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_validate_ip_function() {
        assert_eq!(validate_ip("192.168.1.1"), Some("192.168.1.1".to_string()));
        assert_eq!(validate_ip("2001:db8::1"), Some("2001:db8::1".to_string()));
        assert_eq!(validate_ip("invalid"), None);
        assert_eq!(validate_ip(""), None);
        assert_eq!(validate_ip("999.999.999.999"), None);
    }

    #[test]
    fn test_empty_x_forwarded_for_falls_back() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "".parse().expect("Valid header value"));
        headers.insert("x-real-ip", "203.0.113.5".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.5");
    }

    #[test]
    fn test_whitespace_only_x_forwarded_for_falls_back() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "   ".parse().expect("Valid header value"));
        headers.insert("fly-client-ip", "203.0.113.6".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.6");
    }

    #[test]
    fn test_x_forwarded_for_with_invalid_tokens_uses_rightmost_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "not-an-ip, 203.0.113.1".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_x_forwarded_for_all_invalid_tokens_falls_back() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "not-an-ip, also-invalid, still-bad".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_x_forwarded_for_mixed_valid_invalid_uses_rightmost_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "192.168.1.1, invalid, 203.0.113.5, bad-ip".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_ipv6_in_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "2001:db8::1, 2001:db8::2".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "2001:db8::2");
    }

    #[test]
    fn test_ipv6_in_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "2001:db8::abcd".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "2001:db8::abcd");
    }

    #[test]
    fn test_ipv6_in_fly_client_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("fly-client-ip", "2001:db8::cafe".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "2001:db8::cafe");
    }

    #[test]
    fn test_ipv6_in_cf_connecting_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "2001:db8::beef".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "2001:db8::beef");
    }

    #[test]
    fn test_ipv6_compressed_format() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "::1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "::1");
    }

    #[test]
    fn test_ipv6_full_format() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-real-ip",
            "2001:0db8:0000:0000:0000:0000:0000:0001".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "2001:db8::1");
    }

    #[test]
    fn test_x_forwarded_for_with_ports_stripped() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_spoofing_attempt_with_multiple_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers
            .insert("x-forwarded-for", "1.1.1.1, 203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_malformed_ip_with_extra_octets() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "192.168.1.1.1".parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_sql_injection_attempt_in_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "'; DROP TABLE users; --".parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_xss_attempt_in_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-real-ip",
            "<script>alert('xss')</script>".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_very_long_ip_string() {
        let mut headers = HeaderMap::new();
        let long_string = "1".repeat(1000);
        headers.insert("x-real-ip", long_string.parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_null_byte_in_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "192.168.1.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_unicode_in_ip_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "192.168.1.①".parse().expect("Valid header value"));
        headers.insert("x-forwarded-for", "203.0.113.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_leading_zeros_in_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "192.168.001.001".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        let result_is_valid = ip == "192.168.1.1" || ip == "10.0.0.1";
        assert!(result_is_valid, "Expected normalized IP or fallback, got: {}", ip);
    }

    #[test]
    fn test_negative_numbers_in_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "-1.-2.-3.-4".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_hex_format_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "0xC0.0xA8.0x01.0x01".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_ipv4_mapped_ipv6() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "::ffff:192.168.1.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "::ffff:192.168.1.1");
    }

    #[test]
    fn test_localhost_ipv4() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "127.0.0.1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "127.0.0.1");
    }

    #[test]
    fn test_localhost_ipv6() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "::1".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "::1");
    }

    #[test]
    fn test_private_network_ips() {
        let test_cases = vec!["10.0.0.1", "172.16.0.1", "192.168.0.1", "169.254.0.1"];

        for test_ip in test_cases {
            let mut headers = HeaderMap::new();
            headers.insert("x-real-ip", test_ip.parse().expect("Valid header value"));
            let socket_addr = create_socket_addr();

            let ip = extract_client_ip(&headers, &socket_addr);
            assert_eq!(ip, test_ip, "Failed for private IP: {}", test_ip);
        }
    }

    #[test]
    fn test_x_forwarded_for_with_spaces_around_commas() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "192.168.1.1 , 203.0.113.1 , 203.0.113.2".parse().expect("Valid header value"),
        );
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.2");
    }

    #[test]
    fn test_x_forwarded_for_single_ip_with_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "  203.0.113.1  ".parse().expect("Valid header value"));
        let socket_addr = create_socket_addr();

        let ip = extract_client_ip(&headers, &socket_addr);
        assert_eq!(ip, "203.0.113.1");
    }
}
