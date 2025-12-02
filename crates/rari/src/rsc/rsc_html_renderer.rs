use crate::error::{RariError, StreamingError};
use crate::rsc::rsc_types::RscElement;
use crate::rsc::streaming::{RscChunkType, RscStreamChunk};
use crate::runtime::JsExecutionRuntime;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::{debug, error, warn};

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
    const RENDER_SCRIPT: &'static str = r#"
        function renderElement(element, rendered, rowId) {
            if (typeof element === 'string') {
                return escapeHtml(element);
            }

            if (typeof element === 'number' || typeof element === 'boolean') {
                return escapeHtml(String(element));
            }

            if (element === null || element === undefined) {
                return '';
            }

            if (Array.isArray(element) && element.length >= 4 && element[0] === '$') {
                const tag = element[1];
                const props = element[3] || {};
                return renderTag(tag, props, rendered, rowId);
            }

            if (element.Component) {
                const { tag, props } = element.Component;
                return renderTag(tag, props || {}, rendered, rowId);
            }

            if (element.Text !== undefined) {
                return escapeHtml(String(element.Text));
            }

            if (element.Reference) {
                const ref = element.Reference;
                const match = ref.match(/\$[@L]?(\d+)/);
                if (match) {
                    const refId = parseInt(match[1], 10);
                    return rendered.get(refId) || '';
                }
                return '';
            }

            return '';
        }

        function renderTag(tag, props, rendered, rowId) {
            if (tag === 'react.suspense') {
                return renderSuspense(props, rendered);
            }

            if (typeof tag === 'string' && tag.startsWith('$')) {
                const match = tag.match(/\$[@L]?(\d+)/);
                if (match) {
                    const refId = parseInt(match[1], 10);
                    const referencedContent = rendered.get(refId);

                    if (referencedContent !== undefined && referencedContent !== '') {
                        return referencedContent;
                    }

                    return renderClientComponentPlaceholder(tag, props, rendered, rowId);
                }

                if (tag.startsWith('$@')) {
                    return renderClientComponentPlaceholder(tag, props, rendered, rowId);
                }

                return '';
            }

            const attributes = renderAttributes(props, rowId);

            const rawContentTags = ['style', 'script'];
            let children;
            if (rawContentTags.includes(tag)) {
                children = renderChildrenRaw(props.children, rendered);
            } else {
                children = renderChildren(props.children, rendered);
            }

            const selfClosingTags = ['img', 'br', 'hr', 'input', 'meta', 'link', 'area', 'base', 'col', 'embed', 'source', 'track', 'wbr'];
            if (selfClosingTags.includes(tag)) {
                return `<${tag}${attributes} />`;
            }

            return `<${tag}${attributes}>${children}</${tag}>`;
        }

        function renderSuspense(props, rendered) {
            const children = props.children;
            const boundaryId = props.__boundary_id || props.boundaryId || props.key || 'suspense-boundary';

            if (typeof children === 'string' && children.startsWith('$')) {
                const match = children.match(/\$[@L]?(\d+)/);
                if (match) {
                    const refId = parseInt(match[1], 10);
                    const resolvedContent = rendered.get(refId);

                    if (resolvedContent !== undefined && resolvedContent !== '') {
                        return resolvedContent;
                    }
                }
            }

            if (children !== undefined && children !== null && typeof children !== 'string') {
                const childrenHtml = renderElement(children, rendered, undefined);
                if (childrenHtml) {
                    return childrenHtml;
                }
            }

            return '';
        }

        function renderClientComponentPlaceholder(moduleRef, props, rendered, rowId) {
            const attributes = [];

            attributes.push(`data-client-component="${escapeHtml(moduleRef)}"`);

            if (props && Object.keys(props).length > 0) {
                const propsForSerialization = {};
                for (const [key, value] of Object.entries(props)) {
                    if (key !== 'children') {
                        propsForSerialization[key] = value;
                    }
                }

                if (Object.keys(propsForSerialization).length > 0) {
                    const propsJson = JSON.stringify(propsForSerialization);
                    attributes.push(`data-props="${escapeHtml(propsJson)}"`);
                }
            }

            const children = props && props.children ? renderChildren(props.children, rendered) : '';

            const attrString = attributes.length > 0 ? ' ' + attributes.join(' ') : '';
            return `<div${attrString}>${children}</div>`;
        }

        function renderAttributes(props, rowId) {
            if (!props || typeof props !== 'object') {
                return '';
            }

            const attributes = [];

            for (const [key, value] of Object.entries(props)) {
                if (key === 'children' || key === 'key' || key === 'ref') {
                    continue;
                }

                if (value === null || value === undefined) {
                    continue;
                }

                if (key.startsWith('data-')) {
                    const attrValue = escapeHtml(String(value));
                    attributes.push(`${key}="${attrValue}"`);
                    continue;
                }

                let attrName = key;
                if (key === 'className') {
                    attrName = 'class';
                } else if (key === 'htmlFor') {
                    attrName = 'for';
                }

                if (typeof value === 'boolean') {
                    if (value) {
                        attributes.push(attrName);
                    }
                    continue;
                }

                if (key === 'style' && typeof value === 'object') {
                    const styleStr = Object.entries(value)
                        .map(([k, v]) => {
                            const kebabKey = k.replace(/([A-Z])/g, '-$1').toLowerCase();
                            return `${kebabKey}:${v}`;
                        })
                        .join(';');
                    attributes.push(`style="${escapeHtml(styleStr)}"`);
                    continue;
                }

                const attrValue = escapeHtml(String(value));
                attributes.push(`${attrName}="${attrValue}"`);
            }

            return attributes.length > 0 ? ' ' + attributes.join(' ') : '';
        }

        function renderChildrenRaw(children, rendered) {
            if (children === null || children === undefined) {
                return '';
            }

            if (typeof children === 'string') {
                return children;
            }

            if (typeof children === 'number' || typeof children === 'boolean') {
                return String(children);
            }

            if (Array.isArray(children)) {
                return children.map(child => {
                    if (typeof child === 'string') {
                        return child;
                    }
                    if (typeof child === 'number' || typeof child === 'boolean') {
                        return String(child);
                    }
                    return '';
                }).join('');
            }

            return '';
        }

        function renderChildren(children, rendered) {
            if (children === null || children === undefined) {
                return '';
            }

            if (typeof children === 'string' && children.startsWith('$')) {
                const match = children.match(/\$[@L]?(\d+)/);
                if (match) {
                    const refId = parseInt(match[1], 10);
                    return rendered.get(refId) || '';
                }
            }

            if (Array.isArray(children)) {
                if (children.length >= 4 && children[0] === '$') {
                    return renderElement(children, rendered, undefined);
                }

                const renderedChildren = [];
                let hasMultipleTextNodes = false;
                let textNodeCount = 0;

                for (const child of children) {
                    if (typeof child === 'string' || typeof child === 'number') {
                        textNodeCount++;
                    }
                }
                hasMultipleTextNodes = textNodeCount > 1;

                for (let i = 0; i < children.length; i++) {
                    const child = children[i];
                    const isTextNode = typeof child === 'string' || typeof child === 'number';

                    if (isTextNode && hasMultipleTextNodes) {
                        const renderedChild = renderElement(child, rendered, undefined);
                        if (renderedChild) {
                            renderedChildren.push('<!-- -->' + renderedChild + '<!-- -->');
                        }
                    } else {
                        renderedChildren.push(renderElement(child, rendered, undefined));
                    }
                }

                return renderedChildren.join('');
            }

            return renderElement(children, rendered, undefined);
        }

        function escapeHtml(text) {
            if (typeof text !== 'string') {
                text = String(text);
            }

            const htmlEscapeMap = {
                '&': '&amp;',
                '<': '&lt;',
                '>': '&gt;',
                '"': '&quot;',
                "'": '&#39;'
            };

            return text.replace(/[&<>"']/g, char => htmlEscapeMap[char]);
        }

        globalThis.renderRscToHtml = function(rscRows) {
            const rendered = new Map();

            let lastRowId = -1;
            for (const row of rscRows) {
                lastRowId = row.id;
            }

            for (const row of rscRows) {
                const html = renderElement(row.data, rendered, row.id);
                rendered.set(row.id, html);
            }

            return rendered.get(lastRowId) || rendered.get(0) || '';
        };
    "#;

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
                    warn!("index.html not found, using generated template: {}", e);
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

        if tag == "react.suspense" {
            let boundary_id = props
                .get("__boundary_id")
                .or_else(|| props.get("boundaryId"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            if !props.contains_key("fallback") {
                warn!(
                    "🔍 RSC_RENDERER: Suspense boundary missing fallback prop, boundary_id={}",
                    boundary_id
                );
            }
            if !props.contains_key("children") {
                warn!(
                    "🔍 RSC_RENDERER: Suspense boundary missing children prop, boundary_id={}",
                    boundary_id
                );
            }
        }

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
        debug!(
            "🔍 BOUNDARY_UPDATE: Generating boundary update HTML, boundary_id={}, row_id={}",
            boundary_id, row_id
        );

        let content_html = self.render_rsc_value_to_html(content_rsc).await?;

        let escaped_content =
            content_html.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");

        let update_script = format!(
            r#"<script data-boundary-id="{}" data-row-id="{}">
window.__rari && window.__rari.processBoundaryUpdate('{}', `{}`, {});
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
        debug!(
            "Generating boundary error HTML for boundary_id: {}, row_id: {}, error: {}",
            boundary_id, row_id, error_message
        );

        let error_script = format!(
            r#"<script data-boundary-id="{}" data-row-id="{}">
window.__rari && window.__rari.processBoundaryError('{}', '{}', {});
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
    #[allow(dead_code)]
    boundary_map: FxHashMap<String, u32>,
    shell_sent: bool,
    asset_links: Option<String>,
    #[allow(dead_code)]
    renderer: Arc<RscHtmlRenderer>,
    boundary_id_generator: BoundaryIdGenerator,
    #[allow(dead_code)]
    content_counter: AtomicU32,
    rari_to_react_boundary_map: parking_lot::Mutex<FxHashMap<String, String>>,
}

impl RscToHtmlConverter {
    pub fn new(renderer: Arc<RscHtmlRenderer>) -> Self {
        Self {
            row_cache: FxHashMap::default(),
            boundary_map: FxHashMap::default(),
            shell_sent: false,
            asset_links: None,
            renderer,
            boundary_id_generator: BoundaryIdGenerator::new(),
            content_counter: AtomicU32::new(0),
            rari_to_react_boundary_map: parking_lot::Mutex::new(FxHashMap::default()),
        }
    }

    pub fn with_assets(asset_links: String, renderer: Arc<RscHtmlRenderer>) -> Self {
        Self {
            row_cache: FxHashMap::default(),
            boundary_map: FxHashMap::default(),
            shell_sent: false,
            asset_links: Some(asset_links),
            renderer,
            boundary_id_generator: BoundaryIdGenerator::new(),
            content_counter: AtomicU32::new(0),
            rari_to_react_boundary_map: parking_lot::Mutex::new(FxHashMap::default()),
        }
    }

    fn next_boundary_id(&self) -> String {
        self.boundary_id_generator.next()
    }

    #[allow(dead_code)]
    fn next_content_id(&self) -> String {
        let current = self.content_counter.fetch_add(1, Ordering::SeqCst);
        format!("S:{}", current)
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
                            warn!("Continuing stream with shell only due to RSC parse error");
                            output
                        }
                    }
                } else {
                    match self.parse_and_render_rsc(&chunk.data, chunk.row_id).await {
                        Ok(html) => html,
                        Err(e) => {
                            error!("Error parsing RSC chunk: {}", e);
                            warn!("Skipping chunk due to parse error, continuing stream");
                            Vec::new()
                        }
                    }
                };
                Ok(html)
            }

            RscChunkType::BoundaryUpdate => {
                debug!("🔍 Processing BoundaryUpdate chunk, row_id: {}", chunk.row_id);
                match self.generate_boundary_replacement(&chunk).await {
                    Ok(html) => Ok(html),
                    Err(e) => {
                        error!("Error generating boundary replacement: {}", e);
                        warn!("Skipping boundary update due to error, continuing stream");
                        Ok(Vec::new())
                    }
                }
            }

            RscChunkType::BoundaryError => match self.generate_error_replacement(&chunk).await {
                Ok(html) => Ok(html),
                Err(e) => {
                    error!("Error generating error replacement: {}", e);
                    warn!("Using fallback error message");
                    Ok(self.generate_fallback_error_html())
                }
            },

            RscChunkType::StreamComplete => {
                debug!("🔍 CHUNK_CONVERT: Stream complete");
                Ok(self.generate_html_closing())
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
        r#"</div>
<script>
if (typeof window !== 'undefined') {
    window.__rari_stream_complete = true;
    window.dispatchEvent(new Event('rari:stream-complete'));
}
</script>
</body>
</html>"#
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
            debug!("Skipping chunk with invalid format (expected 'row_id:data'): {:?}", rsc_line);
            return Ok(Vec::new());
        }

        let json_str = parts[1].trim();
        if json_str.is_empty() {
            debug!("Skipping chunk with empty JSON data for row_id: {}", parts[0]);
            return Ok(Vec::new());
        }

        if json_str.starts_with('I') || json_str.starts_with('S') {
            debug!("Skipping metadata chunk (module import or symbol) for row_id: {}", parts[0]);
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
            let rari_boundary_id = props
                .get("boundaryId")
                .or_else(|| props.get("__boundary_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

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
                    debug!(
                        "🔍 RSC_TO_HTML: Created boundary mapping, rari_boundary_id={}, react_boundary_id={}",
                        rari_boundary_id, new_id
                    );
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
        debug!("🔍 Generating boundary replacement for row_id: {}", chunk.row_id);

        let rsc_line = String::from_utf8_lossy(&chunk.data);
        let parts: Vec<&str> = rsc_line.trim().splitn(2, ':').collect();

        if parts.len() != 2 {
            warn!("Invalid boundary update format: missing colon separator");
            return Ok(Vec::new());
        }

        let rsc_data: serde_json::Value = serde_json::from_str(parts[1])
            .map_err(|e| RariError::internal(format!("Invalid boundary update JSON: {}", e)))?;

        if let Some(obj) = rsc_data.as_object() {
            if let (Some(boundary_id_value), Some(content)) =
                (obj.get("boundary_id"), obj.get("content"))
            {
                if let Some(rari_boundary_id) = boundary_id_value.as_str() {
                    let react_boundary_id =
                        self.rari_to_react_boundary_map.lock().get(rari_boundary_id).cloned();

                    if let Some(react_boundary_id) = react_boundary_id {
                        debug!("🔍 Boundary update: {} -> {}", rari_boundary_id, react_boundary_id);

                        let content_html = self.rsc_element_to_html(content).await?;
                        debug!(
                            "🔍 Boundary update content HTML length: {} bytes",
                            content_html.len()
                        );

                        let content_id =
                            format!("S:{}", react_boundary_id.trim_start_matches("B:"));

                        let update_html = format!(
                            r#"<div hidden id="{}">{}</div><script>$RC=window.$RC||function(b,c){{const t=document.getElementById(b);const s=document.getElementById(c);if(t&&s){{const p=t.parentNode;Array.from(s.childNodes).forEach(n=>p.insertBefore(n,t));t.remove();s.remove();}}}};$RC("{}","{}");</script>"#,
                            content_id, content_html, react_boundary_id, content_id
                        );

                        return Ok(update_html.into_bytes());
                    } else {
                        warn!(
                            "No React boundary ID mapping found for Rari boundary: {}",
                            rari_boundary_id
                        );
                    }
                } else {
                    warn!("Boundary update boundary_id is not a string");
                }
            } else {
                warn!("Boundary update missing boundary_id or content property");
            }
        } else {
            warn!("Boundary update is not an object: {:?}", rsc_data);
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
            warn!("Invalid error chunk format, missing colon separator");
            return Ok(self.generate_fallback_error_html());
        }

        let json_part = parts[1].strip_prefix('E').unwrap_or(parts[1]);

        let error_data = match serde_json::from_str::<serde_json::Value>(json_part) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to parse error chunk JSON: {}", e);
                return Ok(self.generate_fallback_error_html());
            }
        };

        let rari_boundary_id = error_data["boundary_id"].as_str().unwrap_or("unknown").to_string();
        let error_message =
            error_data["error"].as_str().unwrap_or("Error loading content").to_string();

        debug!(
            "Generating error replacement for boundary: {}, error: {}",
            rari_boundary_id, error_message
        );

        let react_boundary_id =
            self.rari_to_react_boundary_map.lock().get(&rari_boundary_id).cloned();

        if let Some(react_boundary_id) = react_boundary_id {
            debug!("🔍 Error boundary update: {} -> {}", rari_boundary_id, react_boundary_id);

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
            warn!("No React boundary ID mapping found for Rari boundary: {}", rari_boundary_id);
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_parse_rsc_wire_format_valid() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"children":"Hello"}]
1:["$","span",null,{"children":"World"}]"#;

        let result = renderer.parse_rsc_wire_format(rsc_data);
        assert!(result.is_ok());

        let rows = result.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, 0);
        assert_eq!(rows[1].id, 1);
    }

    #[test]
    fn test_parse_rsc_line_missing_colon() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let result = renderer.parse_rsc_line("0invalid");
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("missing colon"));
    }

    #[test]
    fn test_parse_rsc_line_invalid_row_id() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let result = renderer.parse_rsc_line("abc:{}");
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Invalid row ID"));
    }

    #[test]
    fn test_parse_rsc_line_invalid_json() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let result = renderer.parse_rsc_line("0:{invalid json}");
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Invalid JSON"));
    }

    #[test]
    fn test_parse_react_element_invalid_structure() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let arr = vec![JsonValue::String("$".to_string()), JsonValue::String("div".to_string())];

        let result = renderer.parse_react_element(&arr);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("expected 4 elements"));
    }

    #[test]
    fn test_parse_react_element_non_string_tag() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let arr = vec![
            JsonValue::String("$".to_string()),
            JsonValue::Number(123.into()),
            JsonValue::Null,
            JsonValue::Object(serde_json::Map::new()),
        ];

        let result = renderer.parse_react_element(&arr);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("tag must be a string"));
    }

    #[test]
    fn test_parse_rsc_element_text() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let value = JsonValue::String("Hello World".to_string());
        let result = renderer.parse_rsc_element(&value);
        assert!(result.is_ok());

        if let RscElement::Text(text) = result.unwrap() {
            assert_eq!(text, "Hello World");
        } else {
            panic!("Expected Text element");
        }
    }

    #[test]
    fn test_parse_rsc_element_reference() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let value = JsonValue::String("$L1".to_string());
        let result = renderer.parse_rsc_element(&value);
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
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let mut props = serde_json::Map::new();
        props.insert("children".to_string(), JsonValue::String("Hello".to_string()));

        let value = JsonValue::Array(vec![
            JsonValue::String("$".to_string()),
            JsonValue::String("div".to_string()),
            JsonValue::Null,
            JsonValue::Object(props),
        ]);

        let result = renderer.parse_rsc_element(&value);
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
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let value = JsonValue::Array(vec![]);
        let result = renderer.parse_rsc_element(&value);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("Empty array"));
    }

    #[test]
    fn test_parse_rsc_wire_format_with_empty_lines() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let rsc_data = r#"0:["$","div",null,{"children":"Hello"}]

