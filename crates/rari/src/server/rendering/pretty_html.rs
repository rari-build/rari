//! Lightweight HTML pretty-printer for development View Source readability.
//!
//! Not a full HTML5 parser. Preserves content inside `script`, `style`, `pre`,
//! and `textarea`. Safe to run after all head/asset/metadata injections.
//! Production responses are left as-is (compression handles wire size).

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style", "pre", "textarea"];

/// Pretty-print HTML with 2-space indentation.
///
/// Returns the input unchanged when it is empty or does not look like HTML.
pub fn pretty_print_html(html: &str) -> String {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(html.len() + html.len() / 8);
    let mut indent: usize = 0;
    let mut i = 0;
    let bytes = trimmed.as_bytes();

    // Preserve leading DOCTYPE / comments spacing via trim; rewrite body.
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let Some(tag_end) = find_tag_end(bytes, i) else {
                out.push_str(&trimmed[i..]);
                break;
            };
            let tag = &trimmed[i..=tag_end];
            let tag_name = parse_tag_name(tag);

            if tag.starts_with("<!--") {
                write_indent(&mut out, indent);
                out.push_str(tag);
                out.push('\n');
                i = tag_end + 1;
                continue;
            }

            if tag.starts_with("<!") {
                write_indent(&mut out, indent);
                out.push_str(tag);
                out.push('\n');
                i = tag_end + 1;
                continue;
            }

            if tag.starts_with("</") {
                indent = indent.saturating_sub(1);
                write_indent(&mut out, indent);
                out.push_str(tag);
                out.push('\n');
                i = tag_end + 1;
                continue;
            }

            let is_void = is_void_element(tag_name) || tag.ends_with("/>");
            let is_raw = is_raw_text_element(tag_name);

            write_indent(&mut out, indent);
            out.push_str(tag);
            out.push('\n');
            i = tag_end + 1;

            if is_void {
                continue;
            }

            if is_raw {
                let close_abs = find_closing_tag(trimmed, i, tag_name);
                if let Some(close_abs) = close_abs {
                    let content = &trimmed[i..close_abs];
                    if !content.is_empty() {
                        // Preserve raw text exactly (scripts/styles/pre/textarea).
                        out.push_str(content);
                        if !content.ends_with('\n') {
                            out.push('\n');
                        }
                    }
                    if let Some(close_end) = find_tag_end(bytes, close_abs) {
                        write_indent(&mut out, indent);
                        out.push_str(&trimmed[close_abs..=close_end]);
                        out.push('\n');
                        i = close_end + 1;
                    } else {
                        i = close_abs;
                    }
                }
                continue;
            }

            indent += 1;
            continue;
        }

        // Text node until next tag
        let next_tag = trimmed[i..].find('<').map_or(trimmed.len(), |n| i + n);
        let text = trimmed[i..next_tag].trim();
        if !text.is_empty() {
            write_indent(&mut out, indent);
            out.push_str(text);
            out.push('\n');
        }
        i = next_tag;
    }

    // Drop trailing newline for consistency with common formatters, keep one final newline.
    if out.ends_with('\n') {
        out
    } else {
        out.push('\n');
        out
    }
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn find_tag_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut in_quote: Option<u8> = None;
    let mut i = start + 1;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_quote {
            if b == q {
                in_quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            in_quote = Some(b);
        } else if b == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_tag_name(tag: &str) -> &str {
    let rest = tag.trim_start_matches(['<', '/']).trim_start();
    let end =
        rest.find(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/').unwrap_or(rest.len());
    &rest[..end]
}

fn is_void_element(name: &str) -> bool {
    VOID_ELEMENTS.iter().any(|v| v.eq_ignore_ascii_case(name))
}

fn is_raw_text_element(name: &str) -> bool {
    RAW_TEXT_ELEMENTS.iter().any(|v| v.eq_ignore_ascii_case(name))
}

fn find_closing_tag(html: &str, from: usize, tag_name: &str) -> Option<usize> {
    let bytes = html.as_bytes();
    let name_bytes = tag_name.as_bytes();
    let mut i = from;
    while i + 2 + name_bytes.len() <= bytes.len() {
        if bytes[i] == b'<' && bytes[i + 1] == b'/' {
            let name_start = i + 2;
            let name_end = name_start + name_bytes.len();
            if name_end <= bytes.len()
                && html[name_start..name_end].eq_ignore_ascii_case(tag_name)
                && (name_end == bytes.len()
                    || bytes[name_end].is_ascii_whitespace()
                    || bytes[name_end] == b'>')
            {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_prints_nested_elements() {
        let input = "<!DOCTYPE html><html><head><title>t</title></head><body><div id=\"root\"><p>Hi</p></div></body></html>";
        let out = pretty_print_html(input);
        // DOCTYPE is a sibling of <html>, not a parent — both at indent 0.
        assert!(out.contains("<!DOCTYPE html>\n<html>\n"));
        assert!(out.contains("  <head>\n"));
        assert!(out.contains("    <title>\n"));
        assert!(out.contains("      t\n"));
        assert!(out.contains("    </title>\n"));
        assert!(out.contains("  </head>\n"));
        assert!(out.contains("    <div id=\"root\">\n"));
        assert!(out.contains("      <p>\n"));
        assert!(out.contains("        Hi\n"));
    }

    #[test]
    fn preserves_script_contents() {
        let input = "<html><head><script>const x = 1;\nif (x) { console.log(x); }</script></head><body></body></html>";
        let out = pretty_print_html(input);
        assert!(out.contains("const x = 1;"));
        assert!(out.contains("if (x) { console.log(x); }"));
        assert!(out.contains("</script>"));
    }

    #[test]
    fn handles_void_elements_without_extra_indent() {
        let input = "<html><head><meta charset=\"utf-8\"><link rel=\"stylesheet\" href=\"/a.css\"></head><body></body></html>";
        let out = pretty_print_html(input);
        assert!(out.contains("    <meta charset=\"utf-8\">\n"));
        assert!(out.contains("    <link rel=\"stylesheet\" href=\"/a.css\">\n"));
        // head children should be at same indent; body follows head close
        assert!(out.contains("  </head>\n  <body>\n"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(pretty_print_html(""), "");
        assert_eq!(pretty_print_html("   "), "");
    }
}
