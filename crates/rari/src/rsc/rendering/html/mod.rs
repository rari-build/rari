use crate::error::{RariError, StreamingError};
use crate::rsc::rendering::streaming::{RscChunkType, RscStreamChunk};
use crate::rsc::types::RscElement;
use crate::runtime::JsExecutionRuntime;
use cow_utils::CowUtils;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::error;

pub mod tests;

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
            let kebab_key = k.chars().fold(String::new(), |mut acc, c| {
                if c.is_uppercase() {
                    acc.push('-');
                    acc.push(
                        c.to_lowercase()
                            .next()
                            .expect("to_lowercase() always returns at least one character"),
                    );
                } else {
                    acc.push(c);
                }
                acc
            });

            let value_str = if let Some(s) = v.as_str() {
                Some(s.to_string())
            } else if v.is_null() || v.as_bool().is_some() {
                None
            } else if let Some(u) = v.as_u64() {
                if UNITLESS_PROPERTIES.contains(&kebab_key.as_str()) {
                    Some(u.to_string())
                } else {
                    Some(format!("{}px", u))
                }
            } else if let Some(i) = v.as_i64() {
                if UNITLESS_PROPERTIES.contains(&kebab_key.as_str()) {
                    Some(i.to_string())
                } else {
                    Some(format!("{}px", i))
                }
            } else if let Some(f) = v.as_f64() {
                if f.is_finite() {
                    let formatted = format!("{:.10}", f)
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string();

                    if UNITLESS_PROPERTIES.contains(&kebab_key.as_str()) {
                        Some(formatted)
                    } else {
                        Some(format!("{}px", formatted))
                    }
                } else {
                    None
                }
            } else if v.is_object() || v.is_array() {
                Some(serde_json::to_string(v).unwrap_or_else(|_| String::from("{}")))
            } else {
                Some(v.to_string())
            };
            value_str.map(|val| format!("{}:{}", kebab_key, val))
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
        format!("B:{}", id)
    }
}

