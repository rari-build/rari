#[cfg(test)]
use super::*;
use crate::rsc::rendering::core::RscRenderer;
use crate::runtime::JsExecutionRuntime;
use crate::server::routing::app_router::AppRouteMatch;
use rustc_hash::FxHashMap;
use std::sync::Arc;

#[test]
fn test_get_component_id() {
    assert_eq!(utils::get_component_id("app/page.tsx"), "Page");
    assert_eq!(utils::get_component_id("app/layout.tsx"), "Layout");
    assert_eq!(utils::get_component_id("app/loading.tsx"), "Loading");
    assert_eq!(utils::get_component_id("app/error.tsx"), "Error");
}

#[test]
fn test_create_page_props() {
    let mut params = FxHashMap::default();
    params.insert("id".to_string(), "123".to_string());

    let mut search_params = FxHashMap::default();
    search_params.insert("q".to_string(), vec!["test".to_string()]);

    let context = LayoutRenderContext {
        params: params.clone(),
        search_params: search_params.clone(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params,
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let props = utils::create_page_props(&route_match, &context).unwrap();
    assert!(props.get("params").is_some());
    assert!(props.get("searchParams").is_some());
}

#[test]
fn test_wrapped_html_error_message_contains_key_info() {
    use crate::server::routing::app_router::{AppRouteEntry, LayoutEntry};

    let route_match = AppRouteMatch {
        route: AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![LayoutEntry {
            path: "/".to_string(),
            file_path: "app/layout.tsx".to_string(),
            parent_path: None,
            is_root: true,
        }],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let error_msg =
        error_messages::create_wrapped_html_error_message(&route_match, Some("app/layout.tsx"));

    assert!(error_msg.contains("Hydration Mismatch"));
    assert!(error_msg.contains("app/layout.tsx"));
    assert!(error_msg.contains("/test"));
    assert!(error_msg.contains("HOW TO FIX"));
    assert!(error_msg.contains("CORRECT STRUCTURE"));
    assert!(error_msg.contains("INCORRECT STRUCTURE"));
    assert!(error_msg.contains("Step 1"));
    assert!(error_msg.contains("ROOT"));
}

#[test]
fn test_empty_rsc_error_message_contains_key_info() {
    let error_msg = error_messages::create_empty_rsc_error_message();

    assert!(error_msg.contains("Empty Content"));
    assert!(error_msg.contains("COMMON CAUSES"));
    assert!(error_msg.contains("HOW TO FIX"));
    assert!(error_msg.contains("VALID COMPONENT EXAMPLES"));
    assert!(error_msg.contains("export default"));
    assert!(error_msg.contains("Step 1"));
    assert!(error_msg.contains("DEBUGGING CHECKLIST"));
}

#[test]
fn test_invalid_rsc_format_warning_contains_key_info() {
    let warning_msg = error_messages::create_invalid_rsc_format_warning(
        "missing row ID",
        "Expected numeric row ID at start",
    );

    assert!(warning_msg.contains("Invalid Structure"));
    assert!(warning_msg.contains("missing row ID"));
    assert!(warning_msg.contains("Expected numeric row ID at start"));
    assert!(warning_msg.contains("RSC WIRE FORMAT SPECIFICATION"));
    assert!(warning_msg.contains("VALID RSC WIRE FORMAT EXAMPLES"));
    assert!(warning_msg.contains("TROUBLESHOOTING STEPS"));
    assert!(warning_msg.contains("Step 1"));
}

#[test]
fn test_detect_suspense_boundaries_no_suspense() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let rsc_json = serde_json::json!([
        "$",
        "div",
        null,
        {
            "children": "Hello World"
        }
    ]);

    let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
    assert!(!result.has_suspense);
    assert_eq!(result.boundary_count, 0);
    assert_eq!(result.boundaries.len(), 0);
}

#[test]
fn test_detect_suspense_boundaries_with_suspense() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let rsc_json = serde_json::json!([
        "$",
        "react.suspense",
        null,
        {
            "~boundaryId": "test_boundary_123",
            "fallback": ["$", "div", null, {"children": "Loading..."}],
            "children": ["$", "div", null, {"children": "Content"}]
        }
    ]);

    let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
    assert!(result.has_suspense);
    assert_eq!(result.boundary_count, 1);
    assert_eq!(result.boundaries.len(), 1);
    assert_eq!(result.boundaries[0].id, "test_boundary_123");
    assert!(result.boundaries[0].has_fallback);
}

#[test]
fn test_detect_suspense_boundaries_nested() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let rsc_json = serde_json::json!([
        "$",
        "div",
        null,
        {
            "children": [
                "$",
                "react.suspense",
                null,
                {
                    "~boundaryId": "nested_boundary",
                    "fallback": ["$", "span", null, {"children": "Loading..."}]
                }
            ]
        }
    ]);

    let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
    assert!(result.has_suspense);
    assert_eq!(result.boundary_count, 1);
    assert_eq!(result.boundaries[0].id, "nested_boundary");
}

