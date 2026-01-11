use crate::rsc::rendering::html::{RscHtmlRenderer, RscToHtmlConverter};
use crate::rsc::rendering::layout::types::PageMetadata;
use crate::rsc::rendering::layout::{LayoutRenderContext, LayoutRenderer};
use crate::server::ServerState;
use crate::server::cache::response_cache;
use crate::server::config::Config;
use crate::server::loaders::cache_loader::CacheLoader;
use crate::server::rendering::html_utils::{
    extract_asset_links_from_index_html, inject_assets_into_html, inject_vite_client,
};
use crate::server::rendering::metadata_injection::inject_metadata;
use crate::server::rendering::streaming_response::StreamingHtmlResponse;
use crate::server::routing::app_router::AppRouteMatch;
use crate::server::utils::http_utils::{extract_headers, extract_search_params, get_content_type};
use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cow_utils::CowUtils;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tracing::error;

fn wrap_html_with_metadata(html_content: String, metadata: Option<&PageMetadata>) -> String {
    let is_complete = html_content.trim_start().starts_with("<!DOCTYPE")
        || html_content.trim_start().starts_with("<html");

    if is_complete {
        if let Some(metadata) = metadata {
            inject_metadata(&html_content, metadata)
        } else {
            html_content
        }
    } else {
        let title =
            metadata.and_then(|m| m.title.as_ref()).map(|t| t.as_str()).unwrap_or("Rari App");

        let base_shell = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
</head>
<body>
<div id="root">{}</div>
</body>
</html>"#,
            title, html_content
        );

        if let Some(metadata) = metadata {
            inject_metadata(&base_shell, metadata)
        } else {
            base_shell
        }
    }
}

async fn collect_page_metadata(
    state: &ServerState,
    route_match: &AppRouteMatch,
    context: &LayoutRenderContext,
) -> Option<PageMetadata> {
    let dist_server_path = std::env::current_dir()
        .ok()
        .map(|p| p.join("dist/server"))
        .and_then(|p| p.canonicalize().ok());

    let base_path = match dist_server_path {
        Some(path) => path,
        None => {
            error!("Could not determine dist/server path for metadata collection");
            return None;
        }
    };

    fn convert_route_path_to_dist_path(path: &str) -> String {
        use regex::Regex;
        let re =
            Regex::new(r"\[+([^\]]+)\]+").expect("Invalid regex pattern for route path conversion");
        re.replace_all(path, |caps: &regex::Captures| {
            let param = &caps[1];
            let bracket_count = caps[0].matches('[').count();
            let underscores = "_".repeat(bracket_count);
            format!("{}{}{}", underscores, param, underscores)
        })
        .to_string()
    }

    let layout_paths: Vec<String> = route_match
        .layouts
        .iter()
        .filter_map(|layout| {
            let js_filename =
                layout.file_path.cow_replace(".tsx", ".js").cow_replace(".ts", ".js").into_owned();
            let dist_filename = convert_route_path_to_dist_path(&js_filename);
            let file_path = base_path.join("app").join(&dist_filename);
            if file_path.exists() { Some(format!("file://{}", file_path.display())) } else { None }
        })
        .collect();

    let js_filename = route_match
        .route
        .file_path
        .cow_replace(".tsx", ".js")
        .cow_replace(".ts", ".js")
        .into_owned();
    let dist_filename = convert_route_path_to_dist_path(&js_filename);
    let page_file_path = base_path.join("app").join(&dist_filename);
    let page_path = if page_file_path.exists() {
        format!("file://{}", page_file_path.display())
    } else {
        return None;
    };

    let renderer = state.renderer.lock().await;
    match renderer
        .runtime
        .collect_metadata(
            layout_paths,
            page_path,
            context.params.clone(),
            context.search_params.clone(),
        )
        .await
    {
        Ok(metadata_value) => match serde_json::from_value::<PageMetadata>(metadata_value) {
            Ok(metadata) => Some(metadata),
            Err(e) => {
                error!("Failed to deserialize metadata: {}", e);
                None
            }
        },
        Err(e) => {
            error!("Failed to collect metadata from runtime: {}", e);
            None
        }
    }
}

