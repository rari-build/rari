use std::sync::{Arc, atomic::Ordering};

use cow_utils::CowUtils;
use rari_error::RariError;
use rari_utils::path_to_file_url;
use rustc_hash::FxHashSet;
use serde_json::Value;
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error};

use super::{
    constants::{
        JS_DISABLE_STREAMING, JS_ENABLE_STREAMING, JS_GET_RESULT, JS_PAGE_RENDER_SIMPLE,
        JS_PAGE_RENDER_WITH_LOADING,
    },
    error_messages,
    types::{LayoutRenderContext, RenderResult},
    utils,
};
use crate::{
    RscHtmlRenderer, RscStreamChunk,
    rendering::{
        base::RscRenderer,
        layout::{
            LayoutInfo, LayoutStructure, RouteComposer,
            route_composer::{ErrorBoundaryInfo, TemplateInfo},
        },
        streaming::{RscChunkType, RscStream, StreamingRenderer},
    },
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
            Arc::new(MemoryCacheHandler::with_config(MemoryConfig {
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
                debug!(key = %ns_key, error = %e, "layout cache get failed");
                return None;
            }
        };
        match String::from_utf8(bytes) {
            Ok(s) => Some(s),
            Err(e) => {
                debug!(key = %ns_key, error = %e, "layout cache value not valid utf-8");
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

    async fn enable_streaming_and_inject_lazy_resolver(
        renderer: &RscRenderer,
    ) -> Result<(), RariError> {
        let prev_count = renderer.streaming_refcount.fetch_add(1, Ordering::SeqCst);

        if prev_count == 0
            && let Err(e) = renderer
                .runtime
                .execute_script("enable_streaming".to_string(), JS_ENABLE_STREAMING.to_string())
                .await
        {
            renderer.streaming_refcount.fetch_sub(1, Ordering::SeqCst);
            return Err(e);
        }

        let resolve_helper = include_str!("js/resolve_lazy_helper.js");
        let injection_result = renderer
            .runtime
            .execute_script("inject_lazy_resolver".to_string(), resolve_helper.to_string())
            .await;

        if let Err(e) = injection_result {
            let new_count = renderer.streaming_refcount.fetch_sub(1, Ordering::SeqCst);
            if new_count == 1
                && let Err(cleanup_err) = renderer
                    .runtime
                    .execute_script(
                        "disable_streaming".to_string(),
                        JS_DISABLE_STREAMING.to_string(),
                    )
                    .await
            {
                tracing::debug!(
                    "Failed to disable streaming after injection error: {}",
                    cleanup_err
                );
            }
            return Err(e);
        }

        Ok(())
    }

    async fn disable_streaming(renderer: &RscRenderer) -> Result<(), RariError> {
        let prev_count = renderer.streaming_refcount.fetch_sub(1, Ordering::SeqCst);

        if prev_count == 1 {
            renderer
                .runtime
                .execute_script("disable_streaming".to_string(), JS_DISABLE_STREAMING.to_string())
                .await?;
        }
        Ok(())
    }

    async fn render_fallback(
        &self,
        renderer: &RscRenderer,
        fallback_script: String,
        route_match: &AppRouteMatch,
        return_rsc_on_fallback: bool,
        can_use_html_cache: bool,
        cache_key: u64,
    ) -> Result<RenderResult, RariError> {
        let rsc_wire_format =
            Self::execute_composition_and_serialize(renderer, fallback_script).await?;

        Self::validate_rsc_wire_format(&rsc_wire_format)?;

        if return_rsc_on_fallback {
            Self::validate_html_structure(&rsc_wire_format, route_match)?;
            return Ok(RenderResult::Static(rsc_wire_format));
        }

        Self::validate_html_structure(&rsc_wire_format, route_match)?;

        let html_renderer = RscHtmlRenderer::new(Arc::clone(&renderer.runtime));
        let config = Config::get().ok_or_else(|| RariError::internal("Config not available"))?;
        let html =
            html_renderer.render_to_html_for_route(&rsc_wire_format, config, route_match).await?;
        Self::validate_html_structure(&html, route_match)?;

        let is_not_found = route_match.not_found.is_some();
        if is_not_found {
            return Ok(RenderResult::Static(html));
        }

        if can_use_html_cache && let Err(e) = self.html_cache.insert(cache_key, html.clone()).await
        {
            tracing::debug!("layout_html_cache.insert failed: {}", e);
        }
        Ok(RenderResult::Static(html))
    }

    pub async fn check_page_not_found(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<bool, RariError> {
        let page_props = utils::create_page_props(route_match, context)?;
        let page_props_json = serde_json::to_string(&page_props)?;

        fn component_dist_path(
            base_path: &std::path::Path,
            file_path: &str,
            component_id: Option<&str>,
        ) -> std::path::PathBuf {
            if let Some(component_id) = component_id {
                return base_path.join(format!("{component_id}.js"));
            }

            fn convert_route_path_to_dist_path(path: &str) -> String {
                let (base, ext) = if let Some(pos) = path.rfind('.') {
                    (&path[..pos], &path[pos..])
                } else {
                    (path, "")
                };

                let converted_base = base
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '/' || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect::<String>();

                format!("{converted_base}{ext}")
            }

            let js_filename =
                file_path.cow_replace(".tsx", ".js").cow_replace(".ts", ".js").into_owned();
            let dist_filename = convert_route_path_to_dist_path(&js_filename);
            base_path.join("app").join(&dist_filename)
        }

        let dist_server_path = std::env::current_dir()
            .ok()
            .map(|p| p.join("dist/server"))
            .and_then(|p| p.canonicalize().ok());

        let base_path = match dist_server_path {
            Some(path) => path,
            None => return Ok(false),
        };

        let page_file_path = component_dist_path(
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
        )?;

        let renderer = self.renderer.lock().await;

        let render_operation = async {
            Self::enable_streaming_and_inject_lazy_resolver(&renderer).await?;

            let result = async {
                let rsc_wire_format =
                    Self::execute_composition_and_serialize(&renderer, composition_script).await?;
                Self::validate_rsc_wire_format(&rsc_wire_format)?;

                Self::validate_html_structure(&rsc_wire_format, route_match)?;

                Ok(rsc_wire_format)
            }
            .await;

            let _ = Self::disable_streaming(&renderer).await;

            result
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

        if !needs_streaming {
            let rsc_wire_format = self.render_route(route_match, context, request_context).await?;

            if return_rsc_on_fallback {
                return Ok(RenderResult::Static(rsc_wire_format));
            }

            let runtime = {
                let renderer = self.renderer.lock().await;
                Arc::clone(&renderer.runtime)
            };
            let html_renderer = RscHtmlRenderer::new(runtime);
            let config =
                Config::get().ok_or_else(|| RariError::internal("Config not available"))?;
            let html = html_renderer
                .render_to_html_for_route(&rsc_wire_format, config, route_match)
                .await?;

            let rsc_payload = if rsc_wire_format.ends_with('\n') {
                rsc_wire_format.clone()
            } else {
                format!("{rsc_wire_format}\n")
            };
            let escaped_payload = rsc_payload.cow_replace("</", "<\\/");
            let payload_script = format!(
                r#"<script id="__RARI_RSC_PAYLOAD__" type="text/x-component">{escaped_payload}</script>"#
            );
            let completion_script = r"<script>
if (typeof window !== 'undefined') {
    if (!window['~rari']) window['~rari'] = {};
    if (!window['~rari'].streaming) window['~rari'].streaming = {};
    window['~rari'].streaming.complete = true;
}
</script>";

            let html = if let Some(body_end) = html.rfind("</body>") {
                let mut result = html;
                result.insert_str(body_end, &format!("{payload_script}\n{completion_script}\n"));
                result
            } else {
                format!("{html}{payload_script}\n{completion_script}")
            };

            if route_match.not_found.is_none()
                && let Err(e) = self.html_cache.insert(cache_key, html.clone()).await
            {
                tracing::debug!("layout_html_cache.insert failed: {}", e);
            }

            return Ok(RenderResult::Static(html));
        }

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

        let can_use_html_cache = true;

        if let Some(ctx) = request_context {
            let renderer_guard = self.renderer.lock().await;
            let runtime = Arc::clone(&renderer_guard.runtime);
            let refcount = Arc::clone(&renderer_guard.streaming_refcount);
            drop(renderer_guard);

            let streaming_operation = async {
                let prev_count = refcount.fetch_add(1, Ordering::SeqCst);

                if prev_count == 0
                    && let Err(e) = runtime
                        .execute_script(
                            "enable_streaming".to_string(),
                            JS_ENABLE_STREAMING.to_string(),
                        )
                        .await
                {
                    refcount.fetch_sub(1, Ordering::SeqCst);
                    return Err(e);
                }

                let resolve_helper = include_str!("js/resolve_lazy_helper.js");
                let injection_result = runtime
                    .execute_script("inject_lazy_resolver".to_string(), resolve_helper.to_string())
                    .await;

                if let Err(e) = injection_result {
                    let new_count = refcount.fetch_sub(1, Ordering::SeqCst);
                    if new_count == 1
                        && let Err(cleanup_err) = runtime
                            .execute_script(
                                "disable_streaming".to_string(),
                                JS_DISABLE_STREAMING.to_string(),
                            )
                            .await
                    {
                        tracing::debug!(
                            "Failed to disable streaming after injection error: {}",
                            cleanup_err
                        );
                    }
                    return Err(e);
                }

                let mut streaming_renderer = StreamingRenderer::new(Arc::clone(&runtime));

                let layout_structure = LayoutStructure::new();

                let streaming_result = streaming_renderer
                    .start_streaming_with_composition(composition_script, layout_structure)
                    .await;

                match streaming_result {
                    Ok(stream) => {
                        let runtime_for_cleanup = Arc::clone(&runtime);
                        let refcount_for_cleanup = Arc::clone(&refcount);
                        let stream_with_cleanup = stream.with_cleanup(move || {
                            let prev_count = refcount_for_cleanup.fetch_sub(1, Ordering::SeqCst);

                            if prev_count == 1 {
                                let rt = runtime_for_cleanup;
                                tokio::spawn(async move {
                                    let _ = rt
                                        .execute_script(
                                            "disable_streaming".to_string(),
                                            JS_DISABLE_STREAMING.to_string(),
                                        )
                                        .await;
                                });
                            }
                        });
                        Ok(RenderResult::Streaming(stream_with_cleanup))
                    }
                    Err(e) => {
                        error!("Streaming failed, falling back to static render: {}", e);

                        let prev_count = refcount.fetch_sub(1, Ordering::SeqCst);
                        if prev_count == 1 {
                            let _ = runtime
                                .execute_script(
                                    "disable_streaming".to_string(),
                                    JS_DISABLE_STREAMING.to_string(),
                                )
                                .await;
                        }

                        let renderer_guard = self.renderer.lock().await;
                        self.render_fallback(
                            &renderer_guard,
                            fallback_script,
                            route_match,
                            return_rsc_on_fallback,
                            can_use_html_cache,
                            cache_key,
                        )
                        .await
                    }
                }
            };

            let result = runtime
                .execute_with_persistent_request_context(Arc::clone(&ctx), streaming_operation)
                .await;

            match result {
                Ok(RenderResult::Streaming(stream)) => {
                    let runtime_for_cleanup = Arc::clone(&runtime);
                    let ctx_clone = Arc::clone(&ctx);
                    let stream_with_context = stream
                        .with_request_context(ctx)
                        .with_cleanup(move || {
                            let runtime = Arc::clone(&runtime_for_cleanup);
                            let ctx = Arc::clone(&ctx_clone);
                            tokio::spawn(async move {
                                if let Err(e) = runtime.clear_request_context_if_matches(ctx).await {
                                    tracing::error!(
                                        "Failed to clear request context after streaming completion: {}",
                                        e
                                    );
                                }
                            });
                        });
                    Ok(RenderResult::Streaming(stream_with_context))
                }
                other_result => {
                    if let Err(e) = runtime.clear_request_context().await {
                        tracing::error!(
                            "Failed to clear request context after non-streaming result: {}",
                            e
                        );
                    }
                    other_result
                }
            }
        } else {
            let streaming_operation = async {
                let renderer_guard = self.renderer.lock().await;

                Self::enable_streaming_and_inject_lazy_resolver(&renderer_guard).await?;

                let mut streaming_renderer =
                    StreamingRenderer::new(Arc::clone(&renderer_guard.runtime));

                let layout_structure = LayoutStructure::new();

                let runtime_for_cleanup = Arc::clone(&renderer_guard.runtime);
                let refcount_for_cleanup = Arc::clone(&renderer_guard.streaming_refcount);

                drop(renderer_guard);

                let streaming_result = streaming_renderer
                    .start_streaming_with_composition(composition_script, layout_structure)
                    .await;

                match streaming_result {
                    Ok(stream) => {
                        let stream_with_cleanup = stream.with_cleanup(move || {
                            tokio::spawn(async move {
                                let prev_count =
                                    refcount_for_cleanup.fetch_sub(1, Ordering::SeqCst);

                                if prev_count == 1 {
                                    let _ = runtime_for_cleanup
                                        .execute_script(
                                            "disable_streaming".to_string(),
                                            JS_DISABLE_STREAMING.to_string(),
                                        )
                                        .await;
                                }
                            });
                        });
                        Ok(RenderResult::Streaming(stream_with_cleanup))
                    }
                    Err(e) => {
                        error!("Streaming failed, falling back to static render: {}", e);

                        let renderer = self.renderer.lock().await;

                        let _ = Self::disable_streaming(&renderer).await;

                        let fallback_result = self
                            .render_fallback(
                                &renderer,
                                fallback_script,
                                route_match,
                                return_rsc_on_fallback,
                                can_use_html_cache,
                                cache_key,
                            )
                            .await;

                        drop(renderer);

                        fallback_result
                    }
                }
            };

            streaming_operation.await
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

        if !first_tag_name.is_empty()
            && first_tag_name != "html"
            && (html.contains("<html") || html.contains("\"html\""))
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
        let mut seen_lazy_promise_ids = FxHashSet::default();

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

            let mut batch_seen_ids = FxHashSet::default();
            let unique_promises: Vec<_> = pending_promises
                .into_iter()
                .filter(|p| {
                    !seen_lazy_promise_ids.contains(&p.promise_id)
                        && batch_seen_ids.insert(p.promise_id.clone())
                })
                .collect();

            for lazy_promise in unique_promises {
                let resolve_script = format!(
                    "(async () => {{ return await globalThis['~rari'].lazy.resolve('{}'); }})()",
                    lazy_promise.promise_id
                );

                let result = match renderer
                    .runtime
                    .execute_script(
                        format!("resolve_promise_{}", lazy_promise.promise_id),
                        resolve_script,
                    )
                    .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        let error_msg = e.to_string();
                        if !error_msg.contains("Promise not found") {
                            tracing::error!(
                                "Failed to execute script for lazy promise {}: {}",
                                lazy_promise.promise_id,
                                e
                            );
                        }
                        return Err(RariError::internal(format!(
                            "Failed to materialize lazy promise {}",
                            lazy_promise.promise_id
                        )));
                    }
                };

                if let Some(success) = result.get("success").and_then(Value::as_bool)
                    && !success
                {
                    let error_msg =
                        result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                    tracing::error!(
                        "Failed to resolve lazy promise {}: {}",
                        lazy_promise.promise_id,
                        error_msg
                    );
                    return Err(RariError::internal(format!(
                        "Failed to materialize lazy promise {}: {}",
                        lazy_promise.promise_id, error_msg
                    )));
                }

                let resolved_content = result.get("data").unwrap_or(&result);

                let serialization_result = {
                    let mut serializer = renderer.serializer.lock();
                    serializer.serialize_rsc_json(resolved_content)
                };

                let wire_format = match serialization_result {
                    Ok(format) => format,
                    Err(e) => {
                        tracing::error!(
                            "Failed to serialize resolved content for promise {}: {}",
                            lazy_promise.promise_id,
                            e
                        );
                        return Err(RariError::internal(format!(
                            "Failed to serialize lazy promise {}: {}",
                            lazy_promise.promise_id, e
                        )));
                    }
                };

                let mut remapped_root_row = false;
                for line in wire_format.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let line_to_append = if let Some(colon_pos) = line.find(':') {
                        if line.starts_with(|c: char| c.is_ascii_hexdigit()) {
                            let row_id_str = &line[..colon_pos];
                            if usize::from_str_radix(row_id_str, 16).is_ok() {
                                let content = line.get(colon_pos + 1..).unwrap_or("");
                                let content_trimmed = content.trim_start();

                                let is_import_or_symbol = content_trimmed.starts_with("I[")
                                    || content_trimmed.starts_with("I{")
                                    || content_trimmed.starts_with("\"$S")
                                    || content_trimmed.starts_with("\"react.suspense");

                                let is_reference = content_trimmed.starts_with("\"$")
                                    && content_trimmed.len() > 2
                                    && !content_trimmed.starts_with("\"$S");

                                if is_import_or_symbol || is_reference || remapped_root_row {
                                    line.to_string()
                                } else {
                                    remapped_root_row = true;
                                    format!("{:x}:{}", lazy_promise.lazy_row_id, content)
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

                seen_lazy_promise_ids.insert(lazy_promise.promise_id.clone());
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
                .map_err(|e| RariError::internal(format!("Failed to serialize RSC data: {e}")))?
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
                .send(RscStreamChunk {
                    data: html.into_bytes(),
                    chunk_type: RscChunkType::InitialShell,
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
            .send(RscStreamChunk {
                data: html.into_bytes(),
                chunk_type: RscChunkType::InitialShell,
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
                client_component_id: utils::create_client_component_id(&template.file_path),
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
}
