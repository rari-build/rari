#![expect(clippy::missing_errors_doc)]

use std::{
    fmt::Write,
    future::Future,
    pin::Pin,
    string::ToString,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use cow_utils::CowUtils;
use rari_error::{RariError, StreamingError};
use rari_rsc::flight::escape::unescape_rsc_value;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;
use tokio::{fs, time};
use tracing::error;

use crate::{
    RscElement,
    rendering::streaming::{RscChunkType, RscStreamChunk},
    runtime::JsExecutionRuntime,
    server::{config::Config, routing::app_router::AppRouteMatch},
};

const SELF_CLOSING_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const PAYLOAD_SIZE_LIMIT: usize = 50000;

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
fn serialize_style_object(style_obj: &serde_json::Map<String, serde_json::Value>) -> String {
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

#[derive(Debug)]
pub struct BoundaryIdGenerator {
    counter: AtomicU32,
}

impl BoundaryIdGenerator {
    pub fn new() -> Self {
        Self { counter: AtomicU32::new(0) }
    }

    pub fn next(&self) -> String {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        format!("B:{id}")
    }
}

impl Default for BoundaryIdGenerator {
    fn default() -> Self {
        Self::new()
    }
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

    fn string_looks_like_rsc(s: &str) -> bool {
        if let Some(rest) = s.strip_prefix("$L").or_else(|| s.strip_prefix("$@")) {
            !rest.is_empty() && rest.chars().all(|c| c.is_ascii_hexdigit())
        } else {
            false
        }
    }

    fn value_looks_like_rsc(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::String(s) => Self::string_looks_like_rsc(s),
            serde_json::Value::Array(arr) => arr.iter().any(Self::value_looks_like_rsc),
            serde_json::Value::Object(obj) => obj.values().any(Self::value_looks_like_rsc),
            _ => false,
        }
    }

    async fn ssr_render_client_component(
        &self,
        module_path: &str,
        export_name: &str,
        props_json: &str,
    ) -> Result<String, RariError> {
        let script = format!(
            r"(async function() {{
                const ssrRender = globalThis['~rari']?.ssrRenderComponent;
                if (!ssrRender) return '';
                try {{
                    return await ssrRender({module_path}, {export_name}, {props_json});
                }} catch (e) {{
                    return '';
                }}
            }})()",
            module_path = serde_json::to_string(module_path).unwrap_or_default(),
            export_name = serde_json::to_string(export_name).unwrap_or_default(),
            props_json = props_json,
        );

        let result = self.runtime.execute_script("ssr_render_client".to_string(), script).await?;

        match result.as_str() {
            Some(html) if !html.is_empty() => Ok(html.to_string()),
            _ => Ok(String::new()),
        }
    }

    fn extract_script_tags(template: &str) -> String {
        #[expect(clippy::unwrap_used, reason = "Hardcoded regex pattern is guaranteed to be valid")]
        let script_regex = Regex::new(r"(?s)<script[^>]*>.*?</script>|<script[^>]*/>").unwrap();
        #[expect(clippy::unwrap_used, reason = "Hardcoded regex pattern is guaranteed to be valid")]
        let link_regex = Regex::new(r"<link[^>]*/?>").unwrap();

        let mut tags = Vec::new();

        for m in link_regex.find_iter(template) {
            tags.push(m.as_str().to_string());
        }

        for m in script_regex.find_iter(template) {
            tags.push(m.as_str().to_string());
        }

        tags.join("\n")
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
                format!(
                    r#"<link rel="stylesheet" href="{}">"#,
                    RscHtmlRenderer::escape_html_attribute(href)
                )
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

            let row = Self::parse_rsc_line(line)?;
            rows.push(row);
        }

        Ok(rows)
    }

    fn looks_like_rsc_payload(value: &Value) -> bool {
        match value {
            Value::Array(arr) => arr.first().and_then(|v| v.as_str()) == Some("$"),
            Value::Object(obj) => {
                obj.contains_key("Suspense")
                    || obj.contains_key("Component")
                    || obj.contains_key("Text")
                    || obj.contains_key("Reference")
                    || obj.contains_key("Promise")
            }
            _ => false,
        }
    }

    fn is_rsc_payload(s: &str) -> bool {
        if let Ok(value) = serde_json::from_str::<Value>(s) {
            Self::looks_like_rsc_payload(&value)
        } else {
            false
        }
    }

    fn parse_rsc_line(line: &str) -> Result<RscRow, RariError> {
        let colon_pos = line.find(':').ok_or_else(|| {
            RariError::internal(format!("Invalid RSC line format: missing colon in '{line}'"))
        })?;

        let (id_str, data_str) = line.split_at(colon_pos);
        let data_str = &data_str[1..];

        let id = u32::from_str_radix(id_str, 16)
            .map_err(|e| RariError::internal(format!("Invalid row ID '{id_str}': {e}")))?;

        if let Some(json_str) = data_str.strip_prefix('I') {
            if let Ok(import_data) = serde_json::from_str::<serde_json::Value>(json_str) {
                let module_path =
                    import_data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let export_name = import_data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();
                return Ok(RscRow {
                    id,
                    data: RscElement::ModuleImport { module_path, export_name },
                });
            }
            return Ok(RscRow { id, data: RscElement::Text(data_str.to_string()) });
        }

        let json_value: Value = serde_json::from_str(data_str)
            .map_err(|e| RariError::internal(format!("Invalid JSON in RSC line: {e}")))?;

        let data = Self::parse_rsc_element(&json_value)?;

        Ok(RscRow { id, data })
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

        let html_content = self.render_wire_to_fizz_html(rsc_flight_protocol).await?;

        self.assemble_document(html_content, cache_template, is_dev_mode, &css_links).await
    }

    async fn render_wire_to_fizz_html(
        &self,
        rsc_flight_protocol: &str,
    ) -> Result<String, RariError> {
        let wire_json =
            serde_json::to_string(rsc_flight_protocol).unwrap_or_else(|_| "\"\"".to_string());

        let script = format!(
            r"(async function() {{
                const fn = globalThis['~rari'] && globalThis['~rari'].renderWireToHtml;
                if (!fn) return {{ ok: false, error: 'renderWireToHtml unavailable' }};
                try {{
                    const html = await fn({wire_json});
                    return {{ ok: true, html: html }};
                }} catch (e) {{
                    return {{ ok: false, error: String((e && e.message) || e) }};
                }}
            }})()",
        );

        let result = self.runtime.execute_script("render_wire_to_fizz".to_string(), script).await?;

        let ok = result.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false);
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
            let script_tags = if is_dev_mode {
                String::new()
            } else {
                let template = self.load_template(cache_template, is_dev_mode).await?;
                Self::extract_script_tags(&template)
            };

            let mut final_html = html_content;

            if !script_tags.is_empty()
                && let Some(body_end) = final_html.rfind("</body>")
            {
                final_html.insert_str(body_end, &format!("\n{script_tags}\n"));
            }

            final_html = Self::inject_css_links(&final_html, css_links);

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
                "RSC->HTML: Starting wire format parsing, length: {}",
                rsc_flight_protocol.len()
            );
            let rsc_rows = self.parse_rsc_flight_protocol(rsc_flight_protocol).map_err(|e| {
                tracing::error!("RSC->HTML: Failed to parse wire format: {}", e);
                RariError::internal(format!("Failed to parse RSC wire format: {e}"))
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
                let script_tags = if is_dev_mode {
                    String::new()
                } else {
                    let template =
                        self.load_template(cache_template, is_dev_mode).await.map_err(|e| {
                            RariError::internal(format!("Failed to load HTML template: {e}"))
                        })?;
                    Self::extract_script_tags(&template)
                };

                let mut final_html = html_content.clone();

                if !script_tags.is_empty()
                    && let Some(body_end) = final_html.rfind("</body>")
                {
                    final_html.insert_str(body_end, &format!("\n{script_tags}\n"));
                }

                final_html = Self::inject_css_links(&final_html, &css_links);

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
                    let props_value = serde_json::Value::Object(
                        props.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                    );
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

                RscElement::Suspense { fallback_ref, children_ref, boundary_id, props: _ } => {
                    if let Ok(row_id) = Self::parse_reference(children_ref) {
                        let html = self.render_row(row_id, row_map, row_cache).await?;
                        if !html.is_empty() {
                            return Ok(html);
                        }
                    }

                    if let Ok(row_id) = Self::parse_reference(fallback_ref) {
                        let html = self.render_row(row_id, row_map, row_cache).await?;
                        return Ok(format!(
                            r#"<div data-boundary-id="{}" class="rari-suspense-boundary">{}</div>"#,
                            Self::escape_html_attribute(boundary_id),
                            html
                        ));
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
        props: &'a serde_json::Value,
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
        json: &'a serde_json::Value,
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

    fn escape_js_string(text: &str) -> String {
        text.cow_replace('\\', "\\\\")
            .cow_replace('\'', "\\'")
            .cow_replace('"', "\\\"")
            .cow_replace('\n', "\\n")
            .cow_replace('\r', "\\r")
            .cow_replace('\t', "\\t")
            .cow_replace("</", "<\\/")
            .cow_replace('\u{2028}', "\\u2028")
            .cow_replace('\u{2029}', "\\u2029")
            .cow_replace('\0', "\\0")
            .into_owned()
    }
}

pub struct RscToHtmlConverter {
    row_cache: FxHashMap<u32, String>,
    module_imports: FxHashMap<u32, (String, String)>,
    renderer: Arc<RscHtmlRenderer>,
    shell_sent: bool,
    asset_links: Option<String>,
    boundary_id_generator: BoundaryIdGenerator,
    rari_to_react_boundary_map: parking_lot::Mutex<FxHashMap<String, String>>,
    custom_shell: Option<String>,
    body_scripts: Option<String>,
    rsc_flight_protocol: Vec<String>,
    payload_embedding_disabled: bool,
    root_div_closed: bool,
    content_id_counter: AtomicU32,
}

impl RscToHtmlConverter {
    pub fn new(renderer: Arc<RscHtmlRenderer>) -> Self {
        Self {
            row_cache: FxHashMap::default(),
            module_imports: FxHashMap::default(),
            renderer,
            shell_sent: false,
            asset_links: None,
            boundary_id_generator: BoundaryIdGenerator::new(),
            rari_to_react_boundary_map: parking_lot::Mutex::new(FxHashMap::default()),
            custom_shell: None,
            body_scripts: None,
            rsc_flight_protocol: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
            content_id_counter: AtomicU32::new(0),
        }
    }

    pub fn with_assets(asset_links: String, renderer: Arc<RscHtmlRenderer>) -> Self {
        Self {
            row_cache: FxHashMap::default(),
            module_imports: FxHashMap::default(),
            renderer,
            shell_sent: false,
            asset_links: Some(asset_links),
            boundary_id_generator: BoundaryIdGenerator::new(),
            rari_to_react_boundary_map: parking_lot::Mutex::new(FxHashMap::default()),
            custom_shell: None,
            body_scripts: None,
            rsc_flight_protocol: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
            content_id_counter: AtomicU32::new(0),
        }
    }

    pub fn with_custom_shell(
        custom_shell: String,
        body_scripts: Option<String>,
        renderer: Arc<RscHtmlRenderer>,
    ) -> Self {
        Self {
            row_cache: FxHashMap::default(),
            module_imports: FxHashMap::default(),
            renderer,
            shell_sent: false,
            asset_links: None,
            boundary_id_generator: BoundaryIdGenerator::new(),
            rari_to_react_boundary_map: parking_lot::Mutex::new(FxHashMap::default()),
            custom_shell: Some(custom_shell),
            body_scripts,
            rsc_flight_protocol: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
            content_id_counter: AtomicU32::new(0),
        }
    }

    pub fn disable_payload_embedding(&mut self) {
        self.rsc_flight_protocol.clear();
        self.payload_embedding_disabled = true;
    }

    fn next_boundary_id(&self) -> String {
        self.boundary_id_generator.next()
    }

    #[expect(clippy::too_many_lines)]
    pub async fn convert_chunk(&mut self, chunk: RscStreamChunk) -> Result<Vec<u8>, RariError> {
        let chunk_type_str = format!("{:?}", chunk.chunk_type);

        let result: Result<Vec<u8>, RariError> = match chunk.chunk_type {
            RscChunkType::ModuleImport => {
                let rsc_line = String::from_utf8_lossy(&chunk.data);

                if !self.payload_embedding_disabled {
                    self.rsc_flight_protocol.push(rsc_line.trim().to_string());
                }

                let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();
                if parts.len() == 2
                    && let Ok(row_id) = u32::from_str_radix(parts[0], 16)
                    && let Some(i_data) = parts[1].trim().strip_prefix('I')
                    && let Ok(import_data) = serde_json::from_str::<serde_json::Value>(i_data)
                {
                    let module_path =
                        import_data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let export_name = import_data
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default")
                        .to_string();
                    self.module_imports.insert(row_id, (module_path, export_name));
                }

                Ok(Vec::new())
            }

            RscChunkType::InitialShell => {
                let html = if self.shell_sent {
                    match self.parse_and_render_rsc(&chunk.data, chunk.row_id).await {
                        Ok(rsc_html) => rsc_html,
                        Err(e) => {
                            error!("Error parsing RSC: {}", e);
                            Vec::new()
                        }
                    }
                } else {
                    self.shell_sent = true;
                    let mut output = self.generate_html_shell();

                    match self.parse_and_render_rsc(&chunk.data, chunk.row_id).await {
                        Ok(rsc_html) => {
                            output.extend(rsc_html);
                        }
                        Err(e) => {
                            error!("Error parsing initial RSC shell: {}", e);
                        }
                    }

                    output
                };
                Ok(html)
            }

            RscChunkType::BoundaryUpdate => {
                let mut html = Vec::new();

                if !self.root_div_closed {
                    self.root_div_closed = true;
                    html.extend(b"</div>\n");
                }

                if !self.payload_embedding_disabled {
                    let rsc_line = String::from_utf8_lossy(&chunk.data);
                    self.rsc_flight_protocol.push(rsc_line.trim().to_string());
                }

                match self.generate_boundary_replacement(&chunk).await {
                    Ok(boundary_html) => {
                        html.extend(boundary_html);
                        Ok(html)
                    }
                    Err(e) => {
                        error!("Error generating boundary replacement: {}", e);
                        Ok(html)
                    }
                }
            }

            RscChunkType::BoundaryError => {
                let mut html = Vec::new();

                if !self.root_div_closed {
                    self.root_div_closed = true;
                    html.extend(b"</div>\n");
                }

                if !self.payload_embedding_disabled {
                    let rsc_line = String::from_utf8_lossy(&chunk.data);
                    let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();
                    let error_msg = if parts.len() == 2 {
                        let json_part = parts[1].strip_prefix('E').unwrap_or(parts[1]);
                        serde_json::from_str::<serde_json::Value>(json_part)
                            .ok()
                            .and_then(|error_data| {
                                error_data["error"].as_str().map(ToString::to_string)
                            })
                            .unwrap_or_else(|| "Error loading content".to_string())
                    } else {
                        "Error loading content".to_string()
                    };

                    let placeholder_payload = serde_json::json!([
                        "$",
                        "div",
                        null,
                        {
                            "className": "rari-error",
                            "children": ["Error loading content: ", error_msg],
                        }
                    ]);
                    let placeholder_row = format!("{:x}:{}\n", chunk.row_id, placeholder_payload);
                    self.rsc_flight_protocol.push(placeholder_row.trim().to_string());
                }

                let error_html = self.generate_error_replacement(&chunk);
                html.extend(error_html);
                Ok(html)
            }

            RscChunkType::StreamComplete => {
                let mut html = Vec::new();

                if !self.root_div_closed {
                    self.root_div_closed = true;
                    html.extend(b"</div>\n");
                }

                html.extend(self.generate_html_closing());
                Ok(html)
            }
        };

        if let Err(ref e) = result {
            error!("Chunk conversion error for {:?}: {}", chunk.chunk_type, e);
            return Err(StreamingError::ChunkConversionError {
                message: e.to_string(),
                chunk_type: Some(chunk_type_str),
            }
            .into());
        }

        result
    }

    fn generate_fallback_error_html() -> Vec<u8> {
        r#"<div style="color: red; border: 1px solid red; padding: 10px; margin: 10px 0;">
            An error occurred while loading content.
        </div>"#
            .as_bytes()
            .to_vec()
    }

    fn streaming_bridge_script() -> &'static str {
        r"<script>
(function(){
  if(window['~rari']&&window['~rari'].streamingBridgeInstalled)return;
  if(!window['~rari'])window['~rari']={};
  window['~rari'].streamingBridgeInstalled=true;
  window.addEventListener('rari:html-stream-row',function(e){
    var detail=e.detail;
    if(!detail||!detail.rscRow)return;
    var navigationId=window['~rari']&&window['~rari'].navigationId;
    window.dispatchEvent(new CustomEvent('rari:rsc-row',{detail:{rscRow:detail.rscRow,navigationId:navigationId}}));
  });
})();
</script>"
    }

    fn generate_html_shell(&self) -> Vec<u8> {
        if let Some(custom_shell) = &self.custom_shell {
            let bridge_script = Self::streaming_bridge_script();

            let custom_shell_with_bridge = if let Some(body_pos) = custom_shell.find("<body") {
                if let Some(body_end) = custom_shell[body_pos..].find('>') {
                    let insert_pos = body_pos + body_end + 1;
                    format!(
                        "{}{}{}",
                        &custom_shell[..insert_pos],
                        bridge_script,
                        &custom_shell[insert_pos..]
                    )
                } else {
                    format!("{custom_shell}{bridge_script}")
                }
            } else {
                format!("{custom_shell}{bridge_script}")
            };

            return custom_shell_with_bridge.as_bytes().to_vec();
        }

        let asset_tags = self.asset_links.as_deref().unwrap_or("");
        let bridge_script = Self::streaming_bridge_script();

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>rari App</title>
    {asset_tags}
    <style>
        .rari-loading {{
            animation: rari-pulse 1.5s ease-in-out infinite;
        }}
        @keyframes rari-pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.5; }}
        }}
    </style>