pub async fn render_with_fallback(
    state: Arc<ServerState>,
    route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::rendering::layout::LayoutRenderContext,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_renderer = LayoutRenderer::new(state.renderer.clone());

    match render_streaming_with_layout(
        state.clone(),
        route_match.clone(),
        context.clone(),
        &layout_renderer,
        accept_encoding,
    )
    .await
    {
        Ok(response) => Ok(response),
        Err(e) => {
            error!("Streaming render failed, falling back to synchronous: {}", e);
            render_synchronous(state, route_match, context, accept_encoding).await
        }
    }
}

pub async fn render_rsc_navigation_streaming(
    state: Arc<ServerState>,
    route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::rendering::layout::LayoutRenderContext,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_renderer = LayoutRenderer::new(state.renderer.clone());
    let is_not_found = route_match.not_found.is_some();

    let request_context =
        std::sync::Arc::new(crate::server::middleware::request_context::RequestContext::new(
            route_match.route.path.clone(),
        ));

    let render_result = match layout_renderer
        .render_route_to_html_direct(&route_match, &context, Some(request_context))
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!(
                "Failed to render RSC navigation for streaming '{}': {}",
                route_match.route.path, e
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let rsc_stream = match render_result {
        crate::rsc::rendering::layout::RenderResult::Streaming(stream) => stream,
        crate::rsc::rendering::layout::RenderResult::Static(_) => {
            error!("Expected streaming result for RSC navigation with Suspense");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    render_rsc_streaming_response(
        state,
        route_match,
        context,
        rsc_stream,
        is_not_found,
        accept_encoding,
    )
    .await
}

async fn render_rsc_streaming_response(
    _state: Arc<ServerState>,
    _route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::rendering::layout::LayoutRenderContext,
    mut rsc_stream: crate::rsc::rendering::streaming::stream::RscStream,
    is_not_found: bool,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    use crate::server::compression::{CompressionEncoding, compress_stream};

    let should_continue = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let should_continue_clone = should_continue.clone();

    let rsc_wire_stream = async_stream::stream! {
        while should_continue_clone.load(std::sync::atomic::Ordering::Relaxed) {
            match rsc_stream.next_chunk().await {
                Some(chunk) => {
                    let data = String::from_utf8_lossy(&chunk.data).to_string();
                    yield Ok::<_, std::io::Error>(bytes::Bytes::from(data));
                }
                None => {
                    break;
                }
            }
        }
    };

    let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
    let compressed_stream = compress_stream(rsc_wire_stream, encoding);

    let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

    let mut response_builder = Response::builder()
        .status(status_code)
        .header("content-type", "text/x-component")
        .header("transfer-encoding", "chunked")
        .header("x-render-mode", "streaming")
        .header("cache-control", "no-cache")
        .header("x-content-type-options", "nosniff");

    if let Some(encoding_header) = encoding.as_header_value() {
        response_builder = response_builder.header("content-encoding", encoding_header);
    }

    if let Some(ref metadata) = context.metadata
        && let Ok(metadata_json) = serde_json::to_string(metadata)
    {
        let encoded_metadata = urlencoding::encode(&metadata_json);
        response_builder = response_builder.header("x-rari-metadata", encoded_metadata.as_ref());
    }

    let body = Body::from_stream(compressed_stream);
    Ok(response_builder.body(body).expect("Valid RSC streaming response"))
}

pub async fn render_synchronous(
    state: Arc<ServerState>,
    route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::rendering::layout::LayoutRenderContext,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_renderer = LayoutRenderer::new(state.renderer.clone());
    let request_context =
        std::sync::Arc::new(crate::server::middleware::request_context::RequestContext::new(
            route_match.route.path.clone(),
        ));

    let is_not_found = route_match.not_found.is_some();

    match layout_renderer
        .render_route_to_html_direct(&route_match, &context, Some(request_context))
        .await
    {
        Ok(render_result) => match render_result {
            crate::rsc::rendering::layout::RenderResult::Static(html_content) => {
                let html_with_metadata =
                    wrap_html_with_metadata(html_content, context.metadata.as_ref());

                let final_html =
                    match inject_assets_into_html(&html_with_metadata, &state.config).await {
                        Ok(html) => html,
                        Err(e) => {
                            error!("Failed to inject assets into HTML: {}", e);
                            html_with_metadata
                        }
                    };

                let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

                Ok(Response::builder()
                    .status(status_code)
                    .header("content-type", "text/html; charset=utf-8")
                    .header("x-render-mode", "synchronous")
                    .body(Body::from(final_html))
                    .expect("Valid HTML response"))
            }
            crate::rsc::rendering::layout::RenderResult::Streaming(stream) => {
                render_streaming_response(
                    state,
                    route_match,
                    context,
                    stream,
                    is_not_found,
                    accept_encoding,
                )
                .await
            }
        },
        Err(e) => {
            error!("Synchronous rendering failed: {}", e);
            render_fallback_html(&state, &route_match.route.path, is_not_found).await
        }
    }
}

async fn render_streaming_response(
    state: Arc<ServerState>,
    _route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::rendering::layout::LayoutRenderContext,
    mut rsc_stream: crate::rsc::rendering::streaming::stream::RscStream,
    is_not_found: bool,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    use crate::server::compression::{CompressionEncoding, compress_stream};

    let asset_links = extract_asset_links_from_index_html().await;

    let html_renderer = {
        let renderer = state.renderer.lock().await;
        Arc::new(RscHtmlRenderer::new(Arc::clone(&renderer.runtime)))
    };

    let csrf_token = state.csrf_manager.generate_token();
    let asset_tags = asset_links.as_deref().unwrap_or("");

    let title = context
        .metadata
        .as_ref()
        .and_then(|m| m.title.as_ref())
        .map(|t| t.as_str())
        .unwrap_or("Rari App");

    let base_shell = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <meta name="csrf-token" content="{}" />
    {}
    <style>
        .rari-loading {{
            animation: rari-pulse 1.5s ease-in-out infinite;
        }}
        @keyframes rari-pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.5; }}
        }}
    </style>
