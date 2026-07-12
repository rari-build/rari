#![expect(clippy::missing_errors_doc)]

use std::{env, sync::Arc};

use base64::{Engine, engine::general_purpose::STANDARD};
use bytes::Bytes;
use rari_error::RariError;
use serde_json::Value;
use tokio::{
    sync::{Mutex, mpsc},
    task,
};

use super::{
    types::{ChunkedContentType, LayoutRenderContext, RenderResult},
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
    runtime::JsExecutionRuntime,
    server::{
        cache::{
            handler::{
                CacheError, CacheHandler, CacheHandlerRegistry, MemoryCacheHandler, MemoryConfig,
            },
            response::RouteCachePolicy,
        },
        config::{CacheLayerConfig, Config},
        middleware::request_context::RequestContext,
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
                            await globalThis['~rari']?.injectStreamError?.(caughtErrors);
                        }
";

const FIZZ_STREAM_ERROR_INJECTION: &str = r"
                        await injectRariErrorFromCaught();
";

const FIZZ_CHUNK_PUMP_HELPER: &str = r"
                        let rariStreamDisconnected = false;
                        async function rariPumpFizzChunk(text) {
                            if (!text || rariStreamDisconnected) return false;
                            try {
                                await Deno.core.ops.op_fizz_chunk(text);
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

async fn run_streaming_script(
    runtime: &Arc<JsExecutionRuntime>,
    request_context: Option<Arc<RequestContext>>,
    script_name: String,
    script: String,
    chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
) -> Result<(), RariError> {
    let execute_stream =
        async { runtime.execute_script_for_streaming(script_name, script, chunk_sender).await };

    if let Some(context) = request_context {
        runtime.execute_with_request_context(context, execute_stream).await
    } else {
        execute_stream.await
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
        let page_props = utils::create_page_props(route_match, context)?;
        let page_props_json = serde_json::to_string(&page_props)?;

        let dist_server_path = env::current_dir()
            .ok()
            .map(|p| p.join("dist/server"))
            .and_then(|p| p.canonicalize().ok());

        let Some(base_path) = dist_server_path else {
            return Ok(false);
        };

        let page_file_path = utils::component_dist_path(
            &base_path,
            &route_match.route.file_path,
            route_match.route.component_id.as_deref(),
        );

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

        let renderer = self.renderer.lock().await;
        let runtime = Arc::clone(&renderer.runtime);
        drop(renderer);

        let result = runtime.execute_script("check_not_found".to_string(), check_script).await?;

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
                let loading_id = loading_entry
                    .component_id
                    .clone()
                    .unwrap_or_else(|| utils::create_component_id(&loading_entry.file_path));
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
            false,
        )?;

        let renderer = Arc::clone(&self.renderer);
        run_with_renderer_result(renderer, move |renderer| async move {
            let render_operation = async {
                Self::execute_composition_and_serialize(&renderer, composition_script).await
            };

            if let Some(ctx) = request_context {
                renderer.runtime.execute_with_request_context(ctx, render_operation).await
            } else {
                render_operation.await
            }
        })
        .await
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
    ) -> Result<(), RariError> {
        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            route_match.loading.as_ref().map(|loading_entry| {
                loading_entry
                    .component_id
                    .clone()
                    .unwrap_or_else(|| utils::create_component_id(&loading_entry.file_path))
            })
        } else {
            None
        };

        let composition_script = self.build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            false,
            true,
        )?;

        let renderer = Arc::clone(&self.renderer);
        run_with_renderer_result(renderer, move |renderer| async move {
            let compose_operation = async {
                renderer
                    .runtime
                    .execute_script("action_refresh_compose".to_string(), composition_script)
                    .await?;
                Ok(())
            };

            renderer.runtime.execute_with_request_context(request_context, compose_operation).await
        })
        .await
    }

    #[expect(clippy::too_many_lines)]
    pub async fn render_route_with_streaming(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<Arc<RequestContext>>,
        return_rsc_on_fallback: bool,
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
                let loading_id = loading_entry
                    .component_id
                    .clone()
                    .unwrap_or_else(|| utils::create_component_id(&loading_entry.file_path));
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
                let composition_script = self.build_composition_script(
                    route_match,
                    context,
                    loading_component_id.as_deref(),
                    true,
                    true,
                )?;

                let (chunk_sender, chunk_receiver) = mpsc::channel::<Result<Vec<u8>, String>>(32);

                let runtime =
                    run_with_renderer_result(Arc::clone(&self.renderer), |renderer| async move {
                        renderer.ensure_streaming_pipeline().await?;
                        Ok(Arc::clone(&renderer.runtime))
                    })
                    .await?;

                let script = format!(
                    r"(async function() {{
                        {FIZZ_CHUNK_PUMP_HELPER}
                        try {{
                        try {{ {composition_script} }} catch(e) {{
                            console.error('[rari] Composition error in RSC streaming nav:', e);
                        }}

                        const capturedElement = globalThis['~rari']?.capturedElement;
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
                            Deno.core.ops.op_fizz_done();
                        }}
                    }})()",
                );

                let runtime_clone = Arc::clone(&runtime);
                let request_context_for_stream = request_context.clone();
                tokio::spawn(async move {
                    if let Err(e) = run_streaming_script(
                        &runtime_clone,
                        request_context_for_stream,
                        "rsc_streaming_nav".to_string(),
                        script,
                        chunk_sender,
                    )
                    .await
                    {
                        tracing::error!("RSC streaming navigation failed: {e}");
                    }
                });

                return Ok(RenderResult::Chunked {
                    content_type: ChunkedContentType::RscFlight,
                    shell: Bytes::new(),
                    closing: Bytes::new(),
                    chunks: chunk_receiver,
                });
            }

            let composition_script = self.build_composition_script(
                route_match,
                context,
                loading_component_id.as_deref(),
                loading_component_id.is_some(),
                false,
            )?;

            let rsc_payload =
                run_with_renderer_result(Arc::clone(&self.renderer), move |renderer| async move {
                    let render_and_capture = async {
                        let result = Self::run_composition(&renderer, composition_script).await?;

                        if let Some(bytes) = Self::capture_last_rsc_binary(&renderer).await?
                            && !bytes.is_empty()
                        {
                            return Ok(RscNavPayload::Binary(bytes));
                        }

                        Ok(RscNavPayload::Text(Self::extract_flight_text(&result)?))
                    };

                    if let Some(ctx) = request_context {
                        renderer.runtime.execute_with_request_context(ctx, render_and_capture).await
                    } else {
                        render_and_capture.await
                    }
                })
                .await?;

            match rsc_payload {
                RscNavPayload::Binary(bytes) => Ok(RenderResult::StaticBinary(bytes)),
                RscNavPayload::Text(flight) => Ok(RenderResult::Static(flight)),
            }
        } else {
            let config =
                Config::get().ok_or_else(|| RariError::internal("Config not available"))?;

            if needs_streaming {
                let composition_script = self.build_composition_script(
                    route_match,
                    context,
                    loading_component_id.as_deref(),
                    true,
                    true,
                )?;

                let runtime =
                    run_with_renderer_result(Arc::clone(&self.renderer), |renderer| async move {
                        renderer.ensure_streaming_pipeline().await?;
                        Ok(Arc::clone(&renderer.runtime))
                    })
                    .await?;

                let html_renderer = RscHtmlRenderer::new(Arc::clone(&runtime));
                let css_links = RscHtmlRenderer::css_links_for_route(route_match);
                let cache_template = config.rsc_html.cache_template;
                let is_dev_mode = config.is_development();
                let template = html_renderer.load_template(cache_template, is_dev_mode).await?;
                let template = RscHtmlRenderer::inject_css_links(&template, &css_links);

                let head_content = template
                    .find("<head>")
                    .and_then(|start| template.find("</head>").map(|end| &template[start + 6..end]))
                    .unwrap_or("")
                    .to_string();

                let (chunk_sender, chunk_receiver) = mpsc::channel::<Result<Vec<u8>, String>>(32);

                let head_content_json =
                    serde_json::to_string(&head_content).unwrap_or_else(|_| "\"\"".to_string());

                let script = format!(
                    r"(async function() {{
                        let caughtErrors = [];
                        {FIZZ_STREAM_ERROR_HELPER}
                        try {{
                        try {{ await ({composition_script}); }} catch(e) {{
                            console.error('[rari] Composition error in streaming:', e);
                        }}

                        const capturedElement = globalThis['~rari']?.capturedElement;
                        if (!capturedElement) {{
                            Deno.core.ops.op_fizz_done();
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
                        }});

                        {FIZZ_STREAM_ERROR_INJECTION}

                        await globalThis['~rari']?.pumpStreamingCompleteScript?.();

                        Deno.core.ops.op_fizz_done();

                        }} catch(outerError) {{
                            console.error('[rari] Fizz streaming pipeline fatal error:', outerError);
                            const displayError = caughtErrors.length > 0 ? caughtErrors[0] : outerError;
                            const errMsg = String(displayError?.message || outerError?.message || 'Unknown error').split('<').join('&lt;');
                            const errorHtml = '<!doctype html><html><head></head><body><div id=root><div class=rari-error style=color:red;border:1px_solid_red;padding:10px;border-radius:4px;background-color:#fff5f5><strong>Error loading content: </strong>' + errMsg + '</div></div></body></html>';
                            const pump = globalThis['~rari']?.pumpFizzChunk;
                            if (typeof pump === 'function') {{
                                await pump(errorHtml);
                            }} else {{
                                await Deno.core.ops.op_fizz_chunk(errorHtml);
                            }}
                            Deno.core.ops.op_fizz_done();
                        }}
                    }})()",
                );

                let shell = Bytes::from_static(b"<!DOCTYPE html>");
                let closing = Bytes::new();

                let runtime_clone = Arc::clone(&runtime);
                let request_context_for_stream = request_context.clone();
                tokio::spawn(async move {
                    task::yield_now().await;
                    if let Err(e) = run_streaming_script(
                        &runtime_clone,
                        request_context_for_stream,
                        "fizz_direct_stream".to_string(),
                        script,
                        chunk_sender,
                    )
                    .await
                    {
                        tracing::error!("Fizz direct streaming error: {e}");
                    }
                });

                return Ok(RenderResult::Chunked {
                    content_type: ChunkedContentType::Html,
                    shell,
                    closing,
                    chunks: chunk_receiver,
                });
            }

            let html = {
                let composition_script = self.build_composition_script(
                    route_match,
                    context,
                    loading_component_id.as_deref(),
                    loading_component_id.is_some(),
                    true,
                )?;

                let route_match = route_match.clone();
                let config = config.clone();
                let request_context = request_context.clone();

                run_with_renderer_result(Arc::clone(&self.renderer), move |renderer| async move {
                    renderer.ensure_streaming_pipeline().await?;

                    let html_renderer = RscHtmlRenderer::new(Arc::clone(&renderer.runtime));
                    let css_links = RscHtmlRenderer::css_links_for_route(&route_match);
                    let cache_template = config.rsc_html.cache_template;
                    let is_dev_mode = config.is_development();
                    let template = html_renderer.load_template(cache_template, is_dev_mode).await?;
                    let template = RscHtmlRenderer::inject_css_links(&template, &css_links);

                    let head_content = template
                        .find("<head>")
                        .and_then(|start| {
                            template.find("</head>").map(|end| &template[start + 6..end])
                        })
                        .unwrap_or("")
                        .to_string();

                    let head_content_json =
                        serde_json::to_string(&head_content).unwrap_or_else(|_| "\"\"".to_string());

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

                            return {{ ok: true, html }};
                        }} catch(e) {{
                            return {{ ok: false, error: String(e?.message || e) }};
                        }}
                    }})()",
                    );

                    let render_operation = async {
                        let result = renderer
                            .runtime
                            .execute_script("static_document_render".to_string(), script)
                            .await?;

                        let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
                        if !ok {
                            let err =
                                result.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
                            return Err(RariError::internal(format!(
                                "Static document render failed: {err}"
                            )));
                        }

                        let html = result
                            .get("html")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();

                        html_renderer
                            .assemble_document(html, cache_template, is_dev_mode, &css_links)
                            .await
                    };

                    if let Some(ctx) = request_context {
                        renderer.runtime.execute_with_request_context(ctx, render_operation).await
                    } else {
                        render_operation.await
                    }
                })
                .await?
            };

            if layout_cache_enabled && route_match.not_found.is_none() {
                let runtime = {
                    let renderer = self.renderer.lock().await;
                    Arc::clone(&renderer.runtime)
                };

                let is_dynamic_render = runtime.is_dynamic_render().await.unwrap_or(false);

                if !is_dynamic_render {
                    let page_cache_tags =
                        runtime.collect_page_cache_tags().await.unwrap_or_default();
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
            }

            Ok(RenderResult::Static(html))
        }
    }

    async fn execute_composition_and_serialize(
        renderer: &RscRenderer,
        composition_script: String,
    ) -> Result<String, RariError> {
        let result = Self::run_composition(renderer, composition_script).await?;
        Self::extract_flight_text(&result)
    }

    async fn run_composition(
        renderer: &RscRenderer,
        composition_script: String,
    ) -> Result<Value, RariError> {
        renderer.ensure_rsc_pipeline().await?;

        let promise_result = renderer
            .runtime
            .execute_script("compose_and_render".to_string(), composition_script)
            .await?;

        if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
            Ok(promise_result)
        } else {
            renderer
                .runtime
                .execute_script("get_result".to_string(), JS_GET_RESULT.to_string())
                .await
        }
    }

    async fn capture_last_rsc_binary(renderer: &RscRenderer) -> Result<Option<Vec<u8>>, RariError> {
        let result = renderer
            .runtime
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

    #[expect(clippy::too_many_lines)]
    pub fn build_composition_script(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        loading_component_id: Option<&str>,
        use_suspense: bool,
        defer_rsc: bool,
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
            not_found
                .component_id
                .clone()
                .unwrap_or_else(|| utils::create_component_id(&not_found.file_path))
        } else {
            route_match
                .route
                .component_id
                .clone()
                .unwrap_or_else(|| utils::create_component_id(&route_match.route.file_path))
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
                component_id: layout
                    .component_id
                    .clone()
                    .unwrap_or_else(|| utils::create_component_id(&layout.file_path)),
                is_root: layout.is_root,
                file_path: layout.file_path.clone(),
            })
            .collect();

        let templates: Vec<TemplateInfo> = route_match
            .templates
            .iter()
            .map(|template| TemplateInfo {
                component_id: template
                    .component_id
                    .clone()
                    .unwrap_or_else(|| utils::create_component_id(&template.file_path)),
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