</head>
<body>
{bridge_script}
<div id="root">"#
        )
        .as_bytes()
        .to_vec()
    }

    pub fn generate_html_closing(&self) -> Vec<u8> {
        let body_scripts = self.body_scripts.as_deref().unwrap_or("");

        let mut rows_with_ids: Vec<(u32, String)> = Vec::new();
        for row in &self.rsc_flight_protocol {
            if let Some(colon_pos) = row.find(':') {
                if let Ok(row_id) = u32::from_str_radix(&row[..colon_pos], 16) {
                    rows_with_ids.push((row_id, row.clone()));
                } else {
                    rows_with_ids.push((u32::MAX, row.clone()));
                }
            } else {
                rows_with_ids.push((u32::MAX, row.clone()));
            }
        }

        rows_with_ids.sort_by_key(|(id, _)| if *id == 0 { u32::MAX - 1 } else { *id });

        let mut rsc_payload = String::new();
        for (_, row) in &rows_with_ids {
            rsc_payload.push_str(row);
            rsc_payload.push('\n');
        }

        let has_row_0 = rows_with_ids.iter().any(|(id, _)| *id == 0);

        if !has_row_0
            && let Some((max_id, _)) =
                rows_with_ids.iter().filter(|(id, _)| *id != u32::MAX).max_by_key(|(id, _)| *id)
            && *max_id > 0
        {
            let row_0 = format!("0:\"${max_id:x}\"\n");
            rsc_payload.insert_str(0, &row_0);
        }

        let escaped_payload = rsc_payload.cow_replace("</", "<\\/");

        let rsc_script = if rsc_payload.is_empty() {
            String::new()
        } else {
            format!(
                r#"<script id="__RARI_RSC_PAYLOAD__" type="text/x-component">{escaped_payload}</script>"#
            )
        };

        format!(
            r"{rsc_script}
{body_scripts}
<script>
if (typeof window !== 'undefined') {{
    if (!window['~rari']) window['~rari'] = {{}};
    if (!window['~rari'].streaming) window['~rari'].streaming = {{}};
    window['~rari'].streaming.complete = true;
    window.dispatchEvent(new Event('rari:stream-complete'));
    window.dispatchEvent(new CustomEvent('rari:html-stream-row', {{ detail: {{ rscRow: 'STREAM_COMPLETE' }} }}));
}}
</script>
</body>
</html>"
        )
        .as_bytes()
        .to_vec()
    }

    async fn parse_and_render_rsc(
        &mut self,
        data: &[u8],
        row_id: u32,
    ) -> Result<Vec<u8>, RariError> {
        let rsc_line = String::from_utf8_lossy(data);

        let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();
        if parts.len() != 2 {
            return Ok(Vec::new());
        }

        if !self.payload_embedding_disabled {
            self.rsc_flight_protocol.push(rsc_line.trim().to_string());
        }

        let json_str = parts[1].trim();
        if json_str.is_empty() {
            return Ok(Vec::new());
        }

        if let Some(i_data) = json_str.strip_prefix('I') {
            if let Ok(import_data) = serde_json::from_str::<serde_json::Value>(i_data) {
                let module_path =
                    import_data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let export_name = import_data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();
                self.module_imports.insert(row_id, (module_path, export_name));
            }
            return Ok(Vec::new());
        }

        if json_str.starts_with('S') || json_str.starts_with('E') {
            return Ok(Vec::new());
        }

        if json_str.starts_with('"') && json_str.contains("$S") {
            return Ok(Vec::new());
        }

        if json_str.starts_with('"')
            && let Ok(serde_json::Value::String(s)) = serde_json::from_str(json_str)
            && RscHtmlRenderer::is_rsc_payload(&s)
        {
            return Ok(Vec::new());
        }

        if json_str.starts_with("[[") || json_str.len() > PAYLOAD_SIZE_LIMIT {
            return Ok(Vec::new());
        }

        let rsc_data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| RariError::internal(format!("Invalid RSC JSON: {e}")))?;

        let html = self.rsc_element_to_html(&rsc_data).await?;

        self.row_cache.insert(row_id, html.clone());

        Ok(html.into_bytes())
    }

    #[expect(clippy::too_many_lines)]
    fn rsc_element_to_html<'a>(
        &'a self,
        element: &'a serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<String, RariError>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(s) = element.as_str() {
                if s.starts_with("$$") {
                    return Ok(escape_html(&s[1..]));
                }

                if let Some(stripped) = s.strip_prefix("$L").or_else(|| s.strip_prefix("$@"))
                    && let Ok(row_id) = u32::from_str_radix(stripped, 16)
                {
                    let cached = self.row_cache.get(&row_id).cloned().unwrap_or_default();
                    return Ok(cached);
                }

                if s.starts_with('$')
                    && s.len() > 1
                    && s[1..].chars().all(|c| c.is_ascii_hexdigit())
                {
                    let row_id: u32 = u32::from_str_radix(&s[1..], 16).map_err(|_| {
                        RariError::internal(format!("Invalid chunk reference: {s}"))
                    })?;
                    let cached = self.row_cache.get(&row_id).cloned().unwrap_or_default();
                    return Ok(cached);
                }

                if s.starts_with("I[") || s.starts_with("S[") || s.starts_with("E[") {
                    return Ok(String::new());
                }

                if let Some(colon_pos) = s.find(':')
                    && s[..colon_pos].chars().all(|c| c.is_ascii_hexdigit())
                {
                    let after_colon = &s[colon_pos + 1..];
                    if !after_colon.is_empty()
                        && (after_colon.starts_with('[') || after_colon.starts_with('{'))
                        && serde_json::from_str::<serde_json::Value>(after_colon).is_ok()
                    {
                        return Ok(String::new());
                    }
                }

                return Ok(escape_html(s));
            }

            if let Some(arr) = element.as_array() {
                if arr.len() >= 4 && arr[0].as_str() == Some("$") {
                    let element_type = arr[1].as_str().unwrap_or("div");
                    let props = arr[3].as_object();

                    let is_rsc_reference = element_type.starts_with('$')
                        && element_type.len() > 1
                        && !element_type.starts_with("$S")
                        && element_type[1..].chars().all(|c| c.is_ascii_hexdigit());

                    let is_client_component = element_type.starts_with("$L")
                        || element_type.starts_with("$@")
                        || element_type.contains('#')
                        || element_type.contains('/');

                    if element_type == "$Sreact.suspense"
                        || element_type == "react.suspense"
                        || element_type == "suspense"
                        || element_type.starts_with("$S")
                    {
                        return self.render_suspense_boundary(element_type, props).await;
                    }

                    if is_rsc_reference {
                        let row_id = u32::from_str_radix(&element_type[1..], 16).map_err(|_| {
                            RariError::internal(format!("Invalid chunk reference: {element_type}"))
                        })?;
                        return Ok(self.row_cache.get(&row_id).cloned().unwrap_or_default());
                    }

                    if is_client_component {
                        return self.render_client_component_placeholder(element_type, props).await;
                    }

                    return self.render_html_element(element_type, props).await;
                }

                let mut html = String::new();
                for child in arr {
                    let child_html = self.rsc_element_to_html(child).await?;
                    html.push_str(&child_html);
                }
                return Ok(html);
            }

            if let Some(obj) = element.as_object() {
                if let (Some(element_type), Some(props)) = (obj.get("type"), obj.get("props"))
                    && let Some(type_str) = element_type.as_str()
                {
                    let props_obj = props.as_object();

                    let is_rsc_reference = type_str.starts_with('$')
                        && type_str.len() > 1
                        && !type_str.starts_with("$S")
                        && type_str[1..].chars().all(|c| c.is_ascii_hexdigit());

                    let is_client_component = type_str.starts_with("$L")
                        || type_str.starts_with("$@")
                        || type_str.contains('#')
                        || type_str.contains('/');

                    if type_str == "$Sreact.suspense"
                        || type_str == "react.suspense"
                        || type_str == "suspense"
                        || type_str.starts_with("$S")
                    {
                        return self.render_suspense_boundary(type_str, props_obj).await;
                    }

                    if is_rsc_reference {
                        let row_id = u32::from_str_radix(&type_str[1..], 16).map_err(|_| {
                            RariError::internal(format!("Invalid chunk reference: {type_str}"))
                        })?;
                        return Ok(self.row_cache.get(&row_id).cloned().unwrap_or_default());
                    }

                    if is_client_component {
                        return self.render_client_component_placeholder(type_str, props_obj).await;
                    }

                    return self.render_html_element(type_str, props_obj).await;
                }
                if let Some(children) = obj.get("children") {
                    return self.rsc_element_to_html(children).await;
                }
            }

            Ok(String::new())
        })
    }

    async fn render_client_component_placeholder(
        &self,
        component_ref: &str,
        props: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Result<String, RariError> {
        let props_are_rsc =
            props.is_some_and(|p| p.values().any(RscHtmlRenderer::value_looks_like_rsc));

        if !props_are_rsc
            && let Some(stripped) =
                component_ref.strip_prefix("$L").or_else(|| component_ref.strip_prefix("$@"))
            && let Ok(module_row_id) = u32::from_str_radix(stripped, 16)
            && let Some((module_path, export_name)) = self.module_imports.get(&module_row_id)
        {
            let props_json = if let Some(p) = props {
                serde_json::to_string(&unescape_rsc_value(&serde_json::Value::Object(p.clone())))
                    .unwrap_or_default()
            } else {
                "{}".to_string()
            };
            if let Ok(html) = self
                .renderer
                .ssr_render_client_component(module_path, export_name, &props_json)
                .await
                && !html.is_empty()
            {
                return Ok(html);
            }
        }

        if let Some(props) = props
            && let Some(children) = props.get("children")
        {
            return self.rsc_element_to_html(children).await;
        }
        Ok(String::new())
    }

    async fn render_suspense_boundary(
        &self,
        _element_type: &str,
        props: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Result<String, RariError> {
        if let Some(props) = props {
            let rari_boundary_id =
                props.get("~boundaryId").and_then(|v| v.as_str()).unwrap_or("unknown");

            let (react_boundary_id, is_duplicate) = {
                let map = self.rari_to_react_boundary_map.lock();
                if let Some(existing_id) = map.get(rari_boundary_id) {
                    (existing_id.clone(), true)
                } else {
                    drop(map);
                    let new_id = self.next_boundary_id();
                    self.rari_to_react_boundary_map
                        .lock()
                        .insert(rari_boundary_id.to_string(), new_id.clone());
                    (new_id, false)
                }
            };

            if is_duplicate {
                return Ok(String::new());
            }

            let children = props.get("children");
            let children_html = if let Some(child) = children {
                if child.is_null() {
                    None
                } else {
                    let rendered = self.rsc_element_to_html(child).await?;
                    if !rendered.is_empty() && rendered.trim() != "null" {
                        Some(rendered)
                    } else {
                        None
                    }
                }
            } else {
                None
            };

            let content_html = if let Some(children_html) = children_html {
                children_html
            } else {
                let fallback = props.get("fallback");
                if let Some(fb) = fallback {
                    self.rsc_element_to_html(fb).await?
                } else {
                    String::from("<div class=\"rari-loading\">Loading...</div>")
                }
            };

            let html = format!(
                "<!--$?--><template id=\"{}\"></template><div data-boundary-id=\"{}\">{}</div><!--/$-->",
                react_boundary_id,
                RscHtmlRenderer::escape_html_attribute(rari_boundary_id),
                content_html
            );

            Ok(html)
        } else {
            Ok(String::from("<div class=\"rari-loading\">Loading...</div>"))
        }
    }

    async fn render_html_element(
        &self,
        tag: &str,
        props: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Result<String, RariError> {
        if tag == "$Sreact.suspense" || tag == "react.suspense" || tag == "suspense" {
            return self.render_suspense_boundary(tag, props).await;
        }

        if !tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':') {
            return Err(RariError::internal(format!("Invalid tag name: {tag}")));
        }

        let mut html = format!("<{tag}");

        if let Some(props) = props {
            for (key, value) in props {
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
                        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                        write!(
                            html,
                            " style=\"{}\"",
                            RscHtmlRenderer::escape_html_attribute(&style_str)
                        )
                        .unwrap();
                    }
                    continue;
                }

                if let Some(b) = value.as_bool() {
                    if is_boolean_html_attribute(attr_name) {
                        if b {
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            write!(&mut html, " {attr_name}").unwrap();
                        }
                    } else {
                        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                        write!(html, " {}=\"{}\"", attr_name, if b { "true" } else { "false" })
                            .unwrap();
                    }
                    continue;
                }

                if let Some(s) = value.as_str() {
                    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                    write!(
                        html,
                        " {}=\"{}\"",
                        attr_name,
                        RscHtmlRenderer::escape_html_attribute(s)
                    )
                    .unwrap();
                    continue;
                }

                if value.is_number() {
                    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                    write!(
                        html,
                        " {}=\"{}\"",
                        attr_name,
                        RscHtmlRenderer::escape_html_attribute(&value.to_string())
                    )
                    .unwrap();
                }
            }
        }

        if SELF_CLOSING_TAGS.contains(&tag) {
            html.push_str(" />");
            return Ok(html);
        }

        html.push('>');

        if let Some(props) = props
            && let Some(children) = props.get("children")
        {
            let children_html = self.rsc_element_to_html(children).await?;
            html.push_str(&children_html);
        }

        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
        write!(&mut html, "</{tag}>").unwrap();

        Ok(html)
    }

    async fn generate_boundary_replacement(
        &self,
        chunk: &RscStreamChunk,
    ) -> Result<Vec<u8>, RariError> {
        let rsc_line = String::from_utf8_lossy(&chunk.data);

        let boundary_id = match &chunk.boundary_id {
            Some(id) => id.clone(),
            None => {
                let escaped_row = RscHtmlRenderer::escape_js_string(rsc_line.as_ref());
                let script = format!(
                    r"<script>(function(){{if(!window['~rari'])window['~rari']={{}};if(!window['~rari'].streaming)window['~rari'].streaming={{}};if(!window['~rari'].streaming.bufferedRows)window['~rari'].streaming.bufferedRows=[];window['~rari'].streaming.bufferedRows.push('{escaped_row}');window.dispatchEvent(new CustomEvent('rari:html-stream-row',{{detail:{{rscRow:'{escaped_row}'}}}}));}})();</script>"
                );
                return Ok(script.into_bytes());
            }
        };

        let line_for_parsing = rsc_line
            .strip_suffix("\r\n")
            .or_else(|| rsc_line.strip_suffix('\n'))
            .or_else(|| rsc_line.strip_suffix('\r'))
            .unwrap_or(rsc_line.as_ref());

        let parts: Vec<&str> = line_for_parsing.splitn(2, ':').collect();

        if parts.len() != 2 {
            error!("Invalid boundary update format: {}", rsc_line);
            return Ok(Vec::new());
        }

        let row_id = chunk.row_id;
        let escaped_row = RscHtmlRenderer::escape_js_string(rsc_line.as_ref());

        let react_boundary_id = {
            let map = self.rari_to_react_boundary_map.lock();
            map.get(&boundary_id).cloned()
        };

        let dom_swap_html = if let Some(ref react_id) = react_boundary_id {
            let content_id =
                format!("S:{}", self.content_id_counter.fetch_add(1, Ordering::SeqCst));

            let content_json = parts[1];
            let rendered_html = if let Some(text) = content_json.strip_prefix('T') {
                escape_html(text)
            } else {
                match serde_json::from_str::<serde_json::Value>(content_json) {
                    Ok(content) => match self.rsc_element_to_html(&content).await {
                        Ok(html) if !html.is_empty() => html,
                        _ => String::new(),
                    },
                    Err(_) => String::new(),
                }
            };

            if rendered_html.is_empty() {
                String::new()
            } else {
                format!(
                    "<div hidden id=\"{content_id}\">{rendered_html}</div>\n<script>$RC=window.$RC||function(b,c){{var t=document.getElementById(b);var s=document.getElementById(c);if(t&&s){{var p=t.parentNode;var f=document.createDocumentFragment();Array.from(s.childNodes).forEach(function(n){{f.appendChild(n)}});var d=t.nextSibling;while(d&&!(d.nodeType===8&&d.data==='/$')){{var next=d.nextSibling;d.remove();d=next;}}if(d)d.remove();p.insertBefore(f,t.nextSibling);t.remove();s.remove();}}}};$RC(\"{react_id}\",\"{content_id}\")</script>",
                )
            }
        } else {
            String::new()
        };

        let rsc_buffer_script = format!(
            "<script data-rsc-boundary=\"{}\" data-row-id=\"{}\">\n(function(){{\n  if(!window['~rari'])window['~rari']={{}};\n  if(!window['~rari'].streaming)window['~rari'].streaming={{}};\n  if(!window['~rari'].streaming.bufferedRows)window['~rari'].streaming.bufferedRows=[];\n  window['~rari'].streaming.bufferedRows.push('{}');\n  window.dispatchEvent(new CustomEvent('rari:html-stream-row', {{detail: {{rscRow: '{}'}}}}));\n}})();\n</script>",
            RscHtmlRenderer::escape_html_attribute(&boundary_id),
            row_id,
            escaped_row,
            escaped_row,
        );

        let output = format!("{rsc_buffer_script}{dom_swap_html}");

        Ok(output.into_bytes())
    }

    fn generate_error_replacement(&self, chunk: &RscStreamChunk) -> Vec<u8> {
        let rsc_line = String::from_utf8_lossy(&chunk.data);
        let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();

        if parts.len() != 2 {
            return Self::generate_fallback_error_html();
        }

        let json_part = parts[1].strip_prefix('E').unwrap_or(parts[1]);

        let error_data = match serde_json::from_str::<serde_json::Value>(json_part) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to parse error data from stream, using fallback: {}", e);
                return Self::generate_fallback_error_html();
            }
        };

        let rari_boundary_id = error_data["boundary_id"].as_str().unwrap_or("unknown").to_string();
        let error_message =
            error_data["error"].as_str().unwrap_or("Error loading content").to_string();

        let react_boundary_id =
            self.rari_to_react_boundary_map.lock().get(&rari_boundary_id).cloned();

        if let Some(react_boundary_id) = react_boundary_id {
            let error_html = format!(
                r#"<div class="rari-error" style="color: red; border: 1px solid red; padding: 10px; border-radius: 4px; background-color: #fff5f5;">
                <strong>Error loading content:</strong> {}
            </div>"#,
                escape_html(&error_message)
            );

            let content_id = format!("E:{}", react_boundary_id.trim_start_matches("B:"));

            let error_update = format!(
                r#"<div hidden id="{content_id}">{error_html}</div><script>$RC=window.$RC||function(b,c){{const t=document.getElementById(b);const s=document.getElementById(c);if(t&&s){{const p=t.parentNode;Array.from(s.childNodes).forEach(n=>p.insertBefore(n,t));t.remove();s.remove();}}}};$RC("{react_boundary_id}","{content_id}");</script>"#
            );

            error_update.into_bytes()
        } else {
            Self::generate_fallback_error_html()
        }
    }
}