</head>
<body>
<div id="root">
"#,
        title, csrf_token, asset_tags
    );

    let base_shell = if let Some(ref metadata) = context.metadata {
        inject_metadata(&base_shell, metadata)
    } else {
        base_shell
    };

    let converter = Arc::new(tokio::sync::Mutex::new(RscToHtmlConverter::with_custom_shell(
        base_shell,
        None,
        html_renderer,
    )));

    let should_continue = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let should_continue_clone = should_continue.clone();

    let html_stream = async_stream::stream! {
        while should_continue_clone.load(std::sync::atomic::Ordering::Relaxed) {
            match rsc_stream.next_chunk().await {
                Some(chunk) => {
                    let mut conv = converter.lock().await;

                    match conv.convert_chunk(chunk).await {
                        Ok(html_bytes) => {
                            if !html_bytes.is_empty() {
                                yield Ok::<_, std::io::Error>(bytes::Bytes::from(html_bytes));
                            }
                        }
                        Err(e) => {
                            if e.to_string().contains("disconnected") || e.to_string().contains("broken pipe") {
                                should_continue_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                                break;
                            }

                            error!("Error converting RSC chunk to HTML: {}", e);
                            yield Err(std::io::Error::other(e.to_string()));
                        }
                    }
                }
                None => {
                    break;
                }
            }
        }
    };

    let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
    let compressed_stream = compress_stream(html_stream, encoding);

    let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

    let mut response_builder = Response::builder()
        .status(status_code)
        .header("content-type", "text/html; charset=utf-8")
        .header("transfer-encoding", "chunked")
        .header("x-content-type-options", "nosniff")
        .header("cache-control", "no-cache");

    if let Some(encoding_header) = encoding.as_header_value() {
        response_builder = response_builder.header("content-encoding", encoding_header);
    }

    let body = Body::from_stream(compressed_stream);
    Ok(response_builder.body(body).expect("Valid streaming response"))
}

