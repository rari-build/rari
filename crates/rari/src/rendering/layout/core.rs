#![expect(clippy::missing_errors_doc)]

use std::{env, fmt::Write, sync::Arc};

use base64::{Engine, engine::general_purpose::STANDARD};
use bytes::Bytes;
use cow_utils::CowUtils;
use rari_error::RariError;
use serde_json::Value;
use tokio::{
    sync::{Mutex, mpsc},
    task,
};

use super::{
    error_messages,
    types::{ChunkedContentType, LayoutRenderContext, RenderResult},
    utils,
};
use crate::{
    RscHtmlRenderer,
    rendering::{
        base::RscRenderer,
        layout::{
            LayoutInfo, RouteComposer,
            route_composer::{ErrorBoundaryInfo, TemplateInfo},
        },
    },
    runtime::JsExecutionRuntime,
    server::{
        cache::handler::{
            CacheError, CacheHandler, CacheHandlerRegistry, MemoryCacheHandler, MemoryConfig,
        },
        config::{CacheLayerConfig, Config},
        middleware::request_context::RequestContext,
        routing::app_router::AppRouteMatch,
    },
};

const LAYOUT_KEY_PREFIX: &str = "layout:";
const JS_GET_RESULT: &str = r"
globalThis['~rsc'].renderResult
";

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
        self.handler.set(&Self::namespaced(key), html.into_bytes(), self.default_ttl_secs).await?;
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

        let page_path = rari_utils::path_to_file_url(&page_file_path);

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
        let loading_enabled = Config::get().map(|c| c.loading.enabled).unwrap_or(true);
        let loading_component_id = if loading_enabled {
            route_match.loading.as_ref().map(|e| {
                e.component_id.clone().unwrap_or_else(|| utils::create_component_id(&e.file_path))
            })
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

        let renderer = self.renderer.lock().await;

        let flight_result: Result<String, RariError> = async {
            let rsc_flight_protocol =
                Self::execute_composition_and_serialize(&renderer, composition_script).await?;
            Self::validate_rsc_flight_protocol(&rsc_flight_protocol)?;
            Ok(rsc_flight_protocol)
        }
        .await;

        drop(request_context);

        flight_result
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

        let renderer = self.renderer.lock().await;

        let render_operation = async {
            let rsc_flight_protocol =
                Self::execute_composition_and_serialize(&renderer, composition_script).await?;
            Self::validate_rsc_flight_protocol(&rsc_flight_protocol)?;
            Self::validate_html_structure(&rsc_flight_protocol, route_match)?;
            Ok(rsc_flight_protocol)
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
        request_context: Option<Arc<RequestContext>>,
    ) -> Result<String, RariError> {
        self.render_route(route_match, context, request_context).await
    }

    #[expect(clippy::too_many_lines)]
    pub async fn render_route_with_streaming(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<Arc<RequestContext>>,
        return_rsc_on_fallback: bool,
    ) -> Result<RenderResult, RariError> {
        let cache_key = utils::generate_cache_key(route_match, context);

        if !return_rsc_on_fallback && let Some(cached_html) = self.html_cache.get(cache_key).await {
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

                let runtime = {
                    let renderer = self.renderer.lock().await;
                    Self::ensure_react_server_loaded(&renderer).await?;
                    Arc::clone(&renderer.runtime)
                };

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

                        const ReactServerRenderer = globalThis['~reactServerRenderer'];
                        const bundlerConfig = globalThis['~rari']?.clientReferenceManifest || {{}};

                        const stream = await ReactServerRenderer.renderToReadableStream(
                            capturedElement,
                            bundlerConfig,
                            {{ onError(error) {{ console.error('[rari] RSC stream error:', error); }} }}
                        );

                        const reader = stream.getReader();
                        const decoder = new TextDecoder();
                        while (true) {{
                            const {{ done, value }} = await reader.read();
                            if (done) break;
                            const text = decoder.decode(value, {{ stream: true }});
                            if (!(await rariPumpFizzChunk(text))) break;
                        }}
                        if (!rariStreamDisconnected) {{
                            const tail = decoder.decode();
                            await rariPumpFizzChunk(tail);
                        }}
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

            let (rsc_flight_protocol, binary) = {
                let renderer = self.renderer.lock().await;

                let composition_script = self.build_composition_script(
                    route_match,
                    context,
                    loading_component_id.as_deref(),
                    loading_component_id.is_some(),
                    false,
                )?;

                let render_and_capture = async {
                    Self::ensure_react_server_loaded(&renderer).await?;

                    let rsc_flight_protocol =
                        Self::execute_composition_and_serialize(&renderer, composition_script)
                            .await?;
                    Self::validate_rsc_flight_protocol(&rsc_flight_protocol)?;
                    Self::validate_html_structure(&rsc_flight_protocol, route_match)?;

                    let b64_result = renderer
                        .runtime
                        .execute_script(
                            "get_rsc_binary_b64".to_string(),
                            r"(function() {
                                const bin = globalThis['~rari']?.lastRscBinary;
                                if (!bin || bin.length === 0) return null;
                                let str = '';
                                for (let i = 0; i < bin.length; i++) {
                                    str += String.fromCharCode(bin[i]);
                                }
                                return btoa(str);
                            })()"
                                .to_string(),
                        )
                        .await;

                    let binary = match b64_result {
                        Ok(v) => v.as_str().and_then(|b64| STANDARD.decode(b64).ok()),
                        Err(_) => None,
                    };

                    Ok::<(String, Option<Vec<u8>>), RariError>((rsc_flight_protocol, binary))
                };

                if let Some(ctx) = request_context {
                    renderer.runtime.execute_with_request_context(ctx, render_and_capture).await?
                } else {
                    render_and_capture.await?
                }
            };

            if let Some(bytes) = binary {
                return Ok(RenderResult::StaticBinary(bytes));
            }

            Ok(RenderResult::Static(rsc_flight_protocol))
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

                let runtime = {
                    let mut renderer = self.renderer.lock().await;
                    renderer.initialize().await?;
                    Self::ensure_streaming_fizz_loaded(&renderer).await?;
                    Arc::clone(&renderer.runtime)
                };

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

            let render_result = {
                let renderer = self.renderer.lock().await;

                let composition_script = self.build_composition_script(
                    route_match,
                    context,
                    loading_component_id.as_deref(),
                    loading_component_id.is_some(),
                    false,
                )?;

                let render_and_capture = async {
                    Self::ensure_react_server_loaded(&renderer).await?;

                    let rsc_flight_protocol =
                        Self::execute_composition_and_serialize(&renderer, composition_script)
                            .await?;
                    Self::validate_rsc_flight_protocol(&rsc_flight_protocol)?;
                    Self::validate_html_structure(&rsc_flight_protocol, route_match)?;

                    let html_renderer = RscHtmlRenderer::new(Arc::clone(&renderer.runtime));
                    let html = html_renderer
                        .render_to_html_for_route_fizz(&rsc_flight_protocol, config, route_match)
                        .await?;

                    Ok::<(String, String), RariError>((rsc_flight_protocol, html))
                };

                if let Some(ctx) = request_context.clone() {
                    renderer.runtime.execute_with_request_context(ctx, render_and_capture).await
                } else {
                    render_and_capture.await
                }
            };

            let (rsc_flight_protocol, html) = match render_result {
                Ok(v) => v,
                Err(e) if needs_streaming => {
                    tracing::warn!("Fizz render failed for streaming route: {e}");
                    return Err(RariError::internal(format!(
                        "Fizz render failed for streaming route: {e}"
                    )));
                }
                Err(e) => {
                    tracing::warn!("Fizz render failed for static route: {e}");
                    return Err(RariError::internal(format!("Fizz render failed: {e}")));
                }
            };

            let has_binary_rows = rsc_flight_protocol.lines().any(|line| {
                let trimmed = line.trim();
                if let Some(colon_pos) = trimmed.find(':') {
                    let header = &trimmed[..colon_pos];
                    header.chars().all(|c| c.is_ascii_hexdigit())
                        && !header.is_empty()
                        && trimmed[colon_pos + 1..].starts_with('T')
                } else {
                    false
                }
            });

            let payload_script = if has_binary_rows {
                let binary_b64 = {
                    let renderer = self.renderer.lock().await;
                    let result = renderer
                        .runtime
                        .execute_script(
                            "get_rsc_b64_for_embed".to_string(),
                            r"(function() {
                                const bin = globalThis['~rari']?.lastRscBinary;
                                if (!bin || bin.length === 0) return null;
                                let str = '';
                                for (let i = 0; i < bin.length; i++) {
                                    str += String.fromCharCode(bin[i]);
                                }
                                return btoa(str);
                            })()"
                                .to_string(),
                        )
                        .await;
                    match result {
                        Ok(v) => v.as_str().map(String::from),
                        Err(_) => None,
                    }
                };
                if let Some(b64) = binary_b64 {
                    Self::build_binary_flight_push_script(&b64)
                } else {
                    Self::build_flight_push_scripts(&rsc_flight_protocol)
                }
            } else {
                Self::build_flight_push_scripts(&rsc_flight_protocol)
            };
            let completion_script = r"<script>if(!window['~rari'])window['~rari']={};window['~rari'].streaming={complete:true}</script>";

            let html = if let Some(body_end) = html.rfind("</body>") {
                let mut result = html;
                result.insert_str(body_end, &format!("{payload_script}\n{completion_script}\n"));
                result
            } else {
                format!("{html}{payload_script}\n{completion_script}")
            };

            if route_match.not_found.is_none() {
                let _ = self.html_cache.insert(cache_key, html.clone()).await;
            }

            Ok(RenderResult::Static(html))
        }
    }

    fn validate_html_structure(html: &str, route_match: &AppRouteMatch) -> Result<(), RariError> {
        let root_layout_path =
            route_match.layouts.iter().find(|l| l.is_root).map(|l| l.file_path.as_str());

        let trimmed = html.trim_start();

        if let Some(first_char) = trimmed.chars().next()
            && first_char.is_ascii_digit()
        {
            if trimmed.contains("\"div\"")
                && trimmed.contains("\"html\"")
                && let Some(div_pos) = trimmed.find("[\"$\",\"div\"")
                && let Some(html_pos) = trimmed.find("[\"$\",\"html\"")
                && div_pos < html_pos
            {
                let error_msg = error_messages::create_wrapped_html_error_message(
                    route_match,
                    root_layout_path,
                );
                return Err(RariError::internal(error_msg));
            }
            return Ok(());
        }

        let first_tag_name = if let Some(tag_start) = trimmed.strip_prefix('<') {
            if tag_start.starts_with('!') || tag_start.starts_with('?') {
                if let Some(next_tag_pos) = tag_start.find('<') {
                    let after_special = &tag_start[next_tag_pos + 1..];
                    after_special
                        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                        .next()
                        .unwrap_or("")
                } else {
                    ""
                }
            } else {
                tag_start
                    .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .next()
                    .unwrap_or("")
            }
        } else {
            ""
        };

        if !first_tag_name.is_empty() && (html.contains("<html") || html.contains("\"html\"")) {
            let error_msg =
                error_messages::create_wrapped_html_error_message(route_match, root_layout_path);
            return Err(RariError::internal(error_msg));
        }

        Ok(())
    }

    async fn ensure_react_server_loaded(renderer: &RscRenderer) -> Result<(), RariError> {
        let check_result = renderer
            .runtime
            .execute_script(
                "<check_rsc>".to_string(),
                "typeof globalThis.renderToRsc === 'function'".to_string(),
            )
            .await?;

        if check_result.as_bool() == Some(true) {
            return Ok(());
        }

        let setup_script = r"
            (async function() {
                const [react, flightServer] = await Promise.all([
                    import('file:///react_vendor/react.js'),
                    import('file:///react_vendor/react-server-dom-webpack-server.js'),
                ]);
                if (!globalThis.React?.createElement) {
                    globalThis.React = react.default && react.default.createElement ? react.default : react;
                }
                globalThis['~reactServerRenderer'] = flightServer;
                return !!(globalThis.React.createElement && globalThis['~reactServerRenderer'].renderToReadableStream);
            })()
        ";

        let result = renderer
            .runtime
            .execute_script("<load_react_server>".to_string(), setup_script.to_string())
            .await
            .map_err(|e| {
                RariError::internal(format!("Failed to load React Server renderer: {e}"))
            })?;

        if result.as_bool() != Some(true) {
            return Err(RariError::internal(
                "React Server renderer module failed to initialize".to_string(),
            ));
        }

        let renderer_script = include_str!("../base/js/rsc_renderer.ts");
        renderer
            .runtime
            .execute_script("load_rsc_renderer.ts".to_string(), renderer_script.to_string())
            .await
            .map_err(|e| RariError::internal(format!("Failed to load RSC renderer: {e}")))?;

        Ok(())
    }

    async fn ensure_streaming_fizz_loaded(renderer: &RscRenderer) -> Result<(), RariError> {
        let check = renderer
            .runtime
            .execute_script(
                "check_streaming_fizz".to_string(),
                "typeof globalThis['~rari']?.renderStreamingDocument === 'function'".to_string(),
            )
            .await?;

        if check.as_bool() == Some(true) {
            return Ok(());
        }

        let streaming_fizz_script = include_str!("js/streaming_fizz.ts");
        renderer
            .runtime
            .execute_script("streaming_fizz.ts".to_string(), streaming_fizz_script.to_string())
            .await
            .map_err(|e| {
                RariError::internal(format!("Failed to load streaming Fizz pipeline: {e}"))
            })?;

        Ok(())
    }

    async fn execute_composition_and_serialize(
        renderer: &RscRenderer,
        composition_script: String,
    ) -> Result<String, RariError> {
        Self::ensure_react_server_loaded(renderer).await?;

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

        if let Some(flight_protocol_str) = rsc_data.as_str() {
            return Ok(flight_protocol_str.to_string());
        }

        Err(RariError::internal(
            "RSC render did not produce a Flight protocol string. The renderer may not be loaded."
                .to_string(),
        ))
    }

    fn escape_json_for_html_script(json: &str) -> String {
        json.cow_replace("</", r"\u003c/").into_owned()
    }

    fn sort_flight_protocol_lines(flight_protocol: &str) -> String {
        let mut rows_with_ids: Vec<(u32, String)> = Vec::new();

        for row in flight_protocol.lines() {
            if let Some(colon_pos) = row.find(':') {
                if let Ok(row_id) = u32::from_str_radix(&row[..colon_pos], 16) {
                    rows_with_ids.push((row_id, row.to_string()));
                } else {
                    rows_with_ids.push((u32::MAX, row.to_string()));
                }
            } else {
                rows_with_ids.push((u32::MAX, row.to_string()));
            }
        }

        rows_with_ids.sort_by_key(|(id, _)| *id);

        let mut sorted =
            rows_with_ids.iter().map(|(_, row)| row.as_str()).collect::<Vec<_>>().join("\n");

        if !sorted.is_empty() && !sorted.ends_with('\n') {
            sorted.push('\n');
        }

        let has_row_0 = rows_with_ids.iter().any(|(id, row)| *id == 0 && row.starts_with("0:"));

        if !has_row_0
            && let Some((max_id, _)) =
                rows_with_ids.iter().filter(|(id, _)| *id != u32::MAX).max_by_key(|(id, _)| *id)
            && *max_id > 0
        {
            let row_0 = format!("0:\"${max_id:x}\"\n");
            sorted.insert_str(0, &row_0);
        }

        sorted
    }

    /// Builds `__rari_f` push scripts for client hydration. Used by the static Fizz
    /// fallback path; the streaming pipeline uses pull fanout + live mux helpers instead.
    fn build_flight_push_scripts(rsc_flight_protocol: &str) -> String {
        let mut scripts =
            String::from(r"<script>(self.__rari_f=self.__rari_f||[]).push(0)</script>");
        let payload = Self::sort_flight_protocol_lines(rsc_flight_protocol);

        for line in payload.lines() {
            if line.is_empty() {
                continue;
            }
            let row = format!("{line}\n");
            let json_row = serde_json::to_string(&row).unwrap_or_else(|_| "\"\"".to_string());
            let escaped_row = Self::escape_json_for_html_script(&json_row);
            let _ = write!(
                scripts,
                r"<script>(self.__rari_f=self.__rari_f||[]).push({escaped_row})</script>"
            );
        }

        scripts
    }

    fn build_binary_flight_push_script(b64: &str) -> String {
        let json_payload = serde_json::to_string(&serde_json::json!([2, b64]))
            .unwrap_or_else(|_| "[]".to_string());
        let escaped_payload = Self::escape_json_for_html_script(&json_payload);
        format!(
            r"<script>(self.__rari_f=self.__rari_f||[]).push(0)</script><script>(self.__rari_f=self.__rari_f||[]).push({escaped_payload})</script>"
        )
    }

    fn validate_rsc_flight_protocol(rsc_data: &str) -> Result<(), RariError> {
        if rsc_data.trim().is_empty() {
            let error_msg = error_messages::create_empty_rsc_error_message();
            return Err(RariError::internal(error_msg));
        }

        let looks_like_flight = rsc_data.lines().any(|line| {
            let line = line.trim();
            line.find(':').is_some_and(|colon| {
                !line[..colon].is_empty() && line[..colon].chars().all(|c| c.is_ascii_hexdigit())
            })
        });

        if !looks_like_flight {
            return Err(RariError::internal(
                "RSC output does not look like a valid Flight protocol".to_string(),
            ));
        }

        Ok(())
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
            error_boundary.as_ref(),
            &metadata_json,
            defer_rsc,
        );

        Ok(script)
    }

    pub async fn render_loading(
        &self,
        loading_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(loading_path);

        let renderer = self.renderer.lock().await;
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

        let renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, Some(&props_json)).await
    }

    pub async fn render_not_found(
        &self,
        not_found_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(not_found_path);

        let renderer = self.renderer.lock().await;
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

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
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

    #[test]
    fn test_validate_rsc_flight_protocol_accepts_flight_rows() {
        assert!(
            LayoutRenderer::validate_rsc_flight_protocol("0:\"$1\"\n1:[\"$\",\"div\",null,{}]\n")
                .is_ok()
        );
    }

    #[test]
    fn test_validate_rsc_flight_protocol_rejects_empty() {
        assert!(LayoutRenderer::validate_rsc_flight_protocol("").is_err());
        assert!(LayoutRenderer::validate_rsc_flight_protocol("   \n").is_err());
    }

    #[test]
    fn test_validate_rsc_flight_protocol_rejects_non_flight_output() {
        let result = LayoutRenderer::validate_rsc_flight_protocol("Error: composition failed");
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("does not look like a valid Flight protocol")
        );
    }
}
