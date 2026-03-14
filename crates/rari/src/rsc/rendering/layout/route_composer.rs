#[derive(Debug, Clone)]
pub struct LayoutInfo {
    pub component_id: String,
    pub is_root: bool,
    pub file_path: String,
}

#[derive(Debug, Clone)]
pub struct ErrorBoundaryInfo {
    pub component_id: String,
    pub file_path: String,
}

pub struct RouteComposer;

impl RouteComposer {
    pub fn build_composition_script(
        page_render_script: &str,
        layouts: &[LayoutInfo],
        pathname_json: &str,
    ) -> String {
        Self::build_composition_script_with_error(page_render_script, layouts, pathname_json, None)
    }

    pub fn build_composition_script_with_error(
        page_render_script: &str,
        layouts: &[LayoutInfo],
        pathname_json: &str,
        error_boundary: Option<&ErrorBoundaryInfo>,
    ) -> String {
        let mut script = format!(
            r#"
            (async () => {{
                const timings = {{}};
                const startTotal = performance.now();

                const React = globalThis.React;
                const ReactDOMServer = globalThis.ReactDOMServer;

                if (!globalThis['~suspense']) globalThis['~suspense'] = {{}};
                globalThis['~suspense'].discoveredBoundaries = [];
                globalThis['~suspense'].pendingPromises = [];
                if (!globalThis['~render']) globalThis['~render'] = {{}};
                globalThis['~render'].deferredAsyncComponents = [];

                if (!globalThis['~react']) globalThis['~react'] = {{}};
                if (!globalThis['~react'].originalCreateElement) {{
                    globalThis['~react'].originalCreateElement = React.createElement;

                    React.createElement = function(type, props, ...children) {{
                        if (typeof type === 'function' &&
                            (type.constructor.name === 'AsyncFunction' ||
                             type.toString().trim().startsWith('async '))) {{

                            const AsyncComponentMarker = function(props) {{
                                return null;
                            }};

                            AsyncComponentMarker._isAsyncComponent = true;
                            AsyncComponentMarker._originalType = type;
                            AsyncComponentMarker.displayName = `AsyncWrapper(${{type.name || 'Anonymous'}})`;

                            return globalThis['~react'].originalCreateElement(AsyncComponentMarker, props, ...children);
                        }}

                        return globalThis['~react'].originalCreateElement(type, props, ...children);
                    }};
                }}

                const startPageRender = performance.now();
                {}
            "#,
            page_render_script
        );

        let mut current_element = "pageElement".to_string();

        for (i, layout) in layouts.iter().rev().enumerate() {
            let layout_var = format!("layout{}", i);

            script.push_str(&Self::generate_layout_wrapper(
                i,
                &layout.component_id,
                &current_element,
                &layout_var,
                pathname_json,
            ));

            current_element = layout_var;
        }

        script.push_str(&Self::generate_rsc_conversion(&current_element, error_boundary));

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
                "#,
            index = index,
            layout_component_id = layout_component_id,
            current_element = current_element,
            layout_var = layout_var,
            pathname_json = pathname_json
        )
    }

    fn generate_rsc_conversion(
        final_element: &str,
        error_boundary: Option<&ErrorBoundaryInfo>,
    ) -> String {
        let wrap_with_error_boundary = error_boundary.is_some();
        let error_component_id_json = error_boundary
            .map(|e| serde_json::to_string(&e.component_id).unwrap_or_else(|_| "\"\"".to_string()))
            .unwrap_or_else(|| "\"\"".to_string());

        format!(
            r#"

                const startRSC = performance.now();
                let rscData = await globalThis.renderToRsc({final_element}, globalThis['~clientComponents'] || {{}});

                if ({wrap_with_error_boundary}) {{
                    const errorComponentId = {error_component_id_json};
                    const wrapperComponentId = 'virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper';

                    rscData = [
                        '$',
                        wrapperComponentId,
                        null,
                        {{
                            errorComponentId: errorComponentId,
                            children: rscData
                        }}
                    ];
                }}

                timings.rscConversion = performance.now() - startRSC;

                timings.total = performance.now() - startTotal;

                const deferredComponents = globalThis['~render']?.deferredAsyncComponents || [];
                const hasAsync = deferredComponents.length > 0;
                const deferredCount = deferredComponents.length;

                const result = {{
                    rsc_data: rscData,
                    boundaries: globalThis['~suspense']?.discoveredBoundaries || [],
                    pending_promises: globalThis['~suspense']?.pendingPromises || [],
                    has_suspense: (globalThis['~suspense']?.discoveredBoundaries && globalThis['~suspense'].discoveredBoundaries.length > 0) ||
                                 (globalThis['~suspense']?.pendingPromises && globalThis['~suspense'].pendingPromises.length > 0),
                    metadata: {{
                        hasAsync: hasAsync,
                        deferredCount: deferredCount,
                        executionTime: timings.total
                    }},
                    timings: timings,
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
            "#,
            final_element = final_element,
            wrap_with_error_boundary = wrap_with_error_boundary,
            error_component_id_json = error_component_id_json
        )
    }
}

#[cfg(test)]
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
        assert!(script.contains("renderToRsc(pageElement"));
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
        assert!(script.contains("renderToRsc(layout0"));
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
        assert!(script.contains("renderToRsc(layout1"));
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
        let conversion = RouteComposer::generate_rsc_conversion("finalElement", None);

        assert!(conversion.contains("renderToRsc(finalElement"));
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

        let conversion =
            RouteComposer::generate_rsc_conversion("finalElement", Some(&error_boundary));

        assert!(conversion.contains("virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper"));
        assert!(conversion.contains("src/app/test/error.tsx"));
        assert!(conversion.contains("errorComponentId:"));
        assert!(conversion.contains("rscData = ["));
        assert!(conversion.contains("'$'"));
    }
}
