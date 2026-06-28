use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use futures::stream::{self, StreamExt};
use rustc_hash::FxHashMap;
use tracing::{error, info};

use crate::{
    rendering::layout::{LayoutRenderContext, LayoutRenderer},
    server::{ServerState, cache::response, routing::types::ParamValue},
};

const WARMUP_CONCURRENCY: usize = 10;

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
    app_router: &Arc<crate::server::routing::app_router::AppRouter>,
    path: &str,
) -> Result<(), String> {
    let route_match =
        app_router.match_route(path).map_err(|e| format!("Route match failed: {e}"))?;

    if route_match.loading.is_some() {
        return Ok(());
    }

    let context = create_warmup_context(&route_match);

    let layout_renderer = LayoutRenderer::with_shared_cache(
        Arc::clone(&state.renderer),
        Arc::clone(&state.layout_html_cache),
    );

    let request_context =
        Arc::new(crate::server::middleware::request_context::RequestContext::new(
            route_match.route.path.clone(),
        ));

    let rsc_flight_protocol = layout_renderer
        .render_route_by_mode(&route_match, &context, Some(request_context))
        .await
        .map_err(|e| format!("Render failed: {e}"))?;

    let cache_key = response::ResponseCache::generate_cache_key_with_mode(path, None, Some("rsc"));

    let cache_control = state.config.get_cache_control_for_route(path);
    let cache_policy = response::RouteCachePolicy::from_cache_control(cache_control, path);

    if cache_policy.enabled && state.response_cache.config.enabled {
        let cached_response = response::CachedResponse {
            body: bytes::Bytes::from(rsc_flight_protocol),
            headers: axum::http::HeaderMap::new(),
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

        state.response_cache.set(cache_key, cached_response).await;
    }

    Ok(())
}

fn create_warmup_context(
    route_match: &crate::server::routing::app_router::AppRouteMatch,
) -> LayoutRenderContext {
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
