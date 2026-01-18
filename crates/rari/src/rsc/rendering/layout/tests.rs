#![allow(clippy::disallowed_methods)]

#[cfg(test)]
use super::*;
use crate::rsc::rendering::core::RscRenderer;
use crate::runtime::JsExecutionRuntime;
use crate::server::routing::app_router::AppRouteMatch;
use crate::server::routing::types::ParamValue;
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
    params.insert("id".to_string(), ParamValue::Single("123".to_string()));

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
    assert!(script.contains("const isAsync = PageComponent.constructor.name === 'AsyncFunction'"));
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
    assert!(script.contains("const isAsync = PageComponent.constructor.name === 'AsyncFunction'"));
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

    assert!(script_ssr.contains("AsyncComponentMarker._isAsyncComponent = true"));
    assert!(script_rsc.contains("AsyncComponentMarker._isAsyncComponent = true"));
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

    assert!(script_ssr.contains("AsyncComponentMarker._isAsyncComponent = true"));
    assert!(script_rsc.contains("AsyncComponentMarker._isAsyncComponent = true"));

    assert!(script_ssr.contains("React.createElement = function(type, props, ...children)"));
    assert!(script_rsc.contains("React.createElement = function(type, props, ...children)"));
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

    assert!(script_ssr.contains("boundaries: globalThis['~suspense']?.discoveredBoundaries"));
    assert!(script_rsc.contains("boundaries: globalThis['~suspense']?.discoveredBoundaries"));

    assert!(script_ssr.contains("pending_promises: globalThis['~suspense']?.pendingPromises"));
    assert!(script_rsc.contains("pending_promises: globalThis['~suspense']?.pendingPromises"));

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
    assert!(
        script_ssr.contains("const isAsync = PageComponent.constructor.name === 'AsyncFunction'")
    );

    assert!(script_rsc.contains("const useSuspense = false"));
    assert!(
        script_rsc.contains("const isAsync = PageComponent.constructor.name === 'AsyncFunction'")
    );
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

    assert!(script_ssr.contains("AsyncComponentMarker._isAsyncComponent = true"));
    assert!(script_rsc.contains("AsyncComponentMarker._isAsyncComponent = true"));
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

    assert!(script_ssr.contains("React.createElement"));
    assert!(script_rsc.contains("React.createElement"));

    assert!(script_ssr.contains("PageComponent"));
    assert!(script_rsc.contains("PageComponent"));
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

    assert!(script_ssr.contains("AsyncComponentMarker"));
    assert!(script_rsc.contains("AsyncComponentMarker"));
}
