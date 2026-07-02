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
    pub client_component_id: String,
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
            error_boundary,
            metadata_json,
            false,
        )
    }

    pub fn build_composition_script_with_templates(
        page_render_script: &str,
        layouts: &[LayoutInfo],
        templates: &[TemplateInfo],
        pathname_json: &str,
        error_boundary: Option<&ErrorBoundaryInfo>,
        metadata_json: &str,
        defer_rsc: bool,
    ) -> String {
        let mut script = format!(
            r"
            (async () => {{
                const timings = {{}};
                const startTotal = performance.now();

                const React = globalThis.React;

                if (!globalThis['~suspense']) globalThis['~suspense'] = {{}};
                globalThis['~suspense'].discoveredBoundaries = [];
                globalThis['~suspense'].pendingPromises = [];
                globalThis['~suspense'].pendingPromisesByBoundary = {{}};
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
                &template.client_component_id,
                &current_element,
                &template_var,
                pathname_json,
            ));
            current_element = template_var;
        }

        for (i, layout) in layouts.iter().rev().enumerate() {
            let layout_var = format!("layout{i}");

            script.push_str(&Self::generate_layout_wrapper(
                i,
                &layout.component_id,
                &current_element,
                &layout_var,
                pathname_json,
            ));

            current_element = layout_var;
        }

        script.push_str(&Self::generate_rsc_conversion(
            &current_element,
            error_boundary,
            metadata_json,
            defer_rsc,
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
        _template_component_id: &str,
        template_client_component_id: &str,
        current_element: &str,
        template_var: &str,
        pathname_json: &str,
    ) -> String {
        format!(
            r#"
            const startTemplate{index} = performance.now();
            const TemplateComponent{index} = {{
                $$typeof: Symbol.for('react.client.reference'),
                $$id: "{template_client_component_id}#default",
                $$async: false,
                name: 'default',
                '~isClientComponent': true,
            }};

            const templateKey{index} = {pathname_json} + '::' + Date.now();
            const templateResult{index} = React.createElement(
                TemplateComponent{index},
                {{ key: templateKey{index}, children: {current_element} }},
            );
            const {template_var} = templateResult{index};
            timings.template{index} = performance.now() - startTemplate{index};
            "#
        )
    }

    fn generate_rsc_conversion(
        final_element: &str,
        error_boundary: Option<&ErrorBoundaryInfo>,
        metadata_json: &str,
        defer_rsc: bool,
    ) -> String {
        let wrap_with_error_boundary = error_boundary.is_some();
        let error_component_id_json = error_boundary
            .map(|e| serde_json::to_string(&e.component_id).unwrap_or_else(|_| "\"\"".to_string()))
            .unwrap_or_else(|| "\"\"".to_string());

        let rsc_render = if defer_rsc {
            r"
                if (!globalThis['~rari']) globalThis['~rari'] = {};
                globalThis['~rari'].capturedElement = elementToRender;
                return;
            "
        } else {
            r"
                let rscData = await globalThis.renderToRsc(elementToRender);
            "
        };

        format!(
            r"

                const startRSC = performance.now();

                let elementToRender = {final_element};

                if ({wrap_with_error_boundary}) {{
                    const errorComponentId = {error_component_id_json};
                    const wrapperComponentId = 'virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper';

                    const ErrorWrapper = {{
                        $$typeof: Symbol.for('react.client.reference'),
                        $$id: wrapperComponentId,
                        $$async: false,
                    }};
                    elementToRender = globalThis.React.createElement(
                        ErrorWrapper,
                        {{ errorComponentId: errorComponentId }},
                        elementToRender
                    );
                }}

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
#[allow(
    clippy::allow_attributes,
    clippy::unreadable_literal,
    clippy::needless_raw_string_hashes,
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp,
    clippy::bool_assert_comparison,
    clippy::redundant_clone,
    clippy::redundant_closure_for_method_calls,
    clippy::single_char_pattern,
    clippy::approx_constant,
    clippy::uninlined_format_args,
    clippy::module_inception,
    clippy::return_self_not_must_use,
    clippy::disallowed_methods,
    clippy::clone_on_ref_ptr,
    clippy::get_unwrap
)]
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
        assert!(script.contains("elementToRender = pageElement"));
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
        assert!(script.contains("LayoutComponent0"));
        assert!(script.contains("RootLayout"));
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
        assert!(script.contains("elementToRender = layout1"));
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
        let conversion = RouteComposer::generate_rsc_conversion("finalElement", None, "{}", false);

        assert!(conversion.contains("elementToRender = finalElement"));
        assert!(conversion.contains("renderToRsc(elementToRender"));
        assert!(conversion.contains("rsc_data: rscData"));
        assert!(conversion.contains("timings: timings"));
        assert!(conversion.contains("success: true"));
    }

    #[test]
    fn test_generate_rsc_conversion_with_error_boundary() {
        let error_boundary = ErrorBoundaryInfo {
            component_id: "src/app/test/error.tsx".to_string(),
            file_path: "test/error.tsx".to_string(),
        };

        let conversion = RouteComposer::generate_rsc_conversion(
            "finalElement",
            Some(&error_boundary),
            "{}",
            false,
        );

        assert!(conversion.contains("virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper"));
        assert!(conversion.contains("src/app/test/error.tsx"));
        assert!(conversion.contains("errorComponentId"));
        assert!(conversion.contains("ErrorWrapper"));
        assert!(conversion.contains("renderToRsc(elementToRender"));
    }

    #[test]
    fn test_generate_rsc_conversion_with_metadata() {
        let metadata_json = r#"{"title":"Test Page","description":"A test"}"#;
        let conversion =
            RouteComposer::generate_rsc_conversion("finalElement", None, metadata_json, false);

        assert!(conversion.contains(r#"metadata: {"title":"Test Page","description":"A test"}"#));
    }

    #[test]
    fn test_generate_rsc_conversion_deferred() {
        let conversion = RouteComposer::generate_rsc_conversion("finalElement", None, "{}", true);

        assert!(conversion.contains("capturedElement = elementToRender"));
        assert!(!conversion.contains("renderToRsc(elementToRender"));
    }

    fn template_info(file_path: &str) -> TemplateInfo {
        TemplateInfo {
            component_id: format!("template:{}", file_path),
            client_component_id: format!("src/app/{}", file_path.trim_end_matches(".tsx")),
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
            None,
            "{}",
            false,
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
            None,
            "{}",
            false,
        );

        assert!(script.contains("TemplateComponent0"));
        assert!(script.contains(r#"$$id: "src/app/template#default""#));
        assert!(script.contains("templateKey0 = \"/about\""));
        assert!(script.contains("key: templateKey0"));
        assert!(
            !script.contains("pathname: \"/about\", children: pageElement"),
            "template wrapper must not include pathname as a prop, only key and children"
        );
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
            None,
            "{}",
            false,
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
            None,
            "{}",
            false,
        );

        assert!(script.contains("TemplateComponent0"));
        assert!(script.contains("TemplateComponent1"));
        assert!(script.contains("templateKey0 = \"/about\""));
        assert!(script.contains("templateKey1 = \"/about\""));
    }
}
