use std::borrow::Cow;
use std::fmt::Write;

pub struct HtmlBuilder {
    pub(crate) buffer: String,
}

impl HtmlBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(4096)
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self { buffer: String::with_capacity(capacity) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    #[inline]
    pub fn push_str(&mut self, s: &str) {
        self.buffer.push_str(s);
    }

    #[inline]
    pub fn push(&mut self, ch: char) {
        self.buffer.push(ch);
    }

    #[inline]
    pub fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) {
        let _ = self.buffer.write_fmt(args);
    }

    #[inline]
    pub fn write_attr(&mut self, name: &str, value: &str) {
        if !super::is_safe_attribute_name(name) {
            return;
        }
        self.buffer.push(' ');
        self.buffer.push_str(name);
        self.buffer.push_str("=\"");
        escape_html_into(value, &mut self.buffer);
        self.buffer.push('"');
    }

    #[inline]
    pub fn write_attr_value<T: std::fmt::Display>(&mut self, name: &str, value: T) {
        if !super::is_safe_attribute_name(name) {
            return;
        }
        use std::fmt::Write;
        self.buffer.push(' ');
        self.buffer.push_str(name);
        self.buffer.push_str("=\"");
        let mut writer = EscapingWriter { buffer: &mut self.buffer };
        let _ = write!(writer, "{}", value);
        self.buffer.push('"');
    }

    #[inline]
    pub fn write_bool_attr(&mut self, name: &str) {
        if !super::is_safe_attribute_name(name) {
            return;
        }
        self.buffer.push(' ');
        self.buffer.push_str(name);
    }

    #[inline]
    pub fn write_open_tag(&mut self, tag: &str) {
        self.buffer.push('<');
        self.buffer.push_str(tag);
    }

    #[inline]
    pub fn write_close_tag(&mut self, tag: &str) {
        self.buffer.push_str("</");
        self.buffer.push_str(tag);
        self.buffer.push('>');
    }

    #[inline]
    pub fn write_self_closing_end(&mut self) {
        self.buffer.push_str(" />");
    }

    #[inline]
    pub fn finish_open_tag(&mut self) {
        self.buffer.push('>');
    }

    #[inline]
    pub fn push_escaped(&mut self, text: &str) {
        escape_html_into(text, &mut self.buffer);
    }

    #[inline]
    #[must_use]
    pub fn build(self) -> String {
        self.buffer
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    #[inline]
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.buffer.reserve(additional);
    }
}

impl Default for HtmlBuilder {
    fn default() -> Self {
        Self::new()
    }
}

struct EscapingWriter<'a> {
    buffer: &'a mut String,
}

impl<'a> std::fmt::Write for EscapingWriter<'a> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        escape_html_into(s, self.buffer);
        Ok(())
    }
}

#[inline]
pub fn escape_html_into(text: &str, buffer: &mut String) {
    let bytes = text.as_bytes();
    let mut last_copy = 0;

    for (i, &b) in bytes.iter().enumerate() {
        let replacement = match b {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            b'\'' => "&#39;",
            _ => continue,
        };
        if last_copy < i {
            buffer.push_str(&text[last_copy..i]);
        }
        buffer.push_str(replacement);
        last_copy = i + 1;
    }

    if last_copy < text.len() {
        buffer.push_str(&text[last_copy..]);
    }
}

