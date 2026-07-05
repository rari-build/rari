#![expect(clippy::missing_errors_doc)]

use std::{future::Future, pin::Pin, string::ToString, sync::Arc, time::Duration};

use cow_utils::CowUtils;
use rari_error::RariError;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;
use tokio::{fs, time};

use crate::{
    RscElement,
    runtime::JsExecutionRuntime,
    server::{config::Config, routing::app_router::AppRouteMatch},
};

const SELF_CLOSING_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

pub fn escape_html(text: &str) -> String {
    text.cow_replace('&', "&amp;")
        .cow_replace('<', "&lt;")
        .cow_replace('>', "&gt;")
        .cow_replace('"', "&quot;")
        .cow_replace('\'', "&#39;")
        .into_owned()
}

fn is_valid_attribute_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let name_lower = name.cow_to_lowercase();
    if name_lower.starts_with("on") {
        return false;
    }

    if name
        .chars()
        .any(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '"' | '\'' | '=' | '<' | '>' | '/' | '\\'))
    {
        return false;
    }

    if !name.chars().any(|c| c.is_alphanumeric() || c == '_') {
        return false;
    }

    let mut chars = name.chars();

    if let Some(first) = chars.next() {
        if !(first.is_alphabetic() || first == '_' || first == ':') {
            return false;
        }
    } else {
        return false;
    }

    chars.all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
}

fn is_boolean_html_attribute(name: &str) -> bool {
    matches!(
        name,
        "allowfullscreen"
            | "async"
            | "autofocus"
            | "autoplay"
            | "checked"
            | "controls"
            | "default"
            | "defer"
            | "disabled"
            | "formnovalidate"
            | "hidden"
            | "inert"
            | "ismap"
            | "itemscope"
            | "loop"
            | "multiple"
            | "muted"
            | "nomodule"
            | "novalidate"
            | "open"
            | "playsinline"
            | "readonly"
            | "required"
            | "reversed"
            | "selected"
    )
}

#[cfg(test)]
pub fn test_is_valid_attribute_name(name: &str) -> bool {
    is_valid_attribute_name(name)
}

