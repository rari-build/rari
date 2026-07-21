#![expect(clippy::missing_errors_doc)]

use std::{env, future::Future, pin::Pin, sync::Arc};

use base64::{Engine, engine::general_purpose::STANDARD};
use bytes::Bytes;
use rari_error::RariError;
use serde_json::Value;
use tokio::sync::{Mutex, mpsc, oneshot};
use uuid::Uuid;

use super::{
    types::{ChunkedContentType, LayoutRenderContext, PageMetadata, RenderResult},
    utils,
};
use crate::{
    RscHtmlRenderer,
    rendering::{
        base::{RscRenderer, run_with_renderer_result},
        layout::{
            LayoutInfo, RouteComposer,
            route_composer::{ErrorBoundaryInfo, TemplateInfo},
        },
    },
    runtime::{
        JsExecutionRuntime,
        factory::{JsRuntimeInterface, StreamingSlotGuard},
    },
    server::{
        cache::{
            handler::{
                CacheError, CacheHandler, CacheHandlerRegistry, MemoryCacheHandler, MemoryConfig,
            },
            response::RouteCachePolicy,
        },
        config::{CacheLayerConfig, Config},
        middleware::request_context::RequestContext,
        rendering::metadata_injection::merge_streaming_head_content,
        routing::app_router::AppRouteMatch,
    },
    utils::path::path_to_file_url,
};

const LAYOUT_KEY_PREFIX: &str = "layout:";

fn should_use_layout_html_cache(
    context: &LayoutRenderContext,
    request_context: Option<&RequestContext>,
) -> bool {
    if request_context.is_some_and(|ctx| ctx.skip_layout_html_cache) {
        return false;
    }

    let Some(config) = Config::get() else {
        return true;
    };

    let cache_control = config.get_cache_control_for_route(&context.pathname);
    RouteCachePolicy::from_cache_control(cache_control, &context.pathname).enabled
}
const JS_GET_RESULT: &str = r"
globalThis['~rsc'].renderResult
";

use crate::rendering::base::constants::GET_RSC_BINARY_B64;

const FIZZ_STREAM_ERROR_HELPER: &str = r"
                        let rariErrorInjected = false;
                        async function injectRariErrorFromCaught() {
                            if (rariErrorInjected || caughtErrors.length === 0) return;
                            rariErrorInjected = true;
                            await globalThis['~rari']?.injectStreamError?.(caughtErrors, __RARI_STREAM_ID__);
                        }
";

const FIZZ_STREAM_ERROR_INJECTION: &str = r"
                        if (caughtErrors.length > 0) await injectRariErrorFromCaught();
";

const FIZZ_CHUNK_PUMP_HELPER: &str = r"
                        let rariStreamDisconnected = false;
                        async function rariPumpFizzChunk(text) {
                            if (!text || rariStreamDisconnected) return false;
                            try {
                                const status = Deno.core.ops.op_fizz_chunk_try(__RARI_STREAM_ID__, text);
                                if (status === 0) return true;
                                if (status === 2) {
                                    rariStreamDisconnected = true;
                                    return false;
                                }
                                await Deno.core.ops.op_fizz_chunk(__RARI_STREAM_ID__, text);
                                return true;
                            } catch (e) {
                                if (String(e?.message || e).includes('disconnected')) {
                                    rariStreamDisconnected = true;
                                    return false;
                                }
                                throw e;
                            }
                        }
";

enum RscNavPayload {
    Binary(Vec<u8>),
    Text(String),
}

pub struct LayoutHtmlCache {
    handler: Arc<dyn CacheHandler>,
    default_ttl_secs: u64,
}

impl Default for LayoutHtmlCache {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutHtmlCache {
    pub fn new() -> Self {
        Self::with_ttl(
            Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
                max_entries: 5000,
                default_ttl: 3600,
            })),
            3600,
        )
    }

    pub fn from_config(layer: &CacheLayerConfig, registry: &CacheHandlerRegistry) -> Self {
        let handler = registry.resolve(&layer.handler);
        Self::with_ttl(handler, layer.default_ttl_secs)
    }

    pub fn with_handler(handler: Arc<dyn CacheHandler>) -> Self {
        Self::with_ttl(handler, 3600)
    }

    pub fn with_ttl(handler: Arc<dyn CacheHandler>, default_ttl_secs: u64) -> Self {
        Self { handler, default_ttl_secs }
    }

    fn namespaced(key: u64) -> String {
        format!("{LAYOUT_KEY_PREFIX}{key}")
    }

    pub async fn get(&self, key: u64) -> Option<String> {
        let ns_key = Self::namespaced(key);
        let bytes = match self.handler.get(&ns_key).await {
            Ok(Some(b)) => b,
            Ok(None) => return None,
            Err(e) => {
                tracing::debug!(key = %ns_key, error = %e, "layout cache get failed");
                return None;
            }
        };
        match String::from_utf8(bytes) {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::debug!(key = %ns_key, error = %e, "layout cache value not valid utf-8");
                None
            }
        }
    }

    pub async fn insert(&self, key: u64, html: String) -> Result<(), CacheError> {
        self.insert_with_tags(key, html, &[]).await
    }

    pub async fn insert_with_tags(
        &self,
        key: u64,
        html: String,
        tags: &[String],
    ) -> Result<(), CacheError> {
        self.handler
            .set_with_tags(&Self::namespaced(key), html.into_bytes(), self.default_ttl_secs, tags)
            .await?;
        Ok(())
    }

    pub async fn clear(&self) -> Result<(), CacheError> {
        self.handler.clear_prefix(LAYOUT_KEY_PREFIX).await?;
        Ok(())
    }

    pub async fn invalidate_by_tag(&self, tag: &str) -> Result<(), CacheError> {
        self.handler.invalidate_by_tag(tag).await
    }
}