pub async fn render_streaming_with_layout(
    state: Arc<ServerState>,
    route_match: crate::server::routing::AppRouteMatch,
    context: crate::rsc::rendering::layout::LayoutRenderContext,
    layout_renderer: &LayoutRenderer,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_count = route_match.layouts.len();
    let is_not_found = route_match.not_found.is_some();

    let request_context =
        std::sync::Arc::new(crate::server::middleware::request_context::RequestContext::new(
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

            return render_synchronous(state, route_match, context, accept_encoding).await;
        }
    };

    let rsc_stream = match render_result {
        crate::rsc::rendering::layout::RenderResult::Streaming(stream) => stream,
        crate::rsc::rendering::layout::RenderResult::Static(html) => {
            let html_with_metadata = wrap_html_with_metadata(html, context.metadata.as_ref());

            let final_html = match inject_assets_into_html(&html_with_metadata, &state.config).await
            {
                Ok(html) => html,
                Err(e) => {
                    error!("Failed to inject assets into HTML: {}", e);
                    html_with_metadata
                }
            };

            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

            return Ok(Response::builder()
                .status(status_code)
                .header("content-type", "text/html; charset=utf-8")
                .header("x-render-mode", "static")
                .body(Body::from(final_html))
                .expect("Valid HTML response"));
        }
    };

    render_streaming_response(
        state,
        route_match,
        context,
        rsc_stream,
        is_not_found,
        accept_encoding,
    )
    .await
}