#[test]
fn test_detect_suspense_boundaries_pre_serialized() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let rsc_json = serde_json::json!({
        "~preSerializedSuspense": true,
        "rscArray": [
            "$",
            "react.suspense",
            null,
            {
                "~boundaryId": "pre_serialized_boundary",
                "fallback": ["$", "div", null, {"children": "Loading..."}]
            }
        ]
    });

    let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
    assert!(result.has_suspense);
    assert_eq!(result.boundary_count, 1);
    assert_eq!(result.boundaries[0].id, "pre_serialized_boundary");
}

#[test]
fn test_detect_suspense_boundaries_multiple() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let rsc_json = serde_json::json!([
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
                        "~boundaryId": "boundary_1",
                        "fallback": ["$", "div", null, {"children": "Loading 1..."}]
                    }
                ],
                [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "~boundaryId": "boundary_2",
                        "fallback": ["$", "div", null, {"children": "Loading 2..."}]
                    }
                ]
            ]
        }
    ]);

    let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
    assert!(result.has_suspense);
    assert_eq!(result.boundary_count, 2);
    assert_eq!(result.boundaries.len(), 2);
    assert_eq!(result.boundaries[0].id, "boundary_1");
    assert_eq!(result.boundaries[1].id, "boundary_2");
}

#[test]
fn test_detect_suspense_boundaries_alternative_tag() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let rsc_json = serde_json::json!([
        "$",
        "Suspense",
        null,
        {
            "~boundaryId": "alt_boundary",
            "fallback": ["$", "div", null, {"children": "Loading..."}]
        }
    ]);

    let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
    assert!(result.has_suspense);
    assert_eq!(result.boundary_count, 1);
    assert_eq!(result.boundaries[0].id, "alt_boundary");
}

#[test]
fn test_build_composition_script_with_use_suspense_true() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script = renderer
        .build_composition_script(&route_match, &context, Some("app/test/loading"), true)
        .unwrap();

    assert!(script.contains("const useSuspense = true"));
    assert!(script.contains("if (isAsync && useSuspense)"));
}

#[test]
fn test_build_composition_script_with_use_suspense_false() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script = renderer
        .build_composition_script(&route_match, &context, Some("app/test/loading"), false)
        .unwrap();

    assert!(script.contains("const useSuspense = false"));
    assert!(script.contains("else if (isAsync && !useSuspense)"));
}

#[test]
fn test_layout_structure_new() {
    let layout_structure = LayoutStructure::new();
    assert!(!layout_structure.has_navigation);
    assert!(layout_structure.navigation_position.is_none());
    assert!(layout_structure.content_position.is_none());
    assert_eq!(layout_structure.suspense_boundaries.len(), 0);
}

#[test]
fn test_layout_structure_is_valid_empty() {
    let layout_structure = LayoutStructure::new();
    assert!(layout_structure.is_valid());
}

#[test]
fn test_layout_structure_is_valid_navigation_before_content() {
    let layout_structure = LayoutStructure {
        has_navigation: true,
        navigation_position: Some(0),
        content_position: Some(1),
        suspense_boundaries: Vec::new(),
    };
    assert!(layout_structure.is_valid());
}

#[test]
fn test_layout_structure_is_invalid_navigation_after_content() {
    let layout_structure = LayoutStructure {
        has_navigation: true,
        navigation_position: Some(1),
        content_position: Some(0),
        suspense_boundaries: Vec::new(),
    };
    assert!(!layout_structure.is_valid());
}

#[test]
fn test_layout_structure_is_invalid_boundary_outside_content() {
    let layout_structure = LayoutStructure {
        has_navigation: true,
        navigation_position: Some(0),
        content_position: Some(1),
        suspense_boundaries: vec![BoundaryPosition {
            boundary_id: "test_boundary".to_string(),
            parent_path: vec![0],
            is_in_content_area: false,
            dom_path: vec![0],
        }],
    };
    assert!(!layout_structure.is_valid());
}