#[inline]
#[must_use]
pub fn escape_html_fast(text: &str) -> Cow<'_, str> {
    if !text.bytes().any(|b| matches!(b, b'&' | b'<' | b'>' | b'"' | b'\'')) {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len() + 16);
    escape_html_into(text, &mut result);
    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_builder_basic() {
        let mut builder = HtmlBuilder::new();
        builder.write_open_tag("div");
        builder.write_attr("class", "container");
        builder.finish_open_tag();
        builder.push_str("Hello");
        builder.write_close_tag("div");

        assert_eq!(builder.build(), r#"<div class="container">Hello</div>"#);
    }

    #[test]
    fn test_html_builder_escaped() {
        let mut builder = HtmlBuilder::new();
        builder.write_open_tag("div");
        builder.finish_open_tag();
        builder.push_escaped("<script>alert('xss')</script>");
        builder.write_close_tag("div");

        assert_eq!(builder.build(), "<div>&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;</div>");
    }

    #[test]
    fn test_escape_html_fast_path() {
        let text = "Hello World";
        let escaped = escape_html_fast(text);
        assert_eq!(escaped, text);
    }

    #[test]
    fn test_escape_html_slow_path() {
        let text = "<div>Hello & \"World\"</div>";
        let escaped = escape_html_fast(text);
        assert_eq!(escaped, "&lt;div&gt;Hello &amp; &quot;World&quot;&lt;/div&gt;");
    }

    #[test]
    fn test_escape_html_into() {
        let mut buffer = String::new();
        escape_html_into("Test & <test>", &mut buffer);
        assert_eq!(buffer, "Test &amp; &lt;test&gt;");
    }

    #[test]
    fn test_write_fmt() {
        let mut builder = HtmlBuilder::new();
        builder.write_fmt(format_args!("Value: {}", 42));
        assert_eq!(builder.build(), "Value: 42");
    }

    #[test]
    fn test_bool_attr() {
        let mut builder = HtmlBuilder::new();
        builder.write_open_tag("input");
        builder.write_attr("type", "checkbox");
        builder.write_bool_attr("checked");
        builder.write_self_closing_end();

        assert_eq!(builder.build(), r#"<input type="checkbox" checked />"#);
    }

    #[test]
    fn test_capacity() {
        let builder = HtmlBuilder::with_capacity(1024);
        assert!(builder.buffer.capacity() >= 1024);
    }

    #[test]
    fn test_clear_and_reuse() {
        let mut builder = HtmlBuilder::new();
        builder.push_str("test");
        assert_eq!(builder.len(), 4);

        builder.clear();
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());

        builder.push_str("reused");
        assert_eq!(builder.build(), "reused");
    }

    #[test]
    fn test_write_attr_value() {
        let mut builder = HtmlBuilder::new();
        builder.write_open_tag("input");
        builder.write_attr_value("min", 0);
        builder.write_attr_value("max", 100);
        builder.write_attr_value("step", 0.5);
        builder.write_self_closing_end();

        assert_eq!(builder.build(), r#"<input min="0" max="100" step="0.5" />"#);
    }

    #[test]
    fn test_write_attr_value_escapes_html_special_chars() {
        struct HtmlSpecialChars;
        impl std::fmt::Display for HtmlSpecialChars {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, r#"<script>"alert('xss')"</script>"#)
            }
        }

        let mut builder = HtmlBuilder::new();
        builder.write_open_tag("div");
        builder.write_attr_value("data-value", HtmlSpecialChars);
        builder.write_self_closing_end();

        assert_eq!(
            builder.build(),
            r#"<div data-value="&lt;script&gt;&quot;alert(&#39;xss&#39;)&quot;&lt;/script&gt;" />"#
        );
    }

    #[test]
    fn test_write_attr_blocks_unsafe_names() {
        let mut builder = HtmlBuilder::new();
        builder.write_open_tag("div");
        builder.write_attr("onclick", "alert('xss')");
        builder.write_attr("onload", "alert('xss')");
        builder.write_attr("id", "safe");
        builder.write_self_closing_end();

        let html = builder.build();
        assert!(!html.contains("onclick"));
        assert!(!html.contains("onload"));
        assert!(html.contains("id=\"safe\""));
    }

    #[test]
    fn test_write_attr_value_blocks_unsafe_names() {
        let mut builder = HtmlBuilder::new();
        builder.write_open_tag("input");
        builder.write_attr_value("onclick", 123);
        builder.write_attr_value("min", 0);
        builder.write_self_closing_end();

        let html = builder.build();
        assert!(!html.contains("onclick"));
        assert!(html.contains("min=\"0\""));
    }

    #[test]
    fn test_write_bool_attr_blocks_unsafe_names() {
        let mut builder = HtmlBuilder::new();
        builder.write_open_tag("input");
        builder.write_bool_attr("onclick");
        builder.write_bool_attr("checked");
        builder.write_self_closing_end();

        let html = builder.build();
        assert!(!html.contains("onclick"));
        assert!(html.contains("checked"));
    }
}
