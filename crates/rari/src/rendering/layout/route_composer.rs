use serde_json::{Value, json};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LayoutInfo {
    pub component_id: String,
    pub is_root: bool,
    pub file_path: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct TemplateInfo {
    pub component_id: String,
    pub file_path: String,
}

#[derive(Debug, Clone)]
pub struct ErrorBoundaryInfo {
    pub component_id: String,
    pub file_path: String,
}

#[non_exhaustive]
pub struct RouteComposer;

impl RouteComposer {
    pub fn build_composition_script(
        page_render_script: &str,
        layouts: &[LayoutInfo],
        pathname_json: &str,
    ) -> String {
        Self::build_composition_script_with_error(
            page_render_script,
            layouts,
            pathname_json,
            None,
            "{}",
        )
    }

    pub fn build_composition_script_with_error(
        page_render_script: &str,
        layouts: &[LayoutInfo],
        pathname_json: &str,
        error_boundary: Option<&ErrorBoundaryInfo>,
        metadata_json: &str,
    ) -> String {
        Self::build_composition_script_with_templates(
            page_render_script,
            layouts,
            &[],
            pathname_json,
            pathname_json,
            error_boundary,
            metadata_json,
            false,
            pathname_json,
            None,
            true,
            &[],
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "composition script needs all route render inputs"
    )]
    pub fn build_composition_script_with_templates(
        page_render_script: &str,
        layouts: &[LayoutInfo],
        templates: &[TemplateInfo],
        pathname_json: &str,
        template_key_json: &str,
        error_boundary: Option<&ErrorBoundaryInfo>,
        metadata_json: &str,
        defer_rsc: bool,
        action_post_url_json: &str,
        capture_stream_id: Option<&str>,
        expand_root_layout: bool,
        reuse_layout_paths: &[String],
    ) -> String {
        let layouts_json = Self::layouts_json(layouts);
        let templates_json = Self::templates_json(templates);
        let error_component_id_json =
            serde_json::to_string(error_boundary.map(|b| b.component_id.as_str()).unwrap_or(""))
                .unwrap_or_else(|_| "\"\"".to_string());
        let reuse_paths_json =
            serde_json::to_string(reuse_layout_paths).unwrap_or_else(|_| "[]".to_string());
        let capture_stream_id_json = match capture_stream_id {
            Some(id) => serde_json::to_string(id).unwrap_or_else(|_| "null".to_string()),
            None => "null".to_string(),
        };
        let defer_rsc_js = if defer_rsc { "true" } else { "false" };
        let expand_root_js = if expand_root_layout { "true" } else { "false" };

        format!(
            r"
            (async () => {{
                const timings = {{}};
                const startTotal = performance.now();

                if (!globalThis['~rari']) globalThis['~rari'] = {{}};
                if (typeof globalThis['~rari'].composeRoute !== 'function') {{
                    throw new Error('[rari] composeRoute not loaded');
                }}
                if (typeof globalThis['~rari'].createPageElement !== 'function') {{
                    throw new Error('[rari] createPageElement not loaded');
                }}

                const startPageRender = performance.now();
                {page_render_script}

                return await globalThis['~rari'].composeRoute({{
                    pageElement,
                    layouts: {layouts_json},
                    templates: {templates_json},
                    pathname: {pathname_json},
                    templateKey: {template_key_json},
                    errorComponentId: {error_component_id_json},
                    metadata: {metadata_json},
                    deferRsc: {defer_rsc_js},
                    captureStreamId: {capture_stream_id_json},
                    expandRootLayout: {expand_root_js},
                    reuseLayoutPaths: {reuse_paths_json},
                    actionPostUrl: {action_post_url_json},
                    timings,
                    startTotal,
                }});
            }})()
            "
        )
    }

    fn layouts_json(layouts: &[LayoutInfo]) -> String {
        let value: Vec<Value> = layouts
            .iter()
            .map(|layout| {
                json!({
                    "componentId": layout.component_id,
                    "isRoot": layout.is_root,
                    "filePath": layout.file_path,
                    "path": layout.path,
                })
            })
            .collect();
        serde_json::to_string(&value).unwrap_or_else(|_| "[]".to_string())
    }

    fn templates_json(templates: &[TemplateInfo]) -> String {
        let value: Vec<Value> = templates
            .iter()
            .map(|template| {
                json!({
                    "componentId": template.component_id,
                    "filePath": template.file_path,
                    "ssrModuleKey": super::utils::normalize_route_component_path(&template.file_path),
                })
            })
            .collect();
        serde_json::to_string(&value).unwrap_or_else(|_| "[]".to_string())
    }
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::rendering::base::constants::ROUTE_COMPOSER_SCRIPT;

    #[test]
    fn test_layout_info_creation() {
        let layout = LayoutInfo {
            component_id: "Layout".to_string(),
            is_root: true,
            file_path: "app/layout.tsx".to_string(),
            path: "/".to_string(),
        };
        assert_eq!(layout.component_id, "Layout");
        assert!(layout.is_root);
        assert_eq!(layout.file_path, "app/layout.tsx");
    }

    #[test]
    fn test_layout_info_clone() {
        let layout = LayoutInfo {
            component_id: "Layout".to_string(),
            is_root: false,
            file_path: "app/layout.tsx".to_string(),
            path: "/".to_string(),
        };
        let cloned = layout.clone();
        assert_eq!(layout.component_id, cloned.component_id);
        assert_eq!(layout.is_root, cloned.is_root);
        assert_eq!(layout.file_path, cloned.file_path);
    }

    #[test]
    fn test_build_composition_script_no_layouts() {
        let script =
            RouteComposer::build_composition_script("const pageElement = Page();", &[], "\"/\"");

        assert!(script.contains("const pageElement = Page();"));
        assert!(script.contains("composeRoute"));
        assert!(script.contains("layouts: []"));
        assert!(script.contains("createPageElement"));
        assert!(
            !script.contains("\"componentId\":\"RootLayout\""),
            "empty layouts must not serialize a RootLayout entry"
        );
    }

    #[test]
    fn test_build_composition_script_single_layout() {
        let layouts = vec![LayoutInfo {
            component_id: "RootLayout".to_string(),
            is_root: true,
            file_path: "app/layout.tsx".to_string(),
            path: "/".to_string(),
        }];

        let script = RouteComposer::build_composition_script(
            "const pageElement = Page();",
            &layouts,
            "\"/\"",
        );

        assert!(script.contains("const pageElement = Page();"));
        assert!(script.contains("composeRoute"));
        assert!(script.contains("RootLayout"));
        assert!(script.contains("\"isRoot\":true"));
        assert!(script.contains("expandRootLayout: true"));
    }

    #[test]
    fn test_rsc_soft_nav_expands_root_layout_like_html() {
        let layouts = vec![LayoutInfo {
            component_id: "RootLayout".to_string(),
            is_root: true,
            file_path: "app/layout.tsx".to_string(),
            path: "/".to_string(),
        }];

        let script = RouteComposer::build_composition_script_with_templates(
            "const pageElement = Page();",
            &layouts,
            &[],
            "\"/\"",
            "\"/\"",
            None,
            "{}",
            true,
            "\"/\"",
            Some("stream-1"),
            true,
            &[],
        );

        assert!(script.contains("expandRootLayout: true"));
        assert!(script.contains("deferRsc: true"));
        assert!(script.contains("\"stream-1\""));
        assert!(script.contains("RootLayout"));
    }

    #[test]
    fn test_build_composition_script_multiple_layouts() {
        let layouts = vec![
            LayoutInfo {
                component_id: "RootLayout".to_string(),
                is_root: true,
                file_path: "app/layout.tsx".to_string(),
                path: "/".to_string(),
            },
            LayoutInfo {
                component_id: "DashboardLayout".to_string(),
                is_root: false,
                file_path: "app/dashboard/layout.tsx".to_string(),
                path: "/".to_string(),
            },
        ];

        let script = RouteComposer::build_composition_script(
            "const pageElement = Page();",
            &layouts,
            "\"/dashboard\"",
        );

        assert!(script.contains("DashboardLayout"));
        assert!(script.contains("RootLayout"));
        let dashboard_pos = script.find("DashboardLayout").expect("dashboard");
        let root_pos = script.find("RootLayout").expect("root");
        assert_ne!(dashboard_pos, root_pos);
        assert!(script.contains("\"isRoot\":false"));
        assert!(script.contains("\"isRoot\":true"));
    }

    #[test]
    fn test_build_composition_script_with_templates_empty_matches_no_templates_output() {
        let page_script = "const pageElement = Page();";
        let no_tpl = RouteComposer::build_composition_script_with_error(
            page_script,
            &[],
            "\"/\"",
            None,
            "{}",
        );
        let empty_tpl = RouteComposer::build_composition_script_with_templates(
            page_script,
            &[],
            &[],
            "\"/\"",
            "\"/\"",
            None,
            "{}",
            false,
            "\"/\"",
            None,
            true,
            &[],
        );
        assert_eq!(empty_tpl, no_tpl);
    }

    fn template_info(file_path: &str) -> TemplateInfo {
        TemplateInfo {
            component_id: format!("template:{file_path}"),
            file_path: file_path.to_string(),
        }
    }

    #[test]
    fn test_build_composition_script_with_templates_single() {
        let script = RouteComposer::build_composition_script_with_templates(
            "const pageElement = Page();",
            &[],
            &[template_info("template.tsx")],
            "\"/about\"",
            "\"/about\"",
            None,
            "{}",
            false,
            "\"/about\"",
            None,
            true,
            &[],
        );

        assert!(script.contains("template:template.tsx"));
        assert!(
            script.contains(r#""ssrModuleKey":"src/app/template.tsx""#),
            "client templates resolve via project-relative SSR module keys"
        );
        assert!(
            !script.contains(r#""ssrModuleKey":"src/app/src/app/template.tsx""#),
            "template SSR keys must not double-prefix the app directory"
        );
        assert!(script.contains("templateKey: \"/about\""));
    }

    #[test]
    fn test_template_ssr_module_key_normalizes_app_relative_paths() {
        let already_prefixed = RouteComposer::templates_json(&[TemplateInfo {
            component_id: "template:src/app/template.tsx".to_string(),
            file_path: "src/app/template.tsx".to_string(),
        }]);
        assert!(already_prefixed.contains(r#""ssrModuleKey":"src/app/template.tsx""#));
        assert!(!already_prefixed.contains(r#""ssrModuleKey":"src/app/src/app/template.tsx""#));

        let app_prefixed = RouteComposer::templates_json(&[TemplateInfo {
            component_id: "template:app/template.tsx".to_string(),
            file_path: "app/template.tsx".to_string(),
        }]);
        assert!(app_prefixed.contains(r#""ssrModuleKey":"src/app/template.tsx""#));
        assert!(!app_prefixed.contains(r#""ssrModuleKey":"src/app/app/template.tsx""#));
    }

    #[tokio::test]
    async fn test_server_template_resolves_from_rsc_module_manager_registry() {
        use std::sync::Arc;

        use crate::runtime::JsExecutionRuntime;

        let runtime = Arc::new(JsExecutionRuntime::new(None));
        runtime
            .execute_script("route_composer.ts".to_string(), ROUTE_COMPOSER_SCRIPT.to_string())
            .await
            .expect("route composer should load");

        let script = r"
            (async () => {
            globalThis.React = {
              createElement(type, props, ...children) {
                return { type, props, children };
              },
            };
            globalThis['~rsc'] = { modules: {} };
            function RegisteredTemplate() { return null; }
            globalThis['~rsc'].modules['template:template.tsx'] = { default: RegisteredTemplate };
            delete globalThis['template:template.tsx'];

            const timings = {};
            await globalThis['~rari'].composeRoute({
              pageElement: { kind: 'page' },
              layouts: [],
              templates: [{
                componentId: 'template:template.tsx',
                filePath: 'template.tsx',
                ssrModuleKey: 'src/app/template.tsx',
              }],
              pathname: '/',
              templateKey: '/',
              errorComponentId: '',
              metadata: {},
              deferRsc: true,
              captureStreamId: null,
              expandRootLayout: true,
              reuseLayoutPaths: [],
              timings,
              startTotal: performance.now(),
            });
            const captured = globalThis['~rari'].capturedElement;
            const templateEl = captured.children?.[0] ?? captured;
            if (templateEl.type !== RegisteredTemplate) {
              throw new Error('template did not resolve from ~rsc.modules');
            }
            return true;
            })()
            ";

        let result = runtime
            .execute_script("template_rsc_modules_fallback".to_string(), script.to_string())
            .await
            .expect("template registry fallback script should execute");
        assert_eq!(result, serde_json::Value::Bool(true));
    }

    #[tokio::test]
    async fn test_client_template_resolves_from_ssr_modules_by_file_path() {
        use std::sync::Arc;

        use crate::runtime::JsExecutionRuntime;

        let runtime = Arc::new(JsExecutionRuntime::new(None));
        runtime
            .execute_script("route_composer.ts".to_string(), ROUTE_COMPOSER_SCRIPT.to_string())
            .await
            .expect("route composer should load");

        let script = r"
            (async () => {
            globalThis.React = {
              createElement(type, props, ...children) {
                return { type, props, children };
              },
            };
            globalThis['~rari'] = { ...(globalThis['~rari'] || {}), ssrModules: {} };
            function ClientTemplate() { return null; }
            globalThis['~rari'].ssrModules['src/app/template.tsx'] = { default: ClientTemplate };
            delete globalThis['app/template_6ef52460'];

            const timings = {};
            await globalThis['~rari'].composeRoute({
              pageElement: { kind: 'page' },
              layouts: [],
              templates: [{
                componentId: 'app/template_6ef52460',
                filePath: 'template.tsx',
                ssrModuleKey: 'src/app/template.tsx',
              }],
              pathname: '/',
              templateKey: '/',
              errorComponentId: '',
              metadata: {},
              deferRsc: true,
              captureStreamId: null,
              expandRootLayout: true,
              reuseLayoutPaths: [],
              timings,
              startTotal: performance.now(),
            });
            const captured = globalThis['~rari'].capturedElement;
            const templateEl = captured.children?.[0] ?? captured;
            if (templateEl.type !== ClientTemplate) {
              throw new Error('template did not resolve from ~rari.ssrModules');
            }
            return true;
            })()
            ";

        let result = runtime
            .execute_script("template_ssr_modules_fallback".to_string(), script.to_string())
            .await
            .expect("client template SSR module fallback should execute");
        assert_eq!(result, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_build_composition_script_with_templates_and_layouts() {
        let script = RouteComposer::build_composition_script_with_templates(
            "const pageElement = Page();",
            &[LayoutInfo {
                component_id: "layout:blog".to_string(),
                is_root: false,
                file_path: "blog/layout.tsx".to_string(),
                path: "/".to_string(),
            }],
            &[template_info("blog/template.tsx")],
            "\"/blog/hello\"",
            "\"/blog/hello\"",
            None,
            "{}",
            false,
            "\"/blog/hello\"",
            None,
            true,
            &[],
        );

        assert!(script.contains("pageElement"));
        assert!(script.contains("blog/template.tsx"));
        assert!(script.contains("layout:blog"));
        assert!(script.contains("templateKey: \"/blog/hello\""));
    }

    #[test]
    fn test_build_composition_script_with_templates_multiple() {
        let script = RouteComposer::build_composition_script_with_templates(
            "const pageElement = Page();",
            &[],
            &[template_info("template.tsx"), template_info("about/template.tsx")],
            "\"/about\"",
            "\"/about\"",
            None,
            "{}",
            false,
            "\"/about\"",
            None,
            true,
            &[],
        );

        assert!(script.contains("template:template.tsx"));
        assert!(script.contains("template:about/template.tsx"));
        assert!(script.contains(r#""ssrModuleKey":"src/app/about/template.tsx""#));
    }

    #[test]
    fn test_layout_reuse_emits_marker_without_layout_component() {
        let layouts = vec![LayoutInfo {
            component_id: "RootLayout".to_string(),
            is_root: true,
            file_path: "app/layout.tsx".to_string(),
            path: "/".to_string(),
        }];

        let script = RouteComposer::build_composition_script_with_templates(
            "const pageElement = Page();",
            &layouts,
            &[],
            "\"/about\"",
            "\"/about\"",
            None,
            "{}",
            true,
            "\"/about\"",
            Some("stream-1"),
            true,
            &["/".to_string()],
        );

        assert!(script.contains("reuseLayoutPaths"));
        assert!(script.contains(r#""/""#));
        assert!(script.contains("RootLayout"));
        assert!(script.contains("deferRsc: true"));
        assert!(
            ROUTE_COMPOSER_SCRIPT.contains("wrapLayoutReuse"),
            "layout reuse tree must live in TS helper, not Rust format!"
        );
        assert!(ROUTE_COMPOSER_SCRIPT.contains("rari-layout-reuse"));
        assert!(ROUTE_COMPOSER_SCRIPT.contains("data-rari-layout-path"));
    }

    #[test]
    fn test_error_boundary_id_serialized() {
        let error_boundary = ErrorBoundaryInfo {
            component_id: "src/app/test/error.tsx".to_string(),
            file_path: "test/error.tsx".to_string(),
        };
        let script = RouteComposer::build_composition_script_with_error(
            "const pageElement = Page();",
            &[],
            "\"/\"",
            Some(&error_boundary),
            "{}",
        );
        assert!(script.contains("src/app/test/error.tsx"));
        assert!(script.contains("errorComponentId"));
    }

    #[test]
    fn test_metadata_passed_through() {
        let metadata_json = r#"{"title":"Test Page","description":"A test"}"#;
        let script = RouteComposer::build_composition_script_with_error(
            "const pageElement = Page();",
            &[],
            "\"/\"",
            None,
            metadata_json,
        );
        assert!(script.contains(r#"{"title":"Test Page","description":"A test"}"#));
        assert!(script.contains("metadata:"));
    }

    #[tokio::test]
    async fn test_layout_reuse_compose_emits_marker() {
        use std::sync::Arc;

        use crate::runtime::JsExecutionRuntime;

        let runtime = Arc::new(JsExecutionRuntime::new(None));
        runtime
            .execute_script("route_composer.ts".to_string(), ROUTE_COMPOSER_SCRIPT.to_string())
            .await
            .expect("route composer should load");

        let script = r"
            (async () => {
            globalThis.React = {
              createElement(type, props, ...children) {
                return { type, props: props || null, children };
              },
            };
            const timings = {};
            await globalThis['~rari'].composeRoute({
              pageElement: { kind: 'page' },
              layouts: [{
                componentId: 'RootLayout',
                isRoot: true,
                filePath: 'app/layout.tsx',
                path: '/',
              }],
              templates: [],
              pathname: '/about',
              templateKey: '/about',
              errorComponentId: '',
              metadata: {},
              deferRsc: true,
              captureStreamId: null,
              expandRootLayout: true,
              reuseLayoutPaths: ['/'],
              timings,
              startTotal: performance.now(),
            });
            const marker = globalThis['~rari'].capturedElement;
            if (marker.type !== 'rari-layout-reuse') throw new Error('expected reuse marker');
            if (marker.props['data-rari-layout-path'] !== '/') {
              throw new Error('expected reuse path');
            }
            if (marker.props['data-rari-document-reuse'] !== true) {
              throw new Error('expected document reuse marker');
            }
            return true;
            })()
            ";

        let result = runtime
            .execute_script("layout_reuse_compose".to_string(), script.to_string())
            .await
            .expect("layout reuse compose should execute");
        assert_eq!(result, serde_json::Value::Bool(true));
    }
}