impl Default for BoundaryIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
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
        use regex::Regex;

        #[allow(clippy::disallowed_methods)]
        let script_regex = Regex::new(r#"(?s)<script[^>]*>.*?</script>|<script[^>]*/>"#).unwrap();
        #[allow(clippy::disallowed_methods)]
        let link_regex = Regex::new(r#"<link[^>]*/?>"#).unwrap();

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
                    self.inject_vite_client_if_needed(&content)
                } else {
                    content
                }
            }
            Err(e) => {
                if is_dev_mode {
                    self.generate_dev_template_fallback()
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

    fn inject_vite_client_if_needed(&self, html: &str) -> String {
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
{}"#,
            html
        )
    }

    fn generate_dev_template_fallback(&self) -> String {
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
            if let Ok(content) = tokio::fs::read_to_string(path).await {
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
        use regex::Regex;

        let root_div_regex =
            Regex::new(r#"<div\s+id=["']root["'](?:\s+[^>]*)?\s*(?:/>|>\s*</div>)"#)
                .map_err(|e| RariError::internal(format!("Failed to create regex: {}", e)))?;

        if !root_div_regex.is_match(template) {
            return Err(RariError::internal(
                "Template does not contain a root div with id='root'".to_string(),
            ));
        }

        let replacement = format!(r#"<div id="root">{}</div>"#, html_content);

        let result = root_div_regex.replace(template, replacement.as_str());

        Ok(result.to_string())
    }

    pub fn parse_rsc_wire_format(&self, rsc_data: &str) -> Result<Vec<RscRow>, RariError> {
        let mut rows = Vec::new();

        for line in rsc_data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let row = self.parse_rsc_line(line)?;
            rows.push(row);
        }

        Ok(rows)
    }

    fn looks_like_rsc_payload(value: &JsonValue) -> bool {
        match value {
            JsonValue::Array(arr) => arr.first().and_then(|v| v.as_str()) == Some("$"),
            JsonValue::Object(obj) => {
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
        if let Ok(value) = serde_json::from_str::<JsonValue>(s) {
            Self::looks_like_rsc_payload(&value)
        } else {
            false
        }
    }

    fn parse_rsc_line(&self, line: &str) -> Result<RscRow, RariError> {
        let colon_pos = line.find(':').ok_or_else(|| {
            RariError::internal(format!("Invalid RSC line format: missing colon in '{}'", line))
        })?;

        let (id_str, data_str) = line.split_at(colon_pos);
        let data_str = &data_str[1..];

        let id = u32::from_str_radix(id_str, 16)
            .map_err(|e| RariError::internal(format!("Invalid row ID '{}': {}", id_str, e)))?;

        if data_str.starts_with('I') {
            return Ok(RscRow { id, data: RscElement::Text(data_str.to_string()) });
        }

        let json_value: JsonValue = serde_json::from_str(data_str)
            .map_err(|e| RariError::internal(format!("Invalid JSON in RSC line: {}", e)))?;

        let data = self.parse_rsc_element(&json_value)?;

        Ok(RscRow { id, data })
    }

    fn parse_rsc_element(&self, value: &JsonValue) -> Result<RscElement, RariError> {
        match value {
            JsonValue::String(s) => {
                if s.starts_with('$') {
                    Ok(RscElement::Reference(s.clone()))
                } else {
                    Ok(RscElement::Text(s.clone()))
                }
            }

            JsonValue::Array(arr) => {
                if arr.is_empty() {
                    return Err(RariError::internal("Empty array in RSC element".to_string()));
                }

                if let Some(JsonValue::String(marker)) = arr.first()
                    && marker == "$"
                {
                    return self.parse_react_element(arr);
                }

                let mut children = Vec::new();
                for item in arr {
                    children.push(self.parse_rsc_element(item)?);
                }

                Ok(RscElement::Fragment { children })
            }

            JsonValue::Number(n) => Ok(RscElement::Text(n.to_string())),
            JsonValue::Bool(b) => Ok(RscElement::Text(b.to_string())),
            JsonValue::Null => Ok(RscElement::Text(String::new())),

            JsonValue::Object(_) => {
                Ok(RscElement::Text(serde_json::to_string(value).unwrap_or_default()))
            }
        }
    }

    fn parse_react_element(&self, arr: &[JsonValue]) -> Result<RscElement, RariError> {
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

        let key = arr[2].as_str().map(|s| s.to_string());

        let props_value = &arr[3];
        let props = if let JsonValue::Object(obj) = props_value {
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            FxHashMap::default()
        };

        Ok(RscElement::Component { tag, key, props })
    }

    pub async fn render_to_html(
        &self,
        rsc_wire_format: &str,
        config: &crate::server::config::Config,
    ) -> Result<String, RariError> {
        if !config.rsc_html.enabled {
            return Err(RariError::internal(
                "RSC-to-HTML rendering is disabled in configuration".to_string(),
            ));
        }
        let timeout_ms = config.rsc_html.timeout_ms;
        let cache_template = config.rsc_html.cache_template;
        let is_dev_mode = config.is_development();

        let render_future = async {
            let rsc_rows = self.parse_rsc_wire_format(rsc_wire_format).map_err(|e| {
                RariError::internal(format!("Failed to parse RSC wire format: {}", e))
            })?;

            let html_content = self
                .render_rsc_to_html_string(&rsc_rows)
                .await
                .map_err(|e| RariError::internal(format!("Failed to render RSC to HTML: {}", e)))?;

            let is_complete_document = html_content.trim_start().starts_with("<!DOCTYPE")
                || html_content.trim_start().cow_to_lowercase().starts_with("<html");

            if is_complete_document {
                let script_tags = if !is_dev_mode {
                    let template =
                        self.load_template(cache_template, is_dev_mode).await.map_err(|e| {
                            RariError::internal(format!("Failed to load HTML template: {}", e))
                        })?;
                    Self::extract_script_tags(&template)
                } else {
                    String::new()
                };

                let mut final_html = html_content.clone();

                if !script_tags.is_empty()
                    && let Some(body_end) = final_html.rfind("</body>")
                {
                    final_html.insert_str(body_end, &format!("\n{}\n", script_tags));
                }

                let trimmed_lower = final_html.trim_start().cow_to_lowercase();
                if !trimmed_lower.starts_with("<!doctype") {
                    final_html = format!("<!DOCTYPE html>\n{}", final_html);
                }

                return Ok::<String, RariError>(final_html);
            }

            let template = self
                .load_template(cache_template, is_dev_mode)
                .await
                .map_err(|e| RariError::internal(format!("Failed to load HTML template: {}", e)))?;

            let final_html = self.inject_into_template(&html_content, &template).map_err(|e| {
                RariError::internal(format!("Failed to inject HTML into template: {}", e))
            })?;

            Ok::<String, RariError>(final_html)
        };

        let result = if timeout_ms > 0 {
            match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), render_future)
                .await
            {
                Ok(result) => result,
                Err(_) => {
                    return Err(RariError::timeout(format!(
                        "RSC-to-HTML rendering timed out after {}ms",
                        timeout_ms
                    )));
                }
            }
        } else {
            render_future.await
        };

        match result {
            Ok(html) => Ok(html),
            Err(e) => {
                eprintln!("RSC-to-HTML rendering failed: {}, falling back to shell", e);

                let fallback_template = self.load_template(cache_template, is_dev_mode).await?;
                Ok(fallback_template)
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
                .map_err(|_| RariError::internal(format!("Invalid row reference: {}", ref_str)))
        } else {
            Err(RariError::internal(format!("Invalid reference format: {}", ref_str)))
        }
    }

    fn render_row<'a>(
        &'a self,
        row_id: u32,
        row_map: &'a FxHashMap<u32, &RscElement>,
        row_cache: &'a mut FxHashMap<u32, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, RariError>> + Send + 'a>>
    {
        Box::pin(async move {
            if let Some(cached) = row_cache.get(&row_id) {
                return Ok(cached.clone());
            }

            let element = row_map
                .get(&row_id)
                .ok_or_else(|| RariError::internal(format!("Missing row {}", row_id)))?;

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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, RariError>> + Send + 'a>>
    {
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

                RscElement::Promise { promise_id: _ } => Ok(String::new()),
            }
        })
    }

    fn render_component_to_html<'a>(
        &'a self,
        tag: &'a str,
        props: &'a serde_json::Value,
        row_map: &'a FxHashMap<u32, &RscElement>,
        row_cache: &'a mut FxHashMap<u32, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, RariError>> + Send + 'a>>
    {
        Box::pin(async move {
            if tag.contains('<') || tag.contains('>') || tag.contains('"') || tag.contains('\'') {
                return Err(RariError::internal(format!("Invalid tag name: {}", tag)));
            }

            let is_client_component = tag.starts_with("$L")
                || tag.starts_with("$@")
                || tag.contains('#')
                || tag.contains('/');
            if is_client_component {
                return Ok(format!(
                    r#"<div data-client-component="{}" style="display: contents;"></div>"#,
                    Self::escape_html_attribute(tag)
                ));
            }

            let is_suspense_symbol = tag.starts_with('$')
                && tag.len() > 1
                && tag[1..].chars().all(|c| c.is_ascii_hexdigit());
            if tag == "$Sreact.suspense" || tag == "react.suspense" || is_suspense_symbol {
                if let Some(props_obj) = props.as_object() {
                    let children = props_obj.get("children");
                    if let Some(children) = children {
                        return self.render_json_to_html(children, row_map, row_cache).await;
                    }
                }
                return Ok(String::new());
            }

            if !tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':') {
                return Err(RariError::internal(format!("Invalid tag name: {}", tag)));
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
                            html.push_str(&format!(
                                r#" style="{}""#,
                                Self::escape_html_attribute(&style_str)
                            ));
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
                            html.push_str(&format!(
                                r#" {}="{}""#,
                                attr_name,
                                if b { "true" } else { "false" }
                            ));
                        }
                    } else if let Some(s) = value.as_str() {
                        html.push_str(&format!(
                            r#" {}="{}""#,
                            attr_name,
                            Self::escape_html_attribute(s)
                        ));
                    } else if value.is_number() {
                        html.push_str(&format!(r#" {}="{}""#, attr_name, value));
                    }
                }
            }

            const SELF_CLOSING: &[&str] = SELF_CLOSING_TAGS;
            if SELF_CLOSING.contains(&tag) {
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, RariError>> + Send + 'a>>
    {
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
    shell_sent: bool,
    asset_links: Option<String>,
    boundary_id_generator: BoundaryIdGenerator,
    rari_to_react_boundary_map: parking_lot::Mutex<FxHashMap<String, String>>,
    custom_shell: Option<String>,
    body_scripts: Option<String>,
    rsc_wire_format: Vec<String>,
    payload_embedding_disabled: bool,
    root_div_closed: bool,
    content_id_counter: AtomicU32,
}

impl RscToHtmlConverter {
    pub fn new(_renderer: Arc<RscHtmlRenderer>) -> Self {
        Self {
            row_cache: FxHashMap::default(),
            shell_sent: false,
            asset_links: None,
            boundary_id_generator: BoundaryIdGenerator::new(),
            rari_to_react_boundary_map: parking_lot::Mutex::new(FxHashMap::default()),
            custom_shell: None,
            body_scripts: None,
            rsc_wire_format: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
            content_id_counter: AtomicU32::new(0),
        }
    }

    pub fn with_assets(asset_links: String, _renderer: Arc<RscHtmlRenderer>) -> Self {
        Self {
            row_cache: FxHashMap::default(),
            shell_sent: false,
            asset_links: Some(asset_links),
            boundary_id_generator: BoundaryIdGenerator::new(),
            rari_to_react_boundary_map: parking_lot::Mutex::new(FxHashMap::default()),
            custom_shell: None,
            body_scripts: None,
            rsc_wire_format: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
            content_id_counter: AtomicU32::new(0),
        }
    }

    pub fn with_custom_shell(
        custom_shell: String,
        body_scripts: Option<String>,
        _renderer: Arc<RscHtmlRenderer>,
    ) -> Self {
        Self {
            row_cache: FxHashMap::default(),
            shell_sent: false,
            asset_links: None,
            boundary_id_generator: BoundaryIdGenerator::new(),
            rari_to_react_boundary_map: parking_lot::Mutex::new(FxHashMap::default()),
            custom_shell: Some(custom_shell),
            body_scripts,
            rsc_wire_format: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
            content_id_counter: AtomicU32::new(0),
        }
    }

    pub fn disable_payload_embedding(&mut self) {
        self.rsc_wire_format.clear();
        self.payload_embedding_disabled = true;
    }

    fn next_boundary_id(&self) -> String {
        self.boundary_id_generator.next()
    }

    pub async fn convert_chunk(&mut self, chunk: RscStreamChunk) -> Result<Vec<u8>, RariError> {
        let chunk_type_str = format!("{:?}", chunk.chunk_type);

        let result: Result<Vec<u8>, RariError> = match chunk.chunk_type {
            RscChunkType::ModuleImport => {
                let rsc_line = String::from_utf8_lossy(&chunk.data);

                if !self.payload_embedding_disabled {
                    self.rsc_wire_format.push(rsc_line.trim().to_string());
                }

                Ok(Vec::new())
            }

            RscChunkType::InitialShell => {
                let html = if !self.shell_sent {
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
                } else {
                    match self.parse_and_render_rsc(&chunk.data, chunk.row_id).await {
                        Ok(rsc_html) => rsc_html,
                        Err(e) => {
                            error!("Error parsing RSC: {}", e);
                            Vec::new()
                        }
                    }
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
                    self.rsc_wire_format.push(rsc_line.trim().to_string());
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

            RscChunkType::BoundaryError => match self.generate_error_replacement(&chunk).await {
                Ok(html) => Ok(html),
                Err(e) => {
                    error!("Error generating error replacement: {}", e);
                    Ok(self.generate_fallback_error_html())
                }
            },

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

    fn generate_fallback_error_html(&self) -> Vec<u8> {
        r#"<div style="color: red; border: 1px solid red; padding: 10px; margin: 10px 0;">
            An error occurred while loading content.
        </div>"#
            .as_bytes()
            .to_vec()
    }

    fn streaming_bridge_script() -> &'static str {
        r#"<script>
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
</script>"#
    }

    fn contains_client_reference(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Array(arr) => {
                if arr.len() == 4
                    && arr[0] == "$"
                    && let Some(id) = arr[1].as_str()
                    && (id.starts_with("$L")
                        || id.starts_with("$@")
                        || id.contains('#')
                        || id.contains('/'))
                {
                    return true;
                }
                arr.iter().any(Self::contains_client_reference)
            }
            serde_json::Value::Object(obj) => obj.values().any(Self::contains_client_reference),
            _ => false,
        }
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
                    format!("{}{}", custom_shell, bridge_script)
                }
            } else {
                format!("{}{}", custom_shell, bridge_script)
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
    {}
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
{}
<div id="root">"#,
            asset_tags, bridge_script
        )
        .as_bytes()
        .to_vec()
    }

    pub fn generate_html_closing(&self) -> Vec<u8> {
        let body_scripts = self.body_scripts.as_deref().unwrap_or("");

        let mut rows_with_ids: Vec<(u32, String)> = Vec::new();
        for row in &self.rsc_wire_format {
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

        let mut rsc_payload =
            rows_with_ids.iter().map(|(_, row)| format!("{}\n", row)).collect::<Vec<_>>().join("");

        let has_row_0 = rows_with_ids.iter().any(|(id, _)| *id == 0);

        if !has_row_0
            && let Some((max_id, _)) =
                rows_with_ids.iter().filter(|(id, _)| *id != u32::MAX).max_by_key(|(id, _)| *id)
            && *max_id > 0
        {
            let row_0 = format!("0:\"${:x}\"\n", max_id);
            rsc_payload.insert_str(0, &row_0);
        }

        let escaped_payload = rsc_payload.cow_replace("</", "<\\/");

        let rsc_script = if !rsc_payload.is_empty() {
            format!(
                r#"<script id="__RARI_RSC_PAYLOAD__" type="text/x-component">{}</script>
"#,
                escaped_payload
            )
        } else {
            String::new()
        };

        format!(
            r#"{}
{}
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
</html>"#,
            rsc_script, body_scripts
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
            self.rsc_wire_format.push(rsc_line.trim().to_string());
        }

        let json_str = parts[1].trim();
        if json_str.is_empty() {
            return Ok(Vec::new());
        }

        if json_str.starts_with('I') || json_str.starts_with('S') || json_str.starts_with('E') {
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

        const PAYLOAD_SIZE_LIMIT: usize = 50000;

        if json_str.starts_with("[[") || json_str.len() > PAYLOAD_SIZE_LIMIT {
            return Ok(Vec::new());
        }

        let rsc_data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| RariError::internal(format!("Invalid RSC JSON: {}", e)))?;

        let html = self.rsc_element_to_html(&rsc_data).await?;

        self.row_cache.insert(row_id, html.clone());

        Ok(html.into_bytes())
    }

    fn rsc_element_to_html<'a>(
        &'a self,
        element: &'a serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, RariError>> + Send + 'a>>
    {
        Box::pin(async move {
            if let Some(s) = element.as_str() {
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
                        RariError::internal(format!("Invalid chunk reference: {}", s))
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

                    let is_suspense_symbol = element_type.starts_with('$')
                        && element_type.len() > 1
                        && element_type[1..].chars().all(|c| c.is_ascii_hexdigit());

                    let is_client_component = element_type.starts_with("$L")
                        || element_type.contains('#')
                        || element_type.contains('/');

                    if element_type == "$Sreact.suspense"
                        || element_type == "react.suspense"
                        || is_suspense_symbol
                    {
                        return self.render_suspense_boundary(element_type, props).await;
                    }

                    if is_client_component {
                        return self.render_client_component_placeholder(element_type, props).await;
                    }

                    return self.render_html_element(element_type, props).await;
                } else {
                    let mut html = String::new();
                    for child in arr {
                        let child_html = self.rsc_element_to_html(child).await?;
                        html.push_str(&child_html);
                    }
                    return Ok(html);
                }
            }

            if let Some(obj) = element.as_object() {
                if let (Some(element_type), Some(props)) = (obj.get("type"), obj.get("props"))
                    && let Some(type_str) = element_type.as_str()
                {
                    let props_obj = props.as_object();

                    if type_str == "$Sreact.suspense" || type_str == "react.suspense" {
                        return self.render_suspense_boundary(type_str, props_obj).await;
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
        _component_ref: &str,
        props: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Result<String, RariError> {
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
        if !tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':') {
            return Err(RariError::internal(format!("Invalid tag name: {}", tag)));
        }

        let mut html = format!("<{}", tag);

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
                        html.push_str(&format!(
                            " style=\"{}\"",
                            RscHtmlRenderer::escape_html_attribute(&style_str)
                        ));
                    }
                    continue;
                }

                if let Some(b) = value.as_bool() {
                    if is_boolean_html_attribute(attr_name) {
                        if b {
                            html.push_str(&format!(" {}", attr_name));
                        }
                    } else {
                        html.push_str(&format!(
                            " {}=\"{}\"",
                            attr_name,
                            if b { "true" } else { "false" }
                        ));
                    }
                    continue;
                }

                if let Some(s) = value.as_str() {
                    html.push_str(&format!(
                        " {}=\"{}\"",
                        attr_name,
                        RscHtmlRenderer::escape_html_attribute(s)
                    ));
                    continue;
                }

                if value.is_number() {
                    html.push_str(&format!(
                        " {}=\"{}\"",
                        attr_name,
                        RscHtmlRenderer::escape_html_attribute(&value.to_string())
                    ));
                }
            }
        }

        const SELF_CLOSING: &[&str] = SELF_CLOSING_TAGS;
        if SELF_CLOSING.contains(&tag) {
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

        html.push_str(&format!("</{}>", tag));

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
                    r#"<script>(function(){{if(!window['~rari'])window['~rari']={{}};if(!window['~rari'].streaming)window['~rari'].streaming={{}};if(!window['~rari'].streaming.bufferedRows)window['~rari'].streaming.bufferedRows=[];window['~rari'].streaming.bufferedRows.push('{}');window.dispatchEvent(new CustomEvent('rari:html-stream-row',{{detail:{{rscRow:'{}'}}}}));}})();</script>"#,
                    escaped_row, escaped_row
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
                    Ok(content) => {
                        if Self::contains_client_reference(&content) {
                            String::new()
                        } else {
                            match self.rsc_element_to_html(&content).await {
                                Ok(html) if !html.is_empty() => html,
                                _ => String::new(),
                            }
                        }
                    }
                    Err(_) => String::new(),
                }
            };

            if !rendered_html.is_empty() {
                format!(
                    "<div hidden id=\"{content_id}\">{rendered_html}</div>\n<script>$RC=window.$RC||function(b,c){{var t=document.getElementById(b);var s=document.getElementById(c);if(t&&s){{var p=t.parentNode;var f=document.createDocumentFragment();Array.from(s.childNodes).forEach(function(n){{f.appendChild(n)}});var d=t.nextSibling;while(d&&!(d.nodeType===8&&d.data==='/$')){{var next=d.nextSibling;d.remove();d=next;}}if(d)d.remove();p.insertBefore(f,t.nextSibling);t.remove();s.remove();}}}};$RC(\"{react_id}\",\"{content_id}\")</script>",
                )
            } else {
                String::new()
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

        let output = format!("{}{}", rsc_buffer_script, dom_swap_html);

        Ok(output.into_bytes())
    }

    async fn generate_error_replacement(
        &self,
        chunk: &RscStreamChunk,
    ) -> Result<Vec<u8>, RariError> {
        let rsc_line = String::from_utf8_lossy(&chunk.data);
        let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();

        if parts.len() != 2 {
            return Ok(self.generate_fallback_error_html());
        }

        let json_part = parts[1].strip_prefix('E').unwrap_or(parts[1]);

        let error_data = match serde_json::from_str::<serde_json::Value>(json_part) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to parse error data from stream, using fallback: {}", e);
                return Ok(self.generate_fallback_error_html());
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

            let content_id = format!("S:{}", react_boundary_id.trim_start_matches("B:"));

            let error_update = format!(
                r#"<div hidden id="{}">{}</div><script>$RC=window.$RC||function(b,c){{const t=document.getElementById(b);const s=document.getElementById(c);if(t&&s){{const p=t.parentNode;Array.from(s.childNodes).forEach(n=>p.insertBefore(n,t));t.remove();s.remove();}}}};$RC("{}","{}");</script>"#,
                content_id, error_html, react_boundary_id, content_id
            );

            Ok(error_update.into_bytes())
        } else {
            Ok(self.generate_fallback_error_html())
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
