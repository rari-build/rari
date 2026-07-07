mod core;
mod route_composer;
pub mod types;
mod utils;

pub use core::{LayoutHtmlCache, LayoutRenderer};

pub use route_composer::{LayoutInfo, RouteComposer};
pub use types::*;
pub(crate) use utils::{component_dist_path, drain_chunked_stream};
pub use utils::{
    create_component_id, create_layout_context, generate_cache_key, sort_flight_protocol,
};

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use rustc_hash::FxHashMap;
    use tokio::sync::Mutex;

    use super::*;
    use crate::{
        rendering::base::RscRenderer,
        runtime::JsExecutionRuntime,
        server::routing::{
            app_router::{AppRouteEntry, AppRouteMatch, LayoutEntry},
            types::ParamValue,
        },
    };

    #[test]
    fn test_get_component_id() {
        assert_eq!(utils::get_component_id("app/page.tsx"), "Page");
        assert_eq!(utils::get_component_id("app/layout.tsx"), "Layout");
        assert_eq!(utils::get_component_id("app/loading.tsx"), "Loading");
        assert_eq!(utils::get_component_id("app/error.tsx"), "Error");
    }

    #[test]
    fn test_create_component_id_includes_stable_hash_suffix() {
        assert_eq!(utils::create_component_id("page.tsx"), "app/page_73d7a23e");
        assert_eq!(utils::create_component_id("css/page.tsx"), "app/css/page_1a52d086");

        let at_path = utils::create_component_id("foo@bar/page.tsx");
        let hash_path = utils::create_component_id("foo#bar/page.tsx");

        assert_eq!(at_path, "app/foo_bar/page_e35d0d78");
        assert_eq!(hash_path, "app/foo_bar/page_9744e5ac");
        assert_ne!(at_path, hash_path);
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
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params,
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            templates: vec![],
            pathname: "/test".to_string(),
        };

        let props = utils::create_page_props(&route_match, &context).unwrap();
        assert!(props.get("params").is_some());
        assert!(props.get("searchParams").is_some());
    }

    #[test]
    fn test_build_composition_script_with_use_suspense_true() {
        let renderer = LayoutRenderer::new(Arc::new(Mutex::new(RscRenderer::new(Arc::new(
            JsExecutionRuntime::new(None),
        )))));

        let route_match = AppRouteMatch {
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            templates: vec![],
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
            .build_composition_script(&route_match, &context, Some("app/test/loading"), true, false)
            .unwrap();

        assert!(script.contains("const useSuspense = true"));
        assert!(
            script.contains("const isAsync = PageComponent.constructor.name === 'AsyncFunction'")
        );
    }

    #[test]
    fn test_build_composition_script_with_use_suspense_false() {
        let renderer = LayoutRenderer::new(Arc::new(Mutex::new(RscRenderer::new(Arc::new(
            JsExecutionRuntime::new(None),
        )))));

        let route_match = AppRouteMatch {
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            templates: vec![],
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
            .build_composition_script(
                &route_match,
                &context,
                Some("app/test/loading"),
                false,
                false,
            )
            .unwrap();

        assert!(script.contains("const useSuspense = false"));
        assert!(
            script.contains("const isAsync = PageComponent.constructor.name === 'AsyncFunction'")
        );
    }

    #[test]
    fn test_composition_script_includes_layout_structure_markers() {
        let renderer = LayoutRenderer::new(Arc::new(Mutex::new(RscRenderer::new(Arc::new(
            JsExecutionRuntime::new(None),
        )))));

        let route_match = AppRouteMatch {
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params: FxHashMap::default(),
            layouts: vec![LayoutEntry {
                path: "/".to_string(),
                file_path: "app/layout.tsx".to_string(),
                component_id: None,
                css: vec![],
                parent_path: None,
                additional_paths: None,
                is_root: true,
            }],
            loading: None,
            error: None,
            not_found: None,
            templates: vec![],
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
            metadata: None,
        };

        let script =
            renderer.build_composition_script(&route_match, &context, None, true, false).unwrap();

        assert!(!script.contains("'data-content-slot': true"));
        assert!(!script.contains("const contentSlot = React.createElement"));
        assert!(!script.contains("'data-layout-root': true"));
        assert!(!script.contains("const layoutRoot = React.createElement"));
    }

    #[test]
    fn test_mode_consistency_metadata_structure() {
        let renderer = LayoutRenderer::new(Arc::new(Mutex::new(RscRenderer::new(Arc::new(
            JsExecutionRuntime::new(None),
        )))));

        let route_match = AppRouteMatch {
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            templates: vec![],
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
            metadata: None,
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true, false).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false, false).unwrap();

        assert!(script_ssr.contains("rsc_data: rscData"));
        assert!(script_rsc.contains("rsc_data: rscData"));

        assert!(script_ssr.contains("boundaries: globalThis['~suspense']?.discoveredBoundaries"));
        assert!(script_rsc.contains("boundaries: globalThis['~suspense']?.discoveredBoundaries"));

        assert!(script_ssr.contains("pending_promises: globalThis['~suspense']?.pendingPromises"));
        assert!(script_rsc.contains("pending_promises: globalThis['~suspense']?.pendingPromises"));

        assert!(script_ssr.contains("metadata: {"));
        assert!(script_rsc.contains("metadata: {"));
    }

    #[test]
    fn test_mode_consistency_async_component_handling_with_loading() {
        let renderer = LayoutRenderer::new(Arc::new(Mutex::new(RscRenderer::new(Arc::new(
            JsExecutionRuntime::new(None),
        )))));

        let route_match = AppRouteMatch {
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            templates: vec![],
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
            .build_composition_script(&route_match, &context, Some("app/test/loading"), true, false)
            .unwrap();
        let script_rsc = renderer
            .build_composition_script(
                &route_match,
                &context,
                Some("app/test/loading"),
                false,
                false,
            )
            .unwrap();

        assert!(script_ssr.contains("const useSuspense = true"));
        assert!(
            script_ssr
                .contains("const isAsync = PageComponent.constructor.name === 'AsyncFunction'")
        );

        assert!(script_rsc.contains("const useSuspense = false"));
        assert!(
            script_rsc
                .contains("const isAsync = PageComponent.constructor.name === 'AsyncFunction'")
        );
    }

    #[test]
    fn test_mode_consistency_wrapper_elements() {
        let renderer = LayoutRenderer::new(Arc::new(Mutex::new(RscRenderer::new(Arc::new(
            JsExecutionRuntime::new(None),
        )))));

        let route_match = AppRouteMatch {
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            templates: vec![],
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
            metadata: None,
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true, false).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false, false).unwrap();

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
        let renderer = LayoutRenderer::new(Arc::new(Mutex::new(RscRenderer::new(Arc::new(
            JsExecutionRuntime::new(None),
        )))));

        let route_match = AppRouteMatch {
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            templates: vec![],
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
            metadata: None,
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true, false).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false, false).unwrap();

        assert!(script_ssr.contains("React.createElement"));
        assert!(script_rsc.contains("React.createElement"));

        assert!(script_ssr.contains("PageComponent"));
        assert!(script_rsc.contains("PageComponent"));
    }
}
