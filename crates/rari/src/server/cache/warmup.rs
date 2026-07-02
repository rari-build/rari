use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use axum::http::HeaderMap;
use futures::stream::{self, StreamExt};
use rustc_hash::FxHashMap;
use tokio::sync::{Mutex, OnceCell};
use tracing::{error, info};

use crate::{
    rendering::layout::{LayoutRenderContext, LayoutRenderer, types::RenderResult},
    server::{
        ServerState,
        cache::response,
        handlers::app::{collect_page_metadata, wrap_html_with_metadata},
        middleware::request_context::RequestContext,
        routing::{AppRouteMatch, AppRouter, types::ParamValue},
    },
};

const WARMUP_CONCURRENCY: usize = 10;

/// Serialize warmup renders to prevent V8 global state corruption.
/// The RSC+Fizz pipeline shares V8 globals between the mutex-protected
/// RSC render and the non-mutex Fizz render. Without serialization,
/// concurrent warmup tasks interleave and produce wrong HTML.
static WARMUP_RENDER_LOCK: OnceCell<Mutex<()>> = OnceCell::const_new();

async fn warmup_render_lock() -> &'static Mutex<()> {
    WARMUP_RENDER_LOCK.get_or_init(|| async { Mutex::new(()) }).await
}

pub async fn warm_cache(state: &ServerState) {
    let app_router = match &state.app_router {
        Some(router) => router,
        None => {
            info!("[rari] Cache warmup: No app router available, skipping");
            return;
        }
    };

    let paths = app_router.warmup_paths();

    if paths.is_empty() {
        info!("[rari] Cache warmup: No routes to warm");
        return;
    }

    info!("[rari] Cache warmup: Pre-rendering {} routes...", paths.len());
    let start = Instant::now();

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    stream::iter(paths.iter())
        .for_each_concurrent(WARMUP_CONCURRENCY, |path| {
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);
            async move {
                match warm_route(state, app_router, path).await {
                    Ok(()) => {
                        success_count.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        error!("[rari] Cache warmup: Failed to warm '{}': {}", path, e);
                        error_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        })
        .await;

    let elapsed = start.elapsed();
    info!(
        "[rari] Cache warmup: Completed in {:.1}ms ({} succeeded, {} failed)",
        elapsed.as_secs_f64() * 1000.0,
        success_count.load(Ordering::Relaxed),
        error_count.load(Ordering::Relaxed),
    );
}

async fn warm_route(
    state: &ServerState,
    app_router: &Arc<AppRouter>,
    path: &str,
) -> Result<(), String> {
    let route_match =
        app_router.match_route(path).map_err(|e| format!("Route match failed: {e}"))?;

    if route_match.loading.is_some() {
        return Ok(());
    }

    let mut context = create_warmup_context(&route_match);

    let layout_renderer = LayoutRenderer::with_shared_cache(
        Arc::clone(&state.renderer),
        Arc::clone(&state.layout_html_cache),
    );

    let request_context = Arc::new(RequestContext::new(route_match.route.path.clone()));

    let _render_guard = warmup_render_lock().await.lock().await;

    let render_result = layout_renderer
        .render_route_with_streaming(
            &route_match,
            &context,
            Some(Arc::clone(&request_context)),
            false,
        )
        .await
        .map_err(|e| format!("Render failed: {e}"))?;

    context.metadata = collect_page_metadata(state, &route_match, &context).await;

    let html = match render_result {
        RenderResult::Static(html) => html,
        RenderResult::FizzHtmlStream { shell, closing, mut chunks } => {
            let mut html = String::from_utf8_lossy(&shell).into_owned();
            while let Some(chunk_result) = chunks.recv().await {
                match chunk_result {
                    Ok(data) => html.push_str(&String::from_utf8_lossy(&data)),
                    Err(_) => break,
                }
            }
            html.push_str(&String::from_utf8_lossy(&closing));
            html
        }
        _ => return Ok(()),
    };

    let html = wrap_html_with_metadata(html, context.metadata.as_ref(), state);

    let html_cache_key = response::ResponseCache::generate_cache_key(path, None);
    let etag = response::ResponseCache::generate_etag(html.as_bytes());
    let cache_control = state.config.get_cache_control_for_route(path);
    let cache_policy = response::RouteCachePolicy::from_cache_control(cache_control, path);

    if cache_policy.enabled && state.response_cache.config.enabled {
        let body_bytes = bytes::Bytes::from(html);

        let compressed_gzip = {
            use crate::server::compression::{CompressionEncoding, compress_body};
            let (compressed, enc) =
                compress_body(body_bytes.clone(), CompressionEncoding::Gzip).await;
            if matches!(enc, CompressionEncoding::Gzip) { Some(compressed) } else { None }
        };

        let compressed_zstd = {
            use crate::server::compression::{CompressionEncoding, compress_body};
            let (compressed, enc) =
                compress_body(body_bytes.clone(), CompressionEncoding::Zstd).await;
            if matches!(enc, CompressionEncoding::Zstd) { Some(compressed) } else { None }
        };

        let compressed_br = {
            use crate::server::compression::{CompressionEncoding, compress_body};
            let (compressed, enc) =
                compress_body(body_bytes.clone(), CompressionEncoding::Brotli).await;
            if matches!(enc, CompressionEncoding::Brotli) { Some(compressed) } else { None }
        };

        state.static_fast_cache.insert(
            path.to_string(),
            Arc::new(response::PrebuiltResponse {
                identity: body_bytes.clone(),
                gzip: compressed_gzip.clone(),
                br: compressed_br.clone(),
                zstd: compressed_zstd.clone(),
                etag: etag.clone(),
                content_type: "text/html; charset=utf-8".to_string(),
                cache_control: cache_control.to_string(),
                is_not_found: false,
            }),
        );

        let cached_response = response::CachedResponse {
            body: body_bytes,
            headers: HeaderMap::new(),
            metadata: response::CacheMetadata {
                cached_at: Instant::now(),
                ttl: cache_policy.ttl,
                etag: Some(etag),
                tags: cache_policy.tags.clone(),
            },
            compressed_zstd,
            compressed_br,
            compressed_gzip,
        };

        state.response_cache.set(html_cache_key, cached_response).await;
    }

    let rsc_result =
        layout_renderer.render_route_by_mode(&route_match, &context, Some(request_context)).await;

    if let Ok(rsc_flight_protocol) = rsc_result {
        let rsc_cache_key =
            response::ResponseCache::generate_cache_key_with_mode(path, None, Some("rsc"));

        if cache_policy.enabled && state.response_cache.config.enabled {
            let mut cache_headers = HeaderMap::new();

            if let Some(ref metadata) = context.metadata
                && let Ok(metadata_json) = serde_json::to_string(metadata)
            {
                let encoded_metadata = urlencoding::encode(&metadata_json);
                if let Ok(header_value) = encoded_metadata.as_ref().parse() {
                    cache_headers.insert("x-rari-metadata", header_value);
                }
            }

            let cached_response = response::CachedResponse {
                body: bytes::Bytes::from(rsc_flight_protocol),
                headers: cache_headers,
                metadata: response::CacheMetadata {
                    cached_at: Instant::now(),
                    ttl: cache_policy.ttl,
                    etag: None,
                    tags: cache_policy.tags,
                },
                compressed_zstd: None,
                compressed_br: None,
                compressed_gzip: None,
            };

            state.response_cache.set(rsc_cache_key, cached_response).await;
        }
    }

    Ok(())
}

fn create_warmup_context(route_match: &AppRouteMatch) -> LayoutRenderContext {
    let mut params: FxHashMap<String, ParamValue> = FxHashMap::default();

    for (key, value) in &route_match.params {
        params.insert(key.clone(), value.clone());
    }

    LayoutRenderContext {
        params,
        search_params: FxHashMap::default(),
        headers: FxHashMap::default(),
        pathname: route_match.pathname.clone(),
        metadata: None,
    }
}
