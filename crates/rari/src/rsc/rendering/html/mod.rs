use crate::error::{RariError, StreamingError};
use crate::rsc::rendering::streaming::{RscChunkType, RscStreamChunk};
use crate::rsc::types::RscElement;
use crate::runtime::JsExecutionRuntime;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::error;

pub mod tests;

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
    const RENDER_SCRIPT: &'static str = include_str!("js/render_script.js");

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

    pub async fn initialize(&self) -> Result<(), RariError> {
        self.runtime
            .execute_script("rsc_html_renderer_init".to_string(), Self::RENDER_SCRIPT.to_string())
            .await
            .map_err(|e| {
                RariError::internal(format!("Failed to initialize RSC-to-HTML renderer: {}", e))
            })?;

        Ok(())
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
    <title>Rari App</title>
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
            return Ok(RscRow { id, data: RscElement::Text(String::new()) });
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
                || html_content.trim_start().to_lowercase().starts_with("<html");

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

                if let Some(body_start) = final_html.find("<body")
                    && let Some(body_content_start) = final_html[body_start..].find('>')
                {
                    let body_content_start = body_start + body_content_start + 1;

                    if let Some(body_end) = final_html.rfind("</body>") {
                        let body_content = &final_html[body_content_start..body_end];

                        let new_body_content =
                            format!(r#"<div id="root">{}</div>{}"#, body_content, script_tags);

                        final_html.replace_range(body_content_start..body_end, &new_body_content);
                    }
                }

                if !final_html.trim_start().starts_with("<!DOCTYPE") {
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
        #[allow(clippy::disallowed_methods)]
        let rows_json: Vec<serde_json::Value> = rsc_rows
            .iter()
            .map(|row| {
                let data_json = match &row.data {
                    RscElement::Component { tag, key, props } => {
                        serde_json::json!({
                            "Component": {
                                "tag": tag,
                                "key": key,
                                "props": props
                            }
                        })
                    }
                    RscElement::Text(text) => {
                        serde_json::json!({
                            "Text": text
                        })
                    }
                    RscElement::Reference(ref_str) => {
                        serde_json::json!({
                            "Reference": ref_str
                        })
                    }
                    RscElement::Suspense { fallback_ref, children_ref, boundary_id, props } => {
                        serde_json::json!({
                            "Suspense": {
                                "fallback_ref": fallback_ref,
                                "children_ref": children_ref,
                                "boundary_id": boundary_id,
                                "props": props
                            }
                        })
                    }
                    RscElement::Promise { promise_id } => {
                        serde_json::json!({
                            "Promise": {
                                "promise_id": promise_id
                            }
                        })
                    }
                };

                serde_json::json!({
                    "id": row.id,
                    "data": data_json
                })
            })
            .collect();

        let rows_array = serde_json::Value::Array(rows_json);
        let result =
            self.runtime.execute_function("renderRscToHtml", vec![rows_array]).await.map_err(
                |e| RariError::internal(format!("Failed to execute renderRscToHtml: {}", e)),
            )?;

        let html = result
            .as_str()
            .ok_or_else(|| {
                RariError::internal(format!(
                    "renderRscToHtml did not return a string: {:?}",
                    result
                ))
            })?
            .to_string();

        Ok(html)
    }

    pub async fn generate_boundary_update_html(
        &self,
        boundary_id: &str,
        content_rsc: &JsonValue,
        row_id: u32,
    ) -> Result<String, RariError> {
        let content_html = self.render_rsc_value_to_html(content_rsc).await?;

        let escaped_content =
            content_html.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");

        let update_script = format!(
            r#"<script data-~boundary-id="{}" data-row-id="{}">
window['~rari'] && window['~rari'].processBoundaryUpdate('{}', `{}`, {});
</script>"#,
            Self::escape_html_attribute(boundary_id),
            row_id,
            Self::escape_js_string(boundary_id),
            escaped_content,
            row_id
        );

        Ok(update_script)
    }

    async fn render_rsc_value_to_html(&self, rsc_value: &JsonValue) -> Result<String, RariError> {
        let temp_row = RscRow { id: 0, data: self.parse_rsc_element(rsc_value)? };

        self.render_rsc_to_html_string(&[temp_row]).await
    }

    fn escape_html_attribute(text: &str) -> String {
        text.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
    }

    fn escape_js_string(text: &str) -> String {
        text.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    pub fn generate_boundary_error_html(
        boundary_id: &str,
        error_message: &str,
        row_id: u32,
    ) -> String {
        let error_script = format!(
            r#"<script data-~boundary-id="{}" data-row-id="{}">
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
    rsc_wire_format: Vec<String>,
    manifest_json: Option<String>,
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
            rsc_wire_format: Vec::new(),
            manifest_json: None,
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
            rsc_wire_format: Vec::new(),
            manifest_json: None,
        }
    }

    pub fn with_custom_shell(
        custom_shell: String,
        csrf_script: Option<String>,
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
            rsc_wire_format: Vec::new(),
            manifest_json: None,
        }
    }

    pub fn disable_payload_embedding(&mut self) {
        self.rsc_wire_format.clear();
    }

    pub fn set_manifest(&mut self, manifest_json: String) {
        self.manifest_json = Some(manifest_json);
    }

    fn next_boundary_id(&self) -> String {
        self.boundary_id_generator.next()
    }

    pub async fn convert_chunk(&mut self, chunk: RscStreamChunk) -> Result<Vec<u8>, RariError> {
        let chunk_type_str = format!("{:?}", chunk.chunk_type);

        let result: Result<Vec<u8>, RariError> = match chunk.chunk_type {
            RscChunkType::ModuleImport => {
                let rsc_line = String::from_utf8_lossy(&chunk.data);
                let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();
                if parts.len() == 2 {
                    self.row_cache.insert(chunk.row_id, String::new());
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
                            output
                        }
                        Err(e) => {
                            error!("Error parsing RSC in shell: {}", e);
                            output
                        }
                    }
                } else {
                    match self.parse_and_render_rsc(&chunk.data, chunk.row_id).await {
                        Ok(html) => html,
                        Err(e) => {
                            error!("Error parsing RSC chunk: {}", e);
                            Vec::new()
                        }
                    }
                };
                Ok(html)
            }

            RscChunkType::BoundaryUpdate => {
                match self.generate_boundary_replacement(&chunk).await {
                    Ok(html) => Ok(html),
                    Err(e) => {
                        error!("Error generating boundary replacement: {}", e);
                        Ok(Vec::new())
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

            RscChunkType::StreamComplete => Ok(self.generate_html_closing()),
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
    <title>Rari App</title>
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

    fn generate_html_closing(&self) -> Vec<u8> {
        let csrf_script = self.csrf_script.as_deref().unwrap_or("");

        let rsc_payload = self.rsc_wire_format.join("\n");

        let escaped_payload = rsc_payload.replace("</script>", "<\\/script>");

        let rsc_script = if !rsc_payload.is_empty() {
            format!(
                r#"<script id="__RARI_RSC_PAYLOAD__" type="application/json">{}</script>
"#,
                escaped_payload
            )
        } else {
            String::new()
        };

        let manifest_script = if let Some(ref manifest) = self.manifest_json {
            let escaped_manifest = manifest.replace("</script>", "<\\/script>");
            format!(
                r#"<script id="__RARI_MANIFEST__" type="application/json">{}</script>
"#,
                escaped_manifest
            )
        } else {
            String::new()
        };

        format!(
            r#"</div>
{}{}{}
<script>
if (typeof window !== 'undefined') {{
    if (!window['~rari']) window['~rari'] = {{}};
    window['~rari'].streamComplete = true;
    window.dispatchEvent(new Event('rari:stream-complete'));
}}
</script>
</body>
</html>"#,
            csrf_script, rsc_script, manifest_script
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

        let json_str = parts[1].trim();
        if json_str.is_empty() {
            return Ok(Vec::new());
        }

        if json_str.starts_with('I') || json_str.starts_with('S') {
            return Ok(Vec::new());
        }

        let rsc_data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| RariError::internal(format!("Invalid RSC JSON: {}", e)))?;

        let html = self.rsc_element_to_html(&rsc_data).await?;

        self.row_cache.insert(row_id, html.clone());

        Ok(html.into_bytes())
    }

    fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    fn escape_attribute(text: &str) -> String {
        text.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
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
                    return Ok(self.row_cache.get(&row_id).cloned().unwrap_or_default());
                }
                return Ok(Self::escape_html(s));
            }

            if let Some(arr) = element.as_array() {
                if arr.len() >= 4 && arr[0].as_str() == Some("$") {
                    let element_type = arr[1].as_str().unwrap_or("div");
                    let props = arr[3].as_object();

                    if element_type == "react.suspense" || element_type.starts_with("$L") {
                        return self.render_suspense_boundary(element_type, props).await;
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

                    if type_str == "react.suspense" {
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
                let rendered = self.rsc_element_to_html(child).await?;
                if !rendered.is_empty() { Some(rendered) } else { None }
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

            Ok(format!(
                "<!--$?--><template id=\"{}\"></template>{}<!--/$-->",
                react_boundary_id, content_html
            ))
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
                if key == "children" {
                    continue;
                }

                let attr_name = match key.as_str() {
                    "className" => "class",
                    "htmlFor" => "for",
                    _ => key.as_str(),
                };

                if let Some(s) = value.as_str() {
                    html.push_str(&format!(" {}=\"{}\"", attr_name, Self::escape_attribute(s)));
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
        let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();

        if parts.len() != 2 {
            return Ok(Vec::new());
        }

        let rsc_data: serde_json::Value = serde_json::from_str(parts[1])
            .map_err(|e| RariError::internal(format!("Invalid boundary update JSON: {}", e)))?;

        if let Some(obj) = rsc_data.as_object()
            && let (Some(boundary_id_value), Some(content)) =
                (obj.get("boundary_id"), obj.get("content"))
            && let Some(rari_boundary_id) = boundary_id_value.as_str()
        {
            let react_boundary_id =
                self.rari_to_react_boundary_map.lock().get(rari_boundary_id).cloned();

            if let Some(react_boundary_id) = react_boundary_id {
                let content_html = self.rsc_element_to_html(content).await?;

                let content_id = format!("S:{}", react_boundary_id.trim_start_matches("B:"));

                let update_html = format!(
                    r#"<div hidden id="{}">{}</div><script>$RC=window.$RC||function(b,c){{const t=document.getElementById(b);const s=document.getElementById(c);if(t&&s){{const p=t.parentNode;Array.from(s.childNodes).forEach(n=>p.insertBefore(n,t));t.remove();s.remove();}}}};$RC("{}","{}");</script>"#,
                    content_id, content_html, react_boundary_id, content_id
                );

                return Ok(update_html.into_bytes());
            }
        }

        Ok(Vec::new())
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