fn wrap_streaming_script(request_id: Option<&str>, stream_id: &str, script: &str) -> String {
    let request_id_json = serde_json::to_string(request_id.unwrap_or(stream_id))
        .unwrap_or_else(|_| "\"\"".to_string());
    let stream_id_json = serde_json::to_string(stream_id).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r"(async function() {{
            const __RARI_REQUEST_ID__ = {request_id_json};
            const __RARI_STREAM_ID__ = {stream_id_json};
            const storage = globalThis['~rari']?.requestStorage;
            const body = async () => await ({script});
            if (storage && typeof storage.run === 'function') {{
                return await storage.run(
                    {{ requestId: __RARI_REQUEST_ID__, streamId: __RARI_STREAM_ID__ }},
                    body,
                );
            }}
            return await body();
        }})()"
    )
}

async fn run_streaming_script(
    runtime: &Arc<JsExecutionRuntime>,
    request_context: Option<Arc<RequestContext>>,
    script_name: String,
    stream_id: String,
    script: String,
    chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
) -> Result<(), RariError> {
    let (completion, _stream_lease) = queue_streaming_script(
        runtime,
        request_context,
        script_name,
        stream_id,
        script,
        chunk_sender,
    )
    .await?;
    completion.await
}

async fn queue_streaming_script(
    runtime: &Arc<JsExecutionRuntime>,
    request_context: Option<Arc<RequestContext>>,
    script_name: String,
    stream_id: String,
    script: String,
    chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
) -> Result<
    (Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>, StreamingSlotGuard),
    RariError,
> {
    let err_sender = chunk_sender.clone();
    let (handle, stream_lease) = match runtime.pick_runtime_for_streaming().await {
        Ok(picked) => picked,
        Err(e) => {
            let _ = err_sender.send(Err(e.clone())).await;
            return Err(e);
        }
    };
    if let Some(context) = request_context {
        let request_id = context.request_id().to_string();
        let wrapped = wrap_streaming_script(Some(&request_id), &stream_id, &script);
        let completion = match handle
            .queue_script_for_streaming(
                stream_id,
                script_name,
                wrapped,
                chunk_sender,
                Some(Arc::clone(&context)),
            )
            .await
        {
            Ok(completion) => completion,
            Err(e) => {
                let _ = err_sender.send(Err(e.clone())).await;
                return Err(e);
            }
        };
        let completion = Box::pin(async move {
            let result = completion.await;
            let clear_result = handle.unregister_request_context(&request_id).await;
            result?;
            clear_result
        }) as Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;
        Ok((completion, stream_lease))
    } else {
        let wrapped = wrap_streaming_script(None, &stream_id, &script);
        let completion = match handle
            .queue_script_for_streaming(stream_id, script_name, wrapped, chunk_sender, None)
            .await
        {
            Ok(completion) => completion,
            Err(e) => {
                let _ = err_sender.send(Err(e.clone())).await;
                return Err(e);
            }
        };
        Ok((completion, stream_lease))
    }
}

pub struct LayoutRenderer {
    renderer: Arc<Mutex<RscRenderer>>,
    html_cache: Arc<LayoutHtmlCache>,
}

impl LayoutRenderer {
    pub fn new(renderer: Arc<Mutex<RscRenderer>>) -> Self {
        Self { renderer, html_cache: Arc::new(LayoutHtmlCache::new()) }
    }

    pub fn with_shared_cache(
        renderer: Arc<Mutex<RscRenderer>>,
        html_cache: Arc<LayoutHtmlCache>,
    ) -> Self {
        Self { renderer, html_cache }
    }

    pub fn create_shared_cache() -> Arc<LayoutHtmlCache> {
        Arc::new(LayoutHtmlCache::new())
    }

    pub fn create_shared_cache_from_config(
        layer: &CacheLayerConfig,
        registry: &CacheHandlerRegistry,
    ) -> Arc<LayoutHtmlCache> {
        Arc::new(LayoutHtmlCache::from_config(layer, registry))
    }

