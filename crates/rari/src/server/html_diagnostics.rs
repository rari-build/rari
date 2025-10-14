use tracing::{debug, warn};

pub struct HtmlDiagnostics;

impl HtmlDiagnostics {
    pub fn log_html_snippet(html: &str, label: &str, max_length: usize) {
        let snippet = if html.len() > max_length {
            format!("{}... ({} more bytes)", &html[..max_length], html.len() - max_length)
        } else {
            html.to_string()
        };
        debug!("{}: {}", label, snippet);
    }

    pub fn log_transformation(before: &str, after: &str, operation: &str) {
        debug!("=== {} ===", operation);
        Self::log_html_snippet(before, "BEFORE", 200);
        Self::log_html_snippet(after, "AFTER", 200);

        let before_has_body = before.contains("<body");
        let after_has_body = after.contains("<body");
        let before_has_root = before.contains(r#"id="root""#);
        let after_has_root = after.contains(r#"id="root""#);

        if before_has_body != after_has_body {
            warn!(
                "Body element presence changed: before={}, after={}",
                before_has_body, after_has_body
            );
        }

        if before_has_root != after_has_root {
            warn!(
                "Root div element presence changed: before={}, after={}",
                before_has_root, after_has_root
            );
        }
    }

    pub fn check_root_element(html: &str, context: &str) -> bool {
        let has_body = html.contains("<body");
        let has_root_div = html.contains(r#"id="root""#);
        let has_container = has_body || has_root_div;

        if has_container {
            if has_body {
                debug!("{} - App router body element found", context);
            } else if has_root_div && let Some(pos) = html.find(r#"id="root""#) {
                let start = pos.saturating_sub(50);
                let end = (pos + 100).min(html.len());
                debug!(
                    "{} - Root div element found at position {}: {}",
                    context,
                    pos,
                    &html[start..end]
                );
            }
        } else {
            warn!("{} - No container element (body or root div) found in HTML", context);
        }
        has_container
    }
}
