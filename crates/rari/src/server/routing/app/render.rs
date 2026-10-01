#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{env, io::Error, string::String, sync::Arc, time::Instant};

use axum::{
    body::Body,
    http::{HeaderValue, StatusCode},
    response::Response,
};
use bytes::Bytes;
use rari_error::RariError;
use tokio::{
    fs,
    sync::mpsc::{Receiver, error::TryRecvError},
    time::{self, Duration},
};

use super::cache::rsc_vary_header;
use crate::{
    rendering::{
        html_shell::RscHtmlRenderer,
        layout::{
            ChunkedContentType, LayoutRenderContext, LayoutRenderer, OpenGraphImage,
            OpenGraphImageDescriptor, OpenGraphMetadata, PageMetadata, RenderResult,
            TwitterMetadata, component_dist_path, is_rari_page_not_found, sort_flight_protocol,
        },
    },
    server::{
        ServerState,
        compression::{CompressionEncoding, compress_stream},
        config::Config,
        document::{pretty_html::pretty_print_html, utils::inject_assets_into_html},
        error_response,
        host::utils::http::merge_vary_with_accept,
        image::schedule_image_prewarm,
        middleware::request_context::RequestContext,
        routing::{
            app_icons::inject_app_icons_into_metadata,
            app_router::{AppRouteMatch, AppRouter, NotFoundEntry},
        },
    },
    utils::path::path_to_file_url,
};

pub fn wrap_html_with_metadata(html_content: String, state: &ServerState) -> String {
    schedule_image_prewarm(state, &html_content);
    if state.config.is_development() { pretty_print_html(&html_content) } else { html_content }
}

pub fn should_use_streaming(route_match: &AppRouteMatch, config: &Config) -> bool {
    if route_match.not_found.is_some() {
        return false;
    }
    config.loading.enabled && route_match.loading.is_some()
}

pub fn mark_route_not_found_if_signaled(
    err: &RariError,
    route_match: &mut AppRouteMatch,
    app_router: &AppRouter,
) -> bool {
    if route_match.not_found.is_some() || !is_rari_page_not_found(err) {
        return false;
    }
    if let Some(entry) = resolve_not_found_entry(app_router, route_match) {
        route_match.not_found = Some(entry);
        true
    } else {
        false
    }
}

pub fn resolve_not_found_entry(
    app_router: &AppRouter,
    route_match: &AppRouteMatch,
) -> Option<NotFoundEntry> {
    app_router
        .find_not_found_for_route(&route_match.route)
        .or_else(|| app_router.find_not_found(&route_match.pathname))
}

pub async fn collect_page_metadata(
    state: &ServerState,
    route_match: &AppRouteMatch,
    context: &LayoutRenderContext,
) -> Option<PageMetadata> {
    let dist_server_path = match env::current_dir() {
        Ok(cwd) => {
            let path = cwd.join("dist/server");
            fs::canonicalize(&path).await.ok()
        }
        Err(_) => None,
    };

    let Some(base_path) = dist_server_path else {
        tracing::debug!("Could not determine dist/server path for metadata collection");
        return None;
    };

    let mut layout_paths = Vec::with_capacity(route_match.layouts.len());
    for layout in &route_match.layouts {
        let file_path = component_dist_path(&base_path, &layout.file_path);
        if fs::try_exists(&file_path).await.unwrap_or(false) {
            layout_paths.push(path_to_file_url(&file_path));
        }
    }

    let page_source_path = route_match
        .not_found
        .as_ref()
        .map(|entry| entry.file_path.as_str())
        .unwrap_or(route_match.route.file_path.as_str());
    let page_file_path = component_dist_path(&base_path, page_source_path);

    if !fs::try_exists(&page_file_path).await.unwrap_or(false) {
        return None;
    }

    let page_path = path_to_file_url(&page_file_path);

    let renderer = state.renderer.lock().await;
    let runtime = Arc::clone(&renderer.runtime);
    drop(renderer);

    match runtime
        .collect_metadata(
            layout_paths,
            page_path.clone(),
            context.params.clone(),
            context.search_params.clone(),
        )
        .await
    {
        Ok(metadata_value) => match serde_json::from_value::<PageMetadata>(metadata_value) {
            Ok(mut metadata) => {
                inject_og_image_into_metadata(state, &route_match.pathname, &mut metadata, context)
                    .await;
                inject_app_icons_into_metadata(
                    &state.app_icons,
                    &route_match.route.path,
                    &mut metadata,
                );
                Some(metadata)
            }
            Err(e) => {
                tracing::error!("Failed to deserialize metadata: {}", e);
                None
            }
        },
        Err(e) => {
            tracing::error!("Failed to collect metadata from runtime: {}", e);
            None
        }
    }
}

