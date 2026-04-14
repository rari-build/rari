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

    let rsc_wire_format = r#"1:["$","h1",null,{"children":"Title"}]
2:["$","div",null,{"className":"container","children":"$@1"}]"#;

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
async fn test_render_rsc_to_html_string_row_0_as_root() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data = r#"0:["$","div",null,{"children":"Root Content"}]
1:["$","span",null,{"children":"Other Content"}]
2:["$","p",null,{"children":"More Content"}]"#;

    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();
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

    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();
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

    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();
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

    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();
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

    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();
    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains("<div"), "Should have div from row 0");
    assert!(html.contains("Child Content"), "Should resolve $L1 reference");
    assert!(html.contains("<span"), "Should have span from row 1");
}

#[tokio::test]
async fn test_render_rsc_to_html_string_consistent_with_streaming() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data = r#"0:["$","main",null,{"children":"Main Content"}]
1:["$","aside",null,{"children":"Sidebar"}]"#;

    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();
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

    assert!(!test_is_valid_attribute_name("1invalid"), "starting with number should be invalid");
    assert!(!test_is_valid_attribute_name("-invalid"), "starting with hyphen should be invalid");
    assert!(!test_is_valid_attribute_name(".invalid"), "starting with period should be invalid");
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
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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
    assert!(html.contains("a-b-c=\"multiple hyphens valid\""), "Multiple hyphens should be valid");
    assert!(html.contains("a.b.c=\"multiple periods valid\""), "Multiple periods should be valid");
    assert!(html.contains("a:b:c=\"multiple colons valid\""), "Multiple colons should be valid");
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
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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

    let rsc_data = r#"0:["$","div",null,{"中文":"value1","日本語":"value2","data-한글":"value3"}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

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
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains("color:red"), "color should be in style: {}", html);
    assert!(html.contains("padding:10px"), "padding should be in style: {}", html);

    assert!(!html.contains("display:null"), "display:null should not appear in style: {}", html);
    assert!(!html.contains("margin:null"), "margin:null should not appear in style: {}", html);
    assert!(!html.contains("null"), "The word 'null' should not appear in output: {}", html);
}

#[tokio::test]
async fn test_style_object_all_null_values() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data = r#"0:["$","div",null,{"style":{"display":null,"color":null},"class":"test"}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(!html.contains("null"), "Should not contain 'null' string: {}", html);

    assert!(html.contains("class=\"test\""), "Should have class attribute: {}", html);
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
        panic!("Tag with angle bracket should be rejected, but got: {:?}", html);
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
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();
    let result = renderer.render_rsc_to_html_string(&rows).await;
    assert!(result.is_ok(), "Valid tag 'div' should be accepted");

    let rsc_data_invalid = r#"0:["$","div onclick",null,{}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data_invalid).unwrap();
    let result = renderer.render_rsc_to_html_string(&rows).await;
    assert!(result.is_err(), "Tag with space should be rejected");
    assert!(
        result.unwrap_err().to_string().contains("Invalid tag name"),
        "Error should mention invalid tag"
    );

    let rsc_data_injection = r#"0:["$","div><script>alert(1)</script",null,{}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data_injection).unwrap();
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

        assert!(result.is_err(), "Malicious tag '{}' should be rejected", malicious_tag);
    }
}

#[tokio::test]
async fn test_style_object_numeric_values_with_px() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data =
        r#"0:["$","div",null,{"style":{"width":100,"height":200,"margin":10,"padding":20}}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains("width:100px"), "Width should have px suffix: {}", html);
    assert!(html.contains("height:200px"), "Height should have px suffix: {}", html);
    assert!(html.contains("margin:10px"), "Margin should have px suffix: {}", html);
    assert!(html.contains("padding:20px"), "Padding should have px suffix: {}", html);
}

#[tokio::test]
async fn test_style_object_unitless_properties() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data =
        r#"0:["$","div",null,{"style":{"opacity":0.5,"zIndex":10,"lineHeight":1.5,"flexGrow":2}}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains("opacity:0.5"), "Opacity should not have px suffix: {}", html);
    assert!(html.contains("z-index:10"), "z-index should not have px suffix: {}", html);
    assert!(html.contains("line-height:1.5"), "line-height should not have px suffix: {}", html);
    assert!(html.contains("flex-grow:2"), "flex-grow should not have px suffix: {}", html);
}

#[tokio::test]
async fn test_style_object_mixed_values() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data =
        r#"0:["$","div",null,{"style":{"width":"50%","height":100,"opacity":0.8,"color":"red"}}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains("width:50%"), "String width should be preserved: {}", html);
    assert!(html.contains("height:100px"), "Numeric height should have px: {}", html);
    assert!(html.contains("opacity:0.8"), "Opacity should not have px: {}", html);
    assert!(html.contains("color:red"), "Color string should be preserved: {}", html);
}

