use crate::rsc::layout_renderer::LayoutRenderer;
use crate::rsc::rsc_html_renderer::{RscHtmlRenderer, RscToHtmlConverter};
use crate::server::ServerState;
use crate::server::cache_loader::CacheLoader;
use crate::server::config::Config;
use crate::server::rendering::html_utils::{
    extract_asset_links_from_index_html, inject_assets_into_html, inject_rsc_payload,
    inject_vite_client,
};
use crate::server::response_cache;
use crate::server::streaming_response::StreamingHtmlResponse;
use crate::server::utils::http_utils::{extract_headers, extract_search_params, get_content_type};
use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tracing::{debug, error, warn};

pub async fn render_with_fallback(
    state: Arc<ServerState>,
    route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::layout_renderer::LayoutRenderContext,
) -> Result<Response, StatusCode> {
    debug!("⏱️ render_with_fallback called for route: {}", route_match.route.path);

    let layout_renderer = LayoutRenderer::new(state.renderer.clone());

    match render_streaming_with_layout(
        state.clone(),
        route_match.clone(),
        context.clone(),
        &layout_renderer,
    )
    .await
    {
        Ok(response) => Ok(response),
        Err(e) => {
            warn!(
                "Streaming failed for route {}, falling back to synchronous rendering: {:?}",
                route_match.route.path, e
            );

            render_synchronous(state, route_match, context).await
        }
    }
}

pub async fn render_synchronous(
    state: Arc<ServerState>,
    route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::layout_renderer::LayoutRenderContext,
) -> Result<Response, StatusCode> {
    debug!("Using synchronous rendering for route: {}", route_match.route.path);

    let layout_renderer = LayoutRenderer::new(state.renderer.clone());
    let request_context = std::sync::Arc::new(crate::server::request_context::RequestContext::new(
        route_match.route.path.clone(),
    ));

    match layout_renderer
        .render_route_to_html_direct(&route_match, &context, Some(request_context))
        .await
    {
        Ok(render_result) => match render_result {
            crate::rsc::layout_renderer::RenderResult::Static(html_content)
            | crate::rsc::layout_renderer::RenderResult::StaticWithPayload {
                html: html_content,
                ..
            } => {
                debug!("Successfully rendered HTML synchronously ({} bytes)", html_content.len());

                let final_html = match inject_assets_into_html(&html_content, &state.config).await {
                    Ok(html) => html,
                    Err(e) => {
                        warn!("Failed to inject assets, using original HTML: {}", e);
                        html_content
                    }
                };

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/html; charset=utf-8")
                    .header("x-render-mode", "synchronous")
                    .body(Body::from(final_html))
                    .expect("Valid HTML response"))
            }
            crate::rsc::layout_renderer::RenderResult::Streaming(_) => {
                warn!("Unexpected streaming result in render_synchronous, falling back to shell");
                render_fallback_html(&state, &route_match.route.path).await
            }
        },
        Err(e) => {
            error!("Synchronous rendering failed: {}", e);
            render_fallback_html(&state, &route_match.route.path).await
        }
    }
}