1:["$","span",null,{"children":"World"}]
"#;

        let result = renderer.parse_rsc_wire_format(rsc_data);
        assert!(result.is_ok());

        let rows = result.unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn test_render_rsc_to_html_integration() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let init_result = renderer.initialize().await;
        assert!(init_result.is_ok(), "Failed to initialize renderer: {:?}", init_result.err());

        let rsc_data = r#"0:["$","div",null,{"className":"container","children":"Hello World"}]"#;
        let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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

        renderer.initialize().await.unwrap();

        let rsc_data = r#"0:["$","div",null,{"children":["$","span",null,{"children":"Nested"}]}]"#;
        let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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

        renderer.initialize().await.unwrap();

        let rsc_data = r#"0:["$","div",null,{"children":"<script>alert('xss')</script>"}]"#;
        let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

        let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

        assert!(html.contains("&lt;script&gt;"), "Should escape < and >");
        assert!(!html.contains("<script>alert"), "Should not contain unescaped script tag");
    }

    #[tokio::test]
    async fn test_render_with_attributes() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let rsc_data = r#"0:["$","input",null,{"type":"text","className":"form-control","placeholder":"Enter text"}]"#;
        let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let template = renderer.generate_dev_template_fallback();

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

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"0:["$","div",null,{"children":"Hello World"}]"#;

        let config = crate::server::config::Config::new(crate::server::config::Mode::Development);

        let result = renderer.render_to_html(rsc_wire_format, &config).await;
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

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"1:["$","h1",null,{"children":"Title"}]
0:["$","div",null,{"className":"container","children":"$@1"}]"#;

        let config = crate::server::config::Config::new(crate::server::config::Mode::Development);

        let result = renderer.render_to_html(rsc_wire_format, &config).await;
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

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut config =
            crate::server::config::Config::new(crate::server::config::Mode::Development);
        config.rsc_html.cache_template = false;

        let result = renderer.render_to_html(rsc_wire_format, &config).await;
        assert!(result.is_ok(), "Should succeed with reasonable timeout");
    }

    #[tokio::test]
    async fn test_render_to_html_no_timeout() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut config =
            crate::server::config::Config::new(crate::server::config::Mode::Development);
        config.rsc_html.timeout_ms = 0;
        config.rsc_html.cache_template = false;

        let result = renderer.render_to_html(rsc_wire_format, &config).await;
        assert!(result.is_ok(), "Should succeed with no timeout");
    }

    #[tokio::test]
    async fn test_render_to_html_invalid_rsc_fallback() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let rsc_wire_format = "invalid:format:here";

        let mut config =
            crate::server::config::Config::new(crate::server::config::Mode::Development);
        config.rsc_html.cache_template = false;

        let result = renderer.render_to_html(rsc_wire_format, &config).await;
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

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"0:["$","p",null,{"children":"Content"}]"#;

        let config = crate::server::config::Config::new(crate::server::config::Mode::Development);

        let result = renderer.render_to_html(rsc_wire_format, &config).await;
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

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"0:["$","div",null,{"children":"Test"}]"#;

        let config = crate::server::config::Config::new(crate::server::config::Mode::Development);

        let result1 = renderer.render_to_html(rsc_wire_format, &config).await;
        assert!(result1.is_ok());

        {
            let cache = renderer.template_cache.lock();
            assert!(cache.is_some(), "Template should be cached");
        }

        let result2 = renderer.render_to_html(rsc_wire_format, &config).await;
        assert!(result2.is_ok());

        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[tokio::test]
    async fn test_render_to_html_ssr_disabled() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut config =
            crate::server::config::Config::new(crate::server::config::Mode::Development);
        config.rsc_html.enabled = false;

        let result = renderer.render_to_html(rsc_wire_format, &config).await;
        assert!(result.is_err(), "Should fail when SSR is disabled");

        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("disabled"), "Error should mention SSR is disabled");
    }

    #[tokio::test]
    async fn test_render_to_html_respects_timeout_config() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut config =
            crate::server::config::Config::new(crate::server::config::Mode::Development);
        config.rsc_html.timeout_ms = 10000;

        let result = renderer.render_to_html(rsc_wire_format, &config).await;
        assert!(result.is_ok(), "Should succeed with custom timeout");
    }

    #[tokio::test]
    async fn test_render_to_html_production_vs_dev_mode() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let rsc_wire_format = r#"0:["$","div",null,{"children":"Test"}]"#;

        let mut dev_config =
            crate::server::config::Config::new(crate::server::config::Mode::Development);
        dev_config.rsc_html.cache_template = false;

        let dev_result = renderer.render_to_html(rsc_wire_format, &dev_config).await;
        assert!(dev_result.is_ok());
        let dev_html = dev_result.unwrap();

        assert!(dev_html.contains("/@vite/client"), "Dev mode should have Vite client");

        renderer.clear_template_cache();
    }

    #[test]
    fn test_parse_suspense_boundary() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        let mut props = serde_json::Map::new();
        props.insert("fallback".to_string(), JsonValue::String("$L1".to_string()));
        props.insert("children".to_string(), JsonValue::String("$L2".to_string()));
        props.insert("__boundary_id".to_string(), JsonValue::String("suspense_123".to_string()));

        let value = JsonValue::Array(vec![
            JsonValue::String("$".to_string()),
            JsonValue::String("react.suspense".to_string()),
            JsonValue::Null,
            JsonValue::Object(props),
        ]);

        let result = renderer.parse_rsc_element(&value);
        assert!(result.is_ok());

        if let RscElement::Component { tag, key, props } = result.unwrap() {
            assert_eq!(tag, "react.suspense");
            assert!(key.is_none());
            assert!(props.contains_key("fallback"));
            assert!(props.contains_key("children"));
            assert!(props.contains_key("__boundary_id"));
            assert_eq!(props.get("__boundary_id").unwrap().as_str().unwrap(), "suspense_123");
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
                "__boundary_id": "suspense_test"
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
                "__boundary_id": "suspense_test"
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

        converter.row_cache.insert(1, r#"<div>Outer Loading</div>"#.to_string());
        converter.row_cache.insert(2, r#"<div>Inner Loading</div>"#.to_string());
        converter.row_cache.insert(3, r#"<div>Inner Content</div>"#.to_string());

        let inner_suspense = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": "$L2",
                "children": "$L3",
                "__boundary_id": "inner"
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
                "__boundary_id": "outer"
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
                "__boundary_id": "suspense_test"
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

        converter.row_cache.insert(1, r#"<div>Loading</div>"#.to_string());

        let suspense_element = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": "$L1",
                "children": ["$", "div", null, {"children": "Inline Content"}],
                "__boundary_id": "suspense_test"
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
    async fn test_generate_boundary_update_html_simple() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let content_rsc = serde_json::json!(["$", "div", null, {"children": "Resolved Content"}]);
        let boundary_id = "boundary_123";
        let row_id = 42;

        let result =
            renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
        assert!(result.is_ok(), "Should generate boundary update HTML successfully");

        let html = result.unwrap();

        assert!(html.starts_with("<script"), "Should be a script tag");
        assert!(html.contains("</script>"), "Should close script tag");

        assert!(
            html.contains(r#"data-boundary-id="boundary_123""#),
            "Should have boundary ID data attribute"
        );
        assert!(html.contains(r#"data-row-id="42""#), "Should have row ID data attribute");

        assert!(html.contains("window.__rari"), "Should reference window.__rari namespace");
        assert!(
            html.contains("processBoundaryUpdate"),
            "Should call processBoundaryUpdate function"
        );

        assert!(html.contains("'boundary_123'"), "Should pass boundary ID to function");

        assert!(html.contains("42"), "Should pass row ID to function");

        assert!(html.contains("Resolved Content"), "Should include the resolved content");
    }

    #[tokio::test]
    async fn test_generate_boundary_update_html_with_special_characters() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let content_rsc = serde_json::json!(["$", "div", null, {
            "children": "Content with `backticks` and ${template} and <script>alert('xss')</script>"
        }]);
        let boundary_id = "boundary_special";
        let row_id = 1;

        let result =
            renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
        assert!(result.is_ok(), "Should handle special characters");

        let html = result.unwrap();

        assert!(html.contains("\\`"), "Should escape backticks");

        assert!(html.contains("\\${"), "Should escape template literal interpolation");

        assert!(html.contains("&lt;script&gt;"), "Should escape HTML tags in content");
        assert!(!html.contains("<script>alert"), "Should not have unescaped script tags");
    }

    #[tokio::test]
    async fn test_generate_boundary_update_html_with_nested_elements() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let content_rsc = serde_json::json!(["$", "div", null, {
            "className": "container",
            "children": ["$", "span", null, {
                "children": "Nested Content"
            }]
        }]);
        let boundary_id = "boundary_nested";
        let row_id = 5;

        let result =
            renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
        assert!(result.is_ok(), "Should handle nested elements");

        let html = result.unwrap();

        assert!(html.contains("container"), "Should include className");
        assert!(html.contains("Nested Content"), "Should include nested content");
        assert!(html.contains("<div"), "Should have div tag");
        assert!(html.contains("<span"), "Should have nested span tag");
    }

    #[tokio::test]
    async fn test_generate_boundary_update_html_with_attributes() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let content_rsc = serde_json::json!(["$", "button", null, {
            "type": "button",
            "className": "btn btn-primary",
            "disabled": true,
            "children": "Click Me"
        }]);
        let boundary_id = "boundary_button";
        let row_id = 10;

        let result =
            renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
        assert!(result.is_ok(), "Should handle attributes");

        let html = result.unwrap();

        assert!(html.contains("type=\"button\""), "Should have type attribute");
        assert!(html.contains("class=\"btn btn-primary\""), "Should have class attribute");
        assert!(html.contains("disabled"), "Should have disabled attribute");
        assert!(html.contains("Click Me"), "Should have button text");
    }

    #[tokio::test]
    async fn test_generate_boundary_update_html_boundary_id_escaping() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let content_rsc = serde_json::json!(["$", "div", null, {"children": "Content"}]);
        let boundary_id = "boundary_with_\"quotes\"_and_<tags>";
        let row_id = 1;

        let result =
            renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
        assert!(result.is_ok(), "Should handle special characters in boundary ID");

        let html = result.unwrap();

        assert!(html.contains("&quot;"), "Should escape quotes in data attribute");
        assert!(html.contains("&lt;"), "Should escape < in data attribute");
        assert!(html.contains("&gt;"), "Should escape > in data attribute");

        assert!(html.contains("\\'") || html.contains("\\\""), "Should escape quotes in JS string");
    }

    #[tokio::test]
    async fn test_generate_boundary_update_html_empty_content() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = RscHtmlRenderer::new(runtime);

        renderer.initialize().await.unwrap();

        let content_rsc = serde_json::json!(["$", "div", null, {}]);
        let boundary_id = "boundary_empty";
        let row_id = 1;

        let result =
            renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
        assert!(result.is_ok(), "Should handle empty content");

        let html = result.unwrap();

        assert!(html.contains("<script"), "Should have script tag");
        assert!(html.contains("processBoundaryUpdate"), "Should call update function");
        assert!(html.contains("boundary_empty"), "Should include boundary ID");
    }

    #[test]
    fn test_escape_html_attribute() {
        let text = r#"Hello "world" & <tag>"#;
        let escaped = RscHtmlRenderer::escape_html_attribute(text);

        assert!(escaped.contains("&amp;"), "Should escape ampersand");
        assert!(escaped.contains("&quot;"), "Should escape quotes");
        assert!(escaped.contains("&lt;"), "Should escape less than");
        assert!(escaped.contains("&gt;"), "Should escape greater than");
    }

    #[test]
    fn test_escape_js_string() {
        let text = "Line 1\nLine 2\rLine 3\tTabbed";
        let escaped = RscHtmlRenderer::escape_js_string(text);

        assert!(escaped.contains("\\n"), "Should escape newlines");
        assert!(escaped.contains("\\r"), "Should escape carriage returns");
        assert!(escaped.contains("\\t"), "Should escape tabs");

        let text_with_quotes = r#"Single 'quotes' and "double" quotes"#;
        let escaped_quotes = RscHtmlRenderer::escape_js_string(text_with_quotes);

        assert!(escaped_quotes.contains("\\'"), "Should escape single quotes");
        assert!(escaped_quotes.contains("\\\""), "Should escape double quotes");

        let text_with_backslash = r"Path\to\file";
        let escaped_backslash = RscHtmlRenderer::escape_js_string(text_with_backslash);

        assert!(escaped_backslash.contains("\\\\"), "Should escape backslashes");
    }
}