    pub async fn check_page_not_found(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<bool, RariError> {
        self.check_page_not_found_on(route_match, context, None).await
    }

    /// Prefer [`Self::check_page_not_found_on`] with a sticky runtime when the caller
    /// already holds a pool slot lease — using the pool here would re-acquire it and deadlock.
    pub async fn check_page_not_found_on(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        sticky_runtime: Option<&Arc<dyn JsRuntimeInterface>>,
    ) -> Result<bool, RariError> {
        let page_props = utils::create_page_props(route_match, context)?;
        let page_props_json = serde_json::to_string(&page_props)?;

        let dist_server_path = env::current_dir()
            .ok()
            .map(|p| p.join("dist/server"))
            .and_then(|p| p.canonicalize().ok());

        let Some(base_path) = dist_server_path else {
            return Ok(false);
        };

        let page_file_path = utils::component_dist_path(&base_path, &route_match.route.file_path);

        if !page_file_path.exists() {
            return Ok(false);
        }

        let page_path = path_to_file_url(&page_file_path);

        let check_script = format!(
            r#"
            (async () => {{
                try {{
                    const module = await import("{page_path}");

                    if (typeof module.getData === 'function') {{
                        const pageProps = {page_props_json};
                        const result = await module.getData(pageProps);
                        return {{ notFound: result?.notFound === true }};
                    }}

                    return {{ notFound: false }};
                }} catch (error) {{
                    console.error('[check_page_not_found] Error:', error);
                    return {{ notFound: false }};
                }}
            }})()
            "#
        );

        let result = if let Some(runtime) = sticky_runtime {
            runtime.execute_script("check_not_found".to_string(), check_script).await?
        } else {
            let renderer = self.renderer.lock().await;
            let runtime = Arc::clone(&renderer.runtime);
            drop(renderer);
            runtime.execute_script("check_not_found".to_string(), check_script).await?
        };

        let not_found =
            result.get("notFound").and_then(serde_json::Value::as_bool).unwrap_or(false);

        Ok(not_found)
    }

    pub async fn render_route(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<Arc<RequestContext>>,
    ) -> Result<String, RariError> {
        self.render_route_with_mode_internal(route_match, context, request_context).await
    }

    pub async fn render_route_for_fizz_streaming(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<Arc<RequestContext>>,
    ) -> Result<String, RariError> {
        self.render_route_with_mode_internal(route_match, context, request_context).await
    }

    async fn render_route_with_mode_internal(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<Arc<RequestContext>>,
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

        let composition_script = Self::build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            loading_component_id.is_some(),
            false,
        )?;

        let renderer = Arc::clone(&self.renderer);
        let runtime = {
            run_with_renderer_result(Arc::clone(&renderer), move |renderer| async move {
                renderer.ensure_rsc_pipeline().await?;
                Ok(Arc::clone(&renderer.runtime))
            })
            .await?
        };

        if let Some(ctx) = request_context {
            runtime
                .with_request_context(ctx, move |rt| async move {
                    Self::execute_composition_and_serialize_on(None, Some(rt), composition_script)
                        .await
                })
                .await
        } else {
            run_with_renderer_result(renderer, move |renderer| async move {
                Self::execute_composition_and_serialize(&renderer, composition_script).await
            })
            .await
        }
    }

    pub async fn render_route_by_mode(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<Arc<RequestContext>>,
    ) -> Result<String, RariError> {
        self.render_route(route_match, context, request_context).await
    }

    pub async fn compose_route_for_action_refresh(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Arc<RequestContext>,
        action_refresh_search: String,
    ) -> Result<(), RariError> {
        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            route_match
                .loading
                .as_ref()
                .map(|loading_entry| utils::create_component_id(&loading_entry.file_path))
        } else {
            None
        };