pub async fn render_streaming_with_layout(
    state: Arc<ServerState>,
    route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::layout_renderer::LayoutRenderContext,
    layout_renderer: &LayoutRenderer,
) -> Result<Response, StatusCode> {
    debug!("Starting streaming render for route: {}", route_match.route.path);

    let layout_count = route_match.layouts.len();
    debug!("Rendering route with {} layouts for streaming", layout_count);

    let request_context = std::sync::Arc::new(crate::server::request_context::RequestContext::new(
        route_match.route.path.clone(),
    ));

    let render_result = match layout_renderer
        .render_route_to_html_direct(&route_match, &context, Some(request_context))
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to render route for streaming '{}': {}", route_match.route.path, e);
            error!(
                "Route rendering failure context - Route: {}, Page component: {}, Layout count: {}",
                route_match.route.path, route_match.route.file_path, layout_count
            );

            for (idx, layout) in route_match.layouts.iter().enumerate() {
                error!("  Layout {}: {} (is_root: {})", idx, layout.file_path, layout.is_root);
            }

            warn!("Falling back to synchronous rendering due to render_route failure");
            return render_synchronous(state, route_match, context).await;
        }
    };

    let mut rsc_stream = match render_result {
        crate::rsc::layout_renderer::RenderResult::Streaming(stream) => {
            debug!("Suspense detected, using streaming path");
            stream
        }
        crate::rsc::layout_renderer::RenderResult::Static(html) => {
            debug!("No Suspense detected, returning static HTML");
            let final_html = match inject_assets_into_html(&html, &state.config).await {
                Ok(html) => html,
                Err(e) => {
                    warn!("Failed to inject assets, using original HTML: {}", e);
                    html
                }
            };

            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .header("x-render-mode", "static")
                .body(Body::from(final_html))
                .expect("Valid HTML response"));
        }
        crate::rsc::layout_renderer::RenderResult::StaticWithPayload { html, rsc_payload } => {
            debug!("No Suspense detected, returning static HTML with RSC payload");
            let html_with_payload = inject_rsc_payload(&html, &rsc_payload);
            let final_html = match inject_assets_into_html(&html_with_payload, &state.config).await
            {
                Ok(html) => html,
                Err(e) => {
                    warn!("Failed to inject assets, using original HTML: {}", e);
                    html_with_payload
                }
            };

            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .header("x-render-mode", "static-with-payload")
                .body(Body::from(final_html))
                .expect("Valid HTML response"));
        }
    };

    debug!("Successfully started streaming for route: {}", route_match.route.path);

    let asset_links = extract_asset_links_from_index_html().await;

    let html_renderer = {
        let renderer = state.renderer.lock().await;
        Arc::new(RscHtmlRenderer::new(Arc::clone(&renderer.runtime)))
    };

    let converter = if let Some(links) = asset_links {
        Arc::new(tokio::sync::Mutex::new(RscToHtmlConverter::with_assets(links, html_renderer)))
    } else {
        Arc::new(tokio::sync::Mutex::new(RscToHtmlConverter::new(html_renderer)))
    };

    let should_continue = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let should_continue_clone = should_continue.clone();

    let html_stream = async_stream::stream! {
        let mut chunk_count = 0;

        while should_continue_clone.load(std::sync::atomic::Ordering::Relaxed) {
            match rsc_stream.next_chunk().await {
                Some(chunk) => {
                    chunk_count += 1;
                    let mut conv = converter.lock().await;

                    match conv.convert_chunk(chunk).await {
                        Ok(html_bytes) => {
                            if !html_bytes.is_empty() {
                                debug!("Yielding HTML chunk {} of {} bytes", chunk_count, html_bytes.len());
                                yield Ok(html_bytes);
                            }
                        }
                        Err(e) => {
                            if e.to_string().contains("disconnected") || e.to_string().contains("broken pipe") {
                                warn!("Client disconnected, stopping stream processing");
                                should_continue_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                                break;
                            }

                            error!("Error converting RSC chunk to HTML: {}", e);
                            yield Err(e);
                        }
                    }
                }
                None => {
                    debug!("Stream completed successfully after {} chunks", chunk_count);
                    break;
                }
            }
        }

        if !should_continue_clone.load(std::sync::atomic::Ordering::Relaxed) {
            debug!("Stream stopped early due to client disconnection");
        }
    };

    Ok(StreamingHtmlResponse::new(html_stream).into_response())
}

pub async fn render_fallback_html(state: &ServerState, path: &str) -> Result<Response, StatusCode> {
    debug!("Rendering fallback HTML shell for path: {}", path);

    let index_path = if state.config.is_development() {
        let root_index = std::path::PathBuf::from("index.html");
        if root_index.exists() { root_index } else { state.config.public_dir().join("index.html") }
    } else {
        state.config.public_dir().join("index.html")
    };

    if index_path.exists() {
        if state.config.is_production()
            && let Some(cached_html) = state.html_cache.get(path)
        {
            debug!("✅ Cache HIT for fallback HTML: {}", path);
            let html = cached_html.clone();
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(html))
                .expect("Valid HTML response"));
        }

        match std::fs::read_to_string(&index_path) {
            Ok(html_content) => {
                let final_html = if state.config.is_development() {
                    debug!("Reading index.html and injecting Vite client for development");
                    inject_vite_client(&html_content, state.config.vite.port)
                } else {
                    debug!("Serving built index.html as fallback");
                    html_content
                };

                if state.config.is_production() {
                    state.html_cache.insert(path.to_string(), final_html.clone());
                }

                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/html; charset=utf-8")
                    .body(Body::from(final_html))
                    .expect("Valid HTML response"));
            }
            Err(e) => {
                warn!("Failed to read index.html from {:?}: {}", index_path, e);
            }
        }
    }

    if state.config.is_development() {
        let vite_port = state.config.vite.port;
        let html_shell = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Rari App Router</title>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="http://localhost:{}/@vite/client"></script>
  <script type="module">
    import 'http://localhost:{}/@id/virtual:rari-entry-client';
  </script>
</body>
</html>"#,
            vite_port, vite_port
        );

        debug!("index.html not found, serving generated development HTML shell as fallback");
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(html_shell))
            .expect("Valid HTML response"));
    }

    let error_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Build Required</title>
