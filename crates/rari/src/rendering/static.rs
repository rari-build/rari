#![expect(clippy::missing_errors_doc)]

use std::{path::PathBuf, sync::Arc};

use cow_utils::CowUtils;
use rari_error::RariError;
use rustc_hash::FxHashSet;
use tokio::fs;

use crate::{runtime::JsExecutionRuntime, server::routing::app_router::AppRouteMatch};

fn split_head_inject_units(tags: &str) -> Vec<String> {
    let mut units = Vec::new();
    let mut rest = tags.trim();

    while !rest.is_empty() {
        let trimmed = rest.trim_start();
        if trimmed.is_empty() {
            break;
        }

        let trimmed_lower = trimmed.to_ascii_lowercase();
        if trimmed_lower.starts_with("<script") {
            const CLOSE: &str = "</script>";
            if let Some(rel) = trimmed_lower.find(CLOSE) {
                let end = rel + CLOSE.len();
                units.push(trimmed[..end].trim_end().to_string());
                rest = &trimmed[end..];
                continue;
            }
        }

        if let Some(nl) = trimmed.find('\n') {
            let line = trimmed[..nl].trim();
            if !line.is_empty() {
                units.push(line.to_string());
            }
            rest = &trimmed[nl + 1..];
        } else {
            units.push(trimmed.to_string());
            break;
        }
    }

    units
}

fn find_closing_head_tag(html: &str) -> Option<usize> {
    const NEEDLE: &[u8] = b"</head>";
    let mut masked = html.as_bytes().to_vec();
    RscHtmlRenderer::mask_html_for_head_scan_in_place(&mut masked);
    if masked.len() < NEEDLE.len() {
        return None;
    }
    masked.windows(NEEDLE.len()).position(|window| window.eq_ignore_ascii_case(NEEDLE))
}

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
    public_dir: PathBuf,
}

impl RscHtmlRenderer {
    pub fn new(runtime: Arc<JsExecutionRuntime>) -> Self {
        Self::with_public_dir(runtime, PathBuf::from("dist"))
    }

    pub fn with_public_dir(runtime: Arc<JsExecutionRuntime>, public_dir: PathBuf) -> Self {
        Self { runtime, template_cache: parking_lot::Mutex::new(None), public_dir }
    }

    pub(crate) fn inject_head_tags(template: &str, tags: &str) -> String {
        let tags = tags.trim();
        if tags.is_empty() {
            return template.to_string();
        }

        let tag_block = split_head_inject_units(tags)
            .into_iter()
            .filter(|unit| !template.contains(unit.as_str()))
            .collect::<Vec<_>>()
            .join("\n");

        if tag_block.is_empty() {
            return template.to_string();
        }

        let tag_block = format!("{tag_block}\n");
        if let Some(head_end) = find_closing_head_tag(template) {
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

        let template = if is_dev_mode {
            Self::generate_dev_client_head()
        } else {
            self.read_client_head_file().await?
        };

        if cache_enabled {
            let mut cache = self.template_cache.lock();
            *cache = Some(template.clone());
        }

        Ok(template)
    }

    fn generate_dev_client_head() -> String {
        r#"<script type="module" src="/@vite/client"></script>
<script type="module">
import 'virtual:rari-entry-client';
</script>
"#
        .to_string()
    }

    async fn read_client_head_file(&self) -> Result<String, RariError> {
        let path = self.public_dir.join("rari-client-head.html");
        fs::read_to_string(&path).await.map_err(|_| {
            RariError::internal(format!("Client head file not found. Tried: {}", path.display()))
        })
    }

    pub(crate) fn client_head_fragment(template: &str) -> &str {
        if let Some(start) = template.find("<head>")
            && let Some(end) = template.find("</head>")
            && end >= start + 6
        {
            return &template[start + 6..end];
        }
        template
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

    fn mask_byte_range_preserve_newlines(out: &mut [u8], start: usize, end: usize) {
        for byte in &mut out[start..end] {
            if *byte != b'\n' && *byte != b'\r' {
                *byte = b' ';
            }
        }
    }

    fn mask_html_comments_in_place(out: &mut [u8]) {
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
                Self::mask_byte_range_preserve_newlines(out, start, end);
                if !closed {
                    break;
                }
                continue;
            }
            i += 1;
        }
    }