#[expect(clippy::too_many_lines)]
fn serialize_style_object(style_obj: &serde_json::Map<String, Value>) -> String {
    const UNITLESS_PROPERTIES: &[&str] = &[
        "animation-iteration-count",
        "border-image-outset",
        "border-image-slice",
        "border-image-width",
        "box-flex",
        "box-flex-group",
        "box-ordinal-group",
        "column-count",
        "columns",
        "flex",
        "flex-grow",
        "flex-positive",
        "flex-shrink",
        "flex-negative",
        "flex-order",
        "grid-area",
        "grid-row",
        "grid-row-end",
        "grid-row-span",
        "grid-row-start",
        "grid-column",
        "grid-column-end",
        "grid-column-span",
        "grid-column-start",
        "font-weight",
        "line-clamp",
        "line-height",
        "opacity",
        "order",
        "orphans",
        "tab-size",
        "widows",
        "z-index",
        "zoom",
        "fill-opacity",
        "flood-opacity",
        "stop-opacity",
        "stroke-dasharray",
        "stroke-dashoffset",
        "stroke-miterlimit",
        "stroke-opacity",
        "stroke-width",
    ];

    let style_parts: Vec<String> = style_obj
        .iter()
        .filter_map(|(k, v)| {
            let kebab_key = k.chars().fold(String::with_capacity(k.len() + 4), |mut acc, c| {
                if c.is_ascii_uppercase() {
                    acc.push('-');
                    acc.push(c.to_ascii_lowercase());
                } else {
                    acc.push(c);
                }
                acc
            });

            let value_str = if let Some(s) = v.as_str() {
                Some(s.to_owned())
            } else if v.is_null() || v.as_bool().is_some() {
                None
            } else if let Some(u) = v.as_u64() {
                if UNITLESS_PROPERTIES.contains(&kebab_key.as_str()) {
                    Some(u.to_string())
                } else {
                    let mut result = String::with_capacity(u.to_string().len() + 2);
                    result.push_str(&u.to_string());
                    result.push_str("px");
                    Some(result)
                }
            } else if let Some(i) = v.as_i64() {
                if UNITLESS_PROPERTIES.contains(&kebab_key.as_str()) {
                    Some(i.to_string())
                } else {
                    let mut result = String::with_capacity(i.to_string().len() + 2);
                    result.push_str(&i.to_string());
                    result.push_str("px");
                    Some(result)
                }
            } else if let Some(f) = v.as_f64() {
                if f.is_finite() {
                    let temp = format!("{f:.10}");
                    let formatted = temp.trim_end_matches('0').trim_end_matches('.');

                    if UNITLESS_PROPERTIES.contains(&kebab_key.as_str()) {
                        Some(formatted.to_owned())
                    } else {
                        let mut result = String::with_capacity(formatted.len() + 2);
                        result.push_str(formatted);
                        result.push_str("px");
                        Some(result)
                    }
                } else {
                    None
                }
            } else if v.is_object() || v.is_array() {
                Some(serde_json::to_string(v).unwrap_or_else(|_| String::from("{}")))
            } else {
                Some(v.to_string())
            };
            value_str.map(|val| {
                let mut result = String::with_capacity(kebab_key.len() + val.len() + 1);
                result.push_str(&kebab_key);
                result.push(':');
                result.push_str(&val);
                result
            })
        })
        .collect();
    style_parts.join(";")
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RscRow {
    pub id: u32,
    pub data: RscElement,
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
                r#"    <script type="module" src="/@vite/client"></script>
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
                r#"    <script type="module" src="/@vite/client"></script>
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

    pub fn parse_rsc_flight_protocol(&self, rsc_data: &str) -> Result<Vec<RscRow>, RariError> {
        let estimated_lines = rsc_data.len() / 50;
        let mut rows = Vec::with_capacity(estimated_lines.max(8));

        for line in rsc_data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let Some(row) = Self::parse_rsc_line(line)? else { continue };
            rows.push(row);
        }

        Ok(rows)
    }

    fn parse_rsc_line(line: &str) -> Result<Option<RscRow>, RariError> {
        let colon_pos = line.find(':').ok_or_else(|| {
            RariError::internal(format!("Invalid RSC line format: missing colon in '{line}'"))
        })?;

        let (id_str, data_str) = line.split_at(colon_pos);
        let data_str = &data_str[1..];

        let id = u32::from_str_radix(id_str, 16)
            .map_err(|e| RariError::internal(format!("Invalid row ID '{id_str}': {e}")))?;

        if let Some(json_str) = data_str.strip_prefix('I') {
            if let Ok(import_data) = serde_json::from_str::<Value>(json_str) {
                let module_path =
                    import_data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let export_name = import_data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();
                return Ok(Some(RscRow {
                    id,
                    data: RscElement::ModuleImport { module_path, export_name },
                }));
            }
            return Ok(Some(RscRow { id, data: RscElement::Text(data_str.to_string()) }));
        }

        if Self::is_non_renderable_flight_row(data_str) {
            return Ok(None);
        }

        let json_value: Value = serde_json::from_str(data_str)
            .map_err(|e| RariError::internal(format!("Invalid JSON in RSC line: {e}")))?;

        let data = Self::parse_rsc_element(&json_value)?;

        Ok(Some(RscRow { id, data }))
    }

    fn is_non_renderable_flight_row(data_str: &str) -> bool {
        if data_str.starts_with('{') || data_str.starts_with('[') || data_str.starts_with('"') {
            return false;
        }

        data_str.chars().next().is_some_and(|character| character.is_ascii_uppercase())
    }

    fn parse_rsc_element(value: &Value) -> Result<RscElement, RariError> {
        match value {
            Value::String(s) => {
                if let Some(stripped) = s.strip_prefix('$') {
                    if s.starts_with("$$") {
                        Ok(RscElement::Text(stripped.to_string()))
                    } else {
                        Ok(RscElement::Reference(s.clone()))
                    }
                } else {
                    Ok(RscElement::Text(s.clone()))
                }
            }

            Value::Array(arr) => {
                if arr.is_empty() {
                    return Err(RariError::internal("Empty array in RSC element".to_string()));
                }

                if let Some(Value::String(marker)) = arr.first()
                    && marker == "$"
                {
                    return Self::parse_react_element(arr);
                }

                let mut children = Vec::new();
                for item in arr {
                    children.push(Self::parse_rsc_element(item)?);
                }

                Ok(RscElement::Fragment { children })
            }

            Value::Number(n) => Ok(RscElement::Text(n.to_string())),
            Value::Bool(b) => Ok(RscElement::Text(b.to_string())),
            Value::Null => Ok(RscElement::Text(String::new())),

            Value::Object(_) => {
                Ok(RscElement::Text(serde_json::to_string(value).unwrap_or_default()))
            }
        }
    }

    fn parse_react_element(arr: &[Value]) -> Result<RscElement, RariError> {
        if arr.len() < 4 {
            return Err(RariError::internal(format!(
                "Invalid React element: expected 4 elements, got {}",
                arr.len()
            )));
        }

        let tag = arr[1]
            .as_str()
            .ok_or_else(|| RariError::internal("React element tag must be a string".to_string()))?
            .to_string();

        let key = arr[2].as_str().map(ToString::to_string);

        let props_value = &arr[3];
        let props = if let Value::Object(obj) = props_value {
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            FxHashMap::default()
        };

        Ok(RscElement::Component { tag, key, props })
    }

    pub async fn render_to_html(
        &self,
        rsc_flight_protocol: &str,
        config: &Config,
    ) -> Result<String, RariError> {
        self.render_to_html_inner(rsc_flight_protocol, config, None).await
    }

    pub async fn render_to_html_for_route(
        &self,
        rsc_flight_protocol: &str,
        config: &Config,
        route_match: &AppRouteMatch,
    ) -> Result<String, RariError> {
        self.render_to_html_inner(rsc_flight_protocol, config, Some(route_match)).await
    }

    pub async fn render_to_html_for_route_fizz(
        &self,
        rsc_flight_protocol: &str,
        config: &Config,
        route_match: &AppRouteMatch,
    ) -> Result<String, RariError> {
        let cache_template = config.rsc_html.cache_template;
        let is_dev_mode = config.is_development();
        let css_links = Self::css_links_for_route(route_match);

        let html_content = self.render_flight_to_fizz_html(rsc_flight_protocol).await?;

        self.assemble_document(html_content, cache_template, is_dev_mode, &css_links).await
    }

    async fn render_flight_to_fizz_html(
        &self,
        rsc_flight_protocol: &str,
    ) -> Result<String, RariError> {
        let flight_json =
            serde_json::to_string(rsc_flight_protocol).unwrap_or_else(|_| "\"\"".to_string());

        let script = format!(
            r"(async function() {{
                const fn = globalThis['~rari'] && globalThis['~rari'].renderFlightToHtml;
                if (!fn) return {{ ok: false, error: 'renderFlightToHtml unavailable' }};
                try {{
                    const html = await fn({flight_json});
                    return {{ ok: true, html: html }};
                }} catch (e) {{
                    return {{ ok: false, error: String((e && e.message) || e) }};
                }}
            }})()",
        );

        let result =
            self.runtime.execute_script("render_flight_to_fizz".to_string(), script).await?;

        let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if !ok {
            let err = result.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
            return Err(RariError::js_execution(format!("Fizz reviver failed: {err}")));
        }

        Ok(result.get("html").and_then(|v| v.as_str()).unwrap_or_default().to_string())
    }

    async fn assemble_document(
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

    async fn render_to_html_inner(
        &self,
        rsc_flight_protocol: &str,
        config: &Config,
        route_match: Option<&AppRouteMatch>,
    ) -> Result<String, RariError> {
        if !config.rsc_html.enabled {
            return Err(RariError::internal(
                "RSC-to-HTML rendering is disabled in configuration".to_string(),
            ));
        }
        let timeout_ms = config.rsc_html.timeout_ms;
        let cache_template = config.rsc_html.cache_template;
        let is_dev_mode = config.is_development();
        let css_links = if let Some(route_match) = route_match {
            Self::css_links_for_route(route_match)
        } else {
            Vec::new()
        };

        let render_future = async {
            tracing::info!(
                "RSC->HTML: Starting Flight protocol parsing, length: {}",
                rsc_flight_protocol.len()
            );
            let rsc_rows = self.parse_rsc_flight_protocol(rsc_flight_protocol).map_err(|e| {
                tracing::error!("RSC->HTML: Failed to parse Flight protocol: {}", e);
                RariError::internal(format!("Failed to parse RSC Flight protocol: {e}"))
            })?;

            tracing::info!("RSC->HTML: Parsed {} rows", rsc_rows.len());

            let html_content = self.render_rsc_to_html_string(&rsc_rows).await.map_err(|e| {
                tracing::error!("RSC->HTML: Failed to render to HTML: {}", e);
                RariError::internal(format!("Failed to render RSC to HTML: {e}"))
            })?;

            tracing::info!(
                "RSC->HTML: Rendered HTML length: {}, first 100 chars: {}",
                html_content.len(),
                &html_content.chars().take(100).collect::<String>()
            );

            let is_complete_document = html_content.trim_start().starts_with("<!DOCTYPE")
                || html_content.trim_start().cow_to_lowercase().starts_with("<html");

            if is_complete_document {
                let (script_tags, head_link_tags) = if is_dev_mode {
                    (String::new(), String::new())
                } else {
                    let template =
                        self.load_template(cache_template, is_dev_mode).await.map_err(|e| {
                            RariError::internal(format!("Failed to load HTML template: {e}"))
                        })?;
                    (
                        Self::extract_script_tags(&template),
                        Self::extract_non_stylesheet_link_tags(&template),
                    )
                };

                let mut final_html = html_content.clone();

                if !script_tags.is_empty()
                    && let Some(body_end) = final_html.rfind("</body>")
                {
                    final_html.insert_str(body_end, &format!("\n{script_tags}\n"));
                }

                final_html = Self::inject_css_links(&final_html, &css_links);
                final_html = Self::inject_head_tags(&final_html, &head_link_tags);

                let trimmed_lower = final_html.trim_start().cow_to_lowercase();
                if !trimmed_lower.starts_with("<!doctype") {
                    final_html = format!("<!DOCTYPE html>\n{final_html}");
                }

                return Ok::<String, RariError>(final_html);
            }

            let template = self
                .load_template(cache_template, is_dev_mode)
                .await
                .map_err(|e| RariError::internal(format!("Failed to load HTML template: {e}")))?;

            let template = Self::inject_css_links(&template, &css_links);

            let final_html = self.inject_into_template(&html_content, &template).map_err(|e| {
                RariError::internal(format!("Failed to inject HTML into template: {e}"))
            })?;

            Ok::<String, RariError>(final_html)
        };

        let result = if timeout_ms > 0 {
            match time::timeout(Duration::from_millis(timeout_ms), render_future).await {
                Ok(result) => result,
                Err(_) => {
                    return Err(RariError::timeout(format!(
                        "RSC-to-HTML rendering timed out after {timeout_ms}ms"
                    )));
                }
            }
        } else {
            render_future.await
        };

        match result {
            Ok(html) => Ok(html),
            Err(e) => {
                eprintln!("RSC-to-HTML rendering failed: {e}, falling back to shell");
                tracing::warn!("RSC-to-HTML converter failed: {}", e);

                let fallback_template = self.load_template(cache_template, is_dev_mode).await?;
                Ok(Self::inject_css_links(&fallback_template, &css_links))
            }
        }
    }

    pub async fn render_rsc_to_html_string(
        &self,
        rsc_rows: &[RscRow],
    ) -> Result<String, RariError> {
        if rsc_rows.is_empty() {
            return Ok(String::new());
        }

        let row_map: FxHashMap<u32, &RscElement> =
            rsc_rows.iter().map(|r| (r.id, &r.data)).collect();
        let mut row_cache: FxHashMap<u32, String> = FxHashMap::default();

        let root_row_id = if row_map.contains_key(&0) {
            0
        } else {
            rsc_rows.iter().map(|r| r.id).max().unwrap_or(0)
        };

        self.render_row(root_row_id, &row_map, &mut row_cache).await
    }

    fn parse_reference(ref_str: &str) -> Result<u32, RariError> {
        if let Some(stripped) = ref_str
            .strip_prefix("$L")
            .or_else(|| ref_str.strip_prefix("$@"))
            .or_else(|| ref_str.strip_prefix("$"))
        {
            u32::from_str_radix(stripped, 16)
                .map_err(|_| RariError::internal(format!("Invalid row reference: {ref_str}")))
        } else {
            Err(RariError::internal(format!("Invalid reference format: {ref_str}")))
        }
    }

    fn render_row<'a>(
        &'a self,
        row_id: u32,
        row_map: &'a FxHashMap<u32, &RscElement>,
        row_cache: &'a mut FxHashMap<u32, String>,
    ) -> Pin<Box<dyn Future<Output = Result<String, RariError>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(cached) = row_cache.get(&row_id) {
                return Ok(cached.clone());
            }

            let element = row_map
                .get(&row_id)
                .ok_or_else(|| RariError::internal(format!("Missing row {row_id}")))?;

            let html = self.render_rsc_element(element, row_map, row_cache).await?;

            row_cache.insert(row_id, html.clone());
            Ok(html)
        })
    }

    fn render_rsc_element<'a>(
        &'a self,
        element: &'a RscElement,
        row_map: &'a FxHashMap<u32, &RscElement>,
        row_cache: &'a mut FxHashMap<u32, String>,
    ) -> Pin<Box<dyn Future<Output = Result<String, RariError>> + Send + 'a>> {
        Box::pin(async move {
            match element {
                RscElement::Text(text) => {
                    if (text.starts_with("$L") || text.starts_with("$@"))
                        && let Ok(row_id) = Self::parse_reference(text)
                    {
                        return self.render_row(row_id, row_map, row_cache).await;
                    }

                    if text.starts_with('$')
                        && text.len() > 1
                        && text[1..].chars().all(|c| c.is_ascii_hexdigit())
                        && let Ok(row_id) = u32::from_str_radix(&text[1..], 16)
                    {
                        return self.render_row(row_id, row_map, row_cache).await;
                    }

                    if text.starts_with("I[") || text.starts_with("S[") || text.starts_with("E[") {
                        return Ok(String::new());
                    }

                    Ok(escape_html(text))
                }

                RscElement::Fragment { children } => {
                    let mut html = String::new();
                    for child in children {
                        let child_html = self.render_rsc_element(child, row_map, row_cache).await?;
                        html.push_str(&child_html);
                    }
                    Ok(html)
                }

                RscElement::Component { tag, key: _, props } => {
                    let props_value =
                        Value::Object(props.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
                    self.render_component_to_html(tag, &props_value, row_map, row_cache).await
                }

                RscElement::Reference(ref_str) => {
                    if (ref_str.starts_with("$L") || ref_str.starts_with("$@"))
                        && let Ok(row_id) = Self::parse_reference(ref_str)
                    {
                        return self.render_row(row_id, row_map, row_cache).await;
                    }
                    if ref_str.starts_with('$')
                        && ref_str.len() > 1
                        && ref_str[1..].chars().all(|c| c.is_ascii_hexdigit())
                        && let Ok(row_id) = u32::from_str_radix(&ref_str[1..], 16)
                    {
                        return self.render_row(row_id, row_map, row_cache).await;
                    }
                    Ok(String::new())
                }

                RscElement::Suspense { fallback_ref, children_ref, boundary_id: _, props: _ } => {
                    if let Ok(row_id) = Self::parse_reference(children_ref) {
                        let html = self.render_row(row_id, row_map, row_cache).await?;
                        if !html.is_empty() {
                            return Ok(html);
                        }
                    }

                    if let Ok(row_id) = Self::parse_reference(fallback_ref) {
                        let html = self.render_row(row_id, row_map, row_cache).await?;
                        return Ok(format!(r#"<div class="rari-suspense-boundary">{html}</div>"#));
                    }

                    Ok(String::new())
                }

                RscElement::Promise { promise_id: _ } | RscElement::ModuleImport { .. } => {
                    Ok(String::new())
                }

                _ => Ok(String::new()),
            }
        })
    }

    #[expect(clippy::too_many_lines)]
    fn render_component_to_html<'a>(
        &'a self,
        tag: &'a str,
        props: &'a Value,
        row_map: &'a FxHashMap<u32, &RscElement>,
        row_cache: &'a mut FxHashMap<u32, String>,
    ) -> Pin<Box<dyn Future<Output = Result<String, RariError>> + Send + 'a>> {
        Box::pin(async move {
            if tag.contains('<') || tag.contains('>') || tag.contains('"') || tag.contains('\'') {
                return Err(RariError::internal(format!("Invalid tag name: {tag}")));
            }

            let is_rsc_reference = tag.starts_with('$')
                && tag.len() > 1
                && tag[1..].chars().all(|c| c.is_ascii_hexdigit());
            let is_client_component = tag.starts_with("$L")
                || tag.starts_with("$@")
                || is_rsc_reference
                || tag.contains('#')
                || tag.contains('/');
            if is_client_component {
                if let Some(props_obj) = props.as_object()
                    && let Some(children) = props_obj.get("children")
                {
                    return self.render_json_to_html(children, row_map, row_cache).await;
                }

                return Ok(String::new());
            }

            if tag == "$Sreact.suspense"
                || tag == "react.suspense"
                || tag == "suspense"
                || tag.starts_with("$S")
            {
                if let Some(props_obj) = props.as_object() {
                    let children = props_obj.get("children");
                    if let Some(children) = children {
                        return self.render_json_to_html(children, row_map, row_cache).await;
                    }
                }
                return Ok(String::new());
            }

            if !tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':') {
                return Err(RariError::internal(format!("Invalid tag name: {tag}")));
            }

            let mut html = String::with_capacity(256);
            html.push('<');
            html.push_str(tag);

            if let Some(props_obj) = props.as_object() {
                for (key, value) in props_obj {
                    if key == "children" || key == "key" || key == "ref" {
                        continue;
                    }

                    if value.is_null() {
                        continue;
                    }

                    let attr_name = match key.as_str() {
                        "className" => "class",
                        "htmlFor" => "for",
                        _ => key.as_str(),
                    };

                    if !is_valid_attribute_name(attr_name) {
                        continue;
                    }

                    if key == "style" && value.is_object() {
                        if let Some(style_obj) = value.as_object() {
                            let style_str = serialize_style_object(style_obj);
                            html.push_str(r#" style=""#);
                            html.push_str(&Self::escape_html_attribute(&style_str));
                            html.push('"');
                        }
                        continue;
                    }

                    if let Some(b) = value.as_bool() {
                        if is_boolean_html_attribute(attr_name) {
                            if b {
                                html.push(' ');
                                html.push_str(attr_name);
                            }
                        } else {
                            html.push(' ');
                            html.push_str(attr_name);
                            html.push_str(r#"=""#);
                            html.push_str(if b { "true" } else { "false" });
                            html.push('"');
                        }
                    } else if let Some(s) = value.as_str() {
                        html.push(' ');
                        html.push_str(attr_name);
                        html.push_str(r#"=""#);
                        html.push_str(&Self::escape_html_attribute(s));
                        html.push('"');
                    } else if value.is_number() {
                        html.push(' ');
                        html.push_str(attr_name);
                        html.push_str(r#"=""#);
                        html.push_str(&value.to_string());
                        html.push('"');
                    }
                }
            }

            if SELF_CLOSING_TAGS.contains(&tag) {
                html.push_str(" />");
                return Ok(html);
            }

            html.push('>');

            if let Some(props_obj) = props.as_object()
                && let Some(children) = props_obj.get("children")
            {
                let children_html = self.render_json_to_html(children, row_map, row_cache).await?;
                html.push_str(&children_html);
            }

            html.push_str("</");
            html.push_str(tag);
            html.push('>');

            Ok(html)
        })
    }

    fn render_json_to_html<'a>(
        &'a self,
        json: &'a Value,
        row_map: &'a FxHashMap<u32, &RscElement>,
        row_cache: &'a mut FxHashMap<u32, String>,
    ) -> Pin<Box<dyn Future<Output = Result<String, RariError>> + Send + 'a>> {
        Box::pin(async move {
            if json.is_null() {
                return Ok(String::new());
            }

            if let Some(s) = json.as_str() {
                if let Some(stripped) = s.strip_prefix("$L").or_else(|| s.strip_prefix("$@"))
                    && let Ok(row_id) = u32::from_str_radix(stripped, 16)
                {
                    return self.render_row(row_id, row_map, row_cache).await;
                }

                if s.starts_with('$')
                    && s.len() > 1
                    && s[1..].chars().all(|c| c.is_ascii_hexdigit())
                    && let Ok(row_id) = u32::from_str_radix(&s[1..], 16)
                {
                    return self.render_row(row_id, row_map, row_cache).await;
                }

                if s.starts_with("$$") {
                    return Ok(escape_html(&s[1..]));
                }

                return Ok(escape_html(s));
            }

            if let Some(arr) = json.as_array() {
                if arr.len() >= 4
                    && arr[0].as_str() == Some("$")
                    && let (Some(tag), Some(props)) = (arr[1].as_str(), arr.get(3))
                {
                    return self.render_component_to_html(tag, props, row_map, row_cache).await;
                }

                let mut html = String::with_capacity(arr.len() * 64);
                for child in arr {
                    html.push_str(&self.render_json_to_html(child, row_map, row_cache).await?);
                }
                return Ok(html);
            }

            if let Some(num) = json.as_u64() {
                return Ok(num.to_string());
            }
            if let Some(num) = json.as_i64() {
                return Ok(num.to_string());
            }
            if let Some(num) = json.as_f64() {
                return Ok(num.to_string());
            }
            if json.is_boolean() {
                return Ok(String::new());
            }

            Ok(String::new())
        })
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
#[expect(clippy::panic, clippy::unwrap_used, clippy::clone_on_ref_ptr)]
mod tests {
    use super::*;
    use crate::server::config::Mode;

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

        {
            let cache = renderer.template_cache.lock();
            assert!(cache.is_some());
        }

        renderer.clear_template_cache();

        {
            let cache = renderer.template_cache.lock();
            assert!(cache.is_none());
        }
    }

    #[test]
    fn test_parse_rsc_flight_protocol_valid() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"children":"Hello"}]
