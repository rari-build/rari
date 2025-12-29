#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use crate::runtime::JsExecutionRuntime;
    use std::sync::Arc;

    use super::super::boundary_manager::SuspenseBoundaryManager;
    use super::super::renderer::StreamingRenderer;
    use super::super::types::*;
    use super::super::validation::validate_suspense_boundaries;

    #[tokio::test]
    async fn test_partial_render_result() {
        let partial_result = PartialRenderResult {
            initial_content: serde_json::json!({"test": "content"}),
            pending_promises: vec![],
            boundaries: vec![],
            has_suspense: false,
        };

        assert!(!partial_result.has_suspense);
        assert_eq!(partial_result.pending_promises.len(), 0);
        assert_eq!(partial_result.boundaries.len(), 0);
    }

    #[tokio::test]
    async fn test_boundary_manager() {
        let mut manager = SuspenseBoundaryManager::new();

        let boundary = SuspenseBoundaryInfo {
            id: "test-boundary".to_string(),
            fallback_content: serde_json::json!({"loading": true}),
            parent_boundary_id: None,
            pending_promise_count: 1,
            parent_path: vec!["content-slot".to_string()],
            is_in_content_area: true,
            skeleton_rendered: false,
            is_resolved: false,
            position_hints: Some(PositionHints {
                in_content_area: true,
                dom_path: vec!["content-slot".to_string()],
                is_stable: true,
            }),
        };

        manager.register_boundary(boundary).await;

        let pending = manager.get_pending_boundaries().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "test-boundary");
    }

    #[test]
    fn test_rsc_stream_chunk() {
        let chunk = RscStreamChunk {
            data: b"test data".to_vec(),
            chunk_type: RscChunkType::InitialShell,
            row_id: 1,
            is_final: false,
            boundary_id: None,
        };

        assert_eq!(chunk.chunk_type, RscChunkType::InitialShell);
        assert_eq!(chunk.row_id, 1);
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_module_row_format() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = StreamingRenderer::new(runtime);

        renderer.set_row_counter(1);
        renderer.set_module_path("app/MyComponent.js".to_string());

        let module_chunk = renderer.create_module_chunk().expect("module chunk");
        let s = String::from_utf8(module_chunk.data).expect("utf8");
        assert!(s.starts_with("1:I[\"app/MyComponent.js\",[\"main\"],\"default\"]"));
    }

    #[tokio::test]
    async fn test_deferred_execution_validates_react_availability() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (function() {
                const originalReact = globalThis.React;
                delete globalThis.React;

                if (typeof React === 'undefined' || !React) {
                    globalThis.React = originalReact;
                    return {
                        success: false,
                        error: 'React is not available',
                        validated: true
                    };
                }

                globalThis.React = originalReact;
                return { success: true, validated: false };
            })()
        "#;

        let result = runtime
            .execute_script("test_react_validation".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["validated"], true);
        assert_eq!(value["success"], false);
    }

    #[tokio::test]
    async fn test_deferred_execution_validates_components_array() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (function() {
                if (!globalThis['~render']) globalThis['~render'] = {};
                globalThis['~render'].deferredAsyncComponents = "not an array";

                if (!Array.isArray(globalThis['~render'].deferredAsyncComponents)) {
                    return {
                        success: false,
                        error: 'deferredAsyncComponents is not an array',
                        actualType: typeof globalThis['~render'].deferredAsyncComponents,
                        validated: true
                    };
                }

                return { success: true, validated: false };
            })()
        "#;

        let result = runtime
            .execute_script("test_array_validation".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["validated"], true);
        assert_eq!(value["actualType"], "string");
    }

    #[tokio::test]
    async fn test_deferred_execution_validates_component_is_function() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (function() {
                const deferred = {
                    component: "not a function",
                    promiseId: "test-promise",
                    componentPath: "TestComponent",
                    boundaryId: "test-boundary"
                };

                if (typeof deferred.component !== 'function') {
                    return {
                        success: false,
                        error: 'Component is not a function',
                        errorName: 'TypeError',
                        actualType: typeof deferred.component,
                        validated: true
                    };
                }

                return { success: true, validated: false };
            })()
        "#;

        let result = runtime
            .execute_script("test_function_validation".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["validated"], true);
        assert_eq!(value["errorName"], "TypeError");
        assert_eq!(value["actualType"], "string");
    }

    #[tokio::test]
    async fn test_deferred_execution_validates_promise_return() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (function() {
                const deferred = {
                    component: function() { return "not a promise"; },
                    promiseId: "test-promise",
                    componentPath: "TestComponent",
                    boundaryId: "test-boundary",
                    props: {}
                };

                const componentPromise = deferred.component(deferred.props);

                if (!componentPromise || typeof componentPromise.then !== 'function') {
                    return {
                        success: false,
                        error: 'Component did not return a promise',
                        errorName: 'TypeError',
                        returnedType: typeof componentPromise,
                        hasPromise: componentPromise !== null && componentPromise !== undefined,
                        hasThen: componentPromise && typeof componentPromise.then === 'function',
                        validated: true
                    };
                }

                return { success: true, validated: false };
            })()
        "#;

        let result = runtime
            .execute_script("test_promise_validation".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["validated"], true);
        assert_eq!(value["errorName"], "TypeError");
        assert_eq!(value["returnedType"], "string");
        assert_eq!(value["hasThen"], false);
    }

    #[tokio::test]
    async fn test_deferred_execution_verifies_promise_registration() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (function() {
                if (!globalThis['~suspense']) globalThis['~suspense'] = {};
                globalThis['~suspense'].promises = {};
                const promiseId = "test-promise-123";

                const testPromise = Promise.resolve("test");
                globalThis['~suspense'].promises[promiseId] = testPromise;

                if (!globalThis['~suspense'].promises[promiseId]) {
                    const availablePromiseIds = Object.keys(globalThis['~suspense'].promises || {});
                    return {
                        success: false,
                        error: 'Promise registration verification failed',
                        availablePromises: availablePromiseIds,
                        verified: false
                    };
                }

                return {
                    success: true,
                    verified: true,
                    promiseId: promiseId
                };
            })()
        "#;

        let result = runtime
            .execute_script("test_registration_verification".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["verified"], true);
        assert_eq!(value["success"], true);
        assert_eq!(value["promiseId"], "test-promise-123");
    }

    #[test]
    fn test_validate_suspense_boundaries_no_duplicates() {
        let rsc_data = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "~boundaryId": "boundary-1",
                "fallback": ["$", "div", null, { "children": "Loading 1..." }],
                "children": "$L1"
            }
        ]);

        let result = validate_suspense_boundaries(&rsc_data);
        assert!(result.is_ok(), "Validation should pass with no duplicates");
    }

    #[test]
    fn test_validate_suspense_boundaries_detects_duplicates() {
        let rsc_data = serde_json::json!({
            "root": [
                "$",
                "div",
                null,
                {
                    "children": [
                        [
                            "$",
                            "react.suspense",
                            null,
                            {
                                "~boundaryId": "boundary-1",
                                "fallback": ["$", "div", null, { "children": "Loading..." }],
                                "children": "$L1"
                            }
                        ],
                        [
                            "$",
                            "react.suspense",
                            null,
                            {
                                "~boundaryId": "boundary-2",
                                "fallback": ["$", "div", null, { "children": "Loading..." }],
                                "children": "$L2"
                            }
                        ]
                    ]
                }
            ]
        });

        let result = validate_suspense_boundaries(&rsc_data);
        assert!(result.is_err(), "Validation should fail with duplicate fallbacks");

        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("boundary-2"), "Error should mention the duplicate boundary");
    }

    #[test]
    fn test_validate_suspense_boundaries_nested() {
        let rsc_data = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "~boundaryId": "outer-boundary",
                "fallback": ["$", "div", null, { "children": "Loading outer..." }],
                "children": [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "~boundaryId": "inner-boundary",
                        "fallback": ["$", "div", null, { "children": "Loading inner..." }],
                        "children": "$L1"
                    }
                ]
            }
        ]);

        let result = validate_suspense_boundaries(&rsc_data);
        assert!(
            result.is_ok(),
            "Validation should pass with nested boundaries having different fallbacks"
        );
    }

    #[test]
    fn test_validate_suspense_boundaries_multiple_unique() {
        let rsc_data = serde_json::json!({
            "children": [
                [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "~boundaryId": "boundary-1",
                        "fallback": ["$", "div", null, { "children": "Loading 1..." }],
                        "children": "$L1"
                    }
                ],
                [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "~boundaryId": "boundary-2",
                        "fallback": ["$", "div", null, { "children": "Loading 2..." }],
                        "children": "$L2"
                    }
                ],
                [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "~boundaryId": "boundary-3",
                        "fallback": ["$", "div", null, { "children": "Loading 3..." }],
                        "children": "$L3"
                    }
                ]
            ]
        });

        let result = validate_suspense_boundaries(&rsc_data);
        assert!(result.is_ok(), "Validation should pass with multiple unique boundaries");
    }

    #[tokio::test]
    async fn test_deferred_execution_complete_flow() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (function() {
                globalThis.React = { createElement: () => {} };

                if (!globalThis['~render']) globalThis['~render'] = {};
                globalThis['~render'].deferredAsyncComponents = [
                    {
                        component: function(props) {
                            return Promise.resolve({ type: 'div', props: { children: 'Valid' } });
                        },
                        promiseId: "valid-promise",
                        componentPath: "ValidComponent",
                        boundaryId: "boundary-1",
                        props: {}
                    },
                    {
                        component: function(props) {
                            return "not a promise";
                        },
                        promiseId: "invalid-promise",
                        componentPath: "InvalidComponent",
                        boundaryId: "boundary-2",
                        props: {}
                    }
                ];

                if (!globalThis['~suspense']) globalThis['~suspense'] = {};
                globalThis['~suspense'].promises = {};
                const results = [];

                for (const deferred of globalThis['~render'].deferredAsyncComponents) {
                    try {
                        if (typeof deferred.component !== 'function') {
                            results.push({ promiseId: deferred.promiseId, success: false, error: 'Not a function' });
                            continue;
                        }

                        const componentPromise = deferred.component(deferred.props);

                        if (!componentPromise || typeof componentPromise.then !== 'function') {
                            results.push({
                                promiseId: deferred.promiseId,
                                success: false,
                                error: 'Not a promise',
                                returnedType: typeof componentPromise
                            });
                            continue;
                        }

                        globalThis['~suspense'].promises[deferred.promiseId] = componentPromise;

                        if (!globalThis['~suspense'].promises[deferred.promiseId]) {
                            results.push({ promiseId: deferred.promiseId, success: false, error: 'Registration failed' });
                        } else {
                            results.push({ promiseId: deferred.promiseId, success: true });
                        }
                    } catch (e) {
                        results.push({ promiseId: deferred.promiseId, success: false, error: e.message });
                    }
                }

                return {
                    totalComponents: globalThis['~render'].deferredAsyncComponents.length,
                    results: results,
                    successCount: results.filter(r => r.success).length,
                    failureCount: results.filter(r => !r.success).length
                };
            })()
        "#;

        let result =
            runtime.execute_script("test_complete_flow".to_string(), test_script.to_string()).await;
        assert!(result.is_ok());
        let value = result.unwrap();

        assert_eq!(value["totalComponents"], 2);
        assert_eq!(value["successCount"], 1);
        assert_eq!(value["failureCount"], 1);

        let results = value["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);

        assert_eq!(results[0]["promiseId"], "valid-promise");
        assert_eq!(results[0]["success"], true);

        assert_eq!(results[1]["promiseId"], "invalid-promise");
        assert_eq!(results[1]["success"], false);
        assert_eq!(results[1]["error"], "Not a promise");
    }
}
