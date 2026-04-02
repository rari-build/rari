use crate::error::RariError;
use crate::rsc::rendering::core::RscRenderer;
use crate::rsc::rendering::streaming::RscStream;
use crate::server::config::Config;
use crate::server::routing::app_router::AppRouteMatch;
use crate::server::types::request::RenderMode;
use crate::utils::path_url::path_to_file_url;
use cow_utils::CowUtils;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;

use super::{constants::*, error_messages, types::*, utils};

struct LayoutHtmlCache {
    cache: DashMap<u64, String>,
}

impl LayoutHtmlCache {
    fn new() -> Self {
        Self { cache: DashMap::new() }
    }

    fn get(&self, key: u64) -> Option<String> {
        self.cache.get(&key).map(|v| v.clone())
    }

    fn insert(&self, key: u64, html: String) {
        self.cache.insert(key, html);
    }
}

pub struct LayoutRenderer {
    renderer: Arc<tokio::sync::Mutex<RscRenderer>>,
    html_cache: Arc<LayoutHtmlCache>,
}

impl LayoutRenderer {
    pub fn new(renderer: Arc<tokio::sync::Mutex<RscRenderer>>) -> Self {
        Self { renderer, html_cache: Arc::new(LayoutHtmlCache::new()) }
    }

    async fn enable_streaming_and_inject_lazy_resolver(
        renderer: &RscRenderer,
        is_not_found: bool,
    ) -> Result<(), RariError> {
        if !is_not_found {
            renderer
                .runtime
                .execute_script(
                    "enable_streaming".to_string(),
                    "if (!globalThis['~rari']) globalThis['~rari'] = {}; if (!globalThis['~rari'].streaming) globalThis['~rari'].streaming = {}; globalThis['~rari'].streaming.enabled = true;".to_string(),
                )
                .await?;

            let resolve_helper = include_str!("js/resolve_lazy_helper.js");
            renderer
                .runtime
                .execute_script("inject_lazy_resolver".to_string(), resolve_helper.to_string())
                .await?;
        }
        Ok(())
    }

    pub async fn check_page_not_found(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<bool, RariError> {
        let page_props = utils::create_page_props(route_match, context)?;
        let page_props_json = serde_json::to_string(&page_props)?;

        fn convert_route_path_to_dist_path(path: &str) -> String {
            let (base, ext) = if let Some(pos) = path.rfind('.') {
                (&path[..pos], &path[pos..])
            } else {
                (path, "")
            };

            let converted_base =
                base.chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '/' || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect::<String>();

            format!("{}{}", converted_base, ext)
        }

        let dist_server_path = std::env::current_dir()
            .ok()
            .map(|p| p.join("dist/server"))
            .and_then(|p| p.canonicalize().ok());

        let base_path = match dist_server_path {
            Some(path) => path,
            None => return Ok(false),
        };

        let js_filename = route_match
            .route
            .file_path
            .cow_replace(".tsx", ".js")
            .cow_replace(".ts", ".js")
            .into_owned();
        let dist_filename = convert_route_path_to_dist_path(&js_filename);
        let page_file_path = base_path.join("app").join(&dist_filename);

        if !page_file_path.exists() {
            return Ok(false);
        }

        let page_path = path_to_file_url(&page_file_path);

        let check_script = format!(
            r#"
            (async () => {{
                try {{
                    const module = await import("{}");

                    if (typeof module.getData === 'function') {{
                        const pageProps = {};
                        const result = await module.getData(pageProps);
                        return {{ notFound: result?.notFound === true }};
                    }}

                    return {{ notFound: false }};
                }} catch (error) {{
                    console.error('[check_page_not_found] Error:', error);
                    return {{ notFound: false }};
                }}
            }})()
            "#,
            page_path, page_props_json
        );

        let renderer = self.renderer.lock().await;
        let result =
            renderer.runtime.execute_script("check_not_found".to_string(), check_script).await?;

        let not_found = result.get("notFound").and_then(|v| v.as_bool()).unwrap_or(false);

        Ok(not_found)
    }

    pub async fn render_route(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<
            std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        >,
    ) -> Result<String, RariError> {
        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            if let Some(loading_entry) = &route_match.loading {
                let loading_id = utils::create_component_id(&loading_entry.file_path);
                Some(loading_id)
            } else {
                None
            }
        } else {
            None
        };

        let composition_script = self.build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            loading_component_id.is_some(),
        )?;

