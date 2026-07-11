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

        let result = root_div_regex.replace(template, replacement.as_str());

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

    pub(crate) fn inject_css_links(template: &str, css_links: &[String]) -> String {
        if css_links.is_empty() {
            return template.to_string();
        }

        let links = css_links
            .iter()
            .filter(|href| !template.contains(href.as_str()))
            .map(|href| {
                format!(r#"<link rel="stylesheet" href="{}">"#, Self::escape_html_attribute(href))
            })
            .collect::<Vec<_>>();

        if links.is_empty() {
            return template.to_string();
        }

        let link_block = format!("{}\n", links.join("\n"));
        if let Some(head_end) = template.find("</head>") {
            let mut result = String::with_capacity(template.len() + link_block.len());
            result.push_str(&template[..head_end]);
            result.push_str(&link_block);
            result.push_str(&template[head_end..]);
            result
        } else {
            format!("{link_block}{template}")
        }
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
    fn test_inject_into_template() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);
        let template = r#"<!DOCTYPE html><html><body><div id="root"></div></body></html>"#;
        let html = renderer.inject_into_template("<p>Hello</p>", template).unwrap();
        assert!(html.contains(r#"<div id="root"><p>Hello</p></div>"#));
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