1:["$","span",null,{"children":"World"}]"#;

        let result = renderer.parse_rsc_flight_protocol(rsc_data);
        assert!(result.is_ok());

        let rows = result.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, 0);
        assert_eq!(rows[1].id, 1);
    }

    #[test]
    fn test_parse_rsc_line_missing_colon() {
        let result = RscHtmlRenderer::parse_rsc_line("0invalid");
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("missing colon"));
    }

    #[test]
    fn test_parse_rsc_line_invalid_row_id() {
        let result = RscHtmlRenderer::parse_rsc_line("xyz:{}");
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Invalid row ID"));
    }

    #[test]
    fn test_parse_rsc_line_invalid_json() {
        let result = RscHtmlRenderer::parse_rsc_line("0:{invalid json}");
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Invalid JSON"));
    }

    #[test]
    fn test_parse_react_element_invalid_structure() {
        let arr = vec![Value::String("$".to_string()), Value::String("div".to_string())];

        let result = RscHtmlRenderer::parse_react_element(&arr);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("expected 4 elements"));
    }

    #[test]
    fn test_parse_react_element_non_string_tag() {
        let arr = vec![
            Value::String("$".to_string()),
            Value::Number(123.into()),
            Value::Null,
            Value::Object(serde_json::Map::new()),
        ];

        let result = RscHtmlRenderer::parse_react_element(&arr);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("tag must be a string"));
    }

    #[test]
    fn test_parse_rsc_element_text() {
        let value = Value::String("Hello World".to_string());
        let result = RscHtmlRenderer::parse_rsc_element(&value);
        assert!(result.is_ok());

        if let RscElement::Text(text) = result.unwrap() {
            assert_eq!(text, "Hello World");
        } else {
            panic!("Expected Text element");
        }
    }

    #[test]
    fn test_parse_rsc_element_reference() {
        let value = Value::String("$L1".to_string());
        let result = RscHtmlRenderer::parse_rsc_element(&value);
        assert!(result.is_ok());

        if let RscElement::Reference(ref_str) = result.unwrap() {
            assert_eq!(ref_str, "$L1");
        } else {
            panic!("Expected Reference element");
        }
    }

    #[test]
    fn test_parse_rsc_element_component() {
        let mut props = serde_json::Map::new();
        props.insert("children".to_string(), Value::String("Hello".to_string()));

        let value = Value::Array(vec![
            Value::String("$".to_string()),
            Value::String("div".to_string()),
            Value::Null,
            Value::Object(props),
        ]);

        let result = RscHtmlRenderer::parse_rsc_element(&value);
        assert!(result.is_ok());

        if let RscElement::Component { tag, key, props } = result.unwrap() {
            assert_eq!(tag, "div");
            assert!(key.is_none());
            assert!(props.contains_key("children"));
        } else {
            panic!("Expected Component element");
        }
    }

    #[test]
    fn test_parse_rsc_element_empty_array() {
        let value = Value::Array(vec![]);
        let result = RscHtmlRenderer::parse_rsc_element(&value);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Empty array"));
    }

    #[test]
    fn test_parse_rsc_flight_protocol_with_empty_lines() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"children":"Hello"}]