        let renderer = self.renderer.lock().await;

        let is_not_found = route_match.not_found.is_some();

        let render_operation = async {
            Self::enable_streaming_and_inject_lazy_resolver(&renderer, false).await?;

            if is_not_found {
                let rsc_wire_format =
                    Self::execute_composition_and_serialize(&renderer, composition_script).await?;

                Self::validate_rsc_wire_format(&rsc_wire_format)?;

                return Ok(rsc_wire_format);
            }

            let promise_result = renderer
                .runtime
                .execute_script("compose_and_render".to_string(), composition_script)
                .await?;

            let result = if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
                promise_result
            } else {
                renderer
                    .runtime
                    .execute_script("get_result".to_string(), JS_GET_RESULT.to_string())
                    .await?
            };

            let rsc_data = result.get("rsc_data").ok_or_else(|| {
                tracing::error!(
                    "Failed to extract RSC data from result (keys: {:?})",
                    result.as_object().map(|o| o.keys().collect::<Vec<_>>())
                );
                RariError::internal("No RSC data in render result")
            })?;

            let mut rsc_wire_format = {
                let mut serializer = renderer.serializer.lock();
                serializer.reset_for_new_request();
                serializer.serialize_rsc_json(rsc_data).map_err(|e| {
                    RariError::internal(format!("Failed to serialize RSC data: {}", e))
                })?
            };

            Self::resolve_lazy_promises(&renderer, &mut rsc_wire_format).await?;

            if let Err(e) = Self::validate_html_structure(&rsc_wire_format, route_match) {
                error!("HTML structure validation failed: {}", e);
            }

