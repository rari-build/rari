use std::sync::OnceLock;

use regex::Regex;

pub fn sanitize_component_output(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }

    static PRE_JSON_REGEX: OnceLock<Regex> = OnceLock::new();
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    let pre_json_regex = PRE_JSON_REGEX
        .get_or_init(|| Regex::new(r"<pre>\\?\{.*?\\?\}</pre>").expect("Valid regex pattern"));
    let result = pre_json_regex.replace_all(html, "");

    static ID_JSON_REGEX: OnceLock<Regex> = OnceLock::new();
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    let id_json_regex = ID_JSON_REGEX
        .get_or_init(|| Regex::new(r#"\\?\{"id":.*?\\?\}"#).expect("Valid regex pattern"));
    id_json_regex.replace_all(&result, "").into_owned()
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
    use super::*;

    #[test]
    fn test_sanitize_empty_string() {
        let result = sanitize_component_output("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_sanitize_removes_pre_json() {
        let html = r#"<div>Content<pre>\{"data": "test"\}</pre>More content</div>"#;
        let result = sanitize_component_output(html);
        assert_eq!(result, "<div>ContentMore content</div>");
    }

    #[test]
    fn test_sanitize_removes_id_json() {
        let html = r#"<div>Content\{"id": "123", "name": "test"\}More</div>"#;
        let result = sanitize_component_output(html);
        assert_eq!(result, "<div>ContentMore</div>");
    }

    #[test]
    fn test_sanitize_preserves_normal_content() {
        let html = r#"<div><h1>Title</h1><p>Paragraph</p></div>"#;
        let result = sanitize_component_output(html);
        assert_eq!(result, html);
    }

    #[test]
    fn test_sanitize_with_component_wrapper() {
        let html = r#"<div data-component-id="test-component"><h1>Title</h1></div>"#;
        let result = sanitize_component_output(html);
        assert_eq!(result, html);
    }

    #[test]
    fn test_sanitize_multiple_patterns() {
        let html = r#"<div>Start<pre>\{"debug": "info"\}</pre>Middle\{"id": "456"\}End</div>"#;
        let result = sanitize_component_output(html);
        assert_eq!(result, "<div>StartMiddleEnd</div>");
    }

    #[test]
    fn test_sanitize_with_special_chars_in_component_id() {
        let html = r#"<div data-component-id="app/components/test"><h1>Title</h1></div>"#;
        let result = sanitize_component_output(html);
        assert_eq!(result, html);
    }
}
