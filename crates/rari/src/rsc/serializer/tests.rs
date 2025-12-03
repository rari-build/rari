#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::super::*;
    use crate::error::RariError;
    use crate::rsc::ServerComponentExecutor;
    use rustc_hash::FxHashMap;
    use serde_json::Value;
    use serde_json::json;

    #[test]
    fn test_serialize_html_element() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("className".to_string(), json!("test-class"));
        props.insert("children".to_string(), json!("Hello World"));

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains(r#"["$","div",null,"#));
        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_serialize_client_component() {
        let mut serializer = RscSerializer::new();

        serializer.register_client_component("Button", "./components/Button.client.js", "default");

        let mut props = FxHashMap::default();
        props.insert("onClick".to_string(), json!("handleClick"));
        props.insert("children".to_string(), json!("Click me"));

        let element = SerializedReactElement::create_client_component("Button", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("./components/Button.client.js"));
        assert!(result.contains("$"));
        assert!(result.contains("Click me"));

        assert!(result.contains(":I"));
    }

    #[test]
    fn test_serialize_text_element() {
        let mut serializer = RscSerializer::new();

        let element = SerializedReactElement::create_text_element("Hello, RSC!");
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Hello, RSC!"));
    }

    #[test]
    fn test_serialize_server_component() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("userId".to_string(), json!(123));

        let element = SerializedReactElement::create_server_component("UserProfile", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("UserProfile"));
    }

    #[test]
    fn test_serialize_fragment() {
        let mut serializer = RscSerializer::new();

        let children = vec![json!("First child"), json!("Second child")];

        let element = SerializedReactElement::create_fragment(Some(children));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("First child"));
        assert!(result.contains("Second child"));
    }

    #[test]
    fn test_module_deduplication() {
        let mut serializer = RscSerializer::new();

        serializer.register_client_component("Button", "./components/Button.client.js", "default");

        let element1 = SerializedReactElement::create_client_component("Button", None);
        let _result1 = serializer.serialize_to_rsc_format(&element1);

        serializer.output_lines.clear();
        serializer.serialized_modules.clear();

        let element2 = SerializedReactElement::create_client_component("Button", None);
        let result2 = serializer.serialize_to_rsc_format(&element2);

        assert!(result2.contains("./components/Button.client.js"));
    }

    #[test]
    fn test_complex_nested_structure() {
        let mut serializer = RscSerializer::new();

        serializer.register_client_component("Button", "./components/Button.client.js", "default");

        let mut button_props = FxHashMap::default();
        button_props.insert("children".to_string(), json!("Click me"));

        let mut div_props = FxHashMap::default();
        div_props.insert("className".to_string(), json!("container"));

        let _button_element =
            SerializedReactElement::create_client_component("Button", Some(button_props));
        let div_element = SerializedReactElement::create_html_element("div", Some(div_props));

        let result = serializer.serialize_to_rsc_format(&div_element);

        assert!(result.contains(r#"["$","div""#));
        assert!(result.contains("container"));
    }

    #[test]
    fn test_unregistered_client_component() {
        let mut serializer = RscSerializer::new();

        let element = SerializedReactElement::create_client_component("UnknownButton", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("data-rsc-error"));
        assert!(result.contains("UnknownButton"));
    }

    #[test]
    fn test_empty_props() {
        let mut serializer = RscSerializer::new();

        let element = SerializedReactElement::create_html_element("br", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains(r#"["$","br",null,null]"#));
    }

    #[test]
    fn test_complex_props_serialization() {
        let mut serializer = RscSerializer::new();

        let mut complex_props = FxHashMap::default();
        complex_props.insert("valid_string".to_string(), json!("Hello"));
        complex_props.insert("valid_number".to_string(), json!(42));
        complex_props.insert("valid_boolean".to_string(), json!(true));
        complex_props.insert("valid_null".to_string(), json!(null));
        complex_props.insert(
            "nested_object".to_string(),
            json!({
                "inner": "value",
                "count": 10
            }),
        );
        complex_props.insert("array_prop".to_string(), json!([1, 2, 3]));

        let element = SerializedReactElement::create_html_element("div", Some(complex_props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Hello"));
        assert!(result.contains("42"));
        assert!(result.contains("true"));
        assert!(result.contains("inner"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_props_validation_function_rejection() {
        let mut serializer = RscSerializer::new();

        let mut props_with_function = FxHashMap::default();
        props_with_function
            .insert("onClick".to_string(), json!("function handleClick() { return true; }"));
        props_with_function.insert("valid_prop".to_string(), json!("valid value"));

        let element =
            SerializedReactElement::create_html_element("button", Some(props_with_function));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("null"));
        assert!(result.contains("valid value"));
        assert!(!result.contains("handleClick"));
    }

    #[test]
    fn test_props_validation_circular_reference() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("safe_prop".to_string(), json!("safe"));

        props.insert(
            "nested".to_string(),
            json!({
                "level1": {
                    "level2": {
                        "data": "deep value"
                    }
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("safe"));
        assert!(result.contains("deep value"));
    }

    #[test]
    fn test_props_validation_non_serializable_detection() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("symbol_prop".to_string(), json!("Symbol(test)"));
        props.insert("object_prop".to_string(), json!("Object [object Object]"));
        props.insert("valid_prop".to_string(), json!("normal string"));

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("normal string"));
        assert!(!result.contains("Symbol(test)"));
        assert!(!result.contains("Object [object"));
    }

    struct MockServerComponentExecutor;

    impl ServerComponentExecutor for MockServerComponentExecutor {
        fn execute_server_component(
            &self,
            component_name: &str,
            _props: Option<&FxHashMap<String, Value>>,
        ) -> Result<Value, RariError> {
            match component_name {
                "SuccessfulComponent" => {
                    Ok(json!(["$", "h1", null, {"children": "Server rendered content"}]))
                }
                "HTMLComponent" => Ok(json!("<p>HTML from server</p>")),
                "FailingComponent" => {
                    Err(RariError::js_execution("Component execution failed".to_string()))
                }
                _ => Ok(
                    json!({"type": "div", "props": {"children": format!("Component: {}", component_name)}}),
                ),
            }
        }
    }

    #[test]
    fn test_server_component_execution_successful() {
        let mut serializer = RscSerializer::new();
        serializer.set_server_component_executor(Box::new(MockServerComponentExecutor));

        let element = SerializedReactElement::create_server_component("SuccessfulComponent", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Server rendered content"));
        assert!(result.contains(r#"["$","h1",null"#));
    }

    #[test]
    fn test_server_component_execution_html_result() {
        let mut serializer = RscSerializer::new();
        serializer.set_server_component_executor(Box::new(MockServerComponentExecutor));

        let element = SerializedReactElement::create_server_component("HTMLComponent", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("HTMLComponent"));
        assert!(result.contains(r#"["$","div",null"#));
    }

    #[test]
    fn test_server_component_execution_failure() {
        let mut serializer = RscSerializer::new();
        serializer.set_server_component_executor(Box::new(MockServerComponentExecutor));

        let element = SerializedReactElement::create_server_component("FailingComponent", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Error rendering FailingComponent"));
        assert!(result.contains(r#"["$","div",null"#));
    }

    #[test]
    fn test_server_component_no_executor() {
        let mut serializer = RscSerializer::new();

        let element = SerializedReactElement::create_server_component("TestComponent", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Error rendering TestComponent"));
        assert!(result.contains(r#"["$","div",null"#));
    }

    #[test]
    fn test_server_component_with_props() {
        let mut serializer = RscSerializer::new();
        serializer.set_server_component_executor(Box::new(MockServerComponentExecutor));

        let mut props = FxHashMap::default();
        props.insert("title".to_string(), json!("Test Title"));
        props.insert("count".to_string(), json!(5));

        let element =
            SerializedReactElement::create_server_component("GenericComponent", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("GenericComponent"));
        assert!(result.contains("Component: GenericComponent"));
    }

    #[test]
    fn test_serialize_element_with_suspense() {
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut fallback_props = FxHashMap::default();
        fallback_props.insert("children".to_string(), json!("Loading..."));
        let fallback = LoadingReactElement::with_props("div", fallback_props);

        let mut children_props = FxHashMap::default();
        children_props.insert("children".to_string(), json!("Content loaded"));
        let children = LoadingReactElement::with_props("div", children_props);

        let mut suspense_props = FxHashMap::default();
        suspense_props.insert("fallback".to_string(), serde_json::to_value(&fallback).unwrap());
        suspense_props.insert("children".to_string(), serde_json::to_value(&children).unwrap());
        suspense_props.insert("__boundary_id".to_string(), json!("test-boundary"));

        let suspense = LoadingReactElement::with_props("react.suspense", suspense_props);

        let result = serializer.serialize_element(&suspense).unwrap();

        assert!(result.starts_with("$L"), "Should return a reference to the Suspense boundary");

        let output = serializer.output_lines.join("\n");
        assert!(output.contains("react.suspense"), "Should contain Suspense tag");
        assert!(output.contains("test-boundary"), "Should contain boundary ID");
        assert!(output.contains("Loading..."), "Should contain fallback content");
        assert!(output.contains("Content loaded"), "Should contain children content");
    }

    #[test]
    fn test_serialize_element_regular() {
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("className".to_string(), json!("test-class"));
        props.insert("children".to_string(), json!("Hello World"));

        let element = LoadingReactElement::with_props("div", props).with_key("test-key");

        let result = serializer.serialize_element(&element).unwrap();

        assert!(result.starts_with("$L"), "Should return a reference");

        let output = serializer.output_lines.join("\n");
        assert!(output.contains(r#"["$","div""#), "Should contain div element");
        assert!(output.contains("test-class"), "Should contain className prop");
        assert!(output.contains("Hello World"), "Should contain children");
        assert!(output.contains("test-key"), "Should contain key");
    }

    #[test]
    fn test_emit_suspense_boundary_with_refs() {
        let mut serializer = RscSerializer::new();

        let result =
            serializer.emit_suspense_boundary_with_refs("$L1", "$L2", "boundary-123").unwrap();

        assert!(result.starts_with("$L"), "Should return a reference");

        let output = serializer.output_lines.join("\n");
        assert!(output.contains("react.suspense"), "Should contain Suspense tag");
        assert!(output.contains(r#""fallback":"$L1""#), "Should reference fallback");
        assert!(output.contains(r#""children":"$L2""#), "Should reference children");
        assert!(output.contains("boundary-123"), "Should contain boundary ID");
    }

    #[test]
    fn test_suspense_wire_format_structure() {
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let fallback = LoadingReactElement::with_props("div", {
            let mut props = FxHashMap::default();
            props.insert("className".to_string(), json!("loading-spinner"));
            props.insert("children".to_string(), json!("Loading..."));
            props
        });

        let children = LoadingReactElement::with_props("article", {
            let mut props = FxHashMap::default();
            props.insert("className".to_string(), json!("content"));
            props.insert("children".to_string(), json!("Article content"));
            props
        });

        let suspense = LoadingReactElement::with_props("react.suspense", {
            let mut props = FxHashMap::default();
            props.insert("fallback".to_string(), serde_json::to_value(&fallback).unwrap());
            props.insert("children".to_string(), serde_json::to_value(&children).unwrap());
            props.insert("__boundary_id".to_string(), json!("article-boundary"));
            props
        });

        let _result = serializer.serialize_element(&suspense).unwrap();

        let output = serializer.output_lines.join("\n");

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "Should have 3 rows in wire format");

        assert!(lines[0].contains(r#"["$","div""#), "First row should be fallback div");
        assert!(lines[0].contains("loading-spinner"), "Should contain fallback className");

        assert!(lines[1].contains(r#"["$","article""#), "Second row should be children article");
        assert!(lines[1].contains("content"), "Should contain children className");

        assert!(lines[2].contains("react.suspense"), "Third row should be Suspense boundary");
        assert!(lines[2].contains("$L0"), "Should reference fallback with $L0");
        assert!(lines[2].contains("$L1"), "Should reference children with $L1");
        assert!(lines[2].contains("article-boundary"), "Should contain boundary ID");
    }

    #[test]
    fn test_suspense_missing_fallback_error() {
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("children".to_string(), json!({"tag": "div", "props": {}}));
        props.insert("__boundary_id".to_string(), json!("test"));

        let suspense = LoadingReactElement::with_props("react.suspense", props);

        let result = serializer.serialize_element(&suspense);
        assert!(result.is_err(), "Should error when fallback is missing");
        assert!(
            result.unwrap_err().to_string().contains("fallback"),
            "Error should mention missing fallback"
        );
    }

    #[test]
    fn test_suspense_missing_children_error() {
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("fallback".to_string(), json!({"tag": "div", "props": {}}));
        props.insert("__boundary_id".to_string(), json!("test"));

        let suspense = LoadingReactElement::with_props("react.suspense", props);

        let result = serializer.serialize_element(&suspense);
        assert!(result.is_err(), "Should error when children is missing");
        assert!(
            result.unwrap_err().to_string().contains("children"),
            "Error should mention missing children"
        );
    }
}