1:["$","span",null,{"children":"World"}]
"#;

        let result = renderer.parse_rsc_flight_protocol(rsc_data);
        assert!(result.is_ok());

        let rows = result.unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_integration() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"className":"container","children":"Hello World"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html_result = renderer.render_rsc_to_html_string(&rows).await;
        assert!(html_result.is_ok(), "Failed to render HTML: {:?}", html_result.err());

        let html = html_result.unwrap();

        assert!(html.contains("<div"), "HTML should contain opening div tag");
        assert!(html.contains("class=\"container\""), "HTML should have class attribute");
        assert!(html.contains("Hello World"), "HTML should contain the text content");
        assert!(html.contains("</div>"), "HTML should contain closing div tag");
    }

    #[tokio::test]
    async fn test_render_nested_elements() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"children":["$","span",null,{"children":"Nested"}]}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("<div"), "Should have div");
        assert!(html.contains("<span"), "Should have nested span");
        assert!(html.contains("Nested"), "Should have text content");
        assert!(html.contains("</span>"), "Should close span");
        assert!(html.contains("</div>"), "Should close div");
    }

    #[tokio::test]
    async fn test_html_escaping() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"children":"<script>alert('xss')</script>"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("&lt;script&gt;"), "Should escape < and >");
        assert!(!html.contains("<script>alert"), "Should not contain unescaped script tag");
    }

    #[tokio::test]
    async fn test_render_with_attributes() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","input",null,{"type":"text","className":"form-control","placeholder":"Enter text"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("type=\"text\""), "Should have type attribute");
        assert!(html.contains("class=\"form-control\""), "className should be converted to class");
        assert!(html.contains("placeholder=\"Enter text\""), "Should have placeholder attribute");
        assert!(html.contains("<input"), "Should be an input tag");
        assert!(html.contains("/>"), "Input should be self-closing");
    }

    #[test]
    fn test_inject_into_template_basic() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let template = r#"<html><body><div id="root"></div></body></html>"#;
        let content = "<h1>Hello World</h1>";

        let result = renderer.inject_into_template(content, template);
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains(r#"<div id="root"><h1>Hello World</h1></div>"#));
        assert!(html.contains("<html>"));
        assert!(html.contains("<body>"));
    }

    #[test]
    fn test_inject_into_template_self_closing() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let template = r#"<html><body><div id="root" /></body></html>"#;
        let content = "<p>Content</p>";

        let result = renderer.inject_into_template(content, template);
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains(r#"<div id="root"><p>Content</p></div>"#));
    }

    #[test]
    fn test_extract_non_stylesheet_link_tags() {
        let template = r#"<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="/styles.css">
    <link rel="icon" href="/favicon.ico">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preload" href="/font.woff2" as="font" type="font/woff2" crossorigin>
</head>
<body></body>
</html>"#;

        let links = RscHtmlRenderer::extract_non_stylesheet_link_tags(template);
        assert!(links.contains(r#"<link rel="icon" href="/favicon.ico">"#));
        assert!(links.contains(r#"<link rel="preconnect" href="https://fonts.googleapis.com">"#));
        assert!(links.contains(r#"rel="preload""#));
        assert!(!links.contains(r"/styles.css"));
    }

    #[test]
    fn test_inject_head_tags_skips_duplicates() {
        let html = r#"<!DOCTYPE html><html><head><link rel="icon" href="/favicon.ico"></head><body></body></html>"#;
        let tags = r#"<link rel="icon" href="/favicon.ico">
<link rel="preconnect" href="https://cdn.example.com">"#;

        let result = RscHtmlRenderer::inject_head_tags(html, tags);
        assert_eq!(result.matches("favicon.ico").count(), 1);
        assert!(result.contains("preconnect"));
    }

    #[test]
    fn test_inject_into_template_with_scripts_and_styles() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let template = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Test</title>
    <link rel="stylesheet" href="/styles.css">
    <script src="/app.js"></script>
</head>
<body>
    <div id="root"></div>
</body>
</html>"#;
        let content = "<main>App Content</main>";

        let result = renderer.inject_into_template(content, template);
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains(r#"<div id="root"><main>App Content</main></div>"#));
        assert!(html.contains(r#"<link rel="stylesheet" href="/styles.css">"#));
        assert!(html.contains(r#"<script src="/app.js"></script>"#));
        assert!(html.contains(r#"<meta charset="UTF-8">"#));
        assert!(html.contains("<title>Test</title>"));
    }

    #[test]
    fn test_inject_into_template_missing_root_div() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let template = r#"<html><body><div id="app"></div></body></html>"#;
        let content = "<h1>Hello</h1>";

        let result = renderer.inject_into_template(content, template);
        assert!(result.is_err());

        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("does not contain a root div"));
    }

    #[test]
    fn test_inject_into_template_with_extra_attributes() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let template = r#"<html><body><div id="root" class="container"></div></body></html>"#;
        let content = "<span>Text</span>";

        let result = renderer.inject_into_template(content, template);
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains(r#"<div id="root"><span>Text</span></div>"#));
    }

    #[test]
    fn test_generate_dev_template_fallback() {
        let template = RscHtmlRenderer::generate_dev_template_fallback();

        assert!(template.contains("<!DOCTYPE html>"));
        assert!(template.contains(r#"<div id="root"></div>"#));
        assert!(template.contains(r#"<script type="module" src="/@vite/client"></script>"#));
        assert!(template.contains(r#"<script type="module" src="/src/main.tsx"></script>"#));
        assert!(template.contains(r#"<meta charset="UTF-8">"#));
    }

    #[tokio::test]
    async fn test_load_template_dev_mode() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let result = renderer.load_template(false, true).await;
        assert!(result.is_ok());

        let template = result.unwrap();
        assert!(template.contains("<!DOCTYPE html>"));
        assert!(template.contains(r#"<div id="root"></div>"#));
        assert!(template.contains("/@vite/client"));
    }

    #[tokio::test]
    async fn test_load_template_caching() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let result1 = renderer.load_template(true, true).await;
        assert!(result1.is_ok());

        {
            let cache = renderer.template_cache.lock();
            assert!(cache.is_some());
        }

        let result2 = renderer.load_template(true, true).await;
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), result2.unwrap());

        renderer.clear_template_cache();

        {
            let cache = renderer.template_cache.lock();
            assert!(cache.is_none());
        }
    }

    #[tokio::test]
    async fn test_load_template_no_caching() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let result = renderer.load_template(false, true).await;
        assert!(result.is_ok());

        {
            let cache = renderer.template_cache.lock();
            assert!(cache.is_none());
        }
    }

    #[tokio::test]
    async fn test_render_to_html_success() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"0:["$","div",null,{"children":"Hello World"}]"#;

        let config = Config::new(Mode::Development);

        let result = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result.is_ok());

        let html = result.unwrap();

        assert!(html.contains("Hello World"), "Should contain rendered content");
        assert!(html.contains("<!DOCTYPE html>"), "Should have DOCTYPE");
        assert!(html.contains(r#"<div id="root">"#), "Should have root div");
        assert!(html.contains("</html>"), "Should be complete HTML document");
    }

    #[tokio::test]
    async fn test_render_to_html_complex_structure() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"1:["$","h1",null,{"children":"Title"}]
2:["$","div",null,{"className":"container","children":"$@1"}]"#;

        let config = Config::new(Mode::Development);

        let result = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result.is_ok());

        let html = result.unwrap();

        assert!(html.contains("Title"), "Should contain title text");
        assert!(html.contains("class=\"container\""), "Should have container class");
        assert!(html.contains("<h1"), "Should have h1 tag");
    }

    #[tokio::test]
    async fn test_render_to_html_with_timeout() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut config = Config::new(Mode::Development);
        config.rsc_html.cache_template = false;

        let result = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result.is_ok(), "Should succeed with reasonable timeout");
    }

    #[tokio::test]
    async fn test_render_to_html_no_timeout() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut config = Config::new(Mode::Development);
        config.rsc_html.timeout_ms = 0;
        config.rsc_html.cache_template = false;

        let result = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result.is_ok(), "Should succeed with no timeout");
    }

    #[tokio::test]
    async fn test_render_to_html_invalid_rsc_fallback() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = "invalid:format:here";

        let mut config = Config::new(Mode::Development);
        config.rsc_html.cache_template = false;

        let result = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result.is_ok(), "Should fall back to shell on error");

        let html = result.unwrap();

        assert!(html.contains("<!DOCTYPE html>"), "Should have DOCTYPE in fallback");
        assert!(
            html.contains(r#"<div id="root"></div>"#),
            "Should have empty root div in fallback"
        );
    }

    #[tokio::test]
    async fn test_render_to_html_preserves_template_structure() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"0:["$","p",null,{"children":"Content"}]"#;

        let config = Config::new(Mode::Development);

        let result = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result.is_ok());

        let html = result.unwrap();

        assert!(html.contains("<html"), "Should have html tag");
        assert!(html.contains("<head>"), "Should have head tag");
        assert!(html.contains("<body>"), "Should have body tag");
        assert!(html.contains("/@vite/client"), "Should have Vite client script in dev mode");
    }

    #[tokio::test]
    async fn test_render_to_html_caching_behavior() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"0:["$","div",null,{"children":"Test"}]"#;

        let config = Config::new(Mode::Development);

        let result1 = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result1.is_ok());

        {
            let cache = renderer.template_cache.lock();
            assert!(cache.is_some(), "Template should be cached");
        }

        let result2 = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result2.is_ok());

        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[tokio::test]
    async fn test_render_to_html_ssr_disabled() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut config = Config::new(Mode::Development);
        config.rsc_html.enabled = false;

        let result = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result.is_err(), "Should fail when SSR is disabled");

        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("disabled"), "Error should mention SSR is disabled");
    }

    #[tokio::test]
    async fn test_render_to_html_respects_timeout_config() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut config = Config::new(Mode::Development);
        config.rsc_html.timeout_ms = 10000;

        let result = renderer.render_to_html(rsc_flight_protocol, &config).await;
        assert!(result.is_ok(), "Should succeed with custom timeout");
    }

    #[tokio::test]
    async fn test_render_to_html_production_vs_dev_mode() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_flight_protocol = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut dev_config = Config::new(Mode::Development);
        dev_config.rsc_html.cache_template = false;

        let dev_result = renderer.render_to_html(rsc_flight_protocol, &dev_config).await;
        assert!(dev_result.is_ok());
        let dev_html = dev_result.unwrap();

        assert!(dev_html.contains("/@vite/client"), "Dev mode should have Vite client");

        renderer.clear_template_cache();
    }

    #[test]
    fn test_parse_suspense_boundary() {
        let mut props = serde_json::Map::new();
        props.insert("fallback".to_string(), Value::String("$L1".to_string()));
        props.insert("children".to_string(), Value::String("$L2".to_string()));
        props.insert("~boundaryId".to_string(), Value::String("suspense_123".to_string()));

        let value = Value::Array(vec![
            Value::String("$".to_string()),
            Value::String("react.suspense".to_string()),
            Value::Null,
            Value::Object(props),
        ]);

        let result = RscHtmlRenderer::parse_rsc_element(&value);
        assert!(result.is_ok());

        if let RscElement::Component { tag, key, props } = result.unwrap() {
            assert_eq!(tag, "react.suspense");
            assert!(key.is_none());
            assert!(props.contains_key("fallback"));
            assert!(props.contains_key("children"));
            assert!(props.contains_key("~boundaryId"));
            assert_eq!(props["~boundaryId"].as_str().unwrap(), "suspense_123");
        } else {
            panic!("Expected Component element");
        }
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_string_row_0_as_root() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"children":"Root Content"}]
1:["$","span",null,{"children":"Other Content"}]
2:["$","p",null,{"children":"More Content"}]"#;

        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("Root Content"), "Should render row 0 as root");
        assert!(html.contains("<div"), "Should have div from row 0");
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_string_max_row_fallback() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"1:["$","span",null,{"children":"First"}]
2:["$","p",null,{"children":"Second"}]
5:["$","div",null,{"children":"Last Content"}]"#;

        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("Last Content"), "Should render max row ID as root");
        assert!(html.contains("<div"), "Should have div from row 5");
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_string_empty_rows() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rows: Vec<RscRow> = vec![];
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert_eq!(html, "", "Should return empty string for empty rows");
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_string_single_row() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"42:["$","div",null,{"children":"Single Row"}]"#;

        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("Single Row"), "Should render single row");
        assert!(html.contains("<div"), "Should have div tag");
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_string_row_0_preferred_over_max() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"children":"Row Zero"}]
10:["$","span",null,{"children":"Row Ten"}]
100:["$","p",null,{"children":"Row Hundred"}]"#;

        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("Row Zero"), "Should render row 0 as root");
        assert!(html.contains("<div"), "Should have div from row 0");
        assert!(!html.contains("Row Hundred"), "Should not render row 100 as root");
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_string_backward_references() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"1:["$","span",null,{"children":"Child Content"}]
0:["$","div",null,{"children":"$L1"}]"#;

        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("<div"), "Should have div from row 0");
        assert!(html.contains("Child Content"), "Should resolve $L1 reference");
        assert!(html.contains("<span"), "Should have span from row 1");
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_string_hex_references() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:"$La"
a:["$","div",null,{"children":"Content from row 10 (hex a)"}]
b:["$","span",null,{"children":"Content from row 11 (hex b)"}]
10:["$","p",null,{"children":"Content from row 16 (hex 10)"}]"#;

        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("<div"), "Should have div from row a (10)");
        assert!(html.contains("Content from row 10"), "Should resolve $La reference to row 10");
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_string_consistent_with_streaming() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","main",null,{"children":"Main Content"}]
1:["$","aside",null,{"children":"Sidebar"}]"#;

        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("<main"), "Should have main tag from row 0");
        assert!(html.contains("Main Content"), "Should render row 0 content");
    }

    #[test]
    fn test_is_valid_attribute_name_valid_names() {
        assert!(test_is_valid_attribute_name("class"), "class should be valid");
        assert!(test_is_valid_attribute_name("id"), "id should be valid");
        assert!(test_is_valid_attribute_name("type"), "type should be valid");
        assert!(test_is_valid_attribute_name("href"), "href should be valid");
        assert!(test_is_valid_attribute_name("src"), "src should be valid");

        assert!(test_is_valid_attribute_name("data-id"), "data-id should be valid");
        assert!(test_is_valid_attribute_name("data-test-value"), "data-test-value should be valid");
        assert!(test_is_valid_attribute_name("data-123"), "data-123 should be valid");

        assert!(test_is_valid_attribute_name("aria-label"), "aria-label should be valid");
        assert!(test_is_valid_attribute_name("aria-hidden"), "aria-hidden should be valid");

        assert!(test_is_valid_attribute_name("_private"), "_private should be valid");
        assert!(test_is_valid_attribute_name("my_attr"), "my_attr should be valid");

        assert!(test_is_valid_attribute_name("xml:lang"), "xml:lang should be valid");
        assert!(test_is_valid_attribute_name("xlink:href"), "xlink:href should be valid");
        assert!(test_is_valid_attribute_name(":colon-start"), ":colon-start should be valid");

        assert!(test_is_valid_attribute_name("ng.model"), "ng.model should be valid");
        assert!(test_is_valid_attribute_name("v.bind"), "v.bind should be valid");

        assert!(test_is_valid_attribute_name("中文"), "Chinese characters should be valid");
        assert!(test_is_valid_attribute_name("日本語"), "Japanese characters should be valid");
        assert!(test_is_valid_attribute_name("한글"), "Korean characters should be valid");
        assert!(test_is_valid_attribute_name("data-中文"), "data- with Chinese should be valid");
        assert!(test_is_valid_attribute_name("属性名"), "Chinese attribute name should be valid");
    }

    #[test]
    fn test_is_valid_attribute_name_invalid_names() {
        assert!(!test_is_valid_attribute_name(""), "empty string should be invalid");

        assert!(
            !test_is_valid_attribute_name("1invalid"),
            "starting with number should be invalid"
        );
        assert!(
            !test_is_valid_attribute_name("-invalid"),
            "starting with hyphen should be invalid"
        );
        assert!(
            !test_is_valid_attribute_name(".invalid"),
            "starting with period should be invalid"
        );
        assert!(!test_is_valid_attribute_name("@invalid"), "starting with @ should be invalid");
        assert!(!test_is_valid_attribute_name("#invalid"), "starting with # should be invalid");

        assert!(!test_is_valid_attribute_name("on click"), "space should be invalid");
        assert!(!test_is_valid_attribute_name("on=click"), "equals sign should be invalid");
        assert!(!test_is_valid_attribute_name("on'click"), "single quote should be invalid");
        assert!(!test_is_valid_attribute_name("on\"click"), "double quote should be invalid");
        assert!(!test_is_valid_attribute_name("on<click"), "less than should be invalid");
        assert!(!test_is_valid_attribute_name("on>click"), "greater than should be invalid");
        assert!(!test_is_valid_attribute_name("on/click"), "slash should be invalid");
        assert!(!test_is_valid_attribute_name("on\\click"), "backslash should be invalid");

        assert!(!test_is_valid_attribute_name("onclick"), "onclick should be invalid");
        assert!(!test_is_valid_attribute_name("onClick"), "onClick should be invalid");
        assert!(!test_is_valid_attribute_name("ONCLICK"), "ONCLICK should be invalid");
        assert!(!test_is_valid_attribute_name("onload"), "onload should be invalid");
        assert!(!test_is_valid_attribute_name("onerror"), "onerror should be invalid");
        assert!(!test_is_valid_attribute_name("onmouseover"), "onmouseover should be invalid");
        assert!(!test_is_valid_attribute_name("onsubmit"), "onsubmit should be invalid");

        assert!(!test_is_valid_attribute_name("on中文"), "on with Chinese should be invalid");

        assert!(!test_is_valid_attribute_name(":"), "colon only should be invalid");
        assert!(!test_is_valid_attribute_name("-"), "hyphen only should be invalid");
        assert!(!test_is_valid_attribute_name("."), "period only should be invalid");
        assert!(!test_is_valid_attribute_name("::"), "multiple colons only should be invalid");
        assert!(!test_is_valid_attribute_name("--"), "multiple hyphens only should be invalid");
        assert!(!test_is_valid_attribute_name(".."), "multiple periods only should be invalid");
        assert!(!test_is_valid_attribute_name(":-."), "mixed punctuation only should be invalid");

        assert!(
            !test_is_valid_attribute_name("onclick='alert(1)'"),
            "injection attempt should be invalid"
        );
        assert!(
            !test_is_valid_attribute_name("on click='alert(1)'data-x"),
            "complex injection should be invalid"
        );
        assert!(
            !test_is_valid_attribute_name("x onload=alert(1)"),
            "space-based injection should be invalid"
        );
    }

    #[tokio::test]
    async fn test_render_filters_invalid_attribute_names() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"onclick='alert(1)'":"malicious","class":"safe","on click":"bad","data-valid":"good"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("class=\"safe\""), "Valid class attribute should be rendered");
        assert!(html.contains("data-valid=\"good\""), "Valid data attribute should be rendered");

        assert!(!html.contains("onclick"), "Invalid onclick attribute should be filtered");
        assert!(!html.contains("alert(1)"), "Malicious code should not be in output");
        assert!(!html.contains("on click"), "Attribute with space should be filtered");
    }

    #[tokio::test]
    async fn test_render_allows_valid_special_attributes() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"data-test-id":"123","aria-label":"Description","xml:lang":"en","_private":"value"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("data-test-id=\"123\""), "data- attributes should be allowed");
        assert!(html.contains("aria-label=\"Description\""), "aria- attributes should be allowed");
        assert!(html.contains("xml:lang=\"en\""), "XML namespace attributes should be allowed");
        assert!(
            html.contains("_private=\"value\""),
            "Underscore-prefixed attributes should be allowed"
        );
    }

    #[tokio::test]
    async fn test_attribute_validation_with_classname_mapping() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"className":"container","htmlFor":"input1"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("class=\"container\""), "className should be mapped to class");
        assert!(html.contains("for=\"input1\""), "htmlFor should be mapped to for");
    }

    #[tokio::test]
    async fn test_style_attribute_not_affected_by_validation() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data =
            r#"0:["$","div",null,{"style":{"color":"red","fontSize":"16px"},"class":"test"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("style="), "Style attribute should be present");
        assert!(html.contains("color:red"), "Style should contain color");
        assert!(
            html.contains("font-size:16px"),
            "Style should contain font-size (camelCase converted)"
        );
        assert!(html.contains("class=\"test\""), "Other attributes should still work");
    }

    #[tokio::test]
    async fn test_unicode_attribute_names_allowed() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data =
            r#"0:["$","div",null,{"中文":"value1","日本語":"value2","data-한글":"value3"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("中文=\"value1\""), "Chinese attribute should be rendered");
        assert!(html.contains("日本語=\"value2\""), "Japanese attribute should be rendered");
        assert!(html.contains("data-한글=\"value3\""), "Korean data attribute should be rendered");
    }

    #[tokio::test]
    async fn test_event_handlers_blocked_with_unicode() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"onclick":"alert(1)","onClick":"alert(2)","ONCLICK":"alert(3)","on中文":"bad","中文":"good"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(!html.contains("onclick"), "onclick should be filtered");
        assert!(!html.contains("onClick"), "onClick should be filtered");
        assert!(!html.contains("ONCLICK"), "ONCLICK should be filtered");
        assert!(!html.contains("on中文"), "on with Unicode should be filtered");
        assert!(!html.contains("alert"), "No alert code should be in output");

        assert!(html.contains("中文=\"good\""), "Valid Unicode attribute should be rendered");
    }

    #[tokio::test]
    async fn test_style_object_null_values_omitted() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"style":{"display":null,"color":"red","margin":null,"padding":"10px"}}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("color:red"), "color should be in style: {html}");
        assert!(html.contains("padding:10px"), "padding should be in style: {html}");

        assert!(!html.contains("display:null"), "display:null should not appear in style: {html}");
        assert!(!html.contains("margin:null"), "margin:null should not appear in style: {html}");
        assert!(!html.contains("null"), "The word 'null' should not appear in output: {html}");
    }

    #[tokio::test]
    async fn test_style_object_all_null_values() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data =
            r#"0:["$","div",null,{"style":{"display":null,"color":null},"class":"test"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(!html.contains("null"), "Should not contain 'null' string: {html}");

        assert!(html.contains("class=\"test\""), "Should have class attribute: {html}");
    }

    #[tokio::test]
    async fn test_render_component_to_html_validates_tag_names() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"class":"test"}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let result = renderer.render_rsc_to_html_string(&rows).await;
        assert!(result.is_ok(), "Valid tag 'div' should be accepted");

        let rsc_data_invalid = r#"0:["$","div onclick",null,{}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data_invalid).unwrap();
        let result = renderer.render_rsc_to_html_string(&rows).await;
        assert!(result.is_err(), "Tag with space should be rejected");
        assert!(
            result.unwrap_err().to_string().contains("Invalid tag name"),
            "Error should mention invalid tag"
        );

        let rsc_data_injection = r#"0:["$","div><script>alert(1)</script",null,{}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data_injection).unwrap();
        let result = renderer.render_rsc_to_html_string(&rows).await;
        assert!(result.is_err(), "Tag with injection attempt should be rejected");
    }

    #[tokio::test]
    async fn test_style_object_numeric_values_with_px() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data =
            r#"0:["$","div",null,{"style":{"width":100,"height":200,"margin":10,"padding":20}}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("width:100px"), "Width should have px suffix: {html}");
        assert!(html.contains("height:200px"), "Height should have px suffix: {html}");
        assert!(html.contains("margin:10px"), "Margin should have px suffix: {html}");
        assert!(html.contains("padding:20px"), "Padding should have px suffix: {html}");
    }

    #[tokio::test]
    async fn test_style_object_unitless_properties() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"style":{"opacity":0.5,"zIndex":10,"lineHeight":1.5,"flexGrow":2}}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("opacity:0.5"), "Opacity should not have px suffix: {html}");
        assert!(html.contains("z-index:10"), "z-index should not have px suffix: {html}");
        assert!(html.contains("line-height:1.5"), "line-height should not have px suffix: {html}");
        assert!(html.contains("flex-grow:2"), "flex-grow should not have px suffix: {html}");
    }

    #[tokio::test]
    async fn test_style_object_mixed_values() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"style":{"width":"50%","height":100,"opacity":0.8,"color":"red"}}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("width:50%"), "String width should be preserved: {html}");
        assert!(html.contains("height:100px"), "Numeric height should have px: {html}");
        assert!(html.contains("opacity:0.8"), "Opacity should not have px: {html}");
        assert!(html.contains("color:red"), "Color string should be preserved: {html}");
    }

    #[tokio::test]
    async fn test_style_object_camel_case_with_numeric() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"style":{"fontSize":16,"marginTop":10,"paddingLeft":5,"zIndex":100}}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(
            html.contains("font-size:16px"),
            "fontSize should become font-size with px: {html}"
        );
        assert!(
            html.contains("margin-top:10px"),
            "marginTop should become margin-top with px: {html}"
        );
        assert!(
            html.contains("padding-left:5px"),
            "paddingLeft should become padding-left with px: {html}"
        );
        assert!(html.contains("z-index:100"), "zIndex should become z-index without px: {html}");
    }

    #[tokio::test]
    async fn test_style_object_float_values() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data =
            r#"0:["$","div",null,{"style":{"width":100.5,"opacity":0.75,"lineHeight":1.2}}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("width:100.5px"), "Float width should have px: {html}");
        assert!(html.contains("opacity:0.75"), "Float opacity should not have px: {html}");
        assert!(html.contains("line-height:1.2"), "Float line-height should not have px: {html}");
    }

    #[tokio::test]
    async fn test_boolean_html_attributes_presence_only() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","input",null,{"type":"checkbox","checked":true,"disabled":true,"required":false}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains(" checked"), "checked=true should render as presence-only: {html}");
        assert!(!html.contains("checked=\"true\""), "checked should not have =\"true\": {html}");
        assert!(html.contains(" disabled"), "disabled=true should render as presence-only: {html}");
        assert!(!html.contains("disabled=\"true\""), "disabled should not have =\"true\": {html}");

        assert!(!html.contains("required"), "required=false should be omitted: {html}");
    }

    #[tokio::test]
    async fn test_aria_attributes_render_as_strings() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data =
            r#"0:["$","div",null,{"aria-hidden":true,"aria-expanded":false,"aria-checked":true}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(
            html.contains("aria-hidden=\"true\""),
            "aria-hidden=true should render as \"true\": {html}"
        );
        assert!(
            html.contains("aria-expanded=\"false\""),
            "aria-expanded=false should render as \"false\": {html}"
        );
        assert!(
            html.contains("aria-checked=\"true\""),
            "aria-checked=true should render as \"true\": {html}"
        );
    }

    #[tokio::test]
    async fn test_content_editable_and_draggable_as_strings() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data =
            r#"0:["$","div",null,{"contentEditable":true,"draggable":false,"spellcheck":true}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(
            html.contains("contentEditable=\"true\""),
            "contentEditable=true should render as \"true\": {html}"
        );
        assert!(
            html.contains("draggable=\"false\""),
            "draggable=false should render as \"false\": {html}"
        );
        assert!(
            html.contains("spellcheck=\"true\""),
            "spellcheck=true should render as \"true\": {html}"
        );
    }

    #[tokio::test]
    async fn test_mixed_boolean_attributes() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data =
            r#"0:["$","button",null,{"disabled":true,"aria-disabled":true,"aria-pressed":false}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains(" disabled"), "disabled=true should be presence-only: {html}");
        assert!(!html.contains(" disabled="), "disabled should not have a value: {html}");

        assert!(
            html.contains("aria-disabled=\"true\""),
            "aria-disabled=true should render as \"true\": {html}"
        );
        assert!(
            html.contains("aria-pressed=\"false\""),
            "aria-pressed=false should render as \"false\": {html}"
        );
    }

    #[tokio::test]
    async fn test_all_html_boolean_attributes() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"hidden":true,"readonly":true,"required":true,"autofocus":true,"multiple":true}]"#;
        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains(" hidden"), "hidden should be presence-only: {html}");
        assert!(html.contains(" readonly"), "readonly should be presence-only: {html}");
        assert!(html.contains(" required"), "required should be presence-only: {html}");
        assert!(html.contains(" autofocus"), "autofocus should be presence-only: {html}");
        assert!(html.contains(" multiple"), "multiple should be presence-only: {html}");

        assert!(
            !html.contains("=\"true\""),
            "HTML boolean attributes should not have =\"true\": {html}"
        );
    }

    #[tokio::test]
    async fn test_hexadecimal_row_id_references() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"a:["$","div",null,{"children":"Row 10"}]
1f:["$","span",null,{"children":"Row 31"}]
0:["$","div",null,{"children":["$a","$1f"]}]"#;

        let rows = renderer.parse_rsc_flight_protocol(rsc_data).unwrap();
        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("Row 10"), "Should render content from row 10 (hex 'a'): {html}");
        assert!(html.contains("Row 31"), "Should render content from row 31 (hex '1f'): {html}");
    }
}