            Ok(rsc_wire_format)
        };

        if let Some(ctx) = request_context {
            renderer.runtime.execute_with_request_context(ctx, render_operation).await
        } else {
            render_operation.await
        }
    }

    pub async fn render_route_by_mode(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        mode: RenderMode,
        request_context: Option<
            std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        >,
    ) -> Result<String, RariError> {
        match mode {
            RenderMode::Ssr => self.render_route(route_match, context, request_context).await,
            RenderMode::RscNavigation => {
                self.render_route(route_match, context, request_context).await
            }
        }
    }

    pub async fn render_route_with_streaming(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<
            std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        >,
        return_rsc_on_fallback: bool,
    ) -> Result<RenderResult, RariError> {
        let cache_key = utils::generate_cache_key(route_match, context);

        if let Some(cached_html) = self.html_cache.get(cache_key) {
            return Ok(RenderResult::Static(cached_html));
        }

        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            if let Some(loading_entry) = &route_match.loading {
                let loading_id = utils::create_component_id(&loading_entry.file_path);
                Some(loading_id)
            } else {
                None
            }
        } else {
            None
        };

        let composition_script = self.build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            true,
        )?;

        let fallback_script = self.build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            false,
        )?;

        let runtime = {
            let renderer_guard = self.renderer.lock().await;
            Arc::clone(&renderer_guard.runtime)
        };

        let streaming_operation = async {
            let renderer_guard = self.renderer.lock().await;

            Self::enable_streaming_and_inject_lazy_resolver(&renderer_guard, false).await?;

            let mut streaming_renderer = crate::rsc::rendering::streaming::StreamingRenderer::new(
                Arc::clone(&renderer_guard.runtime),
            );

            let layout_structure = crate::rsc::rendering::layout::LayoutStructure::new();

            drop(renderer_guard);

            match streaming_renderer
                .start_streaming_with_composition(composition_script, layout_structure)
                .await
            {
                Ok(stream) => Ok(RenderResult::Streaming(stream)),
                Err(e) => {
                    error!("Streaming failed, falling back to static render: {}", e);

                    let renderer = self.renderer.lock().await;
                    let rsc_wire_format =
                        Self::execute_composition_and_serialize(&renderer, fallback_script).await?;

                    Self::validate_rsc_wire_format(&rsc_wire_format)?;

                    if return_rsc_on_fallback {
                        drop(renderer);
                        return Ok(RenderResult::Static(rsc_wire_format));
                    }

                    let html_renderer = crate::rsc::rendering::html::RscHtmlRenderer::new(
                        Arc::clone(&renderer.runtime),
                    );
                    drop(renderer);
                    let config =
                        Config::get().ok_or_else(|| RariError::internal("Config not available"))?;
                    let html = html_renderer.render_to_html(&rsc_wire_format, config).await?;

                    let is_not_found = route_match.not_found.is_some();
                    if is_not_found {
                        return Ok(RenderResult::Static(html));
                    }

                    self.html_cache.insert(cache_key, html.clone());
                    Ok(RenderResult::Static(html))
                }
            }
        };

        if let Some(ctx) = request_context {
            runtime.execute_with_request_context(ctx, streaming_operation).await
        } else {
            streaming_operation.await
        }
    }

    fn validate_html_structure(html: &str, route_match: &AppRouteMatch) -> Result<(), RariError> {
        let root_layout_path =
            route_match.layouts.iter().find(|l| l.is_root).map(|l| l.file_path.as_str());

        if html.contains("<div><html")
            || html.contains("\"div\",null,{\"children\":[\"$\",\"html\"")
        {
            let error_msg =
                error_messages::create_wrapped_html_error_message(route_match, root_layout_path);
            return Err(RariError::internal(error_msg));
        }

        Ok(())
    }

    async fn resolve_lazy_promises(
        renderer: &RscRenderer,
        rsc_wire_format: &mut String,
    ) -> Result<(), RariError> {
        let mut seen_lazy_promise_ids = rustc_hash::FxHashSet::default();

        loop {
            let pending_promises = {
                let mut serializer = renderer.serializer.lock();
                let promises = serializer.pending_lazy_promises.clone();
                serializer.pending_lazy_promises.clear();
                promises
            };

            if pending_promises.is_empty() {
                break;
            }

            for lazy_promise in pending_promises {
                if seen_lazy_promise_ids.contains(&lazy_promise.promise_id) {
                    continue;
                }
                seen_lazy_promise_ids.insert(lazy_promise.promise_id.clone());

                let resolve_script = format!(
                    "(async () => {{ return await globalThis['~rari'].lazy.resolve('{}'); }})()",
                    lazy_promise.promise_id
                );

                let result = renderer
                    .runtime
                    .execute_script(
                        format!("resolve_promise_{}", lazy_promise.promise_id),
                        resolve_script,
                    )
                    .await;

                match result {
                    Ok(result) => {
                        if let Some(success) = result.get("success").and_then(|v| v.as_bool())
                            && !success
                        {
                            let error_msg = result
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown error");
                            if error_msg.contains("Promise not found") {
                                continue;
                            }
                            tracing::error!(
                                "Failed to resolve lazy promise {}: {}",
                                lazy_promise.promise_id,
                                error_msg
                            );
                            continue;
                        }

                        let resolved_content = result.get("data").unwrap_or(&result);

                        let wire_format = {
                            let mut serializer = renderer.serializer.lock();
                            match serializer.serialize_rsc_json(resolved_content) {
                                Ok(wf) => wf,
                                Err(e) => {
                                    tracing::error!("Failed to serialize resolved content: {}", e);
                                    continue;
                                }
                            }
                        };

                        for line in wire_format.lines() {
                            if line.trim().is_empty() {
                                continue;
                            }
                            let line_to_append = if let Some(colon_pos) = line.find(':') {
                                if line.starts_with(|c: char| c.is_ascii_digit()) {
                                    let row_id_str = &line[..colon_pos];
                                    if row_id_str.parse::<usize>().is_ok() {
                                        if line.contains("I[") {
                                            line.to_string()
                                        } else {
                                            let content = &line[colon_pos + 1..];
                                            format!("{}:{}", lazy_promise.lazy_row_id, content)
                                        }
                                    } else {
                                        line.to_string()
                                    }
                                } else {
                                    line.to_string()
                                }
                            } else {
                                line.to_string()
                            };
                            rsc_wire_format.push('\n');
                            rsc_wire_format.push_str(&line_to_append);
                        }
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        if error_msg.contains("Promise not found") {
                            continue;
                        }
                        tracing::error!(
                            "Failed to resolve lazy promise {}: {}",
                            lazy_promise.promise_id,
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn execute_composition_and_serialize(
        renderer: &RscRenderer,
        composition_script: String,
    ) -> Result<String, RariError> {
        let promise_result = renderer
            .runtime
            .execute_script("compose_and_render".to_string(), composition_script)
            .await?;

        let result = if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
            promise_result
        } else {
            renderer
                .runtime
                .execute_script("get_result".to_string(), JS_GET_RESULT.to_string())
                .await?
        };

        let rsc_data = result.get("rsc_data").ok_or_else(|| {
            tracing::error!(
                "Failed to extract RSC data from result (keys: {:?})",
                result.as_object().map(|o| o.keys().collect::<Vec<_>>())
            );
            RariError::internal("No RSC data in render result")
        })?;

        let mut rsc_wire_format = {
            let mut serializer = renderer.serializer.lock();
            serializer.reset_for_new_request();
            serializer
                .serialize_rsc_json(rsc_data)
                .map_err(|e| RariError::internal(format!("Failed to serialize RSC data: {}", e)))?
        };

        Self::resolve_lazy_promises(renderer, &mut rsc_wire_format).await?;

        Ok(rsc_wire_format)
    }

    fn validate_rsc_wire_format(rsc_data: &str) -> Result<(), RariError> {
        if rsc_data.trim().is_empty() {
            let error_msg = error_messages::create_empty_rsc_error_message();
            return Err(RariError::internal(error_msg));
        }

        Ok(())
    }

    pub async fn render_route_streaming(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<RscStream, RariError> {
        let has_root_layout = route_match.layouts.iter().any(|l| l.is_root);

        if has_root_layout {
            let html = self.render_route(route_match, context, None).await?;

            Self::validate_rsc_wire_format(&html)?;

            let (tx, rx) = mpsc::channel(1);
            let _ = tx
                .send(crate::rsc::rendering::streaming::RscStreamChunk {
                    data: html.into_bytes(),
                    chunk_type: crate::rsc::rendering::streaming::RscChunkType::InitialShell,
                    row_id: 0,
                    is_final: true,
                    boundary_id: None,
                })
                .await;
            drop(tx);

            Ok(RscStream::new(rx))
        } else {
            self.render_route_streaming_progressive(route_match, context).await
        }
    }

    async fn render_route_streaming_progressive(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<RscStream, RariError> {
        let html = self.render_route(route_match, context, None).await?;

        Self::validate_rsc_wire_format(&html)?;

        let (tx, rx) = mpsc::channel(1);
        let _ = tx
            .send(crate::rsc::rendering::streaming::RscStreamChunk {
                data: html.into_bytes(),
                chunk_type: crate::rsc::rendering::streaming::RscChunkType::InitialShell,
                row_id: 0,
                is_final: true,
                boundary_id: None,
            })
            .await;
        drop(tx);

        Ok(RscStream::new(rx))
    }

    pub fn build_composition_script(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        loading_component_id: Option<&str>,
        use_suspense: bool,
    ) -> Result<String, RariError> {
        let page_props = utils::create_page_props(route_match, context).map_err(|e| {
            error!("Failed to create page props for route '{}': {}", route_match.route.path, e);
            RariError::internal(format!(
                "Failed to create page props for route '{}' (component: {}): {}",
                route_match.route.path, route_match.route.file_path, e
            ))
        })?;

        let page_props_json = serde_json::to_string(&page_props).map_err(|e| {
            error!("Failed to serialize page props for route '{}': {}", route_match.route.path, e);
            RariError::internal(format!(
                "Failed to serialize page props for route '{}' (component: {}): {}",
                route_match.route.path, route_match.route.file_path, e
            ))
        })?;

        let page_file_path = if let Some(ref not_found) = route_match.not_found {
            &not_found.file_path
        } else {
            &route_match.route.file_path
        };

        let page_component_id = utils::create_component_id(page_file_path);

        let page_render_script = if route_match.not_found.is_some() {
            JS_PAGE_RENDER_SIMPLE
                .cow_replace("{page_component_id}", &page_component_id)
                .cow_replace("{page_props_json}", "{}")
                .into_owned()
        } else if let Some(loading_id) = loading_component_id {
            let loading_file_path =
                route_match.loading.as_ref().map(|l| l.file_path.as_str()).unwrap_or("");

            JS_PAGE_RENDER_WITH_LOADING
                .cow_replace("{page_component_id}", &page_component_id)
                .cow_replace("{loading_id}", loading_id)
                .cow_replace("{page_props_json}", &page_props_json)
                .cow_replace("{use_suspense}", if use_suspense { "true" } else { "false" })
                .cow_replace("{route_file_path}", &route_match.route.file_path)
                .cow_replace("{loading_file_path}", loading_file_path)
                .into_owned()
        } else {
            JS_PAGE_RENDER_SIMPLE
                .cow_replace("{page_component_id}", &page_component_id)
                .cow_replace("{page_props_json}", &page_props_json)
                .into_owned()
        };

        let pathname_json =
            serde_json::to_string(&context.pathname).unwrap_or_else(|_| "null".to_string());

        let layouts: Vec<super::route_composer::LayoutInfo> = route_match
            .layouts
            .iter()
            .map(|layout| super::route_composer::LayoutInfo {
                component_id: utils::create_component_id(&layout.file_path),
                is_root: layout.is_root,
                file_path: layout.file_path.clone(),
            })
            .collect();

        let error_boundary = route_match.error.as_ref().map(|error| {
            let component_id = format!("src/{}", utils::create_component_id(&error.file_path));
            super::route_composer::ErrorBoundaryInfo {
                component_id,
                file_path: error.file_path.clone(),
            }
        });

        let metadata_json = context
            .metadata
            .as_ref()
            .and_then(|m| {
                serde_json::to_string(m)
                    .map_err(|e| {
                        tracing::debug!("Failed to serialize metadata: {}", e);
                        e
                    })
                    .ok()
            })
            .unwrap_or_else(|| "{}".to_string());

        let script = super::route_composer::RouteComposer::build_composition_script_with_error(
            &page_render_script,
            &layouts,
            &pathname_json,
            error_boundary.as_ref(),
            &metadata_json,
        );

        Ok(script)
    }

    pub async fn render_loading(
        &self,
        loading_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(loading_path);

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, None).await
    }

    pub async fn render_error(
        &self,
        error_path: &str,
        error: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(error_path);

        let mut props = serde_json::Map::new();
        props.insert("error".to_string(), Value::String(error.to_string()));
        props.insert(
            "reset".to_string(),
            Value::String("() => window.location.reload()".to_string()),
        );

        let props_json = serde_json::to_string(&props)
            .map_err(|e| RariError::internal(format!("Failed to serialize error props: {e}")))?;

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, Some(&props_json)).await
    }

    pub async fn render_not_found(
        &self,
        not_found_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(not_found_path);

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
        self.renderer.lock().await.register_component(component_id, component_code).await
    }
}