#[test]
fn test_layout_structure_is_valid_boundary_in_content() {
    let layout_structure = LayoutStructure {
        has_navigation: true,
        navigation_position: Some(0),
        content_position: Some(1),
        suspense_boundaries: vec![BoundaryPosition {
            boundary_id: "test_boundary".to_string(),
            parent_path: vec![1, 0],
            is_in_content_area: true,
            dom_path: vec![1, 1, 0],
        }],
    };
    assert!(layout_structure.is_valid());
}

#[test]
fn test_validate_layout_structure_no_navigation() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let rsc_json = serde_json::json!([
        "$",
        "div",
        null,
        {
            "children": "Hello World"
        }
    ]);

    let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
    assert!(!result.has_navigation);
    assert!(result.navigation_position.is_none());
    assert!(result.is_valid());
}

#[test]
fn test_validate_layout_structure_with_navigation() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let rsc_json = serde_json::json!([
        "$",
        "div",
        null,
        {
            "children": [
                ["$", "nav", null, {"children": "Navigation"}],
                ["$", "main", null, {"children": "Content"}]
            ]
        }
    ]);

    let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
    assert!(result.has_navigation);
    assert!(result.navigation_position.is_some());
    assert!(result.content_position.is_some());
    assert!(result.is_valid());
}

#[test]
fn test_validate_layout_structure_with_suspense_in_content() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let rsc_json = serde_json::json!([
        "$",
        "div",
        null,
        {
            "children": [
                ["$", "nav", null, {"children": "Navigation"}],
                [
                    "$",
                    "main",
                    null,
                    {
                        "children": [
                            "$",
                            "react.suspense",
                            null,
                            {
                                "~boundaryId": "test_boundary",
                                "fallback": ["$", "div", null, {"children": "Loading..."}]
                            }
                        ]
                    }
                ]
            ]
        }
    ]);

    let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
    assert!(result.has_navigation);
    assert_eq!(result.suspense_boundaries.len(), 1);
    assert_eq!(result.suspense_boundaries[0].boundary_id, "test_boundary");
    assert!(result.suspense_boundaries[0].is_in_content_area);
    assert!(result.is_valid());
}

#[test]
fn test_validate_layout_structure_detects_header_with_navigation_role() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let rsc_json = serde_json::json!([
        "$",
        "div",
        null,
        {
            "children": [
                ["$", "header", null, {"role": "navigation", "children": "Header Nav"}],
                ["$", "main", null, {"children": "Content"}]
            ]
        }
    ]);

    let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
    assert!(result.has_navigation);
    assert!(result.navigation_position.is_some());
    assert!(result.is_valid());
}

#[test]
fn test_validate_layout_structure_detects_section_with_main_role() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let rsc_json = serde_json::json!([
        "$",
        "div",
        null,
        {
            "children": [
                ["$", "nav", null, {"children": "Navigation"}],
                ["$", "section", null, {"role": "main", "children": "Main Content"}]
            ]
        }
    ]);

    let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
    assert!(result.has_navigation);
    assert!(result.content_position.is_some());
    assert!(result.is_valid());
}

#[test]
fn test_composition_script_includes_layout_structure_markers() {
    use crate::server::routing::app_router::LayoutEntry;

    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![LayoutEntry {
            path: "/".to_string(),
            file_path: "app/layout.tsx".to_string(),
            parent_path: None,
            is_root: true,
        }],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script = renderer.build_composition_script(&route_match, &context, None, true).unwrap();

    assert!(!script.contains("'data-content-slot': true"));
    assert!(!script.contains("const contentSlot = React.createElement"));
    assert!(!script.contains("'data-layout-root': true"));
    assert!(!script.contains("const layoutRoot = React.createElement"));
}

#[test]
fn test_calculate_boundary_positions_with_navigation() {
    let layout_structure = LayoutStructure {
        has_navigation: true,
        navigation_position: Some(0),
        content_position: Some(1),
        suspense_boundaries: vec![
            BoundaryPosition {
                boundary_id: "boundary1".to_string(),
                parent_path: vec![0],
                is_in_content_area: true,
                dom_path: Vec::new(),
            },
            BoundaryPosition {
                boundary_id: "boundary2".to_string(),
                parent_path: vec![1, 2],
                is_in_content_area: true,
                dom_path: Vec::new(),
            },
        ],
    };

    let positions = utils::calculate_boundary_positions(&layout_structure);

    assert_eq!(positions.get("boundary1"), Some(&vec![1, 0]));
    assert_eq!(positions.get("boundary2"), Some(&vec![1, 1, 2]));
}

