use crate::error::RariError;
use crate::rsc::renderer::RscRenderer;
use crate::rsc::streaming::RscStream;
use crate::server::app_router::AppRouteMatch;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct LayoutRenderContext {
    pub params: FxHashMap<String, String>,
    pub search_params: FxHashMap<String, Vec<String>>,
    pub headers: FxHashMap<String, String>,
}

pub struct LayoutRenderer {
    renderer: Arc<tokio::sync::Mutex<RscRenderer>>,
}

impl LayoutRenderer {
    pub fn new(renderer: Arc<tokio::sync::Mutex<RscRenderer>>) -> Self {
        Self { renderer }
    }

    pub async fn render_route(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        debug!(
            "Rendering route {} with {} layouts",
            route_match.route.path,
            route_match.layouts.len()
        );

        let composition_script = self.build_composition_script(route_match, context)?;

        debug!("Executing composition script to render composed component tree");

        let renderer = self.renderer.lock().await;

        renderer
            .runtime
            .execute_script("compose_and_render".to_string(), composition_script)
            .await?;

        let get_result_script = r#"globalThis.__rsc_render_result"#.to_string();
        let result =
            renderer.runtime.execute_script("get_result".to_string(), get_result_script).await?;

        debug!("Composition result: {:?}", result);

        let rsc_data = result.get("rsc").ok_or_else(|| {
            tracing::error!("Failed to extract RSC data from result: {:?}", result);
            RariError::internal("No RSC data in composition result")
        })?;

        let rsc_wire_format = {
            let mut serializer = renderer.serializer.lock();
            serializer
                .serialize_rsc_json(rsc_data)
                .map_err(|e| RariError::internal(format!("Failed to serialize RSC data: {e}")))?
        };

        Ok(rsc_wire_format)
    }

