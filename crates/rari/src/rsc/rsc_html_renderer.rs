use crate::error::RariError;
use crate::runtime::JsExecutionRuntime;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct RscRow {
    pub id: u32,
    pub data: RscElement,
}

#[derive(Debug, Clone)]
pub enum RscElement {
    Component { tag: String, key: Option<String>, props: FxHashMap<String, JsonValue> },
    Text(String),
    Reference(String),
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
                debug!("Successfully read template from: {}", path);
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
}