#[test]
fn test_calculate_boundary_positions_without_navigation() {
    let layout_structure = LayoutStructure {
        has_navigation: false,
        navigation_position: None,
        content_position: Some(0),
        suspense_boundaries: vec![
            BoundaryPosition {
                boundary_id: "boundary1".to_string(),
                parent_path: vec![0],
                is_in_content_area: true,
                dom_path: Vec::new(),
            },
            BoundaryPosition {
                boundary_id: "boundary2".to_string(),
                parent_path: vec![1, 2],
                is_in_content_area: true,
                dom_path: Vec::new(),
            },
        ],
    };

    let positions = utils::calculate_boundary_positions(&layout_structure);

    assert_eq!(positions.get("boundary1"), Some(&vec![0, 0]));
    assert_eq!(positions.get("boundary2"), Some(&vec![0, 1, 2]));
}

#[test]
fn test_calculate_boundary_positions_outside_content_area() {
    let layout_structure = LayoutStructure {
        has_navigation: true,
        navigation_position: Some(0),
        content_position: Some(1),
        suspense_boundaries: vec![BoundaryPosition {
            boundary_id: "boundary1".to_string(),
            parent_path: vec![0, 1],
            is_in_content_area: false,
            dom_path: Vec::new(),
        }],
    };

    let positions = utils::calculate_boundary_positions(&layout_structure);

    assert_eq!(positions.get("boundary1"), Some(&vec![0, 1]));
}

#[test]
fn test_mode_consistency_both_modes_generate_render_to_rsc() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script_ssr = renderer.build_composition_script(&route_match, &context, None, true).unwrap();
    let script_rsc =
        renderer.build_composition_script(&route_match, &context, None, false).unwrap();

    assert!(script_ssr.contains("globalThis.renderToRsc"));
    assert!(script_rsc.contains("globalThis.renderToRsc"));

    assert!(script_ssr.contains("~preSerializedSuspense"));
    assert!(script_rsc.contains("~preSerializedSuspense"));
}

#[test]
fn test_mode_consistency_suspense_serialization_format() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script_ssr = renderer.build_composition_script(&route_match, &context, None, true).unwrap();
    let script_rsc =
        renderer.build_composition_script(&route_match, &context, None, false).unwrap();

    assert!(script_ssr.contains(r#"['$', 'react.suspense', uniqueKey, rscProps]"#));
    assert!(script_rsc.contains(r#"['$', 'react.suspense', uniqueKey, rscProps]"#));

    assert!(script_ssr.contains("'~boundaryId': props['~boundaryId']"));
    assert!(script_rsc.contains("'~boundaryId': props['~boundaryId']"));
}

#[test]
fn test_mode_consistency_metadata_structure() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script_ssr = renderer.build_composition_script(&route_match, &context, None, true).unwrap();
    let script_rsc =
        renderer.build_composition_script(&route_match, &context, None, false).unwrap();

    assert!(script_ssr.contains("rsc_data: rscData"));
    assert!(script_rsc.contains("rsc_data: rscData"));

    assert!(script_ssr.contains("boundaries: globalThis.__discovered_boundaries"));
    assert!(script_rsc.contains("boundaries: globalThis.__discovered_boundaries"));

    assert!(script_ssr.contains("pending_promises: globalThis.__pending_promises"));
    assert!(script_rsc.contains("pending_promises: globalThis.__pending_promises"));

    assert!(script_ssr.contains("metadata: {"));
    assert!(script_rsc.contains("metadata: {"));

    assert!(script_ssr.contains("hasAsync: hasAsync"));
    assert!(script_rsc.contains("hasAsync: hasAsync"));

    assert!(script_ssr.contains("deferredCount: deferredCount"));
    assert!(script_rsc.contains("deferredCount: deferredCount"));
}

#[test]
fn test_mode_consistency_async_component_handling_with_loading() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script_ssr = renderer
        .build_composition_script(&route_match, &context, Some("app/test/loading"), true)
        .unwrap();
    let script_rsc = renderer
        .build_composition_script(&route_match, &context, Some("app/test/loading"), false)
        .unwrap();

    assert!(script_ssr.contains("const useSuspense = true"));
    assert!(script_ssr.contains("if (isAsync && useSuspense)"));
    assert!(script_ssr.contains("'~preSerializedSuspense': true"));

    assert!(script_rsc.contains("const useSuspense = false"));
    assert!(script_rsc.contains("else if (isAsync && !useSuspense)"));
}

#[test]
fn test_mode_consistency_boundary_id_format() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script_ssr = renderer
        .build_composition_script(&route_match, &context, Some("app/test/loading"), true)
        .unwrap();
    let script_rsc = renderer
        .build_composition_script(&route_match, &context, Some("app/test/loading"), false)
        .unwrap();

    assert!(script_ssr.contains("const boundaryId = `page_boundary_"));
    assert!(script_rsc.contains("const boundaryId = `page_boundary_"));

    assert!(script_ssr.contains("const componentPathHash ="));
    assert!(script_rsc.contains("const componentPathHash ="));
}

