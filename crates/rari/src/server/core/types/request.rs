use axum::http::HeaderMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderMode {
    Ssr,
    RscNavigation,
}

#[non_exhaustive]
pub struct RequestTypeDetector;

impl RequestTypeDetector {
    pub fn detect_render_mode(headers: &HeaderMap) -> RenderMode {
        if let Some(accept) = headers.get("accept")
            && let Ok(accept_str) = accept.to_str()
            && accept_str.contains("text/x-component")
        {
            return RenderMode::RscNavigation;
        }

        RenderMode::Ssr
    }

    #[cfg(test)]
    pub fn needs_rsc_flight_protocol(mode: RenderMode) -> bool {
        matches!(mode, RenderMode::RscNavigation)
    }
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
    use axum::http::{HeaderMap, HeaderValue};

    use super::*;

    #[test]
    fn test_detect_ssr_mode_with_html_accept() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/html"));

        let mode = RequestTypeDetector::detect_render_mode(&headers);
        assert_eq!(mode, RenderMode::Ssr);
    }

    #[test]
    fn test_detect_rsc_navigation_mode() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/x-component"));

        let mode = RequestTypeDetector::detect_render_mode(&headers);
        assert_eq!(mode, RenderMode::RscNavigation);
    }

    #[test]
    fn test_detect_rsc_navigation_mode_with_multiple_types() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/x-component, application/json"));

        let mode = RequestTypeDetector::detect_render_mode(&headers);
        assert_eq!(mode, RenderMode::RscNavigation);
    }

    #[test]
    fn test_default_to_ssr_when_no_accept_header() {
        let headers = HeaderMap::new();

        let mode = RequestTypeDetector::detect_render_mode(&headers);
        assert_eq!(mode, RenderMode::Ssr);
    }

    #[test]
    fn test_default_to_ssr_with_invalid_accept_header() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("application/json"));

        let mode = RequestTypeDetector::detect_render_mode(&headers);
        assert_eq!(mode, RenderMode::Ssr);
    }

    #[test]
    fn test_needs_rsc_flight_protocol_for_ssr() {
        assert!(!RequestTypeDetector::needs_rsc_flight_protocol(RenderMode::Ssr));
    }

    #[test]
    fn test_needs_rsc_flight_protocol_for_rsc_navigation() {
        assert!(RequestTypeDetector::needs_rsc_flight_protocol(RenderMode::RscNavigation));
    }

    #[test]
    fn test_render_mode_equality() {
        assert_eq!(RenderMode::Ssr, RenderMode::Ssr);
        assert_eq!(RenderMode::RscNavigation, RenderMode::RscNavigation);
        assert_ne!(RenderMode::Ssr, RenderMode::RscNavigation);
    }

    #[test]
    fn test_render_mode_clone() {
        let mode = RenderMode::Ssr;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_render_mode_debug() {
        let mode = RenderMode::Ssr;
        let debug_str = format!("{:?}", mode);
        assert!(debug_str.contains("Ssr"));
    }
}
