#![allow(clippy::disallowed_methods)]

#[cfg(test)]
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

    let mut config = crate::server::config::Config::new(crate::server::config::Mode::Development);
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

    let mut config = crate::server::config::Config::new(crate::server::config::Mode::Development);
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

    let mut config = crate::server::config::Config::new(crate::server::config::Mode::Development);
    config.rsc_html.cache_template = false;

    let result = renderer.render_to_html(rsc_wire_format, &config).await;
    assert!(result.is_ok(), "Should fall back to shell on error");

    let html = result.unwrap();

    assert!(html.contains("<!DOCTYPE html>"), "Should have DOCTYPE in fallback");
    assert!(html.contains(r#"<div id="root"></div>"#), "Should have empty root div in fallback");
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

    let mut config = crate::server::config::Config::new(crate::server::config::Mode::Development);
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

    let mut config = crate::server::config::Config::new(crate::server::config::Mode::Development);
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
    props.insert("~boundaryId".to_string(), JsonValue::String("suspense_123".to_string()));

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
        assert!(props.contains_key("~boundaryId"));
        assert_eq!(props.get("~boundaryId").unwrap().as_str().unwrap(), "suspense_123");
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
    assert!(!html.contains("Loading..."), "Should not render fallback when children are resolved");
    assert!(html.contains("<!--$?-->"), "Should have React boundary start marker");
    assert!(html.contains("<template id=\"B:0\">"), "Should have React template with boundary ID");
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
    assert!(html.contains("<template id=\"B:0\">"), "Should have React template with boundary ID");
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
            "~boundaryId": "suspense_test"
        }
    ]);

    let html = converter.rsc_element_to_html(&suspense_element).await.unwrap();

    assert!(html.contains("Inline Content"), "Should render inline children");
    assert!(!html.contains("Loading"), "Should not render fallback when children are inline");
    assert!(html.contains("<!--$?-->"), "Should have React boundary start marker");
    assert!(html.contains("<template id=\"B:0\">"), "Should have React template with boundary ID");
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

    let result = renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
    assert!(result.is_ok(), "Should generate boundary update HTML successfully");

    let html = result.unwrap();

    assert!(html.starts_with("<script"), "Should be a script tag");
    assert!(html.contains("</script>"), "Should close script tag");

    assert!(
        html.contains(r#"data-boundary-id="boundary_123""#),
        "Should have boundary ID data attribute with tilde"
    );
    assert!(html.contains(r#"data-row-id="42""#), "Should have row ID data attribute");

    assert!(html.contains("window['~rari']"), "Should reference window['~rari'] namespace");
    assert!(html.contains("processBoundaryUpdate"), "Should call processBoundaryUpdate function");

    assert!(html.contains("'boundary_123'"), "Should pass boundary ID to function");

    assert!(html.contains("42"), "Should pass row ID to function");

    assert!(html.contains(r#"42:["#), "Should include RSC row with row ID");
    assert!(html.contains("Resolved Content"), "Should include the content in RSC format");
}

#[tokio::test]
async fn test_generate_boundary_update_html_with_special_characters() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    renderer.initialize().await.unwrap();

    let content_rsc = serde_json::json!(["$", "div", null, {
        "children": "Content with 'quotes' and \"double\" and \n newlines"
    }]);
    let boundary_id = "boundary_special";
    let row_id = 1;

    let result = renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
    assert!(result.is_ok(), "Should handle special characters");

    let html = result.unwrap();

    assert!(html.contains("\\'"), "Should escape single quotes for JavaScript");
    assert!(html.contains("\\\""), "Should escape double quotes for JavaScript");
    assert!(html.contains("\\n"), "Should escape newlines for JavaScript");

    // The script tag itself should be valid HTML
    assert!(html.starts_with("<script"), "Should be a script tag");
    assert!(html.contains("</script>"), "Should close script tag");

    assert!(html.contains(r#"1:["#), "Should include RSC row with row ID");
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

    let result = renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
    assert!(result.is_ok(), "Should handle nested elements");

    let html = result.unwrap();

    assert!(html.contains("container"), "Should include className in RSC data");
    assert!(html.contains("Nested Content"), "Should include nested content in RSC data");

    assert!(html.starts_with("<script"), "Should be a script tag");
    assert!(html.contains("processBoundaryUpdate"), "Should call processBoundaryUpdate");

    assert!(html.contains(r#"5:["#), "Should include RSC row with row ID");
    assert!(
        html.contains(r#"\\"span\\""#) || html.contains("span"),
        "Should include nested span in RSC format"
    );
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

    let result = renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
    assert!(result.is_ok(), "Should handle attributes");

    let html = result.unwrap();

    assert!(
        html.contains("type") || html.contains(r#"\\"type\\""#),
        "Should have type in RSC data"
    );
    assert!(html.contains("button"), "Should have button value in RSC data");
    assert!(
        html.contains("className") || html.contains(r#"\\"className\\""#),
        "Should have className in RSC data"
    );
    assert!(html.contains("btn btn-primary"), "Should have class value in RSC data");
    assert!(html.contains("disabled"), "Should have disabled in RSC data");
    assert!(html.contains("Click Me"), "Should have button text in RSC data");

    assert!(html.starts_with("<script"), "Should be a script tag");
    assert!(html.contains("processBoundaryUpdate"), "Should call processBoundaryUpdate");

    assert!(html.contains(r#"10:["#), "Should include RSC row with row ID");
}

#[tokio::test]
async fn test_generate_boundary_update_html_boundary_id_escaping() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    renderer.initialize().await.unwrap();

    let content_rsc = serde_json::json!(["$", "div", null, {"children": "Content"}]);
    let boundary_id = "boundary_with_\"quotes\"_and_<tags>";
    let row_id = 1;

    let result = renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
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

    let result = renderer.generate_boundary_update_html(boundary_id, &content_rsc, row_id).await;
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