async fn inject_og_image_into_metadata(
    state: &ServerState,
    route_path: &str,
    metadata: &mut PageMetadata,
    context: &LayoutRenderContext,
) {
    let Some(base_url) = get_base_url_from_context(context, &state.config) else {
        return;
    };
    let current_url = format!("{base_url}{route_path}");

    if let Some(ref mut og) = metadata.open_graph {
        if og.url.is_none() {
            og.url = Some(current_url.clone());
        }
    } else {
        metadata.open_graph = Some(OpenGraphMetadata {
            title: None,
            description: None,
            url: Some(current_url.clone()),
            site_name: None,
            images: None,
            og_type: None,
        });
    }

    if let Some(og_generator) = &state.og_generator
        && let Some(og_entry) = og_generator.find_og_image_for_route(route_path).await
    {
        let og_image_url = format!("{base_url}/_rari/og{route_path}");

        let og_image = if og_entry.width.is_some() || og_entry.height.is_some() {
            OpenGraphImage::Detailed(OpenGraphImageDescriptor {
                url: og_image_url.clone(),
                width: og_entry.width,
                height: og_entry.height,
                alt: None,
            })
        } else {
            OpenGraphImage::Simple(og_image_url.clone())
        };

        if let Some(ref mut og) = metadata.open_graph {
            if og.images.is_none() {
                og.images = Some(vec![og_image]);
            } else if let Some(ref mut images) = og.images {
                images.insert(0, og_image);
            }
        }

        if let Some(ref mut twitter) = metadata.twitter {
            if twitter.card.is_none() {
                twitter.card = Some("summary_large_image".to_string());
            }
            if twitter.images.is_none() {
                twitter.images = Some(vec![og_image_url]);
            } else if let Some(ref mut images) = twitter.images {
                images.insert(0, og_image_url);
            }
        } else {
            metadata.twitter = Some(TwitterMetadata {
                card: Some("summary_large_image".to_string()),
                site: None,
                creator: None,
                title: None,
                description: None,
                images: Some(vec![og_image_url]),
            });
        }
    }
}

fn get_base_url_from_context(context: &LayoutRenderContext, config: &Config) -> Option<String> {
    if let Some(origin) = config.server.origin.as_deref().map(str::trim).filter(|o| !o.is_empty()) {
        return Some(origin.trim_end_matches('/').to_string());
    }

    let host = context.headers.get("host")?;
    let forwarded = context
        .headers
        .get("x-forwarded-proto")
        .or_else(|| context.headers.get("x-forwarded-protocol"))
        .map(String::as_str)
        .unwrap_or("");
    let protocol = if forwarded.eq_ignore_ascii_case("https") {
        "https"
    } else if forwarded.eq_ignore_ascii_case("http") {
        "http"
    } else if config.is_production() {
        "https"
    } else {
        "http"
    };

    Some(format!("{protocol}://{host}"))
}

