use regex::Regex;
use std::sync::OnceLock;

pub fn sanitize_component_output(html: &str) -> String {
    if html.is_empty() {
        return html.to_string();
    }

    let mut result = html.to_string();

    static PRE_JSON_REGEX: OnceLock<Regex> = OnceLock::new();
    let pre_json_regex = PRE_JSON_REGEX
        .get_or_init(|| Regex::new(r#"<pre>\\?\{.*?\\?\}</pre>"#).expect("Valid regex pattern"));
    result = pre_json_regex.replace_all(&result, "").to_string();

    static ID_JSON_REGEX: OnceLock<Regex> = OnceLock::new();
    let id_json_regex = ID_JSON_REGEX
        .get_or_init(|| Regex::new(r#"\\?\{"id":.*?\\?\}"#).expect("Valid regex pattern"));
    result = id_json_regex.replace_all(&result, "").to_string();

    result
}

#[cfg(test)]
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