#[tokio::test]
async fn test_style_object_camel_case_with_numeric() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data = r#"0:["$","div",null,{"style":{"fontSize":16,"marginTop":10,"paddingLeft":5,"zIndex":100}}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains("font-size:16px"), "fontSize should become font-size with px: {}", html);
    assert!(
        html.contains("margin-top:10px"),
        "marginTop should become margin-top with px: {}",
        html
    );
    assert!(
        html.contains("padding-left:5px"),
        "paddingLeft should become padding-left with px: {}",
        html
    );
    assert!(html.contains("z-index:100"), "zIndex should become z-index without px: {}", html);
}

#[tokio::test]
async fn test_style_object_float_values() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data =
        r#"0:["$","div",null,{"style":{"width":100.5,"opacity":0.75,"lineHeight":1.2}}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains("width:100.5px"), "Float width should have px: {}", html);
    assert!(html.contains("opacity:0.75"), "Float opacity should not have px: {}", html);
    assert!(html.contains("line-height:1.2"), "Float line-height should not have px: {}", html);
}

#[tokio::test]
async fn test_boolean_html_attributes_presence_only() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data = r#"0:["$","input",null,{"type":"checkbox","checked":true,"disabled":true,"required":false}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains(" checked"), "checked=true should render as presence-only: {}", html);
    assert!(!html.contains("checked=\"true\""), "checked should not have =\"true\": {}", html);
    assert!(html.contains(" disabled"), "disabled=true should render as presence-only: {}", html);
    assert!(!html.contains("disabled=\"true\""), "disabled should not have =\"true\": {}", html);

    assert!(!html.contains("required"), "required=false should be omitted: {}", html);
}

#[tokio::test]
async fn test_aria_attributes_render_as_strings() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data =
        r#"0:["$","div",null,{"aria-hidden":true,"aria-expanded":false,"aria-checked":true}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(
        html.contains("aria-hidden=\"true\""),
        "aria-hidden=true should render as \"true\": {}",
        html
    );
    assert!(
        html.contains("aria-expanded=\"false\""),
        "aria-expanded=false should render as \"false\": {}",
        html
    );
    assert!(
        html.contains("aria-checked=\"true\""),
        "aria-checked=true should render as \"true\": {}",
        html
    );
}

#[tokio::test]
async fn test_content_editable_and_draggable_as_strings() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data =
        r#"0:["$","div",null,{"contentEditable":true,"draggable":false,"spellcheck":true}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(
        html.contains("contentEditable=\"true\""),
        "contentEditable=true should render as \"true\": {}",
        html
    );
    assert!(
        html.contains("draggable=\"false\""),
        "draggable=false should render as \"false\": {}",
        html
    );
    assert!(
        html.contains("spellcheck=\"true\""),
        "spellcheck=true should render as \"true\": {}",
        html
    );
}

#[tokio::test]
async fn test_mixed_boolean_attributes() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data =
        r#"0:["$","button",null,{"disabled":true,"aria-disabled":true,"aria-pressed":false}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains(" disabled"), "disabled=true should be presence-only: {}", html);
    assert!(!html.contains(" disabled="), "disabled should not have a value: {}", html);

    assert!(
        html.contains("aria-disabled=\"true\""),
        "aria-disabled=true should render as \"true\": {}",
        html
    );
    assert!(
        html.contains("aria-pressed=\"false\""),
        "aria-pressed=false should render as \"false\": {}",
        html
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

    assert!(html.contains(" checked"), "checked=true should be presence-only: {}", html);
    assert!(!html.contains(" checked="), "checked should not have a value: {}", html);
    assert!(!html.contains(" disabled"), "disabled=false should be omitted: {}", html);

    assert!(
        html.contains("aria-checked=\"true\""),
        "aria-checked=true should render as \"true\": {}",
        html
    );
    assert!(
        html.contains("aria-disabled=\"false\""),
        "aria-disabled=false should render as \"false\": {}",
        html
    );
}

#[tokio::test]
async fn test_all_html_boolean_attributes() {
    let runtime = Arc::new(JsExecutionRuntime::new(None));
    let renderer = RscHtmlRenderer::new(runtime);

    let rsc_data = r#"0:["$","div",null,{"hidden":true,"readonly":true,"required":true,"autofocus":true,"multiple":true}]"#;
    let rows = renderer.parse_rsc_wire_format(rsc_data).unwrap();

    let html = renderer.render_rsc_to_html_string(&rows).await.unwrap();

    assert!(html.contains(" hidden"), "hidden should be presence-only: {}", html);
    assert!(html.contains(" readonly"), "readonly should be presence-only: {}", html);
    assert!(html.contains(" required"), "required should be presence-only: {}", html);
    assert!(html.contains(" autofocus"), "autofocus should be presence-only: {}", html);
    assert!(html.contains(" multiple"), "multiple should be presence-only: {}", html);

    assert!(
        !html.contains("=\"true\""),
        "HTML boolean attributes should not have =\"true\": {}",
        html
    );
}