    fn find_ascii_tag_ci(haystack: &[u8], from: usize, open: &[u8]) -> Option<usize> {
        let mut i = from;
        while i + open.len() <= haystack.len() {
            if haystack[i..i + open.len()].eq_ignore_ascii_case(open) {
                let after = i + open.len();
                let boundary_ok = after >= haystack.len()
                    || matches!(haystack[after], b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r');
                if boundary_ok {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    fn mask_raw_text_element_in_place(out: &mut [u8], open: &[u8], close: &[u8]) {
        let mut i = 0;
        while let Some(start) = Self::find_ascii_tag_ci(out, i, open) {
            let Some(open_end_rel) = out[start..].iter().position(|&b| b == b'>') else {
                break;
            };
            let open_end = start + open_end_rel + 1;
            let self_closing = open_end >= 2 && out[open_end - 2] == b'/';
            if self_closing {
                i = open_end;
                continue;
            }

            let mut close_at = None;
            let mut search = open_end;
            while search + close.len() <= out.len() {
                if out[search..search + close.len()].eq_ignore_ascii_case(close) {
                    close_at = Some(search);
                    break;
                }
                search += 1;
            }

            let end = close_at.map_or(out.len(), |pos| pos + close.len());
            Self::mask_byte_range_preserve_newlines(out, start, end);
            if close_at.is_none() {
                break;
            }
            i = end;
        }
    }

    fn mask_html_for_link_scan(template: &str) -> String {
        let mut out = template.as_bytes().to_vec();
        Self::mask_html_comments_in_place(&mut out);
        Self::mask_raw_text_element_in_place(&mut out, b"<script", b"</script>");
        Self::mask_raw_text_element_in_place(&mut out, b"<style", b"</style>");
        String::from_utf8(out)
            .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned())
    }

    fn mask_html_for_head_scan_in_place(out: &mut [u8]) {
        Self::mask_html_comments_in_place(out);
        Self::mask_raw_text_element_in_place(out, b"<script", b"</script>");
        Self::mask_raw_text_element_in_place(out, b"<style", b"</style>");
        Self::mask_raw_text_element_in_place(out, b"<title", b"</title>");
        Self::mask_raw_text_element_in_place(out, b"<textarea", b"</textarea>");
        Self::mask_raw_text_element_in_place(out, b"<noscript", b"</noscript>");
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
        let bytes = tag.as_bytes();
        let want = name.to_ascii_lowercase();
        let mut i = 0;

        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
            i += 1;
        }

        while i < bytes.len() {
            while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] == b'>' {
                break;
            }

            let name_start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'='
                && bytes[i] != b'>'
                && bytes[i] != b'/'
            {
                i += 1;
            }
            let attr_name = tag[name_start..i].to_ascii_lowercase();

            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            if i < bytes.len() && bytes[i] == b'=' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }

                let value = if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                    let quote = bytes[i];
                    i += 1;
                    let value_start = i;
                    while i < bytes.len() && bytes[i] != quote {
                        i += 1;
                    }
                    let raw = &tag[value_start..i];
                    if i < bytes.len() {
                        i += 1;
                    }
                    Self::decode_basic_html_entities(raw)
                } else {
                    let value_start = i;
                    while i < bytes.len()
                        && !bytes[i].is_ascii_whitespace()
                        && bytes[i] != b'>'
                        && bytes[i] != b'"'
                        && bytes[i] != b'\''
                        && bytes[i] != b'<'
                    {
                        i += 1;
                    }
                    Self::decode_basic_html_entities(&tag[value_start..i])
                };

                if attr_name == want {
                    return Some(value);
                }
            } else if attr_name == want {
                return Some(String::new());
            }
        }