</head>
<body>
  <div style="padding: 40px; font-family: sans-serif;">
    <h1>Build Required</h1>
    <p>Please build your application first:</p>
    <pre>npm run build</pre>
    <p>Or run in development mode with Vite:</p>
    <pre>npm run dev</pre>
  </div>
</body>
</html>"#;

    warn!("No built files found, serving error page");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(error_html))
        .expect("Valid HTML response"))
}

#[axum::debug_handler]
pub async fn handle_app_route(
    State(state): State<ServerState>,
    uri: axum::http::Uri,
    Query(query_params): Query<FxHashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> Result<Response, StatusCode> {
    use crate::server::request_type::{RenderMode, RequestTypeDetector};

    let path = uri.path();

    fn should_use_streaming(
        route_match: &crate::server::routing::AppRouteMatch,
        config: &Config,
    ) -> bool {
        config.rsc.enable_streaming && route_match.loading.is_some()
    }

    if path.len() > 1 {
        let path_without_leading_slash = &path[1..];

        if path_without_leading_slash.contains('.') {
            let file_path = state.config.public_dir().join(path_without_leading_slash);

            if file_path.exists() && file_path.is_file() {
                match std::fs::read(&file_path) {
                    Ok(content) => {
                        let content_type = get_content_type(path_without_leading_slash);
                        let cache_control = &state.config.caching.static_files;
                        return Ok(Response::builder()
                            .header("content-type", content_type)
                            .header("cache-control", cache_control)
                            .body(Body::from(content))
                            .expect("Valid static file response"));
                    }
                    Err(e) => {
                        error!("Failed to read static file {}: {}", file_path.display(), e);
                    }
                }
            }

            debug!("Static file not found: {}", path);
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let app_router = match &state.app_router {
        Some(router) => router,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let route_match = match app_router.match_route(path) {
        Ok(m) => m,
        Err(_) => match app_router.create_not_found_match(path) {
            Some(not_found_match) => not_found_match,
            None => return Err(StatusCode::NOT_FOUND),
        },
    };

    debug!("App route matched: {} -> {}", path, route_match.route.path);

    let request_context =
        std::sync::Arc::new(crate::server::request_context::RequestContext::new(path.to_string()));

    let render_mode = RequestTypeDetector::detect_render_mode(&headers);
    debug!("Detected render mode: {:?}", render_mode);

    let query_params_for_cache = query_params.clone();
    let search_params = extract_search_params(query_params);

    let request_headers = extract_headers(&headers);

    let context = crate::rsc::layout_renderer::create_layout_context(
        route_match.params.clone(),
        search_params,
        request_headers,
        route_match.pathname.clone(),
    );

    let layout_renderer = LayoutRenderer::new(state.renderer.clone());

    match render_mode {
        RenderMode::RscNavigation => {
            debug!("Rendering RSC wire format for client navigation");
            match layout_renderer
                .render_route_optimized(
                    &route_match,
                    &context,
                    render_mode,
                    Some(request_context.clone()),
                )
                .await
            {
                Ok(rsc_wire_format) => {
                    debug!(
                        "Successfully rendered RSC wire format ({} bytes)",
                        rsc_wire_format.len()
                    );
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/x-component")
                        .body(Body::from(rsc_wire_format))
                        .expect("Valid RSC response"))
                }
                Err(e) => {
                    error!("Failed to render RSC: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        RenderMode::Ssr => {
            debug!("⏱️ SSR mode detected, checking streaming...");

            let use_streaming = should_use_streaming(&route_match, &state.config);

            debug!(
                "Streaming decision: enable_streaming={}, has_loading={}, use_streaming={}",
                state.config.rsc.enable_streaming,
                route_match.loading.is_some(),
                use_streaming
            );

            if use_streaming {
                let streaming_start = std::time::Instant::now();
                debug!("Using streaming SSR with fallback for route: {}", path);
                let result = render_with_fallback(Arc::new(state), route_match, context).await;
                let streaming_duration = streaming_start.elapsed();
                debug!("⚡ Streaming render took: {:?}", streaming_duration);
                return result;
            }

            debug!("Rendering HTML for SSR (initial page load) using direct HTML path");

            let cache_key = response_cache::ResponseCache::generate_cache_key(
                path,
                if query_params_for_cache.is_empty() {
                    None
                } else {
                    Some(&query_params_for_cache)
                },
            );

            let client_etag = headers.get("if-none-match").and_then(|v| v.to_str().ok());

            if let Some(cached) = state.response_cache.get(&cache_key).await {
                debug!("Cache hit for route: {}", path);

                if let (Some(cached_etag), Some(client_etag)) = (&cached.metadata.etag, client_etag)
                    && cached_etag == client_etag
                {
                    debug!("ETag match, returning 304 Not Modified");
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .header("etag", cached_etag)
                        .body(Body::empty())
                        .expect("Valid 304 response"));
                }

                let mut response_builder = Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/html; charset=utf-8")
                    .header("x-cache", "HIT");

                if let Some(etag) = &cached.metadata.etag {
                    response_builder = response_builder.header("etag", etag);
                }

                for (key, value) in cached.headers.iter() {
                    response_builder = response_builder.header(key, value);
                }

                return Ok(response_builder
                    .body(Body::from(cached.body))
                    .expect("Valid cached response"));
            }

            debug!("Cache miss for route: {}", path);
            let total_start = std::time::Instant::now();

            let render_start = std::time::Instant::now();
            let render_result = match layout_renderer
                .render_route_to_html_direct(&route_match, &context, Some(request_context.clone()))
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    error!("Direct HTML rendering failed: {}, falling back to shell", e);
                    return render_fallback_html(&state, path).await;
                }
            };
            let render_duration = render_start.elapsed();
            debug!("⚡ Direct HTML render took: {:?}", render_duration);

            let (html_with_assets, etag) = match render_result {
                crate::rsc::layout_renderer::RenderResult::Static(html_content) => {
                    debug!("Using static rendering path for route: {}", path);

                    let total_duration = total_start.elapsed();
                    debug!(
                        "⚡⚡⚡ Total SSR render took: {:?} (direct HTML: {:?})",
                        total_duration, render_duration
                    );

                    let html_with_assets =
                        match inject_assets_into_html(&html_content, &state.config).await {
                            Ok(html) => html,
                            Err(_) => html_content,
                        };

                    let etag =
                        response_cache::ResponseCache::generate_etag(html_with_assets.as_bytes());

                    (html_with_assets, etag)
                }
                crate::rsc::layout_renderer::RenderResult::StaticWithPayload {
                    html: html_content,
                    rsc_payload,
                } => {
                    debug!("Using static rendering path with RSC payload for route: {}", path);

                    let total_duration = total_start.elapsed();
                    debug!(
                        "⚡⚡⚡ Total SSR render took: {:?} (direct HTML: {:?})",
                        total_duration, render_duration
                    );

                    debug!("Injecting RSC payload for hydration ({} bytes)", rsc_payload.len());
                    let html_with_payload = inject_rsc_payload(&html_content, &rsc_payload);

                    let html_with_assets =
                        match inject_assets_into_html(&html_with_payload, &state.config).await {
                            Ok(html) => html,
                            Err(_) => html_with_payload,
                        };

                    let etag =
                        response_cache::ResponseCache::generate_etag(html_with_assets.as_bytes());

                    (html_with_assets, etag)
                }
                crate::rsc::layout_renderer::RenderResult::Streaming(stream) => {
                    debug!("Using streaming rendering path for route: {}", path);

                    let asset_links = extract_asset_links_from_index_html().await;

                    let html_renderer = {
                        let renderer = state.renderer.lock().await;
                        Arc::new(RscHtmlRenderer::new(Arc::clone(&renderer.runtime)))
                    };

                    let converter = if let Some(links) = asset_links {
                        Arc::new(tokio::sync::Mutex::new(RscToHtmlConverter::with_assets(
                            links,
                            html_renderer,
                        )))
                    } else {
                        Arc::new(tokio::sync::Mutex::new(RscToHtmlConverter::new(html_renderer)))
                    };

                    let should_continue = Arc::new(std::sync::atomic::AtomicBool::new(true));
                    let should_continue_clone = should_continue.clone();

                    let mut rsc_stream = stream;

                    let html_stream = async_stream::stream! {
                        let mut chunk_count = 0;

                        while should_continue_clone.load(std::sync::atomic::Ordering::Relaxed) {
                            match rsc_stream.next_chunk().await {
                                Some(chunk) => {
                                    chunk_count += 1;
                                    let mut conv = converter.lock().await;

                                    match conv.convert_chunk(chunk).await {
                                        Ok(html_bytes) => {
                                            if !html_bytes.is_empty() {
                                                debug!("Yielding HTML chunk {} of {} bytes", chunk_count, html_bytes.len());
                                                yield Ok(html_bytes);
                                            }
                                        }
                                        Err(e) => {
                                            if e.to_string().contains("disconnected") || e.to_string().contains("broken pipe") {
                                                warn!("Client disconnected, stopping stream processing");
                                                should_continue_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                                                break;
                                            }

                                            error!("Error converting RSC chunk to HTML: {}", e);
                                            yield Err(e);
                                        }
                                    }
                                }
                                None => {
                                    debug!("Stream completed successfully after {} chunks", chunk_count);
                                    break;
                                }
                            }
                        }

                        if !should_continue_clone.load(std::sync::atomic::Ordering::Relaxed) {
                            debug!("Stream stopped early due to client disconnection");
                        }
                    };

                    return Ok(StreamingHtmlResponse::new(html_stream).into_response());
                }
            };

            let mut response_builder = Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .header("etag", &etag)
                .header("x-cache", "MISS");

            let page_configs = state.page_cache_configs.read().await;
            let mut cache_control_value = None;
            let mut response_headers = axum::http::HeaderMap::new();

            if let Some(page_cache_config) =
                CacheLoader::find_matching_cache_config(&page_configs, path)
            {
                for (key, value) in page_cache_config {
                    let header_name = key.to_lowercase();
                    response_builder = response_builder.header(&header_name, value);

                    if header_name == "cache-control" {
                        cache_control_value = Some(value.clone());
                    }

                    if let Ok(header_name) =
                        axum::http::HeaderName::from_bytes(header_name.as_bytes())
                        && let Ok(header_value) = axum::http::HeaderValue::from_str(value)
                    {
                        response_headers.insert(header_name, header_value);
                    }
                }
            }

            let cache_policy = if let Some(cc) = cache_control_value.as_deref() {
                response_cache::RouteCachePolicy::from_cache_control(cc, path)
            } else {
                let mut policy = response_cache::RouteCachePolicy {
                    ttl: state.response_cache.config.default_ttl,
                    ..Default::default()
                };
                policy.tags.push(path.to_string());

                policy
            };

            if cache_policy.enabled {
                let cached_response = response_cache::CachedResponse {
                    body: bytes::Bytes::from(html_with_assets.clone()),
                    headers: response_headers,
                    metadata: response_cache::CacheMetadata {
                        cached_at: std::time::Instant::now(),
                        ttl: cache_policy.ttl,
                        etag: Some(etag),
                        tags: cache_policy.tags,
                    },
                };

                state.response_cache.set(cache_key, cached_response).await;
                debug!("Stored response in cache for route: {} (ttl={}s)", path, cache_policy.ttl);
            } else {
                debug!("Caching disabled for route: {}", path);
            }

            Ok(response_builder.body(Body::from(html_with_assets)).expect("Valid HTML response"))
        }
    }
}
