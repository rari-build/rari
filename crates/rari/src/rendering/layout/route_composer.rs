#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LayoutInfo {
    pub component_id: String,
    pub is_root: bool,
    pub file_path: String,
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
    ) -> String {
        let mut script = format!(
            r"
            (async () => {{
                const timings = {{}};
                const startTotal = performance.now();

                const React = globalThis.React;

                if (!globalThis['~rari']) globalThis['~rari'] = {{}};
                globalThis['~rari'].actionPostUrl = {action_post_url_json};

                if (!globalThis['~suspense']) globalThis['~suspense'] = {{}};
                globalThis['~suspense'].discoveredBoundaries = [];
                globalThis['~suspense'].pendingPromises = [];
                globalThis['~suspense'].promises = {{}};
                globalThis['~suspense'].currentBoundaryId = null;

                const startPageRender = performance.now();
                {page_render_script}
            "
        );

        let mut current_element = "pageElement".to_string();

        for (i, template) in templates.iter().rev().enumerate() {
            let template_var = format!("template{i}");
            script.push_str(&Self::generate_template_wrapper(
                i,
                &template.component_id,
                &template.file_path,
                &current_element,
                &template_var,
                template_key_json,
            ));
            current_element = template_var;
        }

        let nested_layouts: Vec<&LayoutInfo> =
            layouts.iter().filter(|layout| !layout.is_root).collect();
        let root_layouts: Vec<&LayoutInfo> =
            layouts.iter().filter(|layout| layout.is_root).collect();

        let mut layout_index = 0usize;
        for layout in nested_layouts.iter().rev() {
            let layout_var = format!("layout{layout_index}");
            script.push_str(&Self::generate_layout_wrapper(
                layout_index,
                &layout.component_id,
                &current_element,
                &layout_var,
                pathname_json,
            ));
            current_element = layout_var;
            layout_index += 1;
        }

        script.push_str(&Self::generate_error_boundary_wrap(&current_element, error_boundary));
        current_element = "errorBoundedElement".to_string();

        for layout in root_layouts.iter().rev() {
            let layout_var = format!("layout{layout_index}");
            script.push_str(&Self::generate_layout_wrapper(
                layout_index,
                &layout.component_id,
                &current_element,
                &layout_var,
                pathname_json,
            ));
            current_element = layout_var;
            layout_index += 1;
        }

        script.push_str(&Self::generate_rsc_conversion(
            &current_element,
            metadata_json,
            defer_rsc,
            capture_stream_id,
        ));

        script
    }

    fn generate_layout_wrapper(
        index: usize,
        layout_component_id: &str,
        current_element: &str,
        layout_var: &str,
        pathname_json: &str,
    ) -> String {
        format!(
            r#"
                const startLayout{index} = performance.now();
                const LayoutComponent{index} = globalThis["{layout_component_id}"];
                if (!LayoutComponent{index} || typeof LayoutComponent{index} !== 'function') {{
                    throw new Error('Layout component {layout_component_id} not found');
                }}

                const layoutResult{index} = React.createElement(LayoutComponent{index}, {{ children: {current_element}, pathname: {pathname_json} }});
                const {layout_var} = layoutResult{index};
                timings.layout{index} = performance.now() - startLayout{index};
                "#
        )
    }

    fn generate_template_wrapper(
        index: usize,
        template_component_id: &str,
        template_file_path: &str,
        current_element: &str,
        template_var: &str,
        template_key_json: &str,
    ) -> String {
        let ssr_module_key = super::utils::normalize_route_component_path(template_file_path);
        let ssr_module_key_json = serde_json::to_string(&ssr_module_key)
            .unwrap_or_else(|_| format!("\"{}\"", ssr_module_key.replace('"', "\\\"")));

        format!(
            r#"
            const startTemplate{index} = performance.now();
            let TemplateComponent{index} = globalThis["{template_component_id}"];
            if (typeof TemplateComponent{index} !== 'function') {{
                const resolveTemplateExport{index} = (moduleNamespace) => {{
                    if (moduleNamespace == null) return null;
                    const resolved = moduleNamespace.default
                        ?? Object.values(moduleNamespace).find((value) => typeof value === 'function');
                    return typeof resolved === 'function' ? resolved : null;
                }};
                TemplateComponent{index} = resolveTemplateExport{index}(
                    globalThis['~rsc']?.modules?.["{template_component_id}"]
                );
                if (typeof TemplateComponent{index} !== 'function') {{
                    const ssrModules{index} = globalThis['~rari']?.ssrModules;
                    TemplateComponent{index} = resolveTemplateExport{index}(
                        ssrModules{index}?.["{template_component_id}"]
                    ) ?? resolveTemplateExport{index}(
                        ssrModules{index}?.[{ssr_module_key_json}]
                    ) ?? resolveTemplateExport{index}(
                        ssrModules{index}?.[{ssr_module_key_json} + '#default']
                    );
                }}
            }}
            if (!TemplateComponent{index} || typeof TemplateComponent{index} !== 'function') {{
                throw new Error('Template component {template_component_id} not found');
            }}

            const templateKey{index} = {template_key_json};
            const templateResult{index} = React.createElement(
                TemplateComponent{index},
                {{ key: templateKey{index}, children: {current_element} }},
            );
            const {template_var} = templateResult{index};
            timings.template{index} = performance.now() - startTemplate{index};
            "#
        )
    }

    fn generate_error_boundary_wrap(
        current_element: &str,
        error_boundary: Option<&ErrorBoundaryInfo>,
    ) -> String {
        let error_component_id = error_boundary.map(|b| b.component_id.as_str()).unwrap_or("");
        let error_component_id_json =
            serde_json::to_string(error_component_id).unwrap_or_else(|_| "\"\"".to_string());
        format!(
            r"
                const errorComponentId = {error_component_id_json};
                const wrapperComponentId = 'virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper';

                const ErrorWrapper = {{
                    $$typeof: Symbol.for('react.client.reference'),
                    $$id: wrapperComponentId,
                    $$async: false,
                }};
                const errorBoundedElement = globalThis.React.createElement(
                    ErrorWrapper,
                    {{ errorComponentId: errorComponentId }},
                    {current_element}
                );
                "
        )
    }

    fn generate_rsc_conversion(
        final_element: &str,
        metadata_json: &str,
        defer_rsc: bool,
        capture_stream_id: Option<&str>,
    ) -> String {
        let rsc_render = if defer_rsc {
            if let Some(stream_id) = capture_stream_id {
                let stream_id_json =
                    serde_json::to_string(stream_id).unwrap_or_else(|_| "\"\"".to_string());
                format!(
                    r"
                if (!globalThis['~rari']) globalThis['~rari'] = {{}};
                if (!globalThis['~rari'].capturedByStream) globalThis['~rari'].capturedByStream = Object.create(null);
                globalThis['~rari'].capturedByStream[{stream_id_json}] = elementToRender;
                return;
            "
                )
            } else {
                r"
                if (!globalThis['~rari']) globalThis['~rari'] = {};
                if (globalThis['~rari'].isActionRefreshCompose) {
                    globalThis['~rari'].actionRefreshElement = elementToRender;
                } else {
                    globalThis['~rari'].capturedElement = elementToRender;
                }
                return;
            "
                .to_string()
            }
        } else {
            r"
                let rscData = await globalThis.renderToRsc(elementToRender);
            "
            .to_string()
        };

        format!(
            r"

                const startRSC = performance.now();

                let elementToRender = {final_element};
                {rsc_render}

                timings.rscConversion = performance.now() - startRSC;

                timings.total = performance.now() - startTotal;

                const result = {{
                    rsc_data: rscData,
                    boundaries: globalThis['~suspense']?.discoveredBoundaries || [],
                    pending_promises: globalThis['~suspense']?.pendingPromises || [],
                    has_suspense: (globalThis['~suspense']?.discoveredBoundaries && globalThis['~suspense'].discoveredBoundaries.length > 0) ||
                                 (globalThis['~suspense']?.pendingPromises && globalThis['~suspense'].pendingPromises.length > 0),
                    timings: timings,
                    metadata: {metadata_json},
                    success: true
                }};

                try {{
                    const jsonString = JSON.stringify(result);
                    const cleanResult = JSON.parse(jsonString);
                    globalThis['~rsc'].renderResult = cleanResult;
                    return cleanResult;
                }} catch (jsonError) {{
                    globalThis['~rsc'].renderResult = result;
                    return result;
                }}
            }})()
            "
        )
    }
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_info_creation() {
        let layout = LayoutInfo {
            component_id: "Layout".to_string(),
            is_root: true,
            file_path: "app/layout.tsx".to_string(),
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
        assert!(script.contains("errorBoundedElement"));
        assert!(script.contains("elementToRender = errorBoundedElement"));
        assert!(!script.contains("LayoutComponent"));
    }

    #[test]
    fn test_build_composition_script_single_layout() {
        let layouts = vec![LayoutInfo {
            component_id: "RootLayout".to_string(),
            is_root: true,
            file_path: "app/layout.tsx".to_string(),
        }];

        let script = RouteComposer::build_composition_script(
            "const pageElement = Page();",
            &layouts,
            "\"/\"",
        );

        assert!(script.contains("const pageElement = Page();"));
        assert!(script.contains("errorBoundedElement"));
        assert!(script.contains("LayoutComponent0"));
        assert!(script.contains("RootLayout"));
        assert!(script.contains("children: errorBoundedElement"));
        assert!(script.contains("elementToRender = layout0"));
    }

    #[test]
    fn test_build_composition_script_multiple_layouts() {
        let layouts = vec![
            LayoutInfo {
                component_id: "RootLayout".to_string(),
                is_root: true,
                file_path: "app/layout.tsx".to_string(),
            },
            LayoutInfo {
                component_id: "DashboardLayout".to_string(),
                is_root: false,
                file_path: "app/dashboard/layout.tsx".to_string(),
            },
        ];

        let script = RouteComposer::build_composition_script(
            "const pageElement = Page();",
            &layouts,
            "\"/dashboard\"",
        );

        assert!(script.contains("LayoutComponent0"));
        assert!(script.contains("LayoutComponent1"));
        assert!(script.contains("DashboardLayout"));
        assert!(script.contains("RootLayout"));
        assert!(script.contains("children: pageElement"));
        assert!(script.contains("children: errorBoundedElement"));
        assert!(script.contains("elementToRender = layout1"));
        let dashboard_pos = script.find("DashboardLayout").expect("dashboard");
        let error_pos = script.find("errorBoundedElement =").expect("error wrap");
        let root_pos = script.find("RootLayout").expect("root");
        assert!(dashboard_pos < error_pos && error_pos < root_pos);
    }

    #[test]
    fn test_generate_layout_wrapper() {
        let wrapper = RouteComposer::generate_layout_wrapper(
            0,
            "TestLayout",
            "pageElement",
            "layout0",
            "\"/test\"",
        );

        assert!(wrapper.contains("LayoutComponent0"));
        assert!(wrapper.contains("TestLayout"));
        assert!(wrapper.contains("pageElement"));
        assert!(wrapper.contains("layout0"));
        assert!(wrapper.contains("\"/test\""));
        assert!(wrapper.contains("timings.layout0"));
    }

    #[test]
    fn test_generate_rsc_conversion() {
        let conversion = RouteComposer::generate_rsc_conversion("finalElement", "{}", false, None);

        assert!(conversion.contains("elementToRender = finalElement"));
        assert!(conversion.contains("renderToRsc(elementToRender"));
        assert!(conversion.contains("rsc_data: rscData"));
        assert!(conversion.contains("timings: timings"));
        assert!(conversion.contains("success: true"));
    }

    #[test]
    fn test_generate_error_boundary_wrap_default() {
        let wrap = RouteComposer::generate_error_boundary_wrap("pageElement", None);

        assert!(wrap.contains("virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper"));
        assert!(wrap.contains(r#"const errorComponentId = """#));
        assert!(wrap.contains("errorBoundedElement"));
        assert!(wrap.contains("pageElement"));
    }

    #[test]
    fn test_generate_error_boundary_wrap_with_custom_component() {
        let error_boundary = ErrorBoundaryInfo {
            component_id: "src/app/test/error.tsx".to_string(),
            file_path: "test/error.tsx".to_string(),
        };

        let wrap =
            RouteComposer::generate_error_boundary_wrap("pageElement", Some(&error_boundary));

        assert!(wrap.contains("virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper"));
        assert!(wrap.contains("src/app/test/error.tsx"));
        assert!(wrap.contains("errorComponentId"));
        assert!(wrap.contains("ErrorWrapper"));
        assert!(wrap.contains("errorBoundedElement"));
    }

    #[test]
    fn test_generate_rsc_conversion_with_metadata() {
        let metadata_json = r#"{"title":"Test Page","description":"A test"}"#;
        let conversion =
            RouteComposer::generate_rsc_conversion("finalElement", metadata_json, false, None);

        assert!(conversion.contains(r#"metadata: {"title":"Test Page","description":"A test"}"#));
    }

    #[test]
    fn test_generate_rsc_conversion_deferred() {
        let conversion = RouteComposer::generate_rsc_conversion("finalElement", "{}", true, None);

        assert!(conversion.contains("capturedElement = elementToRender"));
        assert!(!conversion.contains("renderToRsc(elementToRender"));
    }

    fn template_info(file_path: &str) -> TemplateInfo {
        TemplateInfo {
            component_id: format!("template:{file_path}"),
            file_path: file_path.to_string(),
        }
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
        );
        assert_eq!(empty_tpl, no_tpl);
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
        );

        assert!(script.contains("TemplateComponent0"));
        assert!(script.contains(r#"globalThis["template:template.tsx"]"#));
        assert!(
            script.contains(r#"globalThis['~rsc']?.modules?.["template:template.tsx"]"#),
            "templates must fall back to the SSR module registry used by RscModuleManager.register"
        );
        assert!(
            script.contains(r"globalThis['~rari']?.ssrModules"),
            "client templates must fall back to production SSR client modules"
        );
        assert!(
            script.contains(r#""src/app/template.tsx""#),
            "client templates resolve via project-relative SSR module keys"
        );
        assert!(
            !script.contains(r#""src/app/src/app/template.tsx""#),
            "template SSR keys must not double-prefix the app directory"
        );
        assert!(script.contains("templateKey0 = \"/about\""));
        assert!(script.contains("key: templateKey0"));
        assert!(
            !script.contains("react.client.reference"),
            "server templates must resolve from the SSR module registry, not forced client refs"
        );
        assert!(
            !script.contains("pathname: \"/about\", children: pageElement"),
            "template wrapper must not include pathname as a prop, only key and children"
        );
    }

    #[test]
    fn test_template_ssr_module_key_normalizes_app_relative_paths() {
        let already_prefixed = RouteComposer::generate_template_wrapper(
            0,
            "template:src/app/template.tsx",
            "src/app/template.tsx",
            "pageElement",
            "template0",
            "\"/\"",
        );
        assert!(already_prefixed.contains(r#""src/app/template.tsx""#));
        assert!(!already_prefixed.contains(r#""src/app/src/app/template.tsx""#));

        let app_prefixed = RouteComposer::generate_template_wrapper(
            0,
            "template:app/template.tsx",
            "app/template.tsx",
            "pageElement",
            "template0",
            "\"/\"",
        );
        assert!(app_prefixed.contains(r#""src/app/template.tsx""#));
        assert!(!app_prefixed.contains(r#""src/app/app/template.tsx""#));
    }

    #[tokio::test]
    async fn test_server_template_resolves_from_rsc_module_manager_registry() {
        use std::sync::Arc;

        use crate::runtime::JsExecutionRuntime;

        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let wrapper = RouteComposer::generate_template_wrapper(
            0,
            "template:template.tsx",
            "template.tsx",
            "pageElement",
            "template0",
            "\"/\"",
        );

        let script = format!(
            r"
            globalThis.React = {{
              createElement(type, props) {{
                return {{ type, props }};
              }},
            }};
            const pageElement = {{ kind: 'page' }};
            const timings = {{}};
            globalThis['~rsc'] = {{ modules: {{}} }};
            function RegisteredTemplate() {{ return null; }}
            globalThis['~rsc'].modules['template:template.tsx'] = {{ default: RegisteredTemplate }};
            delete globalThis['template:template.tsx'];
            {wrapper}
            if (template0.type !== RegisteredTemplate) {{
              throw new Error('template did not resolve from ~rsc.modules');
            }}
            true
            "
        );

        let result = runtime
            .execute_script("template_rsc_modules_fallback".to_string(), script)
            .await
            .expect("template registry fallback script should execute");
        assert_eq!(result, serde_json::Value::Bool(true));
    }

    #[tokio::test]
    async fn test_client_template_resolves_from_ssr_modules_by_file_path() {
        use std::sync::Arc;

        use crate::runtime::JsExecutionRuntime;

        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let wrapper = RouteComposer::generate_template_wrapper(
            0,
            "app/template_6ef52460",
            "template.tsx",
            "pageElement",
            "template0",
            "\"/\"",
        );

        let script = format!(
            r"
            globalThis.React = {{
              createElement(type, props) {{
                return {{ type, props }};
              }},
            }};
            const pageElement = {{ kind: 'page' }};
            const timings = {{}};
            globalThis['~rari'] = {{ ssrModules: {{}} }};
            function ClientTemplate() {{ return null; }}
            globalThis['~rari'].ssrModules['src/app/template.tsx'] = {{ default: ClientTemplate }};
            delete globalThis['app/template_6ef52460'];
            {wrapper}
            if (template0.type !== ClientTemplate) {{
              throw new Error('template did not resolve from ~rari.ssrModules');
            }}
            true
            "
        );

        let result = runtime
            .execute_script("template_ssr_modules_fallback".to_string(), script)
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
            }],
            &[template_info("blog/template.tsx")],
            "\"/blog/hello\"",
            "\"/blog/hello\"",
            None,
            "{}",
            false,
            "\"/blog/hello\"",
            None,
        );

        let page_idx = script.find("pageElement").expect("pageElement present");
        let template_idx = script.find("template0").expect("template0 present");
        let layout_idx = script.find("layout0").expect("layout0 present");
        assert!(page_idx < template_idx);
        assert!(template_idx < layout_idx);
        assert!(script.contains("templateKey0 = \"/blog/hello\""));
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
        );

        assert!(script.contains("TemplateComponent0"));
        assert!(script.contains("TemplateComponent1"));
        assert!(script.contains("templateKey0 = \"/about\""));
        assert!(script.contains("templateKey1 = \"/about\""));
    }
}
