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

fn serialize_style_object(style_obj: &serde_json::Map<String, serde_json::Value>) -> String {
    let style_parts: Vec<String> = style_obj
        .iter()
        .map(|(k, v)| {
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
                s.to_string()
            } else if let Some(b) = v.as_bool() {
                if b { "true".to_string() } else { "false".to_string() }
            } else if let Some(i) = v.as_i64() {
                i.to_string()
            } else if let Some(u) = v.as_u64() {
                u.to_string()
            } else if let Some(f) = v.as_f64() {
                if f.is_finite() {
                    format!("{:.10}", f).trim_end_matches('0').trim_end_matches('.').to_string()
                } else {
                    f.to_string()
                }
            } else if v.is_object() || v.is_array() {
                serde_json::to_string(v).unwrap_or_else(|_| String::from("{}"))
            } else {
                v.to_string()
            };
            format!("{}:{}", kebab_key, value_str)
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

        let id = id_str
            .parse::<u32>()
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

                Ok(RscElement::Text(serde_json::to_string(value).unwrap_or_default()))
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

        let mut row_cache: FxHashMap<u32, String> = FxHashMap::default();

        for row in rsc_rows {
            let html = self.render_rsc_element(&row.data, &row_cache).await?;
            row_cache.insert(row.id, html);
        }

        let root_row_id = if row_cache.contains_key(&0) {
            0
        } else {
            rsc_rows.iter().map(|r| r.id).max().unwrap_or(0)
        };

        Ok(row_cache.get(&root_row_id).cloned().unwrap_or_default())
    }

    async fn render_rsc_element(
        &self,
        element: &RscElement,
        row_cache: &FxHashMap<u32, String>,
    ) -> Result<String, RariError> {
        match element {
            RscElement::Text(text) => {
                if let Some(stripped) = text.strip_prefix("$L").or_else(|| text.strip_prefix("$@"))
                    && let Ok(row_id) = stripped.parse::<u32>()
                {
                    return Ok(row_cache.get(&row_id).cloned().unwrap_or_default());
                }

                if text.starts_with("I[") || text.starts_with("S[") || text.starts_with("E[") {
                    return Ok(String::new());
                }

                Ok(text
                    .cow_replace('&', "&amp;")
                    .cow_replace('<', "&lt;")
                    .cow_replace('>', "&gt;")
                    .cow_replace('"', "&quot;")
                    .cow_replace('\'', "&#39;")
                    .into_owned())
            }

            RscElement::Component { tag, key: _, props } => {
                let props_value = serde_json::Value::Object(
                    props.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                );
                self.render_component_to_html(tag, &props_value, row_cache).await
            }

            RscElement::Reference(ref_str) => {
                if let Some(stripped) =
                    ref_str.strip_prefix("$L").or_else(|| ref_str.strip_prefix("$@"))
                    && let Ok(row_id) = stripped.parse::<u32>()
                {
                    return Ok(row_cache.get(&row_id).cloned().unwrap_or_default());
                }
                Ok(String::new())
            }

            RscElement::Suspense { fallback_ref, children_ref, boundary_id, props: _ } => {
                if let Some(stripped) = children_ref.strip_prefix("$L")
                    && let Ok(row_id) = stripped.parse::<u32>()
                    && let Some(html) = row_cache.get(&row_id)
                    && !html.is_empty()
                {
                    return Ok(html.clone());
                }

                if let Some(stripped) = fallback_ref.strip_prefix("$L")
                    && let Ok(row_id) = stripped.parse::<u32>()
                    && let Some(html) = row_cache.get(&row_id)
                {
                    return Ok(format!(
                        r#"<div data-boundary-id="{}" class="rari-suspense-boundary">{}</div>"#,
                        boundary_id.cow_replace('"', "&quot;").into_owned(),
                        html
                    ));
                }

                Ok(String::new())
            }

            RscElement::Promise { promise_id: _ } => Ok(String::new()),
        }
    }

    async fn render_component_to_html(
        &self,
        tag: &str,
        props: &serde_json::Value,
        row_cache: &FxHashMap<u32, String>,
    ) -> Result<String, RariError> {
        if tag.starts_with("$L") || tag.starts_with("$@") {
            return Ok(format!(
                r#"<div data-client-component="{}" style="display: contents;"></div>"#,
                tag.cow_replace('"', "&quot;").into_owned()
            ));
        }

        if tag == "react.suspense" || tag == "$0" {
            if let Some(props_obj) = props.as_object() {
                let children = props_obj.get("children");
                if let Some(children) = children {
                    return self.render_json_to_html(children, row_cache).await;
                }
            }
            return Ok(String::new());
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

                if key == "style" && value.is_object() {
                    if let Some(style_obj) = value.as_object() {
                        let style_str = serialize_style_object(style_obj);
                        html.push_str(&format!(
                            r#" style="{}""#,
                            style_str
                                .cow_replace('&', "&amp;")
                                .cow_replace('"', "&quot;")
                                .cow_replace('<', "&lt;")
                                .cow_replace('>', "&gt;")
                                .into_owned()
                        ));
                    }
                    continue;
                }

                if let Some(b) = value.as_bool() {
                    if b {
                        html.push(' ');
                        html.push_str(attr_name);
                    }
                } else if let Some(s) = value.as_str() {
                    html.push_str(&format!(
                        r#" {}="{}""#,
                        attr_name,
                        s.cow_replace('&', "&amp;")
                            .cow_replace('"', "&quot;")
                            .cow_replace('<', "&lt;")
                            .cow_replace('>', "&gt;")
                            .into_owned()
                    ));
                } else if value.is_number() {
                    html.push_str(&format!(r#" {}="{}""#, attr_name, value));
                }
            }
        }

        const SELF_CLOSING: &[&str] = &[
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
            "source", "track", "wbr",
        ];
        if SELF_CLOSING.contains(&tag) {
            html.push_str(" />");
            return Ok(html);
        }

        html.push('>');

        if let Some(props_obj) = props.as_object()
            && let Some(children) = props_obj.get("children")
        {
            let children_html = self.render_json_to_html(children, row_cache).await?;
            html.push_str(&children_html);
        }

        html.push_str("</");
        html.push_str(tag);
        html.push('>');

        Ok(html)
    }

    fn render_json_to_html<'a>(
        &'a self,
        json: &'a serde_json::Value,
        row_cache: &'a FxHashMap<u32, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, RariError>> + Send + 'a>>
    {
        Box::pin(async move {
            if json.is_null() {
                return Ok(String::new());
            }

            if let Some(s) = json.as_str() {
                if let Some(stripped) = s.strip_prefix("$L").or_else(|| s.strip_prefix("$@"))
                    && let Ok(row_id) = stripped.parse::<u32>()
                {
                    return Ok(row_cache.get(&row_id).cloned().unwrap_or_default());
                }
                return Ok(s
                    .cow_replace('&', "&amp;")
                    .cow_replace('<', "&lt;")
                    .cow_replace('>', "&gt;")
                    .cow_replace('"', "&quot;")
                    .cow_replace('\'', "&#39;")
                    .into_owned());
            }

            if let Some(arr) = json.as_array() {
                if arr.len() >= 4
                    && arr[0].as_str() == Some("$")
                    && let (Some(tag), Some(props)) = (arr[1].as_str(), arr.get(3))
                {
                    return self.render_component_to_html(tag, props, row_cache).await;
                }

                let mut html = String::with_capacity(arr.len() * 64);
                for child in arr {
                    html.push_str(&self.render_json_to_html(child, row_cache).await?);
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
            if let Some(b) = json.as_bool() {
                return Ok(b.to_string());
            }

            Ok(String::new())
        })
    }

    pub async fn generate_boundary_update_html(
        &self,
        boundary_id: &str,
        content_rsc: &JsonValue,
        row_id: u32,
    ) -> Result<String, RariError> {
        let rsc_row = format!("{}:{}", row_id, serde_json::to_string(content_rsc)?);

        let update_script = format!(
            r#"<script data-boundary-id="{}" data-row-id="{}">
(function() {{
    if (!window['~rari']) window['~rari'] = {{}};
    if (!window['~rari'].bufferedRows) window['~rari'].bufferedRows = [];

    window['~rari'].bufferedRows.push('{}');

    if (window['~rari'].processBoundaryUpdate) {{
        window['~rari'].processBoundaryUpdate('{}', '{}', {});
    }}

    window.dispatchEvent(new CustomEvent('rari:boundary-update', {{
        detail: {{
            boundaryId: '{}',
            rscRow: '{}',
            rowId: {}
        }}
    }}));
}})();
</script>

"#,
            Self::escape_html_attribute(boundary_id),
            row_id,
            Self::escape_js_string(&rsc_row),
            Self::escape_js_string(boundary_id),
            Self::escape_js_string(&rsc_row),
            row_id,
            Self::escape_js_string(boundary_id),
            Self::escape_js_string(&rsc_row),
            row_id
        );

        Ok(update_script)
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
            .into_owned()
    }

    pub fn generate_boundary_error_html(
        boundary_id: &str,
        error_message: &str,
        row_id: u32,
    ) -> String {
        let error_script = format!(
            r#"<script data-boundary-id="{}" data-row-id="{}">
window['~rari'] && window['~rari'].processBoundaryError('{}', '{}', {});
</script>"#,
            Self::escape_html_attribute(boundary_id),
            row_id,
            Self::escape_js_string(boundary_id),
            Self::escape_js_string(error_message),
            row_id
        );

        error_script
    }
}

pub struct RscToHtmlConverter {
    row_cache: FxHashMap<u32, String>,
    shell_sent: bool,
    asset_links: Option<String>,
    boundary_id_generator: BoundaryIdGenerator,
    rari_to_react_boundary_map: parking_lot::Mutex<FxHashMap<String, String>>,
    custom_shell: Option<String>,
    csrf_script: Option<String>,
    body_scripts: Option<String>,
    rsc_wire_format: Vec<String>,
    payload_embedding_disabled: bool,
    root_div_closed: bool,
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
            csrf_script: None,
            body_scripts: None,
            rsc_wire_format: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
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
            csrf_script: None,
            body_scripts: None,
            rsc_wire_format: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
        }
    }

    pub fn with_custom_shell(
        custom_shell: String,
        csrf_script: Option<String>,
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
            csrf_script,
            body_scripts,
            rsc_wire_format: Vec::new(),
            payload_embedding_disabled: false,
            root_div_closed: false,
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

                if !self.shell_sent {
                    if !self.payload_embedding_disabled {
                        self.rsc_wire_format.push(rsc_line.trim().to_string());
                    }
                    return Ok(Vec::new());
                }

                let escaped_row = rsc_line
                    .trim()
                    .cow_replace('\\', "\\\\")
                    .cow_replace('\'', "\\'")
                    .cow_replace('\n', "\\n")
                    .into_owned();
                let script = format!(
                    r#"<script>(function(){{if(!window['~rari'])window['~rari']={{}};if(!window['~rari'].bufferedRows)window['~rari'].bufferedRows=[];window['~rari'].bufferedRows.push('{}');window.dispatchEvent(new CustomEvent('rari:rsc-row',{{detail:{{rscRow:'{}'}}}}));}})();</script>"#,
                    escaped_row, escaped_row
                );
                Ok(script.into_bytes())
            }

            RscChunkType::InitialShell => {
                let html = if !self.shell_sent {
                    self.shell_sent = true;
                    let output = self.generate_html_shell();

                    let rsc_line = String::from_utf8_lossy(&chunk.data);

                    if !self.payload_embedding_disabled {
                        self.rsc_wire_format.push(rsc_line.trim().to_string());
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

    fn generate_html_shell(&self) -> Vec<u8> {
        if let Some(custom_shell) = &self.custom_shell {
            return custom_shell.as_bytes().to_vec();
        }

        let asset_tags = self.asset_links.as_deref().unwrap_or("");

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
<div id="root">"#,
            asset_tags
        )
        .as_bytes()
        .to_vec()
    }

    pub fn generate_html_closing(&self) -> Vec<u8> {
        let csrf_script = self.csrf_script.as_deref().unwrap_or("");
        let body_scripts = self.body_scripts.as_deref().unwrap_or("");

        let rsc_payload = self.rsc_wire_format.join("\n");

        let escaped_payload = rsc_payload.cow_replace("</script>", "<\\/script>");

        let rsc_script = if !rsc_payload.is_empty() {
            format!(
                r#"<script id="__RARI_RSC_PAYLOAD__" type="application/json">{}</script>
"#,
                escaped_payload
            )
        } else {
            String::new()
        };

        format!(
            r#"{}{}
{}
<script>
if (typeof window !== 'undefined') {{
    if (!window['~rari']) window['~rari'] = {{}};
    window['~rari'].streamComplete = true;
    window.dispatchEvent(new Event('rari:stream-complete'));
}}
</script>
</body>
</html>"#,
            rsc_script, csrf_script, body_scripts
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

        const PAYLOAD_SIZE_LIMIT: usize = 5000;

        if json_str.starts_with("[[") || json_str.len() > PAYLOAD_SIZE_LIMIT {
            return Ok(Vec::new());
        }

        let rsc_data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| RariError::internal(format!("Invalid RSC JSON: {}", e)))?;

        let html = self.rsc_element_to_html(&rsc_data).await?;

        self.row_cache.insert(row_id, html.clone());

        Ok(html.into_bytes())
    }

    fn escape_html(text: &str) -> String {
        text.cow_replace('&', "&amp;")
            .cow_replace('<', "&lt;")
            .cow_replace('>', "&gt;")
            .cow_replace('"', "&quot;")
            .cow_replace('\'', "&#39;")
            .into_owned()
    }

    fn escape_attribute(text: &str) -> String {
        text.cow_replace('&', "&amp;")
            .cow_replace('"', "&quot;")
            .cow_replace('<', "&lt;")
            .cow_replace('>', "&gt;")
            .into_owned()
    }

    fn rsc_element_to_html<'a>(
        &'a self,
        element: &'a serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, RariError>> + Send + 'a>>
    {
        Box::pin(async move {
            if let Some(s) = element.as_str() {
                if let Some(stripped) = s.strip_prefix("$L") {
                    let row_id: u32 = stripped.parse().map_err(|_| {
                        RariError::internal(format!("Invalid row reference: {}", s))
                    })?;
                    let cached = self.row_cache.get(&row_id).cloned().unwrap_or_default();
                    return Ok(cached);
                }

                if s.starts_with('$') && s.len() > 1 && s[1..].chars().all(|c| c.is_ascii_digit()) {
                    let _row_id: u32 = s[1..].parse().map_err(|_| {
                        RariError::internal(format!("Invalid chunk reference: {}", s))
                    })?;
                    return Ok(s.to_string());
                }

                if s.starts_with("I[") || s.starts_with("S[") || s.starts_with("E[") {
                    return Ok(String::new());
                }

                if let Some(colon_pos) = s.find(':')
                    && s[..colon_pos].chars().all(|c| c.is_ascii_digit())
                {
                    let after_colon = &s[colon_pos + 1..];
                    if !after_colon.is_empty()
                        && (after_colon.starts_with('[') || after_colon.starts_with('{'))
                        && serde_json::from_str::<serde_json::Value>(after_colon).is_ok()
                    {
                        return Ok(String::new());
                    }
                }

                return Ok(Self::escape_html(s));
            }

            if let Some(arr) = element.as_array() {
                if arr.len() >= 4 && arr[0].as_str() == Some("$") {
                    let element_type = arr[1].as_str().unwrap_or("div");
                    let props = arr[3].as_object();

                    let is_suspense_symbol = element_type.starts_with('$')
                        && element_type.len() > 1
                        && element_type[1..].chars().all(|c| c.is_ascii_digit());

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
        _props: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Result<String, RariError> {
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
                react_boundary_id, rari_boundary_id, content_html
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

                if key == "style" && value.is_object() {
                    if let Some(style_obj) = value.as_object() {
                        let style_str = serialize_style_object(style_obj);
                        html.push_str(&format!(
                            " style=\"{}\"",
                            Self::escape_attribute(&style_str)
                        ));
                    }
                    continue;
                }

                if let Some(b) = value.as_bool() {
                    if b {
                        html.push_str(&format!(" {}", attr_name));
                    }
                    continue;
                }

                if let Some(s) = value.as_str() {
                    html.push_str(&format!(" {}=\"{}\"", attr_name, Self::escape_attribute(s)));
                    continue;
                }

                if value.is_number() || value.is_boolean() {
                    html.push_str(&format!(
                        " {}=\"{}\"",
                        attr_name,
                        Self::escape_attribute(&value.to_string())
                    ));
                }
            }
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
                error!("Boundary update chunk missing boundary_id");
                let escaped_row = rsc_line
                    .trim()
                    .cow_replace('\\', "\\\\")
                    .cow_replace('\'', "\\'")
                    .cow_replace('\n', "\\n")
                    .into_owned();
                let script = format!(
                    r#"<script>(function(){{if(!window['~rari'])window['~rari']={{}};if(!window['~rari'].bufferedRows)window['~rari'].bufferedRows=[];window['~rari'].bufferedRows.push('{}');window.dispatchEvent(new CustomEvent('rari:rsc-row',{{detail:{{rscRow:'{}'}}}}));}})();</script>"#,
                    escaped_row, escaped_row
                );
                return Ok(script.into_bytes());
            }
        };

        let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();

        if parts.len() != 2 {
            error!("Invalid boundary update format: {}", rsc_line);
            return Ok(Vec::new());
        }

        let row_id = chunk.row_id;
        let _content_json = parts[1];

        let escaped_row = rsc_line
            .trim()
            .cow_replace('\\', "\\\\")
            .cow_replace('\'', "\\'")
            .cow_replace('\n', "\\n")
            .into_owned();

        let script = format!(
            r#"<script data-boundary-id="{}" data-row-id="{}">
(function(){{
  if(!window['~rari'])window['~rari']={{}};
  if(!window['~rari'].bufferedRows)window['~rari'].bufferedRows=[];
  if(!window['~rari'].bufferedEvents)window['~rari'].bufferedEvents=[];

  window['~rari'].bufferedRows.push('{}');

  if(window['~rari'].processBoundaryUpdate){{
    window['~rari'].processBoundaryUpdate('{}', '{}', {});
  }} else {{
    window['~rari'].bufferedEvents.push({{
      boundaryId: '{}',
      rscRow: '{}',
      rowId: {}
    }});
  }}
}})();
</script>"#,
            Self::escape_attribute(&boundary_id),
            row_id,
            escaped_row,
            RscHtmlRenderer::escape_js_string(&boundary_id),
            escaped_row,
            row_id,
            RscHtmlRenderer::escape_js_string(&boundary_id),
            escaped_row,
            row_id
        );

        Ok(script.into_bytes())
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
                Self::escape_html(&error_message)
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