    pub async fn render_route_streaming(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<RscStream, RariError> {
        debug!(
            "Streaming route {} with {} layouts",
            route_match.route.path,
            route_match.layouts.len()
        );

        // For now, render synchronously and create a simple stream
        // TODO: Implement true streaming with layout boundaries
        let html = self.render_route(route_match, context).await?;

        let (tx, rx) = mpsc::channel(1);
        let _ = tx
            .send(crate::rsc::streaming::RscStreamChunk {
                data: html.into_bytes(),
                chunk_type: crate::rsc::streaming::RscChunkType::InitialShell,
                row_id: 0,
                is_final: true,
            })
            .await;
        drop(tx);

        Ok(RscStream::new(rx))
    }

    fn build_composition_script(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let page_props = self.create_page_props(route_match, context)?;
        let page_props_json = serde_json::to_string(&page_props)
            .map_err(|e| RariError::internal(format!("Failed to serialize page props: {e}")))?;

        let page_component_id = self.create_component_id(&route_match.route.file_path);

        let mut script = format!(
            r#"
            (async function() {{
                const React = globalThis.React || require('react');
                const ReactDOMServer = globalThis.ReactDOMServer || require('react-dom/server');

                const PageComponent = globalThis["{}"];
                if (!PageComponent || typeof PageComponent !== 'function') {{
                    throw new Error('Page component {} not found');
                }}

                const pageProps = {};
                const pageResult = PageComponent(pageProps);
                const pageElement = pageResult && typeof pageResult.then === 'function'
                    ? await pageResult
                    : pageResult;
            "#,
            page_component_id, page_component_id, page_props_json
        );

        let mut current_element = "pageElement".to_string();
        for (i, layout) in route_match.layouts.iter().rev().enumerate() {
            let layout_component_id = self.create_component_id(&layout.file_path);
            let layout_var = format!("layout{}", i);

            script.push_str(&format!(
                r#"
                const LayoutComponent{} = globalThis["{}"];
                if (!LayoutComponent{} || typeof LayoutComponent{} !== 'function') {{
                    throw new Error('Layout component {} not found');
                }}

                const {} = LayoutComponent{}({{ children: {} }});
                "#,
                i, layout_component_id, i, i, layout_component_id, layout_var, i, current_element
            ));

            current_element = layout_var;
        }

        script.push_str(&format!(
            r#"
                globalThis.__rsc_render_result = null;

                const traverseToRSC = globalThis.traverseToRSC;
                if (!traverseToRSC) {{
                    throw new Error('traverseToRSC not available - RSC runtime not initialized');
                }}

                if (!globalThis.renderToRSC) {{
                    globalThis.renderToRSC = function(element, clientComponents = {{}}) {{
                        if (!element) return null;

                        if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {{
                            return element;
                        }}

                        if (Array.isArray(element)) {{
                            return element.map(child => globalThis.renderToRSC(child, clientComponents));
                        }}

                        if (element && typeof element === 'object') {{
                            const uniqueKey = element.key || null;

                            if (element.type) {{
                                if (typeof element.type === 'string') {{
                                    const props = element.props || {{}};
                                    const {{ children: propsChildren, ...otherProps }} = props;

                                    const actualChildren = element.children || propsChildren;

                                    const rscProps = {{
                                        ...otherProps,
                                        children: actualChildren ? globalThis.renderToRSC(actualChildren, clientComponents) : undefined
                                    }};
                                    if (rscProps.children === undefined) {{
                                        delete rscProps.children;
                                    }}
                                    return ["$", element.type, uniqueKey, rscProps];
                                }} else if (typeof element.type === 'function') {{
                                    return ["$", "div", uniqueKey, {{ children: null }}];
                                }}
                            }}

                            return ["$", "div", uniqueKey, {{
                                className: "rsc-unknown",
                                children: "Unknown element type"
                            }}];
                        }}

                        return element;
                    }};
                }}

                const rscData = traverseToRSC({});

                globalThis.__rsc_render_result = {{ rsc: rscData }};
                return globalThis.__rsc_render_result;
            }})()
            "#,
            current_element
        ));

        Ok(script)
    }

    fn create_component_id(&self, file_path: &str) -> String {
        let normalized =
            file_path.replace(".tsx", "").replace(".ts", "").replace("[", "_").replace("]", "_");
        format!("app/{}", normalized)
    }

    fn get_component_id(&self, file_path: &str) -> String {
        let path = std::path::Path::new(file_path);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown");

        let mut chars = stem.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    fn create_page_props(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<Value, RariError> {
        let mut props = serde_json::Map::new();

        let params: FxHashMap<String, String> = route_match.params.clone();
        props.insert("params".to_string(), serde_json::to_value(params)?);

        let search_params: FxHashMap<String, Vec<String>> = context.search_params.clone();
        props.insert("searchParams".to_string(), serde_json::to_value(search_params)?);

        Ok(Value::Object(props))
    }

    pub async fn render_loading(
        &self,
        loading_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = self.get_component_id(loading_path);

        debug!("Rendering loading component: {}", component_id);

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, None).await
    }

    pub async fn render_error(
        &self,
        error_path: &str,
        error: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = self.get_component_id(error_path);

        let mut props = serde_json::Map::new();
        props.insert("error".to_string(), Value::String(error.to_string()));
        props.insert(
            "reset".to_string(),
            Value::String("() => window.location.reload()".to_string()),
        );

        let props_json = serde_json::to_string(&props)
            .map_err(|e| RariError::internal(format!("Failed to serialize error props: {e}")))?;

        debug!("Rendering error component: {} with error: {}", component_id, error);

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, Some(&props_json)).await
    }

    pub async fn render_not_found(
        &self,
        not_found_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = self.get_component_id(not_found_path);

        debug!("Rendering not-found component: {}", component_id);

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, None).await
    }

    pub async fn component_exists(&self, component_id: &str) -> bool {
        let renderer = self.renderer.lock().await;
        renderer.component_exists(component_id)
    }

    pub async fn register_component(
        &self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        let mut renderer = self.renderer.lock().await;
        renderer.register_component(component_id, component_code).await
    }
}

pub fn create_layout_context(
    params: FxHashMap<String, String>,
    search_params: FxHashMap<String, Vec<String>>,
    headers: FxHashMap<String, String>,
) -> LayoutRenderContext {
    LayoutRenderContext { params, search_params, headers }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::JsExecutionRuntime;
    use std::sync::Arc;

    #[test]
    fn test_get_component_id() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        assert_eq!(renderer.get_component_id("app/page.tsx"), "Page");
        assert_eq!(renderer.get_component_id("app/layout.tsx"), "Layout");
        assert_eq!(renderer.get_component_id("app/loading.tsx"), "Loading");
        assert_eq!(renderer.get_component_id("app/error.tsx"), "Error");
    }

    #[test]
    fn test_create_page_props() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let mut params = FxHashMap::default();
        params.insert("id".to_string(), "123".to_string());

        let mut search_params = FxHashMap::default();
        search_params.insert("q".to_string(), vec!["test".to_string()]);

        let context = LayoutRenderContext {
            params: params.clone(),
            search_params: search_params.clone(),
            headers: FxHashMap::default(),
        };

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
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
            pathname: "/test".to_string(),
        };

        let props = renderer.create_page_props(&route_match, &context).unwrap();
        assert!(props.get("params").is_some());
        assert!(props.get("searchParams").is_some());
    }
}