pub async fn render_with_fallback(
    state: Arc<ServerState>,
    route_match: AppRouteMatch,
    context: LayoutRenderContext,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_renderer = LayoutRenderer::with_shared_cache(
        Arc::clone(&state.renderer),
        Arc::clone(&state.layout_html_cache),
    );

    match render_streaming_with_layout(
        Arc::clone(&state),
        route_match.clone(),
        context.clone(),
        &layout_renderer,
        accept_encoding,
    )
    .await
    {
        Ok(response) => Ok(response),
        Err(e) => {
            tracing::error!("Streaming render failed, falling back to synchronous: {}", e);
            render_synchronous(state, route_match, context, accept_encoding).await
        }
    }
}

pub async fn render_rsc_navigation_streaming(
    state: Arc<ServerState>,
    mut route_match: AppRouteMatch,
    mut context: LayoutRenderContext,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_renderer = LayoutRenderer::with_shared_cache(
        Arc::clone(&state.renderer),
        Arc::clone(&state.layout_html_cache),
    );

    let request_context = Arc::new(
        RequestContext::new(route_match.route.path.clone())
            .with_http_headers(context.headers.clone()),
    );

    let render_result = match layout_renderer
        .render_route_with_streaming(
            &route_match,
            &context,
            Some(Arc::clone(&request_context)),
            true,
            None,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            if let Some(app_router) = state.app_router.as_ref()
                && mark_route_not_found_if_signaled(&e, &mut route_match, app_router)
            {
                context.metadata = collect_page_metadata(&state, &route_match, &context).await;
                match layout_renderer
                    .render_route_with_streaming(
                        &route_match,
                        &context,
                        Some(Arc::clone(&request_context)),
                        true,
                        None,
                    )
                    .await
                {
                    Ok(result) => result,
                    Err(retry_err) => {
                        tracing::error!(
                            "Failed to render RSC navigation for streaming '{}': {}",
                            route_match.route.path,
                            retry_err
                        );
                        return Err(error_response::status(&retry_err));
                    }
                }
            } else {
                tracing::error!(
                    "Failed to render RSC navigation for streaming '{}': {}",
                    route_match.route.path,
                    e
                );
                return Err(error_response::status(&e));
            }
        }
    };

    let is_not_found = route_match.not_found.is_some();

    match render_result {
        RenderResult::Chunked {
            content_type: ChunkedContentType::RscFlight,
            shell,
            closing,
            chunks,
        } => Ok(render_chunked_response(
            &state,
            &context,
            ChunkedContentType::RscFlight,
            shell,
            closing,
            chunks,
            is_not_found,
            accept_encoding,
        )),
        RenderResult::Chunked { content_type: ChunkedContentType::Html, .. } => {
            tracing::error!("HTML chunked render not supported in RSC-only mode");
            Err(error_response::status(&RariError::internal(
                "HTML chunked render not supported in RSC-only mode",
            )))
        }
        RenderResult::Static(rsc_flight_protocol) => {
            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

            let sorted_flight_protocol = sort_flight_protocol(&rsc_flight_protocol);

            let final_payload = if sorted_flight_protocol.ends_with('\n') {
                sorted_flight_protocol
            } else {
                format!("{sorted_flight_protocol}\n")
            };

            let router_state_sensitive =
                context.template_navigation_id.is_some() || !context.reuse_layout_paths.is_empty();
            let vary = rsc_vary_header(None, router_state_sensitive);

            let mut response_builder = Response::builder()
                .status(status_code)
                .header("content-type", "text/x-component")
                .header("vary", vary);

            if let Some(ref metadata) = context.metadata
                && let Ok(metadata_json) = serde_json::to_string(metadata)
            {
                let encoded_metadata = urlencoding::encode(&metadata_json);
                response_builder =
                    response_builder.header("x-rari-metadata", encoded_metadata.as_ref());
            }

            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            Ok(response_builder.body(Body::from(final_payload)).expect("Valid RSC response"))
        }
        RenderResult::StaticBinary(binary_payload) => {
            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

            let router_state_sensitive =
                context.template_navigation_id.is_some() || !context.reuse_layout_paths.is_empty();
            let vary = rsc_vary_header(None, router_state_sensitive);

            let mut response_builder = Response::builder()
                .status(status_code)
                .header("content-type", "text/x-component")
                .header("vary", vary);

            if let Some(ref metadata) = context.metadata
                && let Ok(metadata_json) = serde_json::to_string(metadata)
            {
                let encoded_metadata = urlencoding::encode(&metadata_json);
                response_builder =
                    response_builder.header("x-rari-metadata", encoded_metadata.as_ref());
            }

            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            Ok(response_builder.body(Body::from(binary_payload)).expect("Valid RSC response"))
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn render_chunked_response(
    state: &Arc<ServerState>,
    context: &LayoutRenderContext,
    content_type: ChunkedContentType,
    shell: Bytes,
    closing: Bytes,
    mut chunks: Receiver<Result<Vec<u8>, RariError>>,
    is_not_found: bool,
    accept_encoding: Option<&str>,
) -> http::Response<Body> {
    let stall_timeout = Duration::from_millis(chunked_stream_stall_timeout_ms());

    let byte_stream = async_stream::stream! {
        match content_type {
            ChunkedContentType::Html => {
                // After Suspense starts resolving, a large HTML chunk is often
                // followed within ~1ms by the final flight/complete tail on the
                // isolate thread. Wait only after large chunks so we don't tax
                // every small endgame write.
                const ENDGAME_AFTER: Duration = Duration::from_millis(800);
                const ENDGAME_WAIT: Duration = Duration::from_micros(500);
                const ENDGAME_LARGE_MIN: usize = 1024;

                let t0 = Instant::now();
                let mut finished = false;

                yield Ok::<_, Error>(shell);

                while !finished {
                    match time::timeout(stall_timeout, chunks.recv()).await {
                        Ok(Some(Ok(chunk_bytes))) => {
                            if chunk_bytes.is_empty() {
                                continue;
                            }
                            let mut buf = chunk_bytes;
                            let mut stream_error: Option<RariError> = None;

                            loop {
                                match chunks.try_recv() {
                                    Ok(Ok(more)) => {
                                        if !more.is_empty() {
                                            buf.extend_from_slice(&more);
                                        }
                                    }
                                    Ok(Err(e)) => {
                                        stream_error = Some(e);
                                        finished = true;
                                        break;
                                    }
                                    Err(TryRecvError::Empty) => break,
                                    Err(TryRecvError::Disconnected) => {
                                        finished = true;
                                        break;
                                    }
                                }
                            }

                            if !finished
                                && stream_error.is_none()
                                && t0.elapsed() >= ENDGAME_AFTER
                                && buf.len() >= ENDGAME_LARGE_MIN
                            {
                                match time::timeout(ENDGAME_WAIT, chunks.recv()).await {
                                    Ok(Some(Ok(more))) => {
                                        if !more.is_empty() {
                                            buf.extend_from_slice(&more);
                                        }
                                        loop {
                                            match chunks.try_recv() {
                                                Ok(Ok(more)) => {
                                                    if !more.is_empty() {
                                                        buf.extend_from_slice(&more);
                                                    }
                                                }
                                                Ok(Err(e)) => {
                                                    stream_error = Some(e);
                                                    finished = true;
                                                    break;
                                                }
                                                Err(TryRecvError::Empty) => break,
                                                Err(TryRecvError::Disconnected) => {
                                                    finished = true;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Ok(Some(Err(e))) => {
                                        stream_error = Some(e);
                                        finished = true;
                                    }
                                    Ok(None) => {
                                        finished = true;
                                    }
                                    Err(_) => {}
                                }
                            }

                            yield Ok(Bytes::from(buf));

                            if let Some(e) = stream_error {
                                tracing::error!("Error in chunked HTML stream: {}", e);
                                yield Err(Error::other(e.to_string()));
                            }
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error in chunked HTML stream: {}", e);
                            yield Err(Error::other(e.to_string()));
                            finished = true;
                        }
                        Ok(None) => {
                            finished = true;
                        }
                        Err(_) => {
                            tracing::error!(
                                "Chunked HTML stream stalled: no chunk received within {} ms",
                                stall_timeout.as_millis()
                            );
                            yield Ok(chunked_stream_error_chunk(
                                "Stream timed out waiting for content",
                            ));
                            finished = true;
                        }
                    }
                }

                if !closing.is_empty() {
                    yield Ok(closing);
                }
            }
            ChunkedContentType::RscFlight => {
                loop {
                    match time::timeout(stall_timeout, chunks.recv()).await {
                        Ok(Some(Ok(chunk_bytes))) => {
                            if chunk_bytes.is_empty() {
                                continue;
                            }
                            let data = String::from_utf8_lossy(&chunk_bytes);
                            if data.trim() == "STREAM_COMPLETE" {
                                continue;
                            }
                            yield Ok(Bytes::from(chunk_bytes));
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error in chunked RSC stream: {}", e);
                            yield Err(Error::other(e.to_string()));
                            break;
                        }
                        Ok(None) => break,
                        Err(_) => {
                            tracing::error!(
                                "Chunked RSC stream stalled: no chunk received within {} ms",
                                stall_timeout.as_millis()
                            );
                            yield Err(Error::other("RSC stream timed out waiting for content"));
                            break;
                        }
                    }
                }
            }
        }
    };

    let encoding = match content_type {
        // Prefer identity for streaming HTML so compressor setup does not delay the shell.
        ChunkedContentType::Html => CompressionEncoding::Identity,
        ChunkedContentType::RscFlight => CompressionEncoding::from_accept_encoding(accept_encoding),
    };
    let compressed_stream = compress_stream(byte_stream, encoding);
    let router_state_sensitive = matches!(content_type, ChunkedContentType::RscFlight)
        && (context.template_navigation_id.is_some() || !context.reuse_layout_paths.is_empty());
    let vary = {
        let mut parts: Vec<&str> = Vec::new();
        if encoding.as_header_value().is_some() {
            parts.push("Accept-Encoding");
        }
        if router_state_sensitive {
            parts.push("rari-router-state");
        }
        let existing =
            if parts.is_empty() { None } else { HeaderValue::from_str(&parts.join(", ")).ok() };
        merge_vary_with_accept(existing.as_ref())
    };

    let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
    let cache_control = state.config.get_cache_control_for_route(&context.pathname);

    let mut response_builder = Response::builder()
        .status(status_code)
        .header("transfer-encoding", "chunked")
        .header("x-render-mode", "streaming")
        .header("cache-control", cache_control)
        .header("vary", vary)
        .header("x-content-type-options", "nosniff");

    match content_type {
        ChunkedContentType::Html => {
            response_builder = response_builder.header("content-type", "text/html; charset=utf-8");
        }
        ChunkedContentType::RscFlight => {
            response_builder = response_builder.header("content-type", "text/x-component");

            if let Some(ref metadata) = context.metadata
                && let Ok(metadata_json) = serde_json::to_string(metadata)
            {
                let encoded_metadata = urlencoding::encode(&metadata_json);
                response_builder =
                    response_builder.header("x-rari-metadata", encoded_metadata.as_ref());
            }
        }
    }

    if let Some(encoding_header) = encoding.as_header_value() {
        response_builder = response_builder.header("content-encoding", encoding_header);
    }

    let body = Body::from_stream(compressed_stream);
    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    response_builder.body(body).expect("Valid chunked response")
}

fn chunked_stream_stall_timeout_ms() -> u64 {
    env::var("RARI_STREAMING_STALL_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(60_000)
}

fn chunked_stream_error_chunk(message: &str) -> Bytes {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    Bytes::from(format!(
        r#"<div class="rari-error" style="color: red; border: 1px solid red; padding: 10px; border-radius: 4px; background-color: #fff5f5;"><strong>Error loading content: </strong>{escaped}</div>"#
    ))
}

pub async fn render_synchronous(
    state: Arc<ServerState>,
    mut route_match: AppRouteMatch,
    mut context: LayoutRenderContext,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_renderer = LayoutRenderer::with_shared_cache(
        Arc::clone(&state.renderer),
        Arc::clone(&state.layout_html_cache),
    );
    let request_context = Arc::new(
        RequestContext::new(route_match.route.path.clone())
            .with_http_headers(context.headers.clone()),
    );

    let render_result = match layout_renderer
        .render_route_with_streaming(
            &route_match,
            &context,
            Some(Arc::clone(&request_context)),
            false,
            None,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            if let Some(app_router) = state.app_router.as_ref()
                && mark_route_not_found_if_signaled(&e, &mut route_match, app_router)
            {
                context.metadata = collect_page_metadata(&state, &route_match, &context).await;
                match layout_renderer
                    .render_route_with_streaming(
                        &route_match,
                        &context,
                        Some(request_context),
                        false,
                        None,
                    )
                    .await
                {
                    Ok(result) => result,
                    Err(retry_err) => {
                        tracing::error!(
                            "Failed to render route '{}': {}",
                            route_match.route.path,
                            retry_err
                        );
                        if is_rari_page_not_found(&retry_err) {
                            return render_fallback_html(&state, true).await;
                        }
                        return Err(error_response::status(&retry_err));
                    }
                }
            } else {
                tracing::error!("Failed to render route '{}': {}", route_match.route.path, e);
                return render_fallback_html(
                    &state,
                    route_match.not_found.is_some() || is_rari_page_not_found(&e),
                )
                .await;
            }
        }
    };

    let is_not_found = route_match.not_found.is_some();

    match render_result {
        RenderResult::Static(html_content) => {
            let html_with_assets = match inject_assets_into_html(&html_content, &state.config).await
            {
                Ok(html) => html,
                Err(e) => {
                    tracing::error!("Failed to inject assets into HTML: {}", e);
                    html_content
                }
            };

            let final_html = wrap_html_with_metadata(html_with_assets, &state);

            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
            let cache_control = state.config.get_cache_control_for_route(&context.pathname);

            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            Ok(Response::builder()
                .status(status_code)
                .header("content-type", "text/html; charset=utf-8")
                .header("x-render-mode", "synchronous")
                .header("cache-control", cache_control)
                .header("vary", "Accept")
                .body(Body::from(final_html))
                .expect("Valid HTML response"))
        }
        RenderResult::Chunked {
            content_type: ChunkedContentType::Html,
            shell,
            closing,
            chunks,
        } => Ok(render_chunked_response(
            &state,
            &context,
            ChunkedContentType::Html,
            shell,
            closing,
            chunks,
            is_not_found,
            accept_encoding,
        )),
        RenderResult::StaticBinary(bytes) => {
            let html_content = String::from_utf8_lossy(&bytes).into_owned();
            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            Ok(Response::builder()
                .status(status_code)
                .header("content-type", "text/html; charset=utf-8")
                .header("vary", "Accept")
                .body(Body::from(html_content))
                .expect("Valid response"))
        }
        RenderResult::Chunked { content_type: ChunkedContentType::RscFlight, .. } => {
            tracing::error!("RSC chunked render not supported in HTML synchronous mode");
            Err(error_response::status(&RariError::internal(
                "RSC chunked render not supported in HTML synchronous mode",
            )))
        }
    }
}

pub async fn render_streaming_with_layout(
    state: Arc<ServerState>,
    route_match: AppRouteMatch,
    context: LayoutRenderContext,
    layout_renderer: &LayoutRenderer,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_count = route_match.layouts.len();
    let is_not_found = route_match.not_found.is_some();

    let request_context = Arc::new(
        RequestContext::new(route_match.route.path.clone())
            .with_http_headers(context.headers.clone()),
    );

    let render_result = match layout_renderer
        .render_route_with_streaming(&route_match, &context, Some(request_context), false, None)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(
                "Failed to render route for streaming '{}': {}",
                route_match.route.path,
                e
            );
            tracing::error!(
                "Route rendering failure context - Route: {}, Page component: {}, Layout count: {}",
                route_match.route.path,
                route_match.route.file_path,
                layout_count
            );

            for (idx, layout) in route_match.layouts.iter().enumerate() {
                tracing::error!(
                    "  Layout {}: {} (is_root: {})",
                    idx,
                    layout.file_path,
                    layout.is_root
                );
            }

            return render_synchronous(state, route_match, context, accept_encoding).await;
        }
    };

    match render_result {
        RenderResult::Chunked {
            content_type: ChunkedContentType::Html,
            shell,
            closing,
            chunks,
        } => Ok(render_chunked_response(
            &state,
            &context,
            ChunkedContentType::Html,
            shell,
            closing,
            chunks,
            is_not_found,
            accept_encoding,
        )),
        RenderResult::Static(html) => {
            use crate::server::compression::compress_body;

            let html_with_assets = match inject_assets_into_html(&html, &state.config).await {
                Ok(html) => html,
                Err(e) => {
                    tracing::error!("Failed to inject assets into HTML: {}", e);
                    html
                }
            };

            let final_html = wrap_html_with_metadata(html_with_assets, &state);

            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
            let cache_control = state.config.get_cache_control_for_route(&context.pathname);

            let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
            let (body_bytes, actual_encoding) =
                compress_body(Bytes::from(final_html), encoding).await;

            let mut response_builder = Response::builder()
                .status(status_code)
                .header("content-type", "text/html; charset=utf-8")
                .header("x-render-mode", "static")
                .header("cache-control", cache_control)
                .header("vary", "Accept, Accept-Encoding");

            if let Some(encoding_header) = actual_encoding.as_header_value() {
                response_builder = response_builder.header("content-encoding", encoding_header);
            }

            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            Ok(response_builder.body(Body::from(body_bytes)).expect("Valid HTML response"))
        }
        RenderResult::StaticBinary(_) => Err(error_response::status(&RariError::internal(
            "Binary render result not supported in HTML streaming mode",
        ))),
        RenderResult::Chunked { content_type: ChunkedContentType::RscFlight, .. } => {
            tracing::error!("RSC chunked render not supported in HTML streaming mode");
            Err(error_response::status(&RariError::internal(
                "RSC chunked render not supported in HTML streaming mode",
            )))
        }
    }
}

fn fallback_html_response(html: Bytes, is_not_found: bool) -> Response {
    let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    Response::builder()
        .status(status_code)
        .header("content-type", "text/html; charset=utf-8")
        .header("vary", "Accept")
        .body(Body::from(html))
        .expect("Valid HTML response")
}

fn emergency_fallback_shell(client_head: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
{client_head}</head>
<body></body>
</html>"#
    )
}

pub async fn render_fallback_html(
    state: &ServerState,
    is_not_found: bool,
) -> Result<Response, StatusCode> {
    if state.config.is_production()
        && let Some(html) = state.html_cache.get()
    {
        return Ok(fallback_html_response(html, is_not_found));
    }

    let vite_port = state.config.vite.port;
    let cache_generation = state.html_cache.generation();
    let client_head = if state.config.is_development() {
        RscHtmlRenderer::generate_dev_client_head(&state.config.vite.host, vite_port)
    } else {
        fs::read_to_string(state.config.public_dir().join("rari-client-head.html"))
            .await
            .unwrap_or_default()
    };

    let mut html_shell = emergency_fallback_shell(&client_head);

    if state.config.is_development() {
        html_shell = pretty_print_html(&html_shell);
    }

    let body = Bytes::from(html_shell);
    if state.config.is_production() {
        state.html_cache.set_if_generation(body.clone(), cache_generation);
    }

    Ok(fallback_html_response(body, is_not_found))
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use std::{
        fs, path, process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use rustc_hash::FxHashMap;
    use tokio::sync::Mutex;

    use super::*;
    use crate::{
        RscHtmlRenderer, RscRenderer,
        rendering::layout::LayoutHtmlCache,
        runtime::JsExecutionRuntime,
        server::{
            FallbackHtmlCache,
            cache::{
                handler::{CacheHandlerRegistry, MemoryCacheHandler},
                response::{CacheConfig, ResponseCache, StaticFastCache},
            },
            config::Mode,
        },
    };

    #[test]
    fn test_og_base_url_prefers_configured_origin() {
        let mut config = Config::new(Mode::Production);
        config.server.origin = Some("https://rari.build/".to_string());

        let mut context = empty_layout_context();
        context.headers.insert("host".to_string(), "localhost".to_string());

        assert_eq!(
            get_base_url_from_context(&context, &config).as_deref(),
            Some("https://rari.build")
        );
    }

    #[test]
    fn test_og_base_url_uses_host_when_origin_unset() {
        let config = Config::new(Mode::Production);
        let mut context = empty_layout_context();
        context.headers.insert("host".to_string(), "rari.build".to_string());

        assert_eq!(
            get_base_url_from_context(&context, &config).as_deref(),
            Some("https://rari.build")
        );
    }

    #[test]
    fn test_og_base_url_none_without_host_or_origin() {
        let config = Config::new(Mode::Production);
        assert_eq!(get_base_url_from_context(&empty_layout_context(), &config), None);
    }

    #[test]
    fn test_fallback_html_response_status_from_is_not_found() {
        let body = Bytes::from("<html>fallback</html>");

        let ok = fallback_html_response(body.clone(), false);
        assert_eq!(ok.status(), StatusCode::OK);

        let not_found = fallback_html_response(body, true);
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
    }

    fn production_state_with_html_cache(
        html_cache: FallbackHtmlCache,
        public_dir: path::PathBuf,
    ) -> ServerState {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = Arc::new(Mutex::new(RscRenderer::new(Arc::clone(&runtime))));
        let ssr_renderer = Arc::new(RscHtmlRenderer::new(runtime));
        let cache_registry = Arc::new(CacheHandlerRegistry::default_with_memory());
        let image_handler = Arc::new(MemoryCacheHandler::default());

        let mut config = Config::new(Mode::Production);
        config.static_files.prod_public_dir = public_dir;

        ServerState {
            renderer,
            ssr_renderer,
            config: Arc::new(config),
            app_router: None,
            api_route_handler: None,
            html_cache,
            layout_html_cache: Arc::new(LayoutHtmlCache::new()),
            response_cache: Arc::new(ResponseCache::new(CacheConfig::default())),
            static_fast_cache: Arc::new(StaticFastCache::new()),
            og_generator: None,
            app_icons: Arc::new(Vec::new()),
            project_root: path::PathBuf::from("."),
            image_optimizer: None,
            cache_registry,
            image_handler,
        }
    }

    #[tokio::test]
    async fn test_render_fallback_html_cache_hit_returns_not_found() {
        let public_dir = env::temp_dir().join(format!(
            "rari-fallback-html-{}-{}",
            process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).expect("time").as_nanos()
        ));
        fs::create_dir_all(&public_dir).expect("temp public dir");
        fs::write(public_dir.join("rari-client-head.html"), "<!-- client -->")
            .expect("rari-client-head.html");

        let html_cache = FallbackHtmlCache::default();
        html_cache.set(Bytes::from("<html>cached</html>"));

        let state = production_state_with_html_cache(html_cache, public_dir.clone());
        let response = render_fallback_html(&state, true).await.expect("cache hit response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let _ = fs::remove_dir_all(public_dir);
    }

    fn empty_layout_context() -> LayoutRenderContext {
        LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/".to_string(),
            template_navigation_id: None,
            metadata: None,
            reuse_layout_paths: Vec::new(),
        }
    }
}
