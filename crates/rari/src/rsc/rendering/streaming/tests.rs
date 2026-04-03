#[cfg(test)]
#[allow(clippy::disallowed_methods)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::boundary_manager::SuspenseBoundaryManager;
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
}
