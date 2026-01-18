#[cfg(test)]
#[allow(clippy::disallowed_methods)]
#[allow(clippy::module_inception)]
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
        serializer.serialize_to_rsc_format(&element1);

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

        let mut div_props = FxHashMap::default();
        div_props.insert("className".to_string(), json!("container"));

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
        suspense_props.insert("~boundaryId".to_string(), json!("test-boundary"));

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
    fn test_serialize_map_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$map": [["key1", "value1"], ["key2", "value2"]]});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(
            processed.as_str().unwrap().starts_with("$Q"),
            "Expected $Q prefix, got: {:?}",
            processed
        );

        assert_eq!(processed, "$Q0");

        assert!(!serializer.output_lines.is_empty(), "Expected output lines to be emitted");
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];

        assert!(chunk.starts_with("0:"));
        assert!(chunk.contains("key1"));
        assert!(chunk.contains("value1"));
        assert!(chunk.contains("key2"));
        assert!(chunk.contains("value2"));
    }

    #[test]
    fn test_serialize_set_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$set": ["value1", "value2", "value3"]});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$W"));
        assert_eq!(processed, "$W0");

        assert!(!serializer.output_lines.is_empty());
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];
        assert!(chunk.starts_with("0:"));
        assert!(chunk.contains("value1"));
        assert!(chunk.contains("value2"));
        assert!(chunk.contains("value3"));
    }

    #[test]
    fn test_serialize_formdata_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$formdata": [["field1", "value1"], ["field2", "value2"]]});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$K"));
        assert_eq!(processed, "$K0");

        assert!(!serializer.output_lines.is_empty());
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];
        assert!(chunk.starts_with("0:"));
        assert!(chunk.contains("field1"));
        assert!(chunk.contains("value1"));
        assert!(chunk.contains("field2"));
        assert!(chunk.contains("value2"));
    }

    #[test]
    fn test_serialize_nested_collections() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "map": {"$map": [["key1", "value1"]]},
            "set": {"$set": ["item1", "item2"]},
            "nested": {
                "formdata": {"$formdata": [["field", "value"]]}
            }
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert_eq!(serializer.output_lines.len(), 3);

        assert!(processed["map"].as_str().unwrap().starts_with("$Q"));
        assert!(processed["set"].as_str().unwrap().starts_with("$W"));
        assert!(processed["nested"]["formdata"].as_str().unwrap().starts_with("$K"));
    }

    #[test]
    fn test_serialize_map_with_special_values() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "$map": [
                ["date", {"$date": "2025-12-09T18:00:00.000Z"}],
                ["bigint", {"$bigint": "123"}],
                ["nan", "$NaN"]
            ]
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$Q"));
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];

        assert!(chunk.contains("$D2025-12-09T18:00:00.000Z"));
        assert!(chunk.contains("$n123"));
        assert!(chunk.contains("$NaN"));
    }

    #[test]
    fn test_serialize_multiple_maps_unique_ids() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "map1": {"$map": [["a", "1"]]},
            "map2": {"$map": [["b", "2"]]},
            "map3": {"$map": [["c", "3"]]}
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert_eq!(serializer.output_lines.len(), 3);

        let ref1 = processed["map1"].as_str().unwrap();
        let ref2 = processed["map2"].as_str().unwrap();
        let ref3 = processed["map3"].as_str().unwrap();

        assert!(ref1.starts_with("$Q"));
        assert!(ref2.starts_with("$Q"));
        assert!(ref3.starts_with("$Q"));

        assert_ne!(ref1, ref2);
        assert_ne!(ref2, ref3);
        assert_ne!(ref1, ref3);
    }

    #[test]
    fn test_serialize_promise_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$promise": {"status": "pending", "value": null}});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$@"));
        assert_eq!(processed, "$@0");

        assert!(!serializer.output_lines.is_empty());
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];
        assert!(chunk.starts_with("0:"));
        assert!(chunk.contains("pending"));
    }

    #[test]
    fn test_serialize_server_function() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "$function": {
                "id": "actions/todo-actions#addTodo",
                "bound": null
            }
        });
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$F"));
        assert_eq!(processed, "$F0");

        assert!(!serializer.output_lines.is_empty());
        let chunk = &serializer.output_lines[0];
        assert!(chunk.contains("addTodo"));
    }

    #[test]
    fn test_serialize_temporary_reference() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$temp": "ref_123"});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert_eq!(processed, "$Tref_123");

        assert!(serializer.output_lines.is_empty());
    }

    #[test]
    fn test_serialize_symbol_reference() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$symbol": "iterator"});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert_eq!(processed, "$Siterator");

        assert!(serializer.output_lines.is_empty());
    }

    #[test]
    fn test_serialize_deferred_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$deferred": {"type": "debug", "data": "some data"}});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$Y"));
        assert_eq!(processed, "$Y0");

        assert!(!serializer.output_lines.is_empty());
        let chunk = &serializer.output_lines[0];
        assert!(chunk.contains("debug"));
    }

    #[test]
    fn test_serialize_iterator_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$iterator": ["value1", "value2", "value3"]});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$i"));
        assert_eq!(processed, "$i0");

        assert!(!serializer.output_lines.is_empty());
        let chunk = &serializer.output_lines[0];
        assert!(chunk.contains("value1"));
        assert!(chunk.contains("value2"));
    }

    #[test]
    fn test_serialize_mixed_advanced_markers() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "promise": {"$promise": {"status": "fulfilled", "value": 42}},
            "function": {"$function": {"id": "myAction"}},
            "temp": {"$temp": "temp_ref"},
            "symbol": {"$symbol": "toStringTag"},
            "deferred": {"$deferred": {"data": "lazy"}},
            "iterator": {"$iterator": [1, 2, 3]}
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed["promise"].as_str().unwrap().starts_with("$@"));
        assert!(processed["function"].as_str().unwrap().starts_with("$F"));
        assert_eq!(processed["temp"], "$Ttemp_ref");
        assert_eq!(processed["symbol"], "$StoStringTag");
        assert!(processed["deferred"].as_str().unwrap().starts_with("$Y"));
        assert!(processed["iterator"].as_str().unwrap().starts_with("$i"));

        assert_eq!(serializer.output_lines.len(), 4);
    }

    #[test]
    fn test_serialize_nested_advanced_markers() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "$promise": {
                "status": "fulfilled",
                "value": {
                    "map": {"$map": [["key", "value"]]},
                    "date": {"$date": "2025-12-09T18:00:00.000Z"}
                }
            }
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$@"));

        assert_eq!(serializer.output_lines.len(), 2);

        let promise_chunk = serializer
            .output_lines
            .iter()
            .find(|line| line.contains("fulfilled"))
            .expect("Promise chunk not found");

        assert!(promise_chunk.contains("$Q"));
        assert!(promise_chunk.contains("$D"));
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
            props.insert("~boundaryId".to_string(), json!("article-boundary"));
            props
        });

        serializer.serialize_element(&suspense).unwrap();

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
        props.insert("~boundaryId".to_string(), json!("test"));

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
        props.insert("~boundaryId".to_string(), json!("test"));

        let suspense = LoadingReactElement::with_props("react.suspense", props);

        let result = serializer.serialize_element(&suspense);
        assert!(result.is_err(), "Should error when children is missing");
        assert!(
            result.unwrap_err().to_string().contains("children"),
            "Error should mention missing children"
        );
    }

    #[test]
    fn test_serialize_typedarray_uint8() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "buffer".to_string(),
            json!({
                "$typedarray": {
                    "type": "Uint8Array",
                    "data": [1, 2, 3, 4, 5]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":o"), "Should contain Uint8Array tag");
    }

    #[test]
    fn test_serialize_typedarray_int32() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "data".to_string(),
            json!({
                "$typedarray": {
                    "type": "Int32Array",
                    "data": [100, 200, 300]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":L"), "Should contain Int32Array tag");
    }

    #[test]
    fn test_serialize_typedarray_float64() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "floats".to_string(),
            json!({
                "$typedarray": {
                    "type": "Float64Array",
                    "data": [1, 2, 3]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":g"), "Should contain Float64Array tag");
    }

    #[test]
    fn test_serialize_blob() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "file".to_string(),
            json!({
                "$blob": {
                    "type": "image/png",
                    "data": [137, 80, 78, 71]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$B"), "Should contain Blob reference");

        assert!(result.contains("image/png"), "Should contain blob type");
    }

    #[test]
    fn test_serialize_blob_default_type() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "data".to_string(),
            json!({
                "$blob": {
                    "data": [1, 2, 3, 4]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$B"), "Should contain Blob reference");

        assert!(result.contains("application/octet-stream"), "Should use default blob type");
    }

    #[test]
    fn test_serialize_readable_stream() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "stream".to_string(),
            json!({
                "$stream": {
                    "chunks": [
                        [1, 2, 3],
                        [4, 5, 6],
                        [7, 8, 9]
                    ]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":R"), "Should contain stream start marker");

        assert!(result.contains(":C"), "Should contain stream complete marker");

        assert!(result.contains("[1,2,3]"), "Should contain first chunk");
        assert!(result.contains("[4,5,6]"), "Should contain second chunk");
        assert!(result.contains("[7,8,9]"), "Should contain third chunk");
    }

    #[test]
    fn test_serialize_byte_stream() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "byteStream".to_string(),
            json!({
                "$stream": {
                    "byteStream": true,
                    "chunks": [
                        [65, 66, 67],
                        [68, 69, 70]
                    ]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":r"), "Should contain byte stream start marker");

        assert!(result.contains(":C"), "Should contain stream complete marker");
    }

    #[test]
    fn test_serialize_nested_binary_types() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "data".to_string(),
            json!({
                "buffer": {
                    "$typedarray": {
                        "type": "Uint8Array",
                        "data": [1, 2, 3]
                    }
                },
                "file": {
                    "$blob": {
                        "type": "text/plain",
                        "data": [72, 101, 108, 108, 111]
                    }
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain references");
        assert!(result.contains("$B"), "Should contain Blob reference");
        assert!(result.contains(":o"), "Should contain Uint8Array tag");
        assert!(result.contains("text/plain"), "Should contain blob type");
    }

    #[test]
    fn test_serialize_typedarray_all_types() {
        let types = vec![
            ("ArrayBuffer", "A"),
            ("Int8Array", "O"),
            ("Uint8Array", "o"),
            ("Uint8ClampedArray", "U"),
            ("Int16Array", "S"),
            ("Uint16Array", "s"),
            ("Int32Array", "L"),
            ("Uint32Array", "l"),
            ("Float32Array", "G"),
            ("Float64Array", "g"),
            ("BigInt64Array", "M"),
            ("BigUint64Array", "m"),
            ("DataView", "V"),
        ];

        for (type_name, expected_tag) in types {
            let mut serializer = RscSerializer::new();

            let mut props = FxHashMap::default();
            props.insert(
                "data".to_string(),
                json!({
                    "$typedarray": {
                        "type": type_name,
                        "data": [1, 2, 3]
                    }
                }),
            );

            let element = SerializedReactElement::create_html_element("div", Some(props));
            let result = serializer.serialize_to_rsc_format(&element);

            assert!(
                result.contains(&format!(":{}", expected_tag)),
                "Type {} should have tag {}",
                type_name,
                expected_tag
            );
        }
    }

    #[test]
    fn test_serialize_empty_stream() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "stream".to_string(),
            json!({
                "$stream": {
                    "chunks": []
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains(":R"), "Should contain stream start marker");
        assert!(result.contains(":C"), "Should contain stream complete marker");
    }

    #[test]
    fn test_serialize_binary_with_special_values() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "mixed".to_string(),
            json!({
                "nan": "$NaN",
                "buffer": {
                    "$typedarray": {
                        "type": "Uint8Array",
                        "data": [1, 2, 3]
                    }
                },
                "date": {
                    "$date": "2025-12-09T18:00:00.000Z"
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$NaN"), "Should contain NaN marker");
        assert!(result.contains("$D"), "Should contain Date marker");
        assert!(result.contains(":o"), "Should contain TypedArray tag");
    }
}
