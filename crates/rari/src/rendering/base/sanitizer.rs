use std::sync::OnceLock;

use regex::Regex;

static PRE_JSON_REGEX: OnceLock<Regex> = OnceLock::new();
static ID_JSON_REGEX: OnceLock<Regex> = OnceLock::new();
static ID_JSON_INLINE_REGEX: OnceLock<Regex> = OnceLock::new();
static PRE_JSON_INLINE_REGEX: OnceLock<Regex> = OnceLock::new();
static ARRAY_JSON_REGEX: OnceLock<Regex> = OnceLock::new();
static LEAKAGE_IN_ATTR_REGEX: OnceLock<Regex> = OnceLock::new();
static LEAKAGE_IN_TEXT_REGEX: OnceLock<Regex> = OnceLock::new();
static LEAKAGE_ARRAY_IN_ATTR_REGEX: OnceLock<Regex> = OnceLock::new();
static LEAKAGE_CLEANUP_TEXT_REGEX: OnceLock<Regex> = OnceLock::new();
static LEAKAGE_CLEANUP_PRE_REGEX: OnceLock<Regex> = OnceLock::new();
static CALCULATION_REGEX: OnceLock<Regex> = OnceLock::new();

fn init_regex(pattern: &str) -> Regex {
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    Regex::new(pattern).expect("Valid regex pattern")
}

pub fn sanitize_html_output(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }

    let pre_json_regex = PRE_JSON_REGEX.get_or_init(|| init_regex(r"<pre>\\?\{.*?\\?\}</pre>"));
    let mut sanitized_html = pre_json_regex.replace_all(html, "").into_owned();

    let pre_json_inline_regex =
        PRE_JSON_INLINE_REGEX.get_or_init(|| init_regex(r"<pre>\{.*?\}</pre>"));
    sanitized_html = pre_json_inline_regex.replace_all(&sanitized_html, "").into_owned();

    let array_json_regex = ARRAY_JSON_REGEX.get_or_init(|| init_regex(r#"\[(\{".*?},?)+\]"#));
    sanitized_html = array_json_regex.replace_all(&sanitized_html, "[]").into_owned();

    let id_json_regex = ID_JSON_REGEX.get_or_init(|| init_regex(r#"\\?\{"id":.*?\\?\}"#));
    sanitized_html = id_json_regex.replace_all(&sanitized_html, "").into_owned();

    let id_json_inline_regex = ID_JSON_INLINE_REGEX.get_or_init(|| init_regex(r#"\{"id".*?\}"#));
    sanitized_html = id_json_inline_regex.replace_all(&sanitized_html, "").into_owned();

    let leakage_in_attr_regex =
        LEAKAGE_IN_ATTR_REGEX.get_or_init(|| init_regex(r#"=".*?\{"id".*?\}.*?""#));
    let leakage_in_text_regex =
        LEAKAGE_IN_TEXT_REGEX.get_or_init(|| init_regex(r#">.*?\{"id".*?\}.*?<"#));
    let leakage_array_in_attr_regex =
        LEAKAGE_ARRAY_IN_ATTR_REGEX.get_or_init(|| init_regex(r#"=".*?\[.*?\{.*?\}.*?\].*?""#));

    let result_contains_foreign_data = leakage_in_attr_regex.is_match(&sanitized_html)
        || leakage_in_text_regex.is_match(&sanitized_html)
        || leakage_array_in_attr_regex.is_match(&sanitized_html);

    if result_contains_foreign_data {
        let leakage_cleanup_text_regex =
            LEAKAGE_CLEANUP_TEXT_REGEX.get_or_init(|| init_regex(r#">\s*\{[^{]*"id"[^}]*\}\s*<"#));
        sanitized_html = leakage_cleanup_text_regex.replace_all(&sanitized_html, "><").into_owned();

        let leakage_cleanup_pre_regex =
            LEAKAGE_CLEANUP_PRE_REGEX.get_or_init(|| init_regex(r"<pre>.*?\{.*?\}.*?</pre>"));
        sanitized_html = leakage_cleanup_pre_regex.replace_all(&sanitized_html, "").into_owned();
    }

    let calculation_regex = CALCULATION_REGEX
        .get_or_init(|| init_regex(r"([a-zA-Z ]+: [0-9]+ \+ [0-9]+ =)\s*(\d+)([^0-9])"));
    sanitized_html = calculation_regex
        .replace_all(&sanitized_html, |captures: &regex::Captures| {
            format!("{}{}{}", &captures[1], &captures[2], &captures[3])
        })
        .into_owned();

    sanitized_html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_empty_string() {
        let result = sanitize_html_output("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_sanitize_removes_pre_json() {
        let html = r#"<div>Content<pre>\{"data": "test"\}</pre>More content</div>"#;
        let result = sanitize_html_output(html);
        assert_eq!(result, "<div>ContentMore content</div>");
    }

    #[test]
    fn test_sanitize_removes_id_json() {
        let html = r#"<div>Content\{"id": "123", "name": "test"\}More</div>"#;
        let result = sanitize_html_output(html);
        assert_eq!(result, "<div>ContentMore</div>");
    }

    #[test]
    fn test_sanitize_preserves_normal_content() {
        let html = r"<div><h1>Title</h1><p>Paragraph</p></div>";
        let result = sanitize_html_output(html);
        assert_eq!(result, html);
    }

    #[test]
    fn test_sanitize_with_component_wrapper() {
        let html = r#"<div data-component-id="test-component"><h1>Title</h1></div>"#;
        let result = sanitize_html_output(html);
        assert_eq!(result, html);
    }

    #[test]
    fn test_sanitize_multiple_patterns() {
        let html = r#"<div>Start<pre>\{"debug": "info"\}</pre>Middle\{"id": "456"\}End</div>"#;
        let result = sanitize_html_output(html);
        assert_eq!(result, "<div>StartMiddleEnd</div>");
    }

    #[test]
    fn test_sanitize_with_special_chars_in_component_id() {
        let html = r#"<div data-component-id="app/components/test"><h1>Title</h1></div>"#;
        let result = sanitize_html_output(html);
        assert_eq!(result, html);
    }

    #[test]
    fn test_sanitize_removes_array_json() {
        let html = r#"<div>[{"id":"1"},{"id":"2"}]</div>"#;
        let result = sanitize_html_output(html);
        assert_eq!(result, "<div>[]</div>");
    }
}