impl Default for RscToHtmlConverter {
    fn default() -> Self {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        Self::new(renderer)
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
    fn test_boundary_id_generator_sequential() {
        let generator = BoundaryIdGenerator::new();

        assert_eq!(generator.next(), "B:0");
        assert_eq!(generator.next(), "B:1");
        assert_eq!(generator.next(), "B:2");
        assert_eq!(generator.next(), "B:3");
    }

    #[test]
    fn test_boundary_id_generator_default() {
        let generator = BoundaryIdGenerator::default();

        assert_eq!(generator.next(), "B:0");
    }

    #[test]
    fn test_rsc_to_html_converter_uses_boundary_generator() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let converter = RscToHtmlConverter::new(renderer);

        assert_eq!(converter.next_boundary_id(), "B:0");
        assert_eq!(converter.next_boundary_id(), "B:1");
        assert_eq!(converter.next_boundary_id(), "B:2");
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
    async fn test_render_suspense_with_fallback() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let mut converter = RscToHtmlConverter::new(renderer);

        let fallback_html = r#"<div class="loading">Loading...</div>"#;
        converter.row_cache.insert(1, fallback_html.to_string());

        let suspense_element = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": "$L1",
                "children": "$L999",
                "~boundaryId": "suspense_test"
            }
        ]);

        let html = converter.rsc_element_to_html(&suspense_element).await.unwrap();

        assert!(html.contains("Loading..."), "Should render fallback content");
        assert!(html.contains("class=\"loading\""), "Should have loading class");
        assert!(html.contains("<!--$?-->"), "Should have React boundary start marker");
        assert!(
            html.contains("<template id=\"B:0\">"),
            "Should have React template with boundary ID B:0"
        );
        assert!(html.contains("<!--/$-->"), "Should have React boundary end marker");
    }

    #[tokio::test]
    async fn test_render_suspense_with_resolved_children() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let mut converter = RscToHtmlConverter::new(renderer);

        converter.row_cache.insert(1, r#"<div class="loading">Loading...</div>"#.to_string());
        converter.row_cache.insert(2, r#"<div class="content">Actual Content</div>"#.to_string());

        let suspense_element = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": "$L1",
                "children": "$L2",
                "~boundaryId": "suspense_test"
            }
        ]);

        let html = converter.rsc_element_to_html(&suspense_element).await.unwrap();

        assert!(html.contains("Actual Content"), "Should render resolved children");
        assert!(html.contains("class=\"content\""), "Should have content class");
        assert!(
            !html.contains("Loading..."),
            "Should not render fallback when children are resolved"
        );
        assert!(html.contains("<!--$?-->"), "Should have React boundary start marker");
        assert!(
            html.contains("<template id=\"B:0\">"),
            "Should have React template with boundary ID"
        );
        assert!(html.contains("<!--/$-->"), "Should have React boundary end marker");
    }

    #[tokio::test]
    async fn test_render_nested_suspense_boundaries() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let mut converter = RscToHtmlConverter::new(renderer);

        converter.row_cache.insert(1, r"<div>Outer Loading</div>".to_string());
        converter.row_cache.insert(2, r"<div>Inner Loading</div>".to_string());
        converter.row_cache.insert(3, r"<div>Inner Content</div>".to_string());

        let inner_suspense = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": "$L2",
                "children": "$L3",
                "~boundaryId": "inner"
            }
        ]);
        let inner_html = converter.rsc_element_to_html(&inner_suspense).await.unwrap();
        converter.row_cache.insert(4, inner_html);

        let outer_suspense = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": "$L1",
                "children": "$L4",
                "~boundaryId": "outer"
            }
        ]);
        let html = converter.rsc_element_to_html(&outer_suspense).await.unwrap();

        assert!(html.contains("Inner Content"), "Should render inner content");
        assert!(!html.contains("Inner Loading"), "Should not render inner fallback");
        assert!(!html.contains("Outer Loading"), "Should not render outer fallback");
        assert!(html.contains("<!--$?-->"), "Should have React boundary markers");
        assert!(html.contains("<template id=\"B:0\">"), "Should have inner boundary ID B:0");
        assert!(html.contains("<template id=\"B:1\">"), "Should have outer boundary ID B:1");
        assert!(html.contains("<!--/$-->"), "Should have React boundary end markers");
    }

    #[tokio::test]
    async fn test_render_suspense_missing_fallback() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let converter = RscToHtmlConverter::new(renderer);

        let suspense_element = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "children": "$L999",
                "~boundaryId": "suspense_test"
            }
        ]);

        let html = converter.rsc_element_to_html(&suspense_element).await.unwrap();

        assert!(html.contains("<!--$?-->"), "Should have React boundary start marker");
        assert!(
            html.contains("<template id=\"B:0\">"),
            "Should have React template with boundary ID"
        );
        assert!(html.contains("<!--/$-->"), "Should have React boundary end marker");
        assert!(html.contains("rari-loading"), "Should have default loading fallback");
    }

    #[tokio::test]
    async fn test_render_suspense_inline_children() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let mut converter = RscToHtmlConverter::new(renderer);

        converter.row_cache.insert(1, r"<div>Loading</div>".to_string());

        let suspense_element = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": "$L1",
                "children": ["$", "div", null, {"children": "Inline Content"}],
                "~boundaryId": "suspense_test"
            }
        ]);

        let html = converter.rsc_element_to_html(&suspense_element).await.unwrap();

        assert!(html.contains("Inline Content"), "Should render inline children");
        assert!(!html.contains("Loading"), "Should not render fallback when children are inline");
        assert!(html.contains("<!--$?-->"), "Should have React boundary start marker");
        assert!(
            html.contains("<template id=\"B:0\">"),
            "Should have React template with boundary ID"
        );
        assert!(html.contains("<!--/$-->"), "Should have React boundary end marker");
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
    async fn test_converter_filters_invalid_attribute_names() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let converter = RscToHtmlConverter::new(renderer);

        let element = serde_json::json!([
            "$",
            "button",
            null,
            {
                "type": "button",
                "class": "btn",
                "onclick='alert(1)'": "malicious",
                "on load": "bad",
                "data-id": "123",
                "1invalid": "bad",
                "@bad": "bad"
            }
        ]);

        let html = converter.rsc_element_to_html(&element).await.unwrap();

        assert!(html.contains("type=\"button\""), "Valid type attribute should be rendered");
        assert!(html.contains("class=\"btn\""), "Valid class attribute should be rendered");
        assert!(html.contains("data-id=\"123\""), "Valid data attribute should be rendered");

        assert!(!html.contains("onclick"), "Invalid onclick attribute should be filtered");
        assert!(!html.contains("alert(1)"), "Malicious code should not be in output");
        assert!(!html.contains("on load"), "Attribute with space should be filtered");
        assert!(!html.contains("1invalid"), "Attribute starting with number should be filtered");
        assert!(!html.contains("@bad"), "Attribute starting with @ should be filtered");
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
    async fn test_attribute_validation_edge_cases() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let converter = RscToHtmlConverter::new(renderer);

        let element = serde_json::json!([
            "$",
            "div",
            null,
            {
                "a": "single char valid",
                "A": "uppercase valid",
                "_": "underscore only valid",
                ":": "colon only - expected invalid",
                "a-b-c": "multiple hyphens valid",
                "a.b.c": "multiple periods valid",
                "a:b:c": "multiple colons valid",
                "a_b_c": "multiple underscores valid",
                "": "empty invalid",
                "-": "hyphen only invalid",
                ".": "period only invalid"
            }
        ]);

        let html = converter.rsc_element_to_html(&element).await.unwrap();

        assert!(html.contains("a=\"single char valid\""), "Single letter should be valid");
        assert!(html.contains("A=\"uppercase valid\""), "Uppercase letter should be valid");
        assert!(html.contains("_=\"underscore only valid\""), "Underscore only should be valid");
        assert!(
            html.contains("a-b-c=\"multiple hyphens valid\""),
            "Multiple hyphens should be valid"
        );
        assert!(
            html.contains("a.b.c=\"multiple periods valid\""),
            "Multiple periods should be valid"
        );
        assert!(
            html.contains("a:b:c=\"multiple colons valid\""),
            "Multiple colons should be valid"
        );
        assert!(
            html.contains("a_b_c=\"multiple underscores valid\""),
            "Multiple underscores should be valid"
        );

        assert!(!html.contains(":=\"colon only"), "Colon only should be invalid");
        assert!(!html.contains("-=\"hyphen only invalid\""), "Hyphen only should be invalid");
        assert!(!html.contains(".=\"period only invalid\""), "Period only should be invalid");
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
    async fn test_render_html_element_validates_tag_names() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let converter = RscToHtmlConverter::new(renderer);

        let valid_element = serde_json::json!(["$", "div", null, {"class": "test"}]);
        let result = converter.rsc_element_to_html(&valid_element).await;
        assert!(result.is_ok(), "Valid tag 'div' should be accepted");

        let valid_custom = serde_json::json!(["$", "custom-element", null, {}]);
        let result = converter.rsc_element_to_html(&valid_custom).await;
        assert!(result.is_ok(), "Valid custom element with hyphen should be accepted");

        let valid_namespace = serde_json::json!(["$", "svg:circle", null, {}]);
        let result = converter.rsc_element_to_html(&valid_namespace).await;
        assert!(result.is_ok(), "Valid namespaced element should be accepted");

        let invalid_space = serde_json::json!(["$", "div onclick", null, {}]);
        let result = converter.rsc_element_to_html(&invalid_space).await;
        assert!(result.is_err(), "Tag with space should be rejected");
        assert!(
            result.unwrap_err().to_string().contains("Invalid tag name"),
            "Error should mention invalid tag"
        );

        let invalid_angle = serde_json::json!(["$", "div>script", null, {}]);
        let result = converter.rsc_element_to_html(&invalid_angle).await;
        if let Ok(html) = result {
            panic!("Tag with angle bracket should be rejected, but got: {html:?}");
        }
        assert!(result.is_err(), "Tag with angle bracket should be rejected");

        let invalid_quote = serde_json::json!(["$", "div\"onclick=\"alert(1)", null, {}]);
        let result = converter.rsc_element_to_html(&invalid_quote).await;
        assert!(result.is_err(), "Tag with quotes should be rejected");
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
    async fn test_tag_validation_prevents_xss() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let converter = RscToHtmlConverter::new(renderer);

        let xss_attempts = vec![
            "div onload=alert(1)",
            "div>script",
            "div style=xss:expression(alert(1))",
            "div\nonclick=alert(1)",
            "div'onclick='alert(1)",
            "div\"onclick=\"alert(1)",
        ];

        for malicious_tag in xss_attempts {
            let element = serde_json::json!(["$", malicious_tag, null, {}]);
            let result = converter.rsc_element_to_html(&element).await;

            assert!(result.is_err(), "Malicious tag '{malicious_tag}' should be rejected");
        }
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
    async fn test_converter_boolean_attributes() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let converter = RscToHtmlConverter::new(renderer);

        let element = serde_json::json!([
            "$",
            "input",
            null,
            {
                "type": "checkbox",
                "checked": true,
                "disabled": false,
                "aria-checked": true,
                "aria-disabled": false
            }
        ]);

        let html = converter.rsc_element_to_html(&element).await.unwrap();

        assert!(html.contains(" checked"), "checked=true should be presence-only: {html}");
        assert!(!html.contains(" checked="), "checked should not have a value: {html}");
        assert!(!html.contains(" disabled"), "disabled=false should be omitted: {html}");

        assert!(
            html.contains("aria-checked=\"true\""),
            "aria-checked=true should render as \"true\": {html}"
        );
        assert!(
            html.contains("aria-disabled=\"false\""),
            "aria-disabled=false should render as \"false\": {html}"
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