        None
    }

    fn find_html_tag_end(source: &str) -> Option<usize> {
        let bytes = source.as_bytes();
        if bytes.first().copied() != Some(b'<') {
            return None;
        }
        let mut i = 1;
        let mut quote: Option<u8> = None;
        while i < bytes.len() {
            let byte = bytes[i];
            if let Some(q) = quote {
                if byte == q {
                    quote = None;
                }
            } else if byte == b'"' || byte == b'\'' {
                quote = Some(byte);
            } else if byte == b'>' {
                return Some(i + 1);
            }
            i += 1;
        }
        None
    }

    fn for_each_link_tag(template: &str, mut visit: impl FnMut(&str, usize) -> bool) {
        let searchable = Self::mask_html_for_link_scan(template);
        let lower = searchable.to_ascii_lowercase();
        let mut search_from = 0;
        while let Some(rel) = lower[search_from..].find("<link") {
            let start = search_from + rel;
            let after = start + 5;
            let boundary_ok = after >= lower.len()
                || matches!(lower.as_bytes()[after], b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r');
            if !boundary_ok {
                search_from = after;
                continue;
            }
            let Some(end) = Self::find_html_tag_end(&searchable[start..]).map(|len| start + len)
            else {
                break;
            };
            if visit(&searchable[start..end], start) {
                return;
            }
            search_from = end;
        }
    }

    fn template_has_href(template: &str, href: &str) -> bool {
        let mut found = false;
        Self::for_each_link_tag(template, |tag, _start| {
            if Self::html_attr_value(tag, "href").as_deref() == Some(href) {
                found = true;
                true
            } else {
                false
            }
        });
        found
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
        let has_head = find_closing_head_tag(&result).is_some();

        if !has_head {
            let mut combined = Vec::with_capacity(preload_links.len() + stylesheet_links.len());
            combined.extend(preload_links);
            combined.extend(stylesheet_links);
            let block = format!("{}\n", combined.join("\n"));
            return format!("{block}{result}");
        }

        if !preload_links.is_empty() {
            let preload_block = format!("{}\n", preload_links.join("\n"));
            let insert_at = Self::first_stylesheet_link_offset(&result)
                .or_else(|| find_closing_head_tag(&result));
            if let Some(pos) = insert_at {
                result.insert_str(pos, &preload_block);
            }
        }

        if !stylesheet_links.is_empty() {
            let stylesheet_block = format!("{}\n", stylesheet_links.join("\n"));
            if let Some(head_end) = find_closing_head_tag(&result) {
                result.insert_str(head_end, &stylesheet_block);
            }
        }

        result
    }

    fn first_stylesheet_link_offset(template: &str) -> Option<usize> {
        let mut found = None;
        Self::for_each_link_tag(template, |tag, start| {
            let lower = tag.to_ascii_lowercase();
            if lower.contains("stylesheet") || lower.contains("text/css") {
                found = Some(start);
                true
            } else {
                false
            }
        });
        found
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

        if !is_complete_document {
            return Err(RariError::internal(
                "Expected a complete HTML document from the root layout (<html>...</html>)"
                    .to_string(),
            ));
        }

        let client_head = if is_dev_mode {
            String::new()
        } else {
            self.load_template(cache_template, is_dev_mode).await?
        };

        let mut final_html = html_content;
        final_html = Self::inject_head_tags(&final_html, &client_head);
        final_html = Self::inject_css_links(&final_html, css_links);

        let trimmed_lower = final_html.trim_start().cow_to_lowercase();
        if !trimmed_lower.starts_with("<!doctype") {
            final_html = format!("<!DOCTYPE html>\n{final_html}");
        }

        Ok(final_html)
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
#[expect(clippy::expect_used, clippy::clone_on_ref_ptr)]
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
    fn test_generate_dev_client_head() {
        let template = RscHtmlRenderer::generate_dev_client_head();
        assert!(template.contains("/@vite/client"));
        assert!(template.contains("virtual:rari-entry-client"));
        assert!(!template.contains("<!DOCTYPE html>"));
        assert!(!template.contains(r#"id="root""#));
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
    fn test_inject_css_links_uppercase_closing_head() {
        let template = "<html><HEAD></HEAD><body></body></html>";
        let css_links = vec!["/styles/app.css".to_string()];
        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        let head_close = result.find("</HEAD>").expect("preserves casing");
        let link_pos = result.find(r#"href="/styles/app.css""#).expect("css link");
        assert!(link_pos < head_close);
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
    fn test_inject_head_tags_keeps_multiline_script_when_closing_tag_exists() {
        let html = r#"<!DOCTYPE html><html><head>
<script type="module" src="/other.js"></script>
</head><body></body></html>"#;
        let tags = r#"<script type="module">
import '/entry.js';
</script>"#;

        let result = RscHtmlRenderer::inject_head_tags(html, tags);
        assert!(result.contains("import '/entry.js';"));
        assert!(
            result.contains("</script>\n</head>")
                || result.contains("import '/entry.js';\n</script>"),
            "injected multiline script must keep its closing tag; got:\n{result}"
        );
        assert_eq!(result.matches("</script>").count(), 2);
    }

    #[test]
    fn test_inject_head_tags_dedupes_multiline_script_as_unit() {
        let script = r#"<script type="module">
import '/entry.js';
</script>"#;
        let html = format!("<!DOCTYPE html><html><head>\n{script}\n</head><body></body></html>");

        let result = RscHtmlRenderer::inject_head_tags(&html, script);
        assert_eq!(result.matches("import '/entry.js';").count(), 1);
        assert_eq!(result.matches("</script>").count(), 1);
    }

    #[test]
    fn test_inject_head_tags_finds_uppercase_closing_head() {
        let html = "<!DOCTYPE html><html><HEAD></HEAD><body></body></html>";
        let tags = r#"<script type="module" src="/entry.js"></script>"#;

        let result = RscHtmlRenderer::inject_head_tags(html, tags);
        assert!(result.contains("</HEAD>"));
        assert!(result.contains(r#"src="/entry.js""#));
        let head_close = result.find("</HEAD>").expect("preserves original closing tag");
        let script_pos = result.find(r#"src="/entry.js""#).expect("script inserted");
        assert!(script_pos < head_close);
    }

    #[test]
    fn test_inject_head_tags_ignores_false_head_in_script_and_comment() {
        let html = r#"<!DOCTYPE html><html><head>
<!-- fake </head> in comment -->
<script>const s = "</head>";</script>
<style>.x::before { content: "</head>"; }</style>
</head><body></body></html>"#;
        let tags = r#"<script type="module" src="/entry.js"></script>"#;

        let result = RscHtmlRenderer::inject_head_tags(html, tags);
        let real_close = result.rfind("</head>").expect("real closing head");
        let script_pos = result.find(r#"src="/entry.js""#).expect("injected script");
        assert!(script_pos < real_close);
        assert!(
            result[..script_pos].contains(r#"const s = "</head>";"#),
            "must not inject inside the script string"
        );
        assert_eq!(result.matches(r#"src="/entry.js""#).count(), 1);
    }

    #[test]
    fn test_find_closing_head_tag_across_chunk_concatenation() {
        let chunk1 = "<html><head><script>var x = '</he";
        let chunk2 = "ad>';</script></head><body></body></html>";
        let combined = format!("{chunk1}{chunk2}");
        let idx = find_closing_head_tag(&combined).expect("real head close");
        assert_eq!(&combined[idx..idx + 7], "</head>");
        assert!(idx > combined.find("<script>").expect("script"));
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
        let template = r"<html><head><!-- Using font: /assets/Geist-abcd1234.woff2 --></head>
<body>See /styles/app.css in the docs</body></html>";
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
    fn test_inject_css_links_ignores_link_text_inside_script_and_style() {
        let template = r#"<html><head>
<script>const hint = '<link rel="stylesheet" href="/styles/app.css">';</script>
<style>/* <link rel="preload" href="/assets/Geist-abcd1234.woff2" as="font"> */</style>
</head><body></body></html>"#;
        let css_links =
            vec!["preload:/assets/Geist-abcd1234.woff2".to_string(), "/styles/app.css".to_string()];

        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert!(result.contains(
            r#"<link rel="preload" href="/assets/Geist-abcd1234.woff2" as="font" type="font/woff2" crossorigin>"#
        ));
        assert!(result.contains(r#"<link rel="stylesheet" href="/styles/app.css">"#));
        assert!(
            result.contains(r#"const hint = '<link rel="stylesheet" href="/styles/app.css">';"#)
        );
    }

    #[test]
    fn test_inject_css_links_ignores_href_text_inside_other_attributes() {
        let template = r#"<html><head>
<link rel="preload" as="font" data-note="see href='/assets/Geist-abcd1234.woff2'" crossorigin>
<link data-doc="href='/styles/app.css'" rel="icon" href="/favicon.ico">
</head><body></body></html>"#;
        let css_links =
            vec!["preload:/assets/Geist-abcd1234.woff2".to_string(), "/styles/app.css".to_string()];

        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert!(result.contains(
            r#"<link rel="preload" href="/assets/Geist-abcd1234.woff2" as="font" type="font/woff2" crossorigin>"#
        ));
        assert!(result.contains(r#"<link rel="stylesheet" href="/styles/app.css">"#));
        assert!(result.contains(r#"data-note="see href='/assets/Geist-abcd1234.woff2'""#));
        assert!(result.contains(r#"data-doc="href='/styles/app.css'""#));
    }

    #[test]
    fn test_inject_css_links_handles_quoted_gt_and_unquoted_href_paths() {
        let template = r#"<html><head>
<link rel="stylesheet" data-note=">" href=/existing.css>
</head><body></body></html>"#;
        let css_links = vec![
            "preload:/assets/Geist-abcd1234.woff2".to_string(),
            "/existing.css".to_string(),
            "/styles/app.css".to_string(),
        ];

        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert!(result.contains(
            r#"<link rel="preload" href="/assets/Geist-abcd1234.woff2" as="font" type="font/woff2" crossorigin>"#
        ));
        assert!(result.contains(r#"<link rel="stylesheet" href="/styles/app.css">"#));
        assert_eq!(result.matches("href=/existing.css").count(), 1);
        assert_eq!(result.matches(r#"href="/existing.css""#).count(), 0);
        let preload_pos = result.find("rel=\"preload\"").expect("preload");
        let existing_pos = result.find("href=/existing.css").expect("existing stylesheet");
        assert!(preload_pos < existing_pos);
    }

    #[test]
    fn test_inject_css_links_dedupes_unquoted_href_with_query_string() {
        let template = r#"<html><head>
<link rel="stylesheet" href=/styles/app.css?v=42>
</head><body></body></html>"#;
        let css_links = vec!["/styles/app.css?v=42".to_string()];

        let result = RscHtmlRenderer::inject_css_links(template, &css_links);
        assert_eq!(result.matches("href=/styles/app.css?v=42").count(), 1);
        assert_eq!(result.matches(r#"href="/styles/app.css?v=42""#).count(), 0);
        assert!(!result.contains(r#"rel="stylesheet" href="/styles/app.css?v=42""#));
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
    async fn test_assemble_document_rejects_fragment() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let err = renderer
            .assemble_document("<main>Page</main>".to_string(), false, true, &[])
            .await
            .expect_err("fragment HTML should be rejected");

        assert!(err.to_string().contains("complete HTML document"));
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