        let composition_script = Self::build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            false,
            true,
        )?;

        let set_search_script = format!(
            "globalThis['~rari'] = globalThis['~rari'] || {{}}; globalThis['~rari'].isActionRefreshCompose = true; globalThis['~rari'].actionRefreshSearch = {};",
            serde_json::to_string(&action_refresh_search)
                .map_err(|e| RariError::serialization(e.to_string()))?
        );

        let runtime = {
            let renderer = Arc::clone(&self.renderer);
            run_with_renderer_result(renderer, move |renderer| async move {
                renderer.ensure_rsc_pipeline().await?;
                Ok(Arc::clone(&renderer.runtime))
            })
            .await?
        };

        runtime
            .with_request_context(request_context, move |rt| async move {
                Self::run_action_refresh_scripts(&rt, set_search_script, composition_script).await
            })
            .await
    }

    pub async fn compose_route_for_action_refresh_on(
        &self,
        runtime: &Arc<dyn JsRuntimeInterface>,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        action_refresh_search: String,
    ) -> Result<(), RariError> {
        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            route_match
                .loading
                .as_ref()
                .map(|loading_entry| utils::create_component_id(&loading_entry.file_path))
        } else {
            None
        };

        let composition_script = Self::build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            false,
            true,
        )?;

        let set_search_script = format!(
            "globalThis['~rari'] = globalThis['~rari'] || {{}}; globalThis['~rari'].isActionRefreshCompose = true; globalThis['~rari'].actionRefreshSearch = {};",
            serde_json::to_string(&action_refresh_search)
                .map_err(|e| RariError::serialization(e.to_string()))?
        );

        Self::run_action_refresh_scripts(runtime, set_search_script, composition_script).await
    }

    async fn run_action_refresh_scripts(
        runtime: &Arc<dyn JsRuntimeInterface>,
        set_search_script: String,
        composition_script: String,
    ) -> Result<(), RariError> {
        runtime.execute_script("set_action_refresh_search".to_string(), set_search_script).await?;
        runtime.execute_script("action_refresh_compose".to_string(), composition_script).await?;
        Ok(())
    }

    #[expect(clippy::too_many_lines)]
    pub async fn render_route_with_streaming(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<Arc<RequestContext>>,
        return_rsc_on_fallback: bool,
        metadata_rx: Option<oneshot::Receiver<Option<PageMetadata>>>,
    ) -> Result<RenderResult, RariError> {
        let cookie_header = request_context.as_deref().and_then(|ctx| ctx.cookie_header.as_deref());
        let cache_key = utils::generate_cache_key(route_match, context, cookie_header);
        let layout_cache_enabled =
            should_use_layout_html_cache(context, request_context.as_deref());

        if layout_cache_enabled
            && !return_rsc_on_fallback
            && let Some(cached_html) = self.html_cache.get(cache_key).await
        {
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

        let needs_streaming = loading_component_id.is_some();

        if return_rsc_on_fallback {
            if needs_streaming {
                let (chunk_sender, chunk_receiver) =
                    mpsc::channel::<Result<Vec<u8>, RariError>>(128);

                let stream_id = Uuid::new_v4().to_string();
                let composition_script = Self::build_composition_script_with_stream(
                    route_match,
                    context,
                    loading_component_id.as_deref(),
                    true,
                    true,
                    Some(&stream_id),
                )?;

                let script = format!(
                    r"(async function() {{
                        {FIZZ_CHUNK_PUMP_HELPER}
                        try {{
                        try {{ {composition_script} }} catch(e) {{
                            console.error('[rari] Composition error in RSC streaming nav:', e);
                        }}

                        const byStream = globalThis['~rari']?.capturedByStream;
                        const capturedElement = (byStream && __RARI_STREAM_ID__ in byStream)
                            ? byStream[__RARI_STREAM_ID__]
                            : globalThis['~rari']?.capturedElement;
                        if (byStream && __RARI_STREAM_ID__ in byStream)
                            delete byStream[__RARI_STREAM_ID__];
                        if (!capturedElement) {{
                            return;
                        }}

                        const pumpRsc = globalThis['~rari']?.pumpRscElementStream;
                        if (typeof pumpRsc !== 'function') {{
                            throw new Error('[rari] pumpRscElementStream not loaded');
                        }}

                        await pumpRsc(capturedElement, rariPumpFizzChunk);
                        }} catch(e) {{
                            console.error('[rari] RSC streaming navigation fatal error:', e);
                        }} finally {{
                            Deno.core.ops.op_fizz_done(__RARI_STREAM_ID__);
                        }}
                    }})()",
                );

                let renderer = Arc::clone(&self.renderer);
                let request_context_for_stream = request_context.clone();
                tokio::spawn(async move {
                    let prepared = run_with_renderer_result(renderer, |renderer| async move {
                        renderer.ensure_streaming_pipeline().await?;
                        Ok(Arc::clone(&renderer.runtime))
                    })
                    .await;

                    match prepared {
                        Ok(runtime) => {
                            if let Err(e) = run_streaming_script(
                                &runtime,
                                request_context_for_stream,
                                "rsc_streaming_nav".to_string(),
                                stream_id,
                                script,
                                chunk_sender,
                            )
                            .await
                            {
                                tracing::error!("RSC streaming navigation failed: {e}");
                            }
                        }
                        Err(e) => {
                            tracing::error!("RSC streaming navigation setup failed: {e}");
                            let _ = chunk_sender
                                .send(Err(RariError::internal(format!(
                                    "RSC streaming setup failed: {e}"
                                ))))
                                .await;
                        }
                    }
                });

                return Ok(RenderResult::Chunked {
                    content_type: ChunkedContentType::RscFlight,
                    shell: Bytes::new(),
                    closing: Bytes::new(),
                    chunks: chunk_receiver,
                });
            }

            let composition_script = Self::build_composition_script(
                route_match,
                context,
                loading_component_id.as_deref(),
                loading_component_id.is_some(),
                false,
            )?;

            let rsc_payload = {
                let runtime = {
                    run_with_renderer_result(
                        Arc::clone(&self.renderer),
                        move |renderer| async move {
                            renderer.ensure_rsc_pipeline().await?;
                            Ok(Arc::clone(&renderer.runtime))
                        },
                    )
                    .await?
                };

                if let Some(ctx) = request_context {
                    runtime
                        .with_request_context(ctx, move |rt| async move {
                            let result = Self::run_composition_on(
                                None,
                                Some(Arc::clone(&rt)),
                                composition_script,
                            )
                            .await?;

                            if let Some(bytes) = Self::capture_last_rsc_binary_on(Some(rt)).await?
                                && !bytes.is_empty()
                            {
                                return Ok(RscNavPayload::Binary(bytes));
                            }

                            Ok(RscNavPayload::Text(Self::extract_flight_text(&result)?))
                        })
                        .await?
                } else {
                    let handle = runtime.pick_runtime().await?;
                    let rt = Arc::clone(handle.runtime());
                    let result =
                        Self::run_composition_on(None, Some(Arc::clone(&rt)), composition_script)
                            .await?;

                    if let Some(bytes) = Self::capture_last_rsc_binary_on(Some(rt)).await?
                        && !bytes.is_empty()
                    {
                        RscNavPayload::Binary(bytes)
                    } else {
                        RscNavPayload::Text(Self::extract_flight_text(&result)?)
                    }
                }
            };

            match rsc_payload {
                RscNavPayload::Binary(bytes) => Ok(RenderResult::StaticBinary(bytes)),
                RscNavPayload::Text(flight) => Ok(RenderResult::Static(flight)),
            }
        } else {
            let config =
                Config::get().ok_or_else(|| RariError::internal("Config not available"))?;

            if needs_streaming {
                let (chunk_sender, chunk_receiver) =
                    mpsc::channel::<Result<Vec<u8>, RariError>>(128);

                let stream_id = Uuid::new_v4().to_string();
                let shell = Bytes::from_static(b"<!DOCTYPE html>");
                let closing = Bytes::new();

                let renderer = Arc::clone(&self.renderer);
                let route_match = route_match.clone();
                let mut context = context.clone();
                let loading_component_id = loading_component_id.clone();
                let config = config.clone();
                let request_context_for_stream = request_context.clone();

                if let Some(mut rx) = metadata_rx {
                    match rx.try_recv() {
                        Ok(metadata) => context.metadata = metadata,
                        Err(
                            oneshot::error::TryRecvError::Empty
                            | oneshot::error::TryRecvError::Closed,
                        ) => {}
                    }
                }

                let composition_script = match Self::build_composition_script_with_stream(
                    &route_match,
                    &context,
                    loading_component_id.as_deref(),
                    true,
                    true,
                    Some(&stream_id),
                ) {
                    Ok(script) => script,
                    Err(e) => {
                        tracing::error!("Fizz streaming composition error: {e}");
                        let _ = chunk_sender
                            .send(Err(RariError::internal(format!(
                                "Fizz streaming composition failed: {e}"
                            ))))
                            .await;
                        return Ok(RenderResult::Chunked {
                            content_type: ChunkedContentType::Html,
                            shell,
                            closing,
                            chunks: chunk_receiver,
                        });
                    }
                };

                let prepared = run_with_renderer_result(renderer, move |renderer| {
                    async move {
                        renderer.ensure_streaming_pipeline().await?;

                        let html_renderer = RscHtmlRenderer::new(Arc::clone(&renderer.runtime));
                        let css_links = RscHtmlRenderer::css_links_for_route(&route_match);
                        let cache_template = config.rsc_html.cache_template;
                        let is_dev_mode = config.is_development();
                        let template =
                            html_renderer.load_template(cache_template, is_dev_mode).await?;
                        let template = RscHtmlRenderer::inject_css_links(&template, &css_links);

                        let head_content = {
                            let template_head = template
                                .find("<head>")
                                .and_then(|start| {
                                    template.find("</head>").map(|end| &template[start + 6..end])
                                })
                                .unwrap_or("");
                            merge_streaming_head_content(
                                template_head,
                                context.streaming_head_extra.as_deref(),
                            )
                        };

                        let head_content_json = serde_json::to_string(&head_content)
                            .unwrap_or_else(|_| "\"\"".to_string());

                        let script = format!(
                            r"(async function() {{
                        let caughtErrors = [];
                        {FIZZ_STREAM_ERROR_HELPER}
                        try {{
                        try {{ await ({composition_script}); }} catch(e) {{
                            console.error('[rari] Composition error in streaming:', e);
                        }}

                        const byStream = globalThis['~rari']?.capturedByStream;
                        const capturedElement = (byStream && __RARI_STREAM_ID__ in byStream)
                            ? byStream[__RARI_STREAM_ID__]
                            : globalThis['~rari']?.capturedElement;
                        if (byStream && __RARI_STREAM_ID__ in byStream)
                            delete byStream[__RARI_STREAM_ID__];
                        if (!capturedElement) {{
                            Deno.core.ops.op_fizz_done(__RARI_STREAM_ID__);
                            return;
                        }}

                        const renderStreaming = globalThis['~rari']?.renderStreamingDocument;
                        if (typeof renderStreaming !== 'function') {{
                            throw new Error('[rari] streaming_fizz.ts not loaded');
                        }}

                        await renderStreaming({{
                            capturedElement,
                            headContent: {head_content_json},
                            caughtErrors,
                            streamId: __RARI_STREAM_ID__,
                        }});

                        {FIZZ_STREAM_ERROR_INJECTION}

                        Deno.core.ops.op_fizz_done(__RARI_STREAM_ID__);

                        }} catch(outerError) {{
                            console.error('[rari] Fizz streaming pipeline fatal error:', outerError);
                            const displayError = caughtErrors.length > 0 ? caughtErrors[0] : outerError;
                            const errMsg = String(displayError?.message || outerError?.message || 'Unknown error').split('<').join('&lt;');
                            const errorHtml = '<!doctype html><html><head></head><body><div id=root><div class=rari-error style=color:red;border:1px_solid_red;padding:10px;border-radius:4px;background-color:#fff5f5><strong>Error loading content: </strong>' + errMsg + '</div></div></body></html>';
                            await Deno.core.ops.op_fizz_chunk(__RARI_STREAM_ID__, errorHtml);
                            Deno.core.ops.op_fizz_done(__RARI_STREAM_ID__);
                        }}
                    }})()",
                        );

                        Ok((Arc::clone(&renderer.runtime), script))
                    }
                })
                .await;

                match prepared {
                    Ok((runtime, script)) => {
                        match queue_streaming_script(
                            &runtime,
                            request_context_for_stream,
                            "fizz_direct_stream".to_string(),
                            stream_id,
                            script,
                            chunk_sender,
                        )
                        .await
                        {
                            Ok((completion, stream_lease)) => {
                                tokio::spawn(async move {
                                    let _stream_lease = stream_lease;
                                    if let Err(e) = completion.await {
                                        tracing::error!("Fizz direct streaming error: {e}");
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Fizz streaming queue error: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Fizz streaming setup error: {e}");
                        let _ = chunk_sender
                            .send(Err(RariError::internal(format!(
                                "Fizz streaming setup failed: {e}"
                            ))))
                            .await;
                    }
                }

                return Ok(RenderResult::Chunked {
                    content_type: ChunkedContentType::Html,
                    shell,
                    closing,
                    chunks: chunk_receiver,
                });
            }

            let html = {
                let composition_script = Self::build_composition_script(
                    route_match,
                    context,
                    loading_component_id.as_deref(),
                    loading_component_id.is_some(),
                    true,
                )?;

                let route_match = route_match.clone();
                let config = config.clone();
                let request_context = request_context.clone();

                let prepared = {
                    let composition_script = composition_script.clone();
                    run_with_renderer_result(Arc::clone(&self.renderer), move |renderer| async move {
                        renderer.ensure_streaming_pipeline().await?;

                        let runtime = Arc::clone(&renderer.runtime);
                        let html_renderer =
                            Arc::new(RscHtmlRenderer::new(Arc::clone(&renderer.runtime)));
                        let css_links = RscHtmlRenderer::css_links_for_route(&route_match);
                        let cache_template = config.rsc_html.cache_template;
                        let is_dev_mode = config.is_development();
                        let template =
                            html_renderer.load_template(cache_template, is_dev_mode).await?;
                        let template = RscHtmlRenderer::inject_css_links(&template, &css_links);

                        let head_content = template
                            .find("<head>")
                            .and_then(|start| {
                                template.find("</head>").map(|end| &template[start + 6..end])
                            })
                            .unwrap_or("")
                            .to_string();

                        let head_content_json = serde_json::to_string(&head_content)
                            .unwrap_or_else(|_| "\"\"".to_string());

                        let script = format!(
                            r"(async function() {{
                        let caughtErrors = [];
                        try {{
                            try {{ await ({composition_script}); }} catch(e) {{
                                console.error('[rari] Composition error in static:', e);
                            }}

                            const capturedElement = globalThis['~rari']?.capturedElement;
                            if (!capturedElement) {{
                                return {{ ok: false, error: 'No captured element' }};
                            }}

                            const renderStatic = globalThis['~rari']?.renderStaticDocument;
                            if (typeof renderStatic !== 'function') {{
                                return {{ ok: false, error: 'renderStaticDocument not loaded' }};
                            }}

                            const html = await renderStatic({{
                                capturedElement,
                                headContent: {head_content_json},
                                caughtErrors,
                            }});

                            const isDynamic = (globalThis['~rari']?.useCacheDynamicDepth ?? 0) > 0;
                            let pageCacheTags = [];
                            if (!isDynamic) {{
                                const tags = new Set(
                                    globalThis['~rari']?.pageCacheTags ? [...globalThis['~rari'].pageCacheTags] : [],
                                );
                                const fromRegistry = globalThis.__rariGetActiveUseCacheTags?.() ?? [];
                                for (const tag of fromRegistry)
                                    tags.add(tag);
                                pageCacheTags = [...tags];
                            }}

                            return {{ ok: true, html, isDynamic, pageCacheTags }};
                        }} catch(e) {{
                            return {{ ok: false, error: String(e?.message || e) }};
                        }}
                    }})()",
                        );

                        Ok((
                            runtime,
                            html_renderer,
                            css_links,
                            script,
                            cache_template,
                            is_dev_mode,
                        ))
                    })
                    .await?
                };

                let (runtime, html_renderer, css_links, script, cache_template, is_dev_mode) =
                    prepared;

                let render_static = {
                    let script = script.clone();
                    let html_renderer = Arc::clone(&html_renderer);
                    let css_links = css_links.clone();
                    move |rt: Arc<dyn JsRuntimeInterface>| {
                        let script = script.clone();
                        let html_renderer = Arc::clone(&html_renderer);
                        let css_links = css_links.clone();
                        async move {
                            let result = rt
                                .execute_script("static_document_render".to_string(), script)
                                .await?;

                            let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
                            if !ok {
                                let err = result
                                    .get("error")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                return Err(RariError::internal(format!(
                                    "Static document render failed: {err}"
                                )));
                            }

                            let html = result
                                .get("html")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();

                            let assembled = html_renderer
                                .assemble_document(html, cache_template, is_dev_mode, &css_links)
                                .await?;

                            let is_dynamic =
                                result.get("isDynamic").and_then(Value::as_bool).unwrap_or(false);

                            let page_cache_tags = if is_dynamic {
                                Vec::new()
                            } else {
                                result
                                    .get("pageCacheTags")
                                    .and_then(|v| {
                                        v.as_array().map(|arr| {
                                            arr.iter()
                                                .filter_map(|item| {
                                                    item.as_str().map(str::to_string)
                                                })
                                                .collect::<Vec<_>>()
                                        })
                                    })
                                    .unwrap_or_default()
                            };

                            Ok((assembled, is_dynamic, page_cache_tags))
                        }
                    }
                };

                if let Some(ctx) = request_context {
                    runtime.with_request_context(ctx, render_static).await?
                } else {
                    let handle = runtime.pick_runtime().await?;
                    render_static(Arc::clone(handle.runtime())).await?
                }
            };

            let (html, is_dynamic_render, page_cache_tags) = html;

            if layout_cache_enabled && route_match.not_found.is_none() && !is_dynamic_render {
                let mut layout_cache_tags = page_cache_tags;
                let route_path = route_match.route.path.clone();
                if !layout_cache_tags.iter().any(|tag| tag == &route_path) {
                    layout_cache_tags.push(route_path);
                }
                let _ = self
                    .html_cache
                    .insert_with_tags(cache_key, html.clone(), &layout_cache_tags)
                    .await;
            }

            Ok(RenderResult::Static(html))
        }
    }

    async fn execute_composition_and_serialize(
        renderer: &RscRenderer,
        composition_script: String,
    ) -> Result<String, RariError> {
        Self::execute_composition_and_serialize_on(Some(renderer), None, composition_script).await
    }

    async fn execute_composition_and_serialize_on(
        renderer: Option<&RscRenderer>,
        runtime: Option<Arc<dyn JsRuntimeInterface>>,
        composition_script: String,
    ) -> Result<String, RariError> {
        let result = Self::run_composition_on(renderer, runtime, composition_script).await?;
        Self::extract_flight_text(&result)
    }

    async fn run_composition_on(
        renderer: Option<&RscRenderer>,
        runtime: Option<Arc<dyn JsRuntimeInterface>>,
        composition_script: String,
    ) -> Result<Value, RariError> {
        let rt = if let Some(rt) = runtime {
            rt
        } else {
            let renderer = renderer.ok_or_else(|| {
                RariError::internal("run_composition_on requires renderer or runtime")
            })?;
            renderer.ensure_rsc_pipeline().await?;
            let handle = renderer.runtime.pick_runtime().await?;
            Arc::clone(handle.runtime())
        };

        let promise_result =
            rt.execute_script("compose_and_render".to_string(), composition_script).await?;

        if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
            Ok(promise_result)
        } else {
            rt.execute_script("get_result".to_string(), JS_GET_RESULT.to_string()).await
        }
    }

    async fn capture_last_rsc_binary_on(
        runtime: Option<Arc<dyn JsRuntimeInterface>>,
    ) -> Result<Option<Vec<u8>>, RariError> {
        let Some(rt) = runtime else {
            return Ok(None);
        };
        let result = rt
            .execute_script("get_rsc_binary_b64".to_string(), GET_RSC_BINARY_B64.to_string())
            .await?;

        Ok(result.as_str().and_then(|b64| STANDARD.decode(b64).ok()))
    }

    fn extract_flight_text(result: &Value) -> Result<String, RariError> {
        let rsc_data = result.get("rsc_data").ok_or_else(|| {
            tracing::error!(
                "Failed to extract RSC data from result (keys: {:?})",
                result.as_object().map(|o| o.keys().collect::<Vec<_>>())
            );
            RariError::internal("No RSC data in render result")
        })?;

        if let Some(flight_protocol_str) = rsc_data.as_str() {
            if flight_protocol_str.trim().is_empty() {
                return Err(RariError::internal("No RSC data in render result"));
            }
            return Ok(flight_protocol_str.to_string());
        }

        Err(RariError::internal(
            "RSC render did not produce a Flight protocol string. The renderer may not be loaded."
                .to_string(),
        ))
    }

    pub fn build_composition_script(
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        loading_component_id: Option<&str>,
        use_suspense: bool,
        defer_rsc: bool,
    ) -> Result<String, RariError> {
        Self::build_composition_script_with_stream(
            route_match,
            context,
            loading_component_id,
            use_suspense,
            defer_rsc,
            None,
        )
    }

    #[expect(clippy::too_many_lines)]
    pub fn build_composition_script_with_stream(
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        loading_component_id: Option<&str>,
        use_suspense: bool,
        defer_rsc: bool,
        capture_stream_id: Option<&str>,
    ) -> Result<String, RariError> {
        let page_props = utils::create_page_props(route_match, context).map_err(|e| {
            tracing::error!(
                "Failed to create page props for route '{}': {}",
                route_match.route.path,
                e
            );
            RariError::internal(format!(
                "Failed to create page props for route '{}' (component: {}): {}",
                route_match.route.path, route_match.route.file_path, e
            ))
        })?;

        let page_props_json = serde_json::to_string(&page_props).map_err(|e| {
            tracing::error!(
                "Failed to serialize page props for route '{}': {}",
                route_match.route.path,
                e
            );
            RariError::internal(format!(
                "Failed to serialize page props for route '{}' (component: {}): {}",
                route_match.route.path, route_match.route.file_path, e
            ))
        })?;

        let page_component_id = if let Some(ref not_found) = route_match.not_found {
            utils::create_component_id(&not_found.file_path)
        } else {
            utils::create_component_id(&route_match.route.file_path)
        };

        let page_render_script = if route_match.not_found.is_some() {
            format!(
                r#"
                const PageComponent = globalThis["{page_component_id}"];
                if (!PageComponent || typeof PageComponent !== 'function') {{
                    throw new Error('Page component {page_component_id} not found');
                }}
                const pageElement = React.createElement(PageComponent, {{}});
                timings.pageRender = performance.now() - startPageRender;
                "#
            )
        } else if let Some(loading_id) = loading_component_id {
            let loading_file_path =
                route_match.loading.as_ref().map(|l| l.file_path.as_str()).unwrap_or("");

            format!(
                r#"
                const PageComponent = globalThis["{}"];
                if (!PageComponent || typeof PageComponent !== 'function') {{
                    throw new Error('Page component {} not found in route {}');
                }}

                const LoadingComponent = globalThis["{}"];
                if (!LoadingComponent || typeof LoadingComponent !== 'function') {{
                    throw new Error('Loading component {} not found in route {}');
                }}

                const pageProps = {};
                const useSuspense = {};
                const isAsync = PageComponent.constructor.name === 'AsyncFunction';

                const pageElement = (isAsync && useSuspense)
                    ? React.createElement(
                        React.Suspense,
                        {{ fallback: React.createElement(LoadingComponent, {{}}) }},
                        React.createElement(PageComponent, pageProps)
                      )
                    : React.createElement(PageComponent, pageProps);

                timings.pageRender = performance.now() - startPageRender;
                "#,
                page_component_id,
                page_component_id,
                route_match.route.file_path,
                loading_id,
                loading_id,
                loading_file_path,
                page_props_json,
                if use_suspense { "true" } else { "false" }
            )
        } else {
            format!(
                r#"
                const PageComponent = globalThis["{page_component_id}"];
                if (!PageComponent || typeof PageComponent !== 'function') {{
                    throw new Error('Page component {page_component_id} not found');
                }}
                const pageProps = {page_props_json};
                const pageElement = React.createElement(PageComponent, pageProps);
                timings.pageRender = performance.now() - startPageRender;
                "#
            )
        };

        let pathname_json =
            serde_json::to_string(&context.pathname).unwrap_or_else(|_| "null".to_string());
        let template_key_json = utils::template_key_json(context);
        let action_post_url =
            utils::format_action_post_url(&context.pathname, &context.search_params);
        let action_post_url_json =
            serde_json::to_string(&action_post_url).unwrap_or_else(|_| "\"/\"".to_string());

        let layouts: Vec<LayoutInfo> = route_match
            .layouts
            .iter()
            .map(|layout| LayoutInfo {
                component_id: utils::create_component_id(&layout.file_path),
                is_root: layout.is_root,
                file_path: layout.file_path.clone(),
            })
            .collect();

        let templates: Vec<TemplateInfo> = route_match
            .templates
            .iter()
            .map(|template| TemplateInfo {
                component_id: utils::create_component_id(&template.file_path),
                client_component_id: utils::normalize_route_component_path_public(
                    &template.file_path,
                ),
                file_path: template.file_path.clone(),
            })
            .collect();

        let error_boundary = route_match.error.as_ref().map(|error| {
            let component_id = utils::create_client_component_id(&error.file_path);
            ErrorBoundaryInfo { component_id, file_path: error.file_path.clone() }
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

        let script = RouteComposer::build_composition_script_with_templates(
            &page_render_script,
            &layouts,
            &templates,
            &pathname_json,
            &template_key_json,
            error_boundary.as_ref(),
            &metadata_json,
            defer_rsc,
            &action_post_url_json,
            capture_stream_id,
        );

        Ok(script)
    }

    pub async fn render_loading(
        &self,
        loading_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(loading_path);

        run_with_renderer_result(Arc::clone(&self.renderer), move |renderer| async move {
            renderer.render_to_string(&component_id, None).await
        })
        .await
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

        run_with_renderer_result(Arc::clone(&self.renderer), move |renderer| async move {
            renderer.render_to_string(&component_id, Some(&props_json)).await
        })
        .await
    }

    pub async fn render_not_found(
        &self,
        not_found_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(not_found_path);

        run_with_renderer_result(Arc::clone(&self.renderer), move |renderer| async move {
            renderer.render_to_string(&component_id, None).await
        })
        .await
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
        let component_id = component_id.to_string();
        let component_code = component_code.to_string();
        run_with_renderer_result(Arc::clone(&self.renderer), move |renderer| async move {
            renderer.register_component(&component_id, &component_code).await
        })
        .await
    }
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::server::cache::handler::NoOpCacheHandler;

    #[tokio::test]
    async fn test_layout_handler_round_trip() {
        let cache = LayoutHtmlCache::new();
        let html = "<!DOCTYPE html><html><body>hi</body></html>".to_string();

        cache.insert(42, html.clone()).await.expect("insert");
        let got = cache.get(42).await.expect("get");
        assert_eq!(got, html);

        assert!(cache.get(9999).await.is_none());
    }

    #[tokio::test]
    async fn test_layout_clear() {
        let cache = LayoutHtmlCache::new();
        cache.insert(1, "one".to_string()).await.expect("insert");
        cache.insert(2, "two".to_string()).await.expect("insert");
        cache.insert(3, "three".to_string()).await.expect("insert");

        assert!(cache.get(1).await.is_some());
        assert!(cache.get(2).await.is_some());
        assert!(cache.get(3).await.is_some());

        cache.clear().await.expect("clear");

        assert!(cache.get(1).await.is_none());
        assert!(cache.get(2).await.is_none());
        assert!(cache.get(3).await.is_none());
    }

    #[tokio::test]
    async fn test_layout_with_noop_handler() {
        let cache = LayoutHtmlCache::with_handler(Arc::new(NoOpCacheHandler));

        cache.insert(1, "x".to_string()).await.expect("insert is no-op but Ok");
        assert!(cache.get(1).await.is_none());
        cache.clear().await.expect("clear is no-op but Ok");
    }

    #[tokio::test]
    async fn test_layout_custom_ttl_passes_through() {
        let handler = Arc::new(MemoryCacheHandler::default());
        let cache = LayoutHtmlCache::with_ttl(handler, 60);
        cache.insert(7, "alive".to_string()).await.expect("insert");
        assert!(cache.get(7).await.is_some());
    }

    #[tokio::test]
    async fn test_layout_invalidate_by_tag() {
        let cache = LayoutHtmlCache::new();
        cache
            .insert_with_tags(42, "tagged".to_string(), &["products".to_string()])
            .await
            .expect("insert");
        assert!(cache.get(42).await.is_some());

        cache.invalidate_by_tag("products").await.expect("invalidate");
        assert!(cache.get(42).await.is_none());
    }

    #[tokio::test]
    async fn test_layout_clear_removes_all_layout_keys() {
        let cache = LayoutHtmlCache::new();
        for i in 0..50 {
            cache.insert(i, format!("v{i}")).await.expect("insert");
        }
        cache.clear().await.expect("clear");
        for i in 0..50 {
            assert!(cache.get(i).await.is_none(), "key {i} survived clear");
        }
    }
}