pub async fn render_fallback_html(
    state: &ServerState,
    path: &str,
    is_not_found: bool,
) -> Result<Response, StatusCode> {
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
            let html = cached_html.clone();
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(html))
                .expect("Valid HTML response"));
        }

        if let Ok(html_content) = std::fs::read_to_string(&index_path) {
            let final_html = if state.config.is_development() {
                inject_vite_client(&html_content, state.config.vite.port)
            } else {
                html_content
            };

            if state.config.is_production() {
                state.html_cache.insert(path.to_string(), final_html.clone());
            }

            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

            return Ok(Response::builder()
                .status(status_code)
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(final_html))
                .expect("Valid HTML response"));
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

        let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

        return Ok(Response::builder()
            .status(status_code)
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

    let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

    Ok(Response::builder()
        .status(status_code)
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
    use crate::server::types::request::{RenderMode, RequestTypeDetector};

    let path = uri.path();

    fn should_use_streaming(
        route_match: &crate::server::routing::AppRouteMatch,
        config: &Config,
    ) -> bool {
        if route_match.not_found.is_some() {
            return false;
        }
        config.rsc.enable_streaming
    }

    if path.len() > 1 {
        let path_without_leading_slash = &path[1..];

        if path_without_leading_slash.contains('.') {
            const BLOCKED_FILES: &[&str] =
                &["server/manifest.json", "server/routes.json", "server/"];

            for blocked in BLOCKED_FILES {
                if path_without_leading_slash.starts_with(blocked)
                    || path_without_leading_slash == *blocked
                {
                    return Err(StatusCode::NOT_FOUND);
                }
            }

            use crate::server::utils::path_validation::validate_safe_path;

            let file_path =
                match validate_safe_path(state.config.public_dir(), path_without_leading_slash) {
                    Ok(path) => path,
                    Err(_) => return Err(StatusCode::NOT_FOUND),
                };

            if file_path.is_file() {
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

            return Err(StatusCode::NOT_FOUND);
        }
    }

    let app_router = match &state.app_router {
        Some(router) => router,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let mut route_match = match app_router.match_route(path) {
        Ok(m) => m,
        Err(_) => match app_router.create_not_found_match(path) {
            Some(not_found_match) => not_found_match,
            None => return Err(StatusCode::NOT_FOUND),
        },
    };

    let request_context = std::sync::Arc::new(
        crate::server::middleware::request_context::RequestContext::new(path.to_string()),
    );

    let render_mode = RequestTypeDetector::detect_render_mode(&headers);
    let accept_encoding = headers.get("accept-encoding").and_then(|v| v.to_str().ok());

    let query_params_for_cache = query_params.clone();
    let search_params = extract_search_params(query_params);

    let request_headers = extract_headers(&headers);

    let mut context = crate::rsc::rendering::layout::create_layout_context(
        route_match.params.clone(),
        search_params.clone(),
        request_headers,
        route_match.pathname.clone(),
    );

    context.metadata = collect_page_metadata(&state, &route_match, &context).await;

    let layout_renderer = LayoutRenderer::new(state.renderer.clone());

    if route_match.not_found.is_none() && route_match.route.is_dynamic {
        match layout_renderer.check_page_not_found(&route_match, &context).await {
            Ok(true) => {
                if let Some(not_found_entry) = app_router.find_not_found(&route_match.route.path) {
                    route_match.not_found = Some(not_found_entry);
                }
            }
            Ok(false) => {}
            Err(e) => {
                error!("Failed to check if page is not found: {}", e);
            }
        }
    }

    match render_mode {
        RenderMode::RscNavigation => {
            let use_streaming = should_use_streaming(&route_match, &state.config);

            if use_streaming {
                return render_rsc_navigation_streaming(
                    Arc::new(state),
                    route_match,
                    context,
                    accept_encoding,
                )
                .await;
            }
            let cache_key = response_cache::ResponseCache::generate_cache_key(
                path,
                if query_params_for_cache.is_empty() {
                    None
                } else {
                    Some(&query_params_for_cache)
                },
            );

            if let Some(cached) = state.response_cache.get(&cache_key).await {
                let status_code = if route_match.not_found.is_some() {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::OK
                };

                let mut response_builder = Response::builder()
                    .status(status_code)
                    .header("content-type", "text/x-component")
                    .header("x-cache", "HIT");

                for (key, value) in cached.headers.iter() {
                    response_builder = response_builder.header(key, value);
                }

                return Ok(response_builder
                    .body(Body::from(cached.body))
                    .expect("Valid cached RSC response"));
            }

            match layout_renderer
                .render_route_by_mode(
                    &route_match,
                    &context,
                    render_mode,
                    Some(request_context.clone()),
                )
                .await
            {
                Ok(rsc_wire_format) => {
                    let status_code = if route_match.not_found.is_some() {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::OK
                    };

                    let mut response_builder = Response::builder()
                        .status(status_code)
                        .header("content-type", "text/x-component")
                        .header("x-cache", "MISS");

                    let mut cache_headers = axum::http::HeaderMap::new();

                    if let Some(ref metadata) = context.metadata
                        && let Ok(metadata_json) = serde_json::to_string(metadata)
                    {
                        let encoded_metadata = urlencoding::encode(&metadata_json);
                        response_builder =
                            response_builder.header("x-rari-metadata", encoded_metadata.as_ref());
                        if let Ok(header_value) = encoded_metadata.as_ref().parse() {
                            cache_headers.insert("x-rari-metadata", header_value);
                        }
                    }

                    let cache_control = state.config.get_cache_control_for_route(path);
                    let cache_policy =
                        response_cache::RouteCachePolicy::from_cache_control(cache_control, path);

                    if cache_policy.enabled && state.response_cache.config.enabled {
                        let cached_response = response_cache::CachedResponse {
                            body: bytes::Bytes::from(rsc_wire_format.clone()),
                            headers: cache_headers,
                            metadata: response_cache::CacheMetadata {
                                cached_at: std::time::Instant::now(),
                                ttl: cache_policy.ttl,
                                etag: None,
                                tags: cache_policy.tags,
                            },
                        };

                        state.response_cache.set(cache_key, cached_response).await;
                    }

                    Ok(response_builder
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
            let use_streaming = should_use_streaming(&route_match, &state.config);

            if use_streaming {
                return render_with_fallback(
                    Arc::new(state),
                    route_match,
                    context,
                    accept_encoding,
                )
                .await;
            }

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
                if let (Some(cached_etag), Some(client_etag)) = (&cached.metadata.etag, client_etag)
                    && cached_etag == client_etag
                {
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .header("etag", cached_etag)
                        .body(Body::empty())
                        .expect("Valid 304 response"));
                }

                let status_code = if route_match.not_found.is_some() {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::OK
                };

                let mut response_builder = Response::builder()
                    .status(status_code)
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
            let render_result = match layout_renderer
                .render_route_to_html_direct(&route_match, &context, Some(request_context.clone()))
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    error!("Direct HTML rendering failed: {}, falling back to shell", e);
                    return render_fallback_html(&state, path, route_match.not_found.is_some())
                        .await;
                }
            };

            let (html_with_assets, etag) = match render_result {
                crate::rsc::rendering::layout::RenderResult::Static(html_content) => {
                    let html_with_metadata =
                        wrap_html_with_metadata(html_content, context.metadata.as_ref());

                    let html_with_assets =
                        match inject_assets_into_html(&html_with_metadata, &state.config).await {
                            Ok(html) => html,
                            Err(e) => {
                                error!("Failed to inject assets into HTML: {}", e);
                                html_with_metadata
                            }
                        };

                    let etag =
                        response_cache::ResponseCache::generate_etag(html_with_assets.as_bytes());

                    (html_with_assets, etag)
                }
                crate::rsc::rendering::layout::RenderResult::Streaming(stream) => {
                    let asset_links = extract_asset_links_from_index_html().await;

                    let html_renderer = {
                        let renderer = state.renderer.lock().await;
                        Arc::new(RscHtmlRenderer::new(Arc::clone(&renderer.runtime)))
                    };

                    let title = context
                        .metadata
                        .as_ref()
                        .and_then(|m| m.title.as_ref())
                        .map(|t| t.as_str())
                        .unwrap_or("Rari App");

                    let asset_tags = asset_links.as_deref().unwrap_or("");
                    let base_shell = format!(
                        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    {}
    <style>
        .rari-loading {{
            animation: rari-pulse 1.5s ease-in-out infinite;
        }}
        @keyframes rari-pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.5; }}
        }}
    </style>
</head>
<body>
<div id="root">"#,
                        title, asset_tags
                    );

                    let base_shell = if let Some(ref metadata) = context.metadata {
                        inject_metadata(&base_shell, metadata)
                    } else {
                        base_shell
                    };

                    let converter = Arc::new(tokio::sync::Mutex::new(
                        RscToHtmlConverter::with_custom_shell(base_shell, None, html_renderer),
                    ));

                    let should_continue = Arc::new(std::sync::atomic::AtomicBool::new(true));
                    let should_continue_clone = should_continue.clone();

                    let mut rsc_stream = stream;

                    let html_stream = async_stream::stream! {
                        while should_continue_clone.load(std::sync::atomic::Ordering::Relaxed) {
                            match rsc_stream.next_chunk().await {
                                Some(chunk) => {
                                    let mut conv = converter.lock().await;

                                    match conv.convert_chunk(chunk).await {
                                        Ok(html_bytes) => {
                                            if !html_bytes.is_empty() {
                                                yield Ok(html_bytes);
                                            }
                                        }
                                        Err(e) => {
                                            if e.to_string().contains("disconnected") || e.to_string().contains("broken pipe") {
                                                should_continue_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                                                break;
                                            }

                                            error!("Error converting RSC chunk to HTML: {}", e);
                                            yield Err(e);
                                        }
                                    }
                                }
                                None => {
                                    break;
                                }
                            }
                        }
                    };

                    let status_code = if route_match.not_found.is_some() {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::OK
                    };

                    return Ok(StreamingHtmlResponse::with_status(html_stream, status_code)
                        .into_response());
                }
            };

            let status_code = if route_match.not_found.is_some() {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::OK
            };

            let mut response_builder = Response::builder()
                .status(status_code)
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
                    let header_name = key.cow_to_lowercase();
                    response_builder = response_builder.header(header_name.as_ref(), value);

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
            }

            Ok(response_builder.body(Body::from(html_with_assets)).expect("Valid HTML response"))
        }
    }
}
