#![expect(clippy::missing_errors_doc)]

use std::sync::Arc;

use cow_utils::CowUtils;
use rari_error::RariError;
use regex::Regex;
use rustc_hash::FxHashSet;
use tokio::fs;

use crate::{runtime::JsExecutionRuntime, server::routing::app_router::AppRouteMatch};

pub fn escape_html(text: &str) -> String {
    text.cow_replace('&', "&amp;")
        .cow_replace('<', "&lt;")
        .cow_replace('>', "&gt;")
        .cow_replace('"', "&quot;")
        .cow_replace('\'', "&#39;")
        .into_owned()
}

pub struct RscHtmlRenderer {
    runtime: Arc<JsExecutionRuntime>,
    template_cache: parking_lot::Mutex<Option<String>>,
}

impl RscHtmlRenderer {
    pub fn new(runtime: Arc<JsExecutionRuntime>) -> Self {
        Self { runtime, template_cache: parking_lot::Mutex::new(None) }
    }

    fn extract_script_tags(template: &str) -> String {
        #[expect(clippy::unwrap_used, reason = "Hardcoded regex pattern is guaranteed to be valid")]
        let script_regex = Regex::new(r"(?s)<script[^>]*>.*?</script>|<script[^>]*/>").unwrap();

        script_regex
            .find_iter(template)
            .map(|m| m.as_str().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn is_stylesheet_link_tag(tag: &str) -> bool {
        let lower = tag.to_lowercase();
        lower.contains("stylesheet") || lower.contains("text/css")
    }

    fn extract_non_stylesheet_link_tags(template: &str) -> String {
        #[expect(clippy::unwrap_used, reason = "Hardcoded regex pattern is guaranteed to be valid")]
        let link_regex = Regex::new(r"(?i)<link\b[^>]*/?>").unwrap();

        link_regex
            .find_iter(template)
            .map(|m| m.as_str())
            .filter(|tag| !Self::is_stylesheet_link_tag(tag))
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn inject_head_tags(template: &str, tags: &str) -> String {
        let tags = tags.trim();
        if tags.is_empty() {
            return template.to_string();
        }

        let tag_block = tags
            .lines()
            .filter(|line| !line.trim().is_empty() && !template.contains(line))
            .collect::<Vec<_>>()
            .join("\n");

        if tag_block.is_empty() {
            return template.to_string();
        }

        let tag_block = format!("{tag_block}\n");
        if let Some(head_end) = template.find("</head>") {
            let mut result = String::with_capacity(template.len() + tag_block.len());
            result.push_str(&template[..head_end]);
            result.push_str(&tag_block);
            result.push_str(&template[head_end..]);
            result
        } else {
            format!("{tag_block}{template}")
        }
    }

    pub fn runtime(&self) -> &Arc<JsExecutionRuntime> {
        &self.runtime
    }

    pub fn clear_template_cache(&self) {
        let mut cache = self.template_cache.lock();
        *cache = None;
    }

    pub async fn load_template(
        &self,
        cache_enabled: bool,
        is_dev_mode: bool,
    ) -> Result<String, RariError> {
        if cache_enabled {
            let cache = self.template_cache.lock();
            if let Some(cached_template) = cache.as_ref() {
                return Ok(cached_template.clone());
            }
        }

        let template = match self.read_template_file(is_dev_mode).await {
            Ok(content) => {
                if is_dev_mode {
                    Self::inject_vite_client_if_needed(&content)
                } else {
                    content
                }
            }
            Err(e) => {
                if is_dev_mode {
                    Self::generate_dev_template_fallback()
                } else {
                    return Err(e);
                }
            }
        };

        if cache_enabled {
            let mut cache = self.template_cache.lock();
            *cache = Some(template.clone());
        }

        Ok(template)
    }

    fn inject_vite_client_if_needed(html: &str) -> String {
        if html.contains("/@vite/client") || html.contains("@vite/client") {
            return html.to_string();
        }

        if let Some(head_end) = html.find("</head>") {
            let mut result = String::new();
            result.push_str(&html[..head_end]);
            result.push_str(
                r#"<script type="module" src="/@vite/client"></script>
<script type="module" src="/src/main.tsx"></script>
"#,
            );
            result.push_str(&html[head_end..]);
            return result;
        }

        if let Some(body_end) = html.find("</body>") {
            let mut result = String::new();
            result.push_str(&html[..body_end]);
            result.push_str(
                r#"<script type="module" src="/@vite/client"></script>
<script type="module" src="/src/main.tsx"></script>
"#,
            );
            result.push_str(&html[body_end..]);
            return result;
        }

        format!(
            r#"<script type="module" src="/@vite/client"></script>
<script type="module" src="/src/main.tsx"></script>
{html}"#
        )
    }

    fn generate_dev_template_fallback() -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>rari App</title>
    <script type="module" src="/@vite/client"></script>
    <script type="module" src="/src/main.tsx"></script>
</head>
<body>
    <div id="root"></div>
</body>
</html>"#
            .to_string()
    }

    async fn read_template_file(&self, is_dev_mode: bool) -> Result<String, RariError> {
        let possible_paths = if is_dev_mode {
            vec!["index.html", "public/index.html", "dist/index.html", "build/index.html"]
        } else {
            vec!["dist/index.html", "build/index.html", "index.html", "public/index.html"]
        };

        for path in possible_paths {
            if let Ok(content) = fs::read_to_string(path).await {
                return Ok(content);
            }
        }

        Err(RariError::internal(
            "Template file not found. Tried: index.html, public/index.html, dist/index.html, build/index.html"
                .to_string(),
        ))
    }

    pub fn inject_into_template(
        &self,
        html_content: &str,
        template: &str,
    ) -> Result<String, RariError> {
        let root_div_regex =
            Regex::new(r#"<div\s+id=["']root["'](?:\s+[^>]*)?\s*(?:/>|>\s*</div>)"#)
                .map_err(|e| RariError::internal(format!("Failed to create regex: {e}")))?;

        if !root_div_regex.is_match(template) {
            return Err(RariError::internal(
                "Template does not contain a root div with id='root'".to_string(),
            ));
        }

        let replacement = format!(r#"<div id="root">{html_content}</div>"#);

        // NoExpand: the rendered app HTML is a literal replacement, not a
        // pattern. Without it, `$0`/`$1`/`$&` in page content (e.g. a "$0.20"
        // headline) would be interpreted as capture-group references and expand
        // to the matched root div, corrupting the output.
        let result = root_div_regex.replace(template, regex::NoExpand(replacement.as_str()));

        Ok(result.to_string())
    }

    pub(crate) fn css_links_for_route(route_match: &AppRouteMatch) -> Vec<String> {
        let mut seen = FxHashSet::default();
        let mut css_links = Vec::new();

        let mut push_css = |links: &[String]| {
            for css in links {
                if !seen.contains(css.as_str()) {
                    seen.insert(css.clone());
                    css_links.push(css.clone());
                }
            }
        };

        for layout in &route_match.layouts {
            push_css(&layout.css);
        }

        if let Some(loading) = &route_match.loading {
            push_css(&loading.css);
        }

        if let Some(error) = &route_match.error {
            push_css(&error.css);
        }

        if let Some(not_found) = &route_match.not_found {
            push_css(&not_found.css);
        } else {
            push_css(&route_match.route.css);
        }

        css_links
    }

    fn mask_html_comments(template: &str) -> String {
        let mut out = template.as_bytes().to_vec();
        let mut i = 0;
        while i < out.len() {
            if out[i..].starts_with(b"<!--") {
                let start = i;
                i += 4;
                let mut closed = false;
                while i + 2 < out.len() {
                    if &out[i..i + 3] == b"-->" {
                        i += 3;
                        closed = true;
                        break;
                    }
                    i += 1;
                }
                let end = if closed { i } else { out.len() };
                for byte in &mut out[start..end] {
                    if *byte != b'\n' && *byte != b'\r' {
                        *byte = b' ';
                    }
                }
                if !closed {
                    break;
                }
                continue;
            }
            i += 1;
        }
        String::from_utf8(out)
            .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned())
    }

    fn decode_basic_html_entities(value: &str) -> String {
        value
            .cow_replace("&quot;", "\"")
            .cow_replace("&#39;", "'")
            .cow_replace("&lt;", "<")
            .cow_replace("&gt;", ">")
            .cow_replace("&amp;", "&")
            .into_owned()
    }

    fn html_attr_value(tag: &str, name: &str) -> Option<String> {
        let lower = tag.to_ascii_lowercase();
        let name = name.to_ascii_lowercase();
        let mut search_at = 0;
        while let Some(rel) = lower[search_at..].find(&name) {
            let start = search_at + rel;
            let before_ok = start == 0 || {
                let before = lower.as_bytes()[start - 1];
                !before.is_ascii_alphanumeric()
                    && before != b'-'
                    && before != b'_'
                    && before != b':'
            };
            if !before_ok {
                search_at = start + 1;
                continue;
            }

            let mut pos = start + name.len();
            while pos < tag.len() && tag.as_bytes()[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if pos >= tag.len() || tag.as_bytes()[pos] != b'=' {
                search_at = start + 1;
                continue;
            }
            pos += 1;
            while pos < tag.len() && tag.as_bytes()[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if pos >= tag.len() {
                return None;
            }

            let bytes = tag.as_bytes();
            let quote = bytes[pos];
            if quote == b'"' || quote == b'\'' {
                let value_start = pos + 1;
                let quote_char = quote as char;
                let end = tag[value_start..].find(quote_char)?;
                return Some(Self::decode_basic_html_entities(
                    &tag[value_start..value_start + end],
                ));
            }

            let value_start = pos;
            let value_end = tag[value_start..]
                .find(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/')
                .map_or(tag.len(), |offset| value_start + offset);
            return Some(Self::decode_basic_html_entities(&tag[value_start..value_end]));
        }
        None
    }

    fn template_has_href(template: &str, href: &str) -> bool {
        let searchable = Self::mask_html_comments(template);
        #[expect(clippy::unwrap_used, reason = "Hardcoded regex pattern is guaranteed to be valid")]
        let link_regex = Regex::new(r"(?i)<link\b[^>]*/?>").unwrap();
        link_regex
            .find_iter(&searchable)
            .any(|mat| Self::html_attr_value(mat.as_str(), "href").as_deref() == Some(href))
    }

    pub(crate) fn inject_css_links(template: &str, css_links: &[String]) -> String {
        if css_links.is_empty() {
            return template.to_string();
        }

        let mut stylesheet_links = Vec::new();
        let mut preload_links = Vec::new();

        for href in css_links {
            if let Some(font_url) = href.strip_prefix("preload:") {
                if Self::template_has_href(template, font_url) {
                    continue;
                }
                let type_attr = if font_url.ends_with(".woff") {
                    "font/woff"
                } else if font_url.ends_with(".ttf") {
                    "font/ttf"
                } else if font_url.ends_with(".otf") {
                    "font/otf"
                } else {
                    "font/woff2"
                };
                preload_links.push(format!(
                    r#"<link rel="preload" href="{}" as="font" type="{}" crossorigin>"#,
                    Self::escape_html_attribute(font_url),
                    type_attr
                ));
            } else if !Self::template_has_href(template, href) {
                stylesheet_links.push(format!(
                    r#"<link rel="stylesheet" href="{}">"#,
                    Self::escape_html_attribute(href)
                ));
            }
        }

        if preload_links.is_empty() && stylesheet_links.is_empty() {
            return template.to_string();
        }

        let mut result = template.to_string();
        let has_head = result.find("</head>").is_some();

        if !has_head {
            let mut combined = Vec::with_capacity(preload_links.len() + stylesheet_links.len());
            combined.extend(preload_links);
            combined.extend(stylesheet_links);
            let block = format!("{}\n", combined.join("\n"));
            return format!("{block}{result}");
        }

        if !preload_links.is_empty() {
            let preload_block = format!("{}\n", preload_links.join("\n"));
            let insert_at =
                Self::first_stylesheet_link_offset(&result).or_else(|| result.find("</head>"));
            if let Some(pos) = insert_at {
                result.insert_str(pos, &preload_block);
            }
        }

        if !stylesheet_links.is_empty() {
            let stylesheet_block = format!("{}\n", stylesheet_links.join("\n"));
            if let Some(head_end) = result.find("</head>") {
                result.insert_str(head_end, &stylesheet_block);
            }
        }

        result
    }

    fn first_stylesheet_link_offset(template: &str) -> Option<usize> {
        let searchable = Self::mask_html_comments(template);
        let lower = searchable.to_ascii_lowercase();
        let mut search_from = 0;
        while let Some(rel) = lower[search_from..].find("<link") {
            let start = search_from + rel;
            let end = lower[start..].find('>').map(|offset| start + offset + 1)?;
            let tag = &lower[start..end];
            if tag.contains("stylesheet") || tag.contains("text/css") {
                return Some(start);
            }
            search_from = end;
        }
        None
    }

    pub(crate) async fn assemble_document(
        &self,
        html_content: String,
        cache_template: bool,
        is_dev_mode: bool,
        css_links: &[String],
    ) -> Result<String, RariError> {
        let is_complete_document = html_content.trim_start().starts_with("<!DOCTYPE")
            || html_content.trim_start().cow_to_lowercase().starts_with("<html");

        if is_complete_document {
            let (script_tags, head_link_tags) = if is_dev_mode {
                (String::new(), String::new())
            } else {
                let template = self.load_template(cache_template, is_dev_mode).await?;
                (
                    Self::extract_script_tags(&template),
                    Self::extract_non_stylesheet_link_tags(&template),
                )
            };

            let mut final_html = html_content;

            if !script_tags.is_empty()
                && let Some(body_end) = final_html.rfind("</body>")
            {
                final_html.insert_str(body_end, &format!("\n{script_tags}\n"));
            }

            final_html = Self::inject_css_links(&final_html, css_links);
            final_html = Self::inject_head_tags(&final_html, &head_link_tags);

            let trimmed_lower = final_html.trim_start().cow_to_lowercase();
            if !trimmed_lower.starts_with("<!doctype") {
                final_html = format!("<!DOCTYPE html>\n{final_html}");
            }

            return Ok(final_html);
        }

        let template = self.load_template(cache_template, is_dev_mode).await?;
        let template = Self::inject_css_links(&template, css_links);
        self.inject_into_template(&html_content, &template)
    }

    fn escape_html_attribute(text: &str) -> String {
        text.cow_replace('&', "&amp;")
            .cow_replace('"', "&quot;")
            .cow_replace('<', "&lt;")
            .cow_replace('>', "&gt;")
            .into_owned()
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used, clippy::clone_on_ref_ptr)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;
    use crate::server::routing::app_router::{
        AppRouteEntry, AppRouteMatch, LayoutEntry, LoadingEntry,
    };

    fn sample_route_match() -> AppRouteMatch {
        AppRouteMatch {
            route: AppRouteEntry {
                path: "/".to_string(),
                file_path: "page.tsx".to_string(),
                component_id: None,
                css: vec!["/page.css".to_string(), "/shared.css".to_string()],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params: FxHashMap::default(),
            layouts: vec![LayoutEntry {
                path: "/".to_string(),
                file_path: "layout.tsx".to_string(),
                component_id: None,
                css: vec!["/layout.css".to_string(), "/shared.css".to_string()],
                parent_path: None,
                is_root: true,
                additional_paths: None,
            }],
            loading: Some(LoadingEntry {
                path: "/loading".to_string(),
                file_path: "loading.tsx".to_string(),
                component_id: None,
                css: vec!["/loading.css".to_string()],
                additional_paths: None,
            }),
            error: None,
            not_found: None,
            templates: vec![],
            pathname: "/".to_string(),
        }
    }

    #[test]
    fn test_rsc_html_renderer_creation() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime.clone());

        assert!(Arc::ptr_eq(renderer.runtime(), &runtime));
    }

    #[test]
    fn test_template_cache_clear() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        {
            let mut cache = renderer.template_cache.lock();
            *cache = Some("<html></html>".to_string());
        }

        renderer.clear_template_cache();

        {
            let cache = renderer.template_cache.lock();
            assert!(cache.is_none());
        }
    }

    #[test]
    fn test_generate_dev_template_fallback() {
        let template = RscHtmlRenderer::generate_dev_template_fallback();
        assert!(template.contains("<!DOCTYPE html>"));
        assert!(template.contains(r#"<div id="root""#));
        assert!(template.contains("/@vite/client"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(
            escape_html("<script>alert('x')</script>"),
            "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"
        );
        assert_eq!(escape_html("Dumb & Dumber"), "Dumb &amp; Dumber");
        assert_eq!(escape_html(r#""quoted""#), "&quot;quoted&quot;");
    }

    #[test]
    fn test_inject_css_links() {
        let template = "<html><head></head><body></body></html>";
        let css_links = vec!["/styles/app.css".to_string()];
        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert!(result.contains(r#"<link rel="stylesheet" href="/styles/app.css">"#));
    }

    #[test]
    fn test_inject_css_links_font_preloads() {
        let template = "<html><head></head><body></body></html>";
        let css_links = vec![
            "preload:/assets/Geist-abcd1234.woff2".to_string(),
            "/assets/server/comp.css".to_string(),
        ];
        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert!(result.contains(
            r#"<link rel="preload" href="/assets/Geist-abcd1234.woff2" as="font" type="font/woff2" crossorigin>"#
        ));
        assert!(result.contains(r#"<link rel="stylesheet" href="/assets/server/comp.css">"#));
        let preload_pos = result.find("rel=\"preload\"").expect("preload");
        let style_pos = result.find("rel=\"stylesheet\"").expect("stylesheet");
        assert!(preload_pos < style_pos);
    }

    #[test]
    fn test_inject_css_links_preloads_before_existing_stylesheet() {
        let template = r#"<html><head>
<link rel="stylesheet" href="/existing.css">
</head><body></body></html>"#;
        let css_links = vec![
            "preload:/assets/Geist-abcd1234.woff2".to_string(),
            "/assets/server/comp.css".to_string(),
        ];
        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        let preload_pos = result.find("rel=\"preload\"").expect("preload");
        let existing_pos = result.find("/existing.css").expect("existing stylesheet");
        let generated_pos = result.find("/assets/server/comp.css").expect("generated stylesheet");
        assert!(preload_pos < existing_pos);
        assert!(existing_pos < generated_pos);
    }

    #[test]
    fn test_inject_css_links_no_head_preserves_preload_before_stylesheet() {
        let template = "<html><body>hi</body></html>";
        let css_links = vec![
            "preload:/assets/Geist-abcd1234.woff2".to_string(),
            "/assets/server/comp.css".to_string(),
        ];
        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        let preload_pos = result.find("rel=\"preload\"").expect("preload");
        let style_pos = result.find("rel=\"stylesheet\"").expect("stylesheet");
        assert!(preload_pos < style_pos);
        assert!(result.starts_with("<link rel=\"preload\""));
    }

    #[test]
    fn test_inject_into_template() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);
        let template = r#"<!DOCTYPE html><html><body><div id="root"></div></body></html>"#;
        let html = renderer.inject_into_template("<p>Hello</p>", template).unwrap();
        assert!(html.contains(r#"<div id="root"><p>Hello</p></div>"#));
    }

    #[test]
    fn test_inject_into_template_preserves_dollar_sequences() {
        // Regression: `$0`/`$1`/`$&` in page content must not be expanded as
        // regex capture references during root-div injection.
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let template = r#"<html><body><div id="root"></div></body></html>"#;
        let content = r"<h1>XLM eyes $0.20 breakout</h1><p>$1 &amp; $&amp;</p>";

        let html = renderer.inject_into_template(content, template).expect("inject should succeed");

        assert!(
            html.contains(
                r#"<div id="root"><h1>XLM eyes $0.20 breakout</h1><p>$1 &amp; $&amp;</p></div>"#
            ),
            "dollar sequences must survive verbatim, got: {html}"
        );
    }

    #[test]
    fn test_inject_into_template_self_closing_root_div() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let template = r#"<html><body><div id="root"/></body></html>"#;
        let html = renderer.inject_into_template("<p>Hi</p>", template).unwrap();
        assert!(html.contains(r#"<div id="root"><p>Hi</p></div>"#));
    }

    #[test]
    fn test_extract_non_stylesheet_link_tags() {
        let template = r#"<html><head>
<link rel="stylesheet" href="/app.css">
<link rel="icon" href="/favicon.ico">
<link rel="preload" href="/font.woff2" as="font">
</head></html>"#;

        let tags = RscHtmlRenderer::extract_non_stylesheet_link_tags(template);
        assert!(tags.contains(r#"<link rel="icon" href="/favicon.ico">"#));
        assert!(tags.contains(r#"<link rel="preload" href="/font.woff2" as="font">"#));
        assert!(!tags.contains("stylesheet"));
    }

    #[test]
    fn test_inject_head_tags_deduplicates_existing_tags() {
        let html = r#"<!DOCTYPE html><html><head>
<link rel="icon" href="/favicon.ico">
</head><body></body></html>"#;
        let tags = r#"<link rel="icon" href="/favicon.ico">
<link rel="manifest" href="/manifest.webmanifest">"#;

        let result = RscHtmlRenderer::inject_head_tags(html, tags);
        assert_eq!(result.matches("/favicon.ico").count(), 1);
        assert!(result.contains("/manifest.webmanifest"));
    }

    #[test]
    fn test_inject_css_links_skips_existing_href() {
        let template =
            r#"<html><head><link rel="stylesheet" href="/styles/app.css"></head></html>"#;
        let css_links = vec!["/styles/app.css".to_string(), "/styles/new.css".to_string()];

        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert_eq!(result.matches("/styles/app.css").count(), 1);
        assert!(result.contains("/styles/new.css"));
    }

    #[test]
    fn test_inject_css_links_ignores_href_text_outside_attributes() {
        let template = r#"<html><head><!-- Using font: /assets/Geist-abcd1234.woff2 --></head>
<body>See /styles/app.css in the docs</body></html>"#;
        let css_links =
            vec!["preload:/assets/Geist-abcd1234.woff2".to_string(), "/styles/app.css".to_string()];

        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert!(result.contains(
            r#"<link rel="preload" href="/assets/Geist-abcd1234.woff2" as="font" type="font/woff2" crossorigin>"#
        ));
        assert!(result.contains(r#"<link rel="stylesheet" href="/styles/app.css">"#));
        assert_eq!(result.matches("/assets/Geist-abcd1234.woff2").count(), 2);
        assert_eq!(result.matches("/styles/app.css").count(), 2);
    }

    #[test]
    fn test_inject_css_links_ignores_anchor_and_commented_link_hrefs() {
        let template = r#"<html><head>
<!-- <link rel="stylesheet" href="/styles/app.css"> -->
<!-- <link rel="preload" href="/assets/Geist-abcd1234.woff2" as="font"> -->
</head>
<body><a href="/styles/app.css">docs</a>
<a href="/assets/Geist-abcd1234.woff2">font</a>
</body></html>"#;
        let css_links =
            vec!["preload:/assets/Geist-abcd1234.woff2".to_string(), "/styles/app.css".to_string()];

        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert!(result.contains(
            r#"<link rel="preload" href="/assets/Geist-abcd1234.woff2" as="font" type="font/woff2" crossorigin>"#
        ));
        assert!(result.contains(r#"<link rel="stylesheet" href="/styles/app.css">"#));
        assert!(result.contains(r#"<a href="/styles/app.css">docs</a>"#));
        assert!(result.contains(r#"<a href="/assets/Geist-abcd1234.woff2">font</a>"#));
        assert!(result.contains("<!-- <link rel=\"stylesheet\" href=\"/styles/app.css\"> -->"));
    }

    #[test]
    fn test_css_links_for_route_deduplicates() {
        let links = RscHtmlRenderer::css_links_for_route(&sample_route_match());

        assert_eq!(
            links,
            vec![
                "/layout.css".to_string(),
                "/shared.css".to_string(),
                "/loading.css".to_string(),
                "/page.css".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn test_assemble_document_wraps_fragment_in_dev_template() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let html = renderer
            .assemble_document("<main>Page</main>".to_string(), false, true, &[])
            .await
            .expect("assemble_document should succeed");

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains(r#"<div id="root"><main>Page</main></div>"#));
        assert!(html.contains("/@vite/client"));
    }

    #[tokio::test]
    async fn test_assemble_document_complete_doc_injects_css() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);
        let css_links = vec!["/extra.css".to_string()];
        let html_content =
            "<!DOCTYPE html><html><head></head><body><main>Page</main></body></html>";

        let html = renderer
            .assemble_document(html_content.to_string(), false, true, &css_links)
            .await
            .expect("assemble_document should succeed");

        assert!(html.contains(r#"<link rel="stylesheet" href="/extra.css">"#));
        assert!(html.contains("<main>Page</main>"));
    }
}