#[test]
fn test_mode_consistency_wrapper_elements() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script_ssr = renderer.build_composition_script(&route_match, &context, None, true).unwrap();
    let script_rsc =
        renderer.build_composition_script(&route_match, &context, None, false).unwrap();

    assert!(!script_ssr.contains("const contentSlot = React.createElement"));
    assert!(!script_ssr.contains("'data-content-slot': true"));
    assert!(!script_rsc.contains("const contentSlot = React.createElement"));
    assert!(!script_rsc.contains("'data-content-slot': true"));

    assert_eq!(
        script_ssr.contains("const contentSlot"),
        script_rsc.contains("const contentSlot"),
        "Both modes should have same contentSlot behavior"
    );
}

#[test]
fn test_mode_consistency_error_handling() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script_ssr = renderer.build_composition_script(&route_match, &context, None, true).unwrap();
    let script_rsc =
        renderer.build_composition_script(&route_match, &context, None, false).unwrap();

    assert!(script_ssr.contains("console.error('Error rendering function component:', error)"));
    assert!(script_rsc.contains("console.error('Error rendering function component:', error)"));

    assert!(script_ssr.contains(r#"children: `Error: ${error.message}`"#));
    assert!(script_rsc.contains(r#"children: `Error: ${error.message}`"#));

    assert!(
        script_ssr.contains("style: { color: 'red', border: '1px solid red', padding: '10px' }")
    );
    assert!(
        script_rsc.contains("style: { color: 'red', border: '1px solid red', padding: '10px' }")
    );
}

#[test]
fn test_mode_consistency_rsc_props_cleanup() {
    let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
        Arc::new(JsExecutionRuntime::new(None)),
    ))));

    let route_match = AppRouteMatch {
        route: crate::server::routing::app_router::AppRouteEntry {
            path: "/test".to_string(),
            file_path: "app/test/page.tsx".to_string(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        },
        params: FxHashMap::default(),
        layouts: vec![],
        loading: None,
        error: None,
        not_found: None,
        pathname: "/test".to_string(),
    };

    let context = LayoutRenderContext {
        params: FxHashMap::default(),
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: "/test".to_string(),
        metadata: None,
    };

    let script_ssr = renderer.build_composition_script(&route_match, &context, None, true).unwrap();
    let script_rsc =
        renderer.build_composition_script(&route_match, &context, None, false).unwrap();

    assert!(script_ssr.contains("if (rscProps.fallback === null)"));
    assert!(script_ssr.contains("delete rscProps.fallback"));
    assert!(script_rsc.contains("if (rscProps.fallback === null)"));
    assert!(script_rsc.contains("delete rscProps.fallback"));

    assert!(script_ssr.contains("if (rscProps.children === null)"));
    assert!(script_ssr.contains("delete rscProps.children"));
    assert!(script_rsc.contains("if (rscProps.children === null)"));
    assert!(script_rsc.contains("delete rscProps.children"));

    assert!(script_ssr.contains("if (!rscProps['~boundaryId'])"));
    assert!(script_ssr.contains("delete rscProps['~boundaryId']"));
    assert!(script_rsc.contains("if (!rscProps['~boundaryId'])"));
    assert!(script_rsc.contains("delete rscProps['~boundaryId']"));

    assert!(script_ssr.contains("if (rscProps.children === undefined)"));
    assert!(script_rsc.contains("if (rscProps.children === undefined)"));
}
