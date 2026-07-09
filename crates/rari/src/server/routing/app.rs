#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{
    env,
    io::{Cursor, Error},
    path::PathBuf,
    string::String,
    sync::Arc,
    time::Instant,
};

use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZstdDecoder};
use axum::{
    body,
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header::CACHE_CONTROL},
    response::Response,
};
use bytes::Bytes;
use cow_utils::CowUtils;
use rustc_hash::FxHashMap;
use tokio::{
    fs,
    sync::mpsc::Receiver,
    time::{self, Duration},
};

use crate::{
    rendering::layout::{
        ChunkedContentType, LayoutRenderContext, LayoutRenderer, OpenGraphImage,
        OpenGraphImageDescriptor, OpenGraphMetadata, PageMetadata, RenderResult, TwitterMetadata,
        component_dist_path, create_layout_context, drain_chunked_stream, sort_flight_protocol,
    },
    server::{
        ServerState,
        actions::{
            has_action_form_state_cookie, inject_action_form_state_from_cookie,
            response_cache_cookie_partition,
        },
        cache::response,
        compression::{CompressionEncoding, compress_body, compress_stream},
        config::Config,
        core::{
            types::request::{RenderMode, RequestTypeDetector},
            utils::{
                self,
                http::{
                    extract_headers, extract_search_params, get_content_type,
                    merge_vary_with_accept,
                },
                path_validation::validate_safe_path,
            },
        },
        middleware::request_context::RequestContext,
        rendering::{
            metadata_injection::inject_metadata,
            utils::{inject_assets_into_html, inject_vite_client},
        },
        routing::app_router::AppRouteMatch,
    },
    utils::path::path_to_file_url,
};

fn request_cookie_header(headers: &HeaderMap) -> Option<&str> {
    headers.get("cookie").and_then(|value| value.to_str().ok()).filter(|value| !value.is_empty())
}

fn route_query_params_for_cache(
    query_params: &FxHashMap<String, String>,
) -> Option<&FxHashMap<String, String>> {
    if query_params.is_empty() { None } else { Some(query_params) }
}

fn static_html_vary_header(cookie_header: Option<&str>) -> String {
    let mut parts = vec!["Accept", "Accept-Encoding"];
    if cookie_header.is_some() {
        parts.push("Cookie");
    }
    parts.join(", ")
}

/// Static fast-cache entries are keyed without cookies because only cookie-independent
/// HTML is stored. Skip the fast path when action form state is present since that
/// cookie is injected into SSR before render.
fn can_use_static_fast_cache(cookie_header: Option<&str>) -> bool {
    !has_action_form_state_cookie(cookie_header)
}

fn response_cache_key(
    path: &str,
    query_params_ref: Option<&FxHashMap<String, String>>,
    render_mode: Option<&str>,
    cookie_header: Option<&str>,
) -> String {
    let cache_cookie = response_cache_cookie_partition(cookie_header);
    response::ResponseCache::generate_cache_key_with_mode(
        path,
        query_params_ref,
        render_mode,
        cache_cookie.as_deref(),
    )
}

fn rsc_vary_header(cookie_header: Option<&str>) -> String {
    let mut headers = HeaderMap::new();
    if cookie_header.is_some() {
        headers.insert("vary", HeaderValue::from_static("Cookie"));
    }
    merge_vary_with_accept(headers.get("vary"))
}

async fn should_store_response_cache(
    state: &ServerState,
    cache_policy: &response::RouteCachePolicy,
) -> bool {
    if !cache_policy.enabled || !state.response_cache.config.enabled {
        return false;
    }

    let runtime = {
        let renderer = state.renderer.lock().await;
        Arc::clone(&renderer.runtime)
    };

    !runtime.is_dynamic_render().await.unwrap_or(true)
}

async fn decompress_bytes(data: &Bytes, encoding: CompressionEncoding) -> Result<Bytes, Error> {
    use tokio::io::AsyncReadExt;

    let data = data.clone();
    match encoding {
        CompressionEncoding::Gzip => {
            let mut decoder = GzipDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(Bytes::from(decompressed))
        }
        CompressionEncoding::Brotli => {
            let mut decoder = BrotliDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(Bytes::from(decompressed))
        }
        CompressionEncoding::Zstd => {
            let mut decoder = ZstdDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(Bytes::from(decompressed))
        }
        CompressionEncoding::Identity => Ok(data),
    }
}

async fn merge_response_cache_tags(state: &ServerState, base_tags: Vec<String>) -> Vec<String> {
    let page_cache_tags = {
        let renderer = state.renderer.lock().await;
        let runtime = Arc::clone(&renderer.runtime);
        drop(renderer);
        runtime.collect_page_cache_tags().await.unwrap_or_default()
    };
    response::RouteCachePolicy::merge_cache_tags(base_tags, &page_cache_tags)
}

pub(crate) fn wrap_html_with_metadata(
    html_content: String,
    metadata: Option<&PageMetadata>,
    state: &ServerState,
) -> String {
    let trimmed = html_content.trim_start();
    let trimmed_lower = trimmed.cow_to_lowercase();
    let is_complete = trimmed_lower.starts_with("<!doctype") || trimmed_lower.starts_with("<html");

    if is_complete {
        if let Some(metadata) = metadata {
            inject_metadata(&html_content, metadata, state.image_optimizer.as_deref())
        } else {
            html_content
        }
    } else {
        html_content
    }
}

fn should_use_streaming(route_match: &AppRouteMatch, config: &Config) -> bool {
    if route_match.not_found.is_some() {
        return false;
    }
    config.loading.enabled && route_match.loading.is_some()
}

pub(crate) async fn collect_page_metadata(
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
        tracing::error!("Could not determine dist/server path for metadata collection");
        return None;
    };

    let mut layout_paths = Vec::with_capacity(route_match.layouts.len());
    for layout in &route_match.layouts {
        let file_path =
            component_dist_path(&base_path, &layout.file_path, layout.component_id.as_deref());
        if fs::try_exists(&file_path).await.unwrap_or(false) {
            layout_paths.push(path_to_file_url(&file_path));
        }
    }

    let page_file_path = component_dist_path(
        &base_path,
        &route_match.route.file_path,
        route_match.route.component_id.as_deref(),
    );

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
    let base_url = get_base_url_from_context(context, &state.config);
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

fn get_base_url_from_context(context: &LayoutRenderContext, config: &Config) -> String {
    if let Some(host) = context.headers.get("host") {
        let protocol = context
            .headers
            .get("x-forwarded-proto")
            .or_else(|| context.headers.get("x-forwarded-protocol"))
            .map(String::as_str)
            .unwrap_or_else(|| if config.is_production() { "https" } else { "http" });

        format!("{protocol}://{host}")
    } else if config.is_production() {
        "https://localhost".to_string()
    } else {
        format!("http://localhost:{}", config.server.port)
    }
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
    route_match: AppRouteMatch,
    context: LayoutRenderContext,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let layout_renderer = LayoutRenderer::with_shared_cache(
        Arc::clone(&state.renderer),
        Arc::clone(&state.layout_html_cache),
    );
    let is_not_found = route_match.not_found.is_some();

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
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(
                "Failed to render RSC navigation for streaming '{}': {}",
                route_match.route.path,
                e
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

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
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        RenderResult::Static(rsc_flight_protocol) => {
            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

            let sorted_flight_protocol = sort_flight_protocol(&rsc_flight_protocol);

            let final_payload = if sorted_flight_protocol.ends_with('\n') {
                sorted_flight_protocol
            } else {
                format!("{sorted_flight_protocol}\n")
            };

            let mut response_builder = Response::builder()
                .status(status_code)
                .header("content-type", "text/x-component")
                .header("vary", "Accept");

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

            let mut response_builder = Response::builder()
                .status(status_code)
                .header("content-type", "text/x-component")
                .header("vary", "Accept");

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
    mut chunks: Receiver<Result<Vec<u8>, String>>,
    is_not_found: bool,
    accept_encoding: Option<&str>,
) -> http::Response<Body> {
    let stall_timeout = Duration::from_millis(chunked_stream_stall_timeout_ms());

    let byte_stream = async_stream::stream! {
        match content_type {
            ChunkedContentType::Html => {
                yield Ok::<_, Error>(shell);

                loop {
                    match time::timeout(stall_timeout, chunks.recv()).await {
                        Ok(Some(Ok(chunk_bytes))) => {
                            if !chunk_bytes.is_empty() {
                                yield Ok(Bytes::from(chunk_bytes));
                            }
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error in chunked HTML stream: {}", e);
                            yield Err(Error::other(e));
                            break;
                        }
                        Ok(None) => break,
                        Err(_) => {
                            tracing::error!(
                                "Chunked HTML stream stalled: no chunk received within {} ms",
                                stall_timeout.as_millis()
                            );
                            yield Ok(chunked_stream_error_chunk("Stream timed out waiting for content"));
                            break;
                        }
                    }
                }

                yield Ok(closing);
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
                            yield Err(Error::other(e));
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

    let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
    let compressed_stream = compress_stream(byte_stream, encoding);
    let vary =
        if encoding.as_header_value().is_some() { "Accept, Accept-Encoding" } else { "Accept" };

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
    route_match: AppRouteMatch,
    context: LayoutRenderContext,
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

    let is_not_found = route_match.not_found.is_some();

    match layout_renderer
        .render_route_with_streaming(&route_match, &context, Some(request_context), false)
        .await
    {
        Ok(render_result) => match render_result {
            RenderResult::Static(html_content) => {
                let html_with_assets =
                    match inject_assets_into_html(&html_content, &state.config).await {
                        Ok(html) => html,
                        Err(e) => {
                            tracing::error!("Failed to inject assets into HTML: {}", e);
                            html_content
                        }
                    };

                let final_html =
                    wrap_html_with_metadata(html_with_assets, context.metadata.as_ref(), &state);

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
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Err(e) => {
            tracing::error!("Synchronous rendering failed: {}", e);
            render_fallback_html(&state, &route_match.route.path, is_not_found).await
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
        .render_route_with_streaming(&route_match, &context, Some(request_context), false)
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

            let final_html =
                wrap_html_with_metadata(html_with_assets, context.metadata.as_ref(), &state);

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
        RenderResult::StaticBinary(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        RenderResult::Chunked { content_type: ChunkedContentType::RscFlight, .. } => {
            tracing::error!("RSC chunked render not supported in HTML streaming mode");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn render_fallback_html(
    state: &ServerState,
    path: &str,
    is_not_found: bool,
) -> Result<Response, StatusCode> {
    let index_path = if state.config.is_development() {
        let root_index = PathBuf::from("index.html");
        if fs::try_exists(&root_index).await.unwrap_or(false) {
            root_index
        } else {
            state.config.public_dir().join("index.html")
        }
    } else {
        state.config.public_dir().join("index.html")
    };

    if fs::try_exists(&index_path).await.unwrap_or(false) {
        if state.config.is_production()
            && let Some(cached_html) = state.html_cache.get(path)
        {
            let html = cached_html.clone();
            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .header("vary", "Accept")
                .body(Body::from(html))
                .expect("Valid HTML response"));
        }

        if let Ok(html_content) = fs::read_to_string(&index_path).await {
            let final_html = if state.config.is_development() {
                inject_vite_client(&html_content, state.config.vite.port)
            } else {
                html_content
            };

            if state.config.is_production() {
                state.html_cache.insert(path.to_string(), final_html.clone());
            }

            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            return Ok(Response::builder()
                .status(status_code)
                .header("content-type", "text/html; charset=utf-8")
                .header("vary", "Accept")
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
  <title>rari App Router</title>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="http://localhost:{vite_port}/@vite/client"></script>
  <script type="module">
    import 'http://localhost:{vite_port}/@id/virtual:rari-entry-client';
  </script>
</body>
</html>"#
        );

        let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

        #[expect(
            clippy::expect_used,
            reason = "Response::builder() with valid components never fails"
        )]
        return Ok(Response::builder()
            .status(status_code)
            .header("content-type", "text/html; charset=utf-8")
            .header("vary", "Accept")
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

    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    Ok(Response::builder()
        .status(status_code)
        .header("content-type", "text/html; charset=utf-8")
        .header("vary", "Accept")
        .body(Body::from(error_html))
        .expect("Valid HTML response"))
}

#[axum::debug_handler]
#[expect(
    clippy::implicit_hasher,
    reason = "FxHashMap is the specific hasher needed for query params"
)]
pub async fn handle_app_route(
    State(state): State<ServerState>,
    uri: Uri,
    Query(query_params): Query<FxHashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let path = uri.path();

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

            if let Ok(file_path) =
                validate_safe_path(state.config.public_dir(), path_without_leading_slash).await
                && let Ok(metadata) = fs::metadata(&file_path).await
                && metadata.is_file()
            {
                match fs::read(&file_path).await {
                    Ok(content) => {
                        let content_type = get_content_type(path_without_leading_slash);
                        let cache_control = &state.config.caching.static_files;
                        #[expect(
                            clippy::expect_used,
                            reason = "Response::builder() with valid components never fails"
                        )]
                        return Ok(Response::builder()
                            .header("content-type", content_type)
                            .header("cache-control", cache_control)
                            .body(Body::from(content))
                            .expect("Valid static file response"));
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to read static file {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    let Some(app_router) = &state.app_router else {
        tracing::error!(
            "App router not initialized - routes.json may be missing or invalid. Path: {}",
            path
        );
        return Err(StatusCode::NOT_FOUND);
    };

    let mut route_match = match app_router.match_route(path) {
        Ok(m) => m,
        Err(_) => match app_router.create_not_found_match(path) {
            Some(not_found_match) => not_found_match,
            None => return Err(StatusCode::NOT_FOUND),
        },
    };

    let request_context = Arc::new(
        RequestContext::new(path.to_string()).with_http_headers(extract_headers(&headers)),
    );

    let runtime = {
        let renderer = state.renderer.lock().await;
        Arc::clone(&renderer.runtime)
    };
    inject_action_form_state_from_cookie(&runtime, request_cookie_header(&headers)).await;

    let render_mode = RequestTypeDetector::detect_render_mode(&headers);
    let accept_encoding = headers.get("accept-encoding").and_then(|v| v.to_str().ok());

    let query_params_for_cache = query_params.clone();
    let cookie_header = request_cookie_header(&headers);
    let query_params_ref = route_query_params_for_cache(&query_params_for_cache);

    if matches!(render_mode, RenderMode::Ssr) && can_use_static_fast_cache(cookie_header) {
        let fast_key =
            response::ResponseCache::generate_static_fast_cache_key(path, query_params_ref, None);

        if let Some(prebuilt) = state.static_fast_cache.get(&fast_key) {
            let prebuilt = Arc::clone(prebuilt.value());

            if let Some(client_etag) = headers.get("if-none-match").and_then(|v| v.to_str().ok())
                && client_etag == prebuilt.etag
            {
                #[expect(
                    clippy::expect_used,
                    reason = "Response::builder() with valid components never fails"
                )]
                return Ok(Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .header("etag", &prebuilt.etag)
                    .header("vary", static_html_vary_header(None))
                    .body(Body::empty())
                    .expect("Valid 304 response"));
            }

            let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
            let (body, encoding_header) = prebuilt.body_for(encoding);
            let status = if prebuilt.is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

            let mut builder = Response::builder()
                .status(status)
                .header("content-type", prebuilt.content_type.as_str())
                .header("cache-control", prebuilt.cache_control.as_str())
                .header("etag", &prebuilt.etag)
                .header("vary", static_html_vary_header(None))
                .header("x-cache", "HIT");

            if let Some(enc) = encoding_header {
                builder = builder.header("content-encoding", enc);
            }

            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            return Ok(builder.body(Body::from(body)).expect("Valid fast-path response"));
        }
    }
    let search_params = extract_search_params(query_params);

    let request_headers = extract_headers(&headers);

    let mut context = create_layout_context(
        route_match.params.clone(),
        search_params.clone(),
        request_headers,
        route_match.pathname.clone(),
    );
    context.template_navigation_id = utils::http::parse_navigation_id(&context.headers);

    let layout_renderer = LayoutRenderer::with_shared_cache(
        Arc::clone(&state.renderer),
        Arc::clone(&state.layout_html_cache),
    );

    if route_match.not_found.is_none() && route_match.route.is_dynamic {
        match layout_renderer.check_page_not_found(&route_match, &context).await {
            Ok(true) => {
                if let Some(not_found_entry) = app_router.find_not_found(&route_match.route.path) {
                    route_match.not_found = Some(not_found_entry);
                }
            }
            Ok(false) => {}
            Err(e) => {
                tracing::error!("Failed to check if page is not found: {}", e);
            }
        }
    }

    match render_mode {
        RenderMode::RscNavigation => {
            let use_streaming = should_use_streaming(&route_match, &state.config);

            if use_streaming {
                context.metadata = collect_page_metadata(&state, &route_match, &context).await;

                return render_rsc_navigation_streaming(
                    Arc::new(state),
                    route_match,
                    context,
                    accept_encoding,
                )
                .await;
            }
            let cache_key = response_cache_key(path, query_params_ref, Some("rsc"), cookie_header);

            if context.template_navigation_id.is_none()
                && let Some(cached) = state.response_cache.get(&cache_key).await
            {
                let status_code = if route_match.not_found.is_some() {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::OK
                };

                let merged_vary = merge_vary_with_accept(cached.headers.get("vary"));

                let mut response_builder = Response::builder()
                    .status(status_code)
                    .header("content-type", "text/x-component")
                    .header("vary", merged_vary)
                    .header("x-cache", "HIT");

                for (key, value) in &cached.headers {
                    if key.as_str() != "vary" {
                        response_builder = response_builder.header(key, value);
                    }
                }

                #[expect(
                    clippy::expect_used,
                    reason = "Response::builder() with valid components never fails"
                )]
                return Ok(response_builder
                    .body(Body::from(cached.body))
                    .expect("Valid cached RSC response"));
            }

            context.metadata = collect_page_metadata(&state, &route_match, &context).await;

            match layout_renderer
                .render_route_by_mode(&route_match, &context, Some(Arc::clone(&request_context)))
                .await
            {
                Ok(rsc_flight_protocol) => {
                    let status_code = if route_match.not_found.is_some() {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::OK
                    };

                    let mut response_builder = Response::builder()
                        .status(status_code)
                        .header("content-type", "text/x-component")
                        .header("vary", rsc_vary_header(cookie_header))
                        .header("x-cache", "MISS");

                    let mut cache_headers = HeaderMap::new();

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
                        response::RouteCachePolicy::from_cache_control(cache_control, path);

                    if should_store_response_cache(&state, &cache_policy).await {
                        let response_cache_tags =
                            merge_response_cache_tags(&state, cache_policy.tags.clone()).await;
                        if cookie_header.is_some() {
                            cache_headers.insert("vary", HeaderValue::from_static("Cookie"));
                        }
                        let cached_response = response::CachedResponse {
                            body: Bytes::from(rsc_flight_protocol.clone()),
                            headers: cache_headers,
                            metadata: response::CacheMetadata {
                                cached_at: Instant::now(),
                                ttl: cache_policy.ttl,
                                etag: None,
                                tags: response_cache_tags,
                            },
                            compressed_zstd: None,
                            compressed_br: None,
                            compressed_gzip: None,
                        };

                        state.response_cache.set(cache_key, cached_response).await;
                    }

                    #[expect(
                        clippy::expect_used,
                        reason = "Response::builder() with valid components never fails"
                    )]
                    Ok(response_builder
                        .body(Body::from(rsc_flight_protocol))
                        .expect("Valid RSC response"))
                }
                Err(e) => {
                    tracing::error!("Failed to render RSC: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        RenderMode::Ssr => {
            let cache_key = response_cache_key(path, query_params_ref, None, cookie_header);

            let client_etag = headers.get("if-none-match").and_then(|v| v.to_str().ok());

            if let Some(cached) = state.response_cache.get(&cache_key).await {
                if let (Some(cached_etag), Some(client_etag)) = (&cached.metadata.etag, client_etag)
                    && cached_etag == client_etag
                {
                    let merged_vary = merge_vary_with_accept(cached.headers.get("vary"));

                    #[expect(
                        clippy::expect_used,
                        reason = "Response::builder() with valid components never fails"
                    )]
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .header("etag", cached_etag)
                        .header("vary", merged_vary)
                        .body(Body::empty())
                        .expect("Valid 304 response"));
                }

                let status_code = if route_match.not_found.is_some() {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::OK
                };

                let merged_vary = merge_vary_with_accept(cached.headers.get("vary"));

                let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);

                let (body_bytes, actual_encoding) =
                    if let Some(pre_compressed) = cached.get_compressed(&encoding) {
                        (pre_compressed.clone(), encoding)
                    } else if matches!(encoding, CompressionEncoding::Identity) {
                        (cached.body.clone(), CompressionEncoding::Identity)
                    } else {
                        let (compressed, actual_enc) =
                            compress_body(cached.body.clone(), encoding).await;
                        if !matches!(actual_enc, CompressionEncoding::Identity) {
                            let mut updated = cached.clone();
                            match actual_enc {
                                CompressionEncoding::Zstd => {
                                    updated.compressed_zstd = Some(compressed.clone());
                                }
                                CompressionEncoding::Brotli => {
                                    updated.compressed_br = Some(compressed.clone());
                                }
                                CompressionEncoding::Gzip => {
                                    updated.compressed_gzip = Some(compressed.clone());
                                }
                                CompressionEncoding::Identity => {}
                            }
                            state.response_cache.update_in_place(&cache_key, updated).await;
                        }
                        (compressed, actual_enc)
                    };

                let mut response_builder = Response::builder()
                    .status(status_code)
                    .header("content-type", "text/html; charset=utf-8")
                    .header("vary", merged_vary)
                    .header("x-cache", "HIT");

                if let Some(encoding_header) = actual_encoding.as_header_value() {
                    response_builder = response_builder.header("content-encoding", encoding_header);
                }

                if let Some(etag) = &cached.metadata.etag {
                    response_builder = response_builder.header("etag", etag);
                }

                for (key, value) in &cached.headers {
                    if key.as_str() != "vary"
                        && key.as_str() != "content-encoding"
                        && key.as_str() != "content-length"
                        && key.as_str() != "content-type"
                        && key.as_str() != "etag"
                    {
                        response_builder = response_builder.header(key, value);
                    }
                }

                #[expect(
                    clippy::expect_used,
                    reason = "Response::builder() with valid components never fails"
                )]
                return Ok(response_builder
                    .body(Body::from(body_bytes))
                    .expect("Valid cached response"));
            }

            context.metadata = collect_page_metadata(&state, &route_match, &context).await;

            let use_streaming = should_use_streaming(&route_match, &state.config);

            if use_streaming {
                let response = render_with_fallback(
                    Arc::new(state.clone()),
                    route_match.clone(),
                    context.clone(),
                    accept_encoding,
                )
                .await?;

                if (response.status() == StatusCode::OK
                    || response.status() == StatusCode::NOT_FOUND)
                    && let Some(render_mode) = response.headers().get("x-render-mode")
                    && render_mode == "static"
                {
                    let (parts, body) = response.into_parts();
                    let body_bytes = body::to_bytes(body, usize::MAX)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    let cache_control_value =
                        parts.headers.get("cache-control").and_then(|v| v.to_str().ok());

                    let cache_policy = if let Some(cc) = cache_control_value {
                        response::RouteCachePolicy::from_cache_control(cc, path)
                    } else {
                        let mut policy = response::RouteCachePolicy {
                            ttl: state.response_cache.config.default_ttl,
                            ..Default::default()
                        };
                        policy.tags.push(path.to_string());
                        policy
                    };

                    if should_store_response_cache(&state, &cache_policy).await {
                        let response_encoding = parts
                            .headers
                            .get("content-encoding")
                            .and_then(|v| v.to_str().ok())
                            .map(|enc| match enc {
                                "gzip" => CompressionEncoding::Gzip,
                                "br" => CompressionEncoding::Brotli,
                                "zstd" => CompressionEncoding::Zstd,
                                _ => CompressionEncoding::Identity,
                            })
                            .unwrap_or(CompressionEncoding::Identity);

                        let (raw_body, compressed_gzip, compressed_br, compressed_zstd) =
                            if matches!(response_encoding, CompressionEncoding::Identity) {
                                (body_bytes.clone(), None, None, None)
                            } else {
                                let decompressed =
                                    decompress_bytes(&body_bytes, response_encoding).await;
                                match decompressed {
                                    Ok(raw) => {
                                        let compressed_variant = body_bytes.clone();
                                        let (gzip, br, zstd) = match response_encoding {
                                            CompressionEncoding::Gzip => {
                                                (Some(compressed_variant), None, None)
                                            }
                                            CompressionEncoding::Brotli => {
                                                (None, Some(compressed_variant), None)
                                            }
                                            CompressionEncoding::Zstd => {
                                                (None, None, Some(compressed_variant))
                                            }
                                            CompressionEncoding::Identity => (None, None, None),
                                        };
                                        (raw, gzip, br, zstd)
                                    }
                                    Err(_) => (body_bytes.clone(), None, None, None),
                                }
                            };

                        let etag = response::ResponseCache::generate_etag(&raw_body);
                        let mut response_headers = HeaderMap::new();
                        for (key, value) in &parts.headers {
                            if key.as_str() != "content-encoding"
                                && key.as_str() != "content-length"
                            {
                                response_headers.insert(key.clone(), value.clone());
                            }
                        }
                        if cookie_header.is_some() {
                            response_headers.insert("vary", HeaderValue::from_static("Cookie"));
                        }

                        let merged_tags =
                            merge_response_cache_tags(&state, cache_policy.tags.clone()).await;

                        let cached_response = response::CachedResponse {
                            body: raw_body,
                            headers: response_headers,
                            metadata: response::CacheMetadata {
                                cached_at: Instant::now(),
                                ttl: cache_policy.ttl,
                                etag: Some(etag.clone()),
                                tags: merged_tags,
                            },
                            compressed_zstd,
                            compressed_br,
                            compressed_gzip,
                        };

                        state.response_cache.set(cache_key.clone(), cached_response).await;

                        let merged_vary = merge_vary_with_accept(parts.headers.get("vary"));

                        let mut response_builder = Response::builder().status(parts.status);

                        for (key, value) in &parts.headers {
                            if key.as_str() != "vary" {
                                response_builder = response_builder.header(key, value);
                            }
                        }

                        #[expect(
                            clippy::expect_used,
                            reason = "Response::builder() with valid components never fails"
                        )]
                        return Ok(response_builder
                            .header("etag", etag)
                            .header("vary", merged_vary)
                            .header("x-cache", "MISS")
                            .body(Body::from(body_bytes))
                            .expect("Valid response"));
                    }

                    return Ok(Response::from_parts(parts, Body::from(body_bytes)));
                }

                return Ok(response);
            }

            let render_result = match layout_renderer
                .render_route_with_streaming(
                    &route_match,
                    &context,
                    Some(Arc::clone(&request_context)),
                    false,
                )
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("Direct HTML rendering failed: {}, falling back to shell", e);
                    return render_fallback_html(&state, path, route_match.not_found.is_some())
                        .await;
                }
            };

            let (final_html, etag) = match render_result {
                RenderResult::Static(html_content) => {
                    let html_with_assets =
                        match inject_assets_into_html(&html_content, &state.config).await {
                            Ok(html) => html,
                            Err(e) => {
                                tracing::error!("Failed to inject assets into HTML: {}", e);
                                html_content
                            }
                        };

                    let final_html = wrap_html_with_metadata(
                        html_with_assets,
                        context.metadata.as_ref(),
                        &state,
                    );

                    let etag = response::ResponseCache::generate_etag(final_html.as_bytes());

                    (final_html, etag)
                }
                RenderResult::Chunked {
                    content_type: ChunkedContentType::Html,
                    shell,
                    closing,
                    mut chunks,
                } => {
                    let html = match drain_chunked_stream(shell, closing, &mut chunks).await {
                        Ok(html) => html,
                        Err(error) => {
                            tracing::error!(
                                "Failed to drain chunked HTML stream for build cache: {error}"
                            );
                            return render_fallback_html(
                                &state,
                                path,
                                route_match.not_found.is_some(),
                            )
                            .await;
                        }
                    };
                    let etag = response::ResponseCache::generate_etag(html.as_bytes());
                    (html, etag)
                }
                RenderResult::StaticBinary(_bytes) => {
                    tracing::error!("StaticBinary not supported in build mode");
                    return render_fallback_html(&state, path, route_match.not_found.is_some())
                        .await;
                }
                RenderResult::Chunked { content_type: ChunkedContentType::RscFlight, .. } => {
                    tracing::error!(
                        "RSC chunked render not supported in build mode HTML rendering"
                    );
                    return render_fallback_html(&state, path, route_match.not_found.is_some())
                        .await;
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
                .header("vary", rsc_vary_header(cookie_header))
                .header("x-cache", "MISS");

            let cache_control_value = state.config.get_cache_control_for_route(path);
            let mut response_headers = HeaderMap::new();

            response_builder = response_builder.header("cache-control", cache_control_value);
            if let Ok(header_value) = HeaderValue::from_str(cache_control_value) {
                response_headers.insert(CACHE_CONTROL, header_value);
            }
            if cookie_header.is_some() {
                response_headers.insert("vary", HeaderValue::from_static("Cookie"));
            }

            let cache_policy =
                response::RouteCachePolicy::from_cache_control(cache_control_value, path);

            if should_store_response_cache(&state, &cache_policy).await {
                let response_cache_tags =
                    merge_response_cache_tags(&state, cache_policy.tags.clone()).await;
                let body_bytes = Bytes::from(final_html.clone());

                let (compressed_gzip, compressed_zstd, compressed_br) = {
                    let (gz, gz_enc) =
                        compress_body(body_bytes.clone(), CompressionEncoding::Gzip).await;
                    let (zs, zs_enc) =
                        compress_body(body_bytes.clone(), CompressionEncoding::Zstd).await;
                    let (br, br_enc) =
                        compress_body(body_bytes.clone(), CompressionEncoding::Brotli).await;
                    (
                        if matches!(gz_enc, CompressionEncoding::Gzip) { Some(gz) } else { None },
                        if matches!(zs_enc, CompressionEncoding::Zstd) { Some(zs) } else { None },
                        if matches!(br_enc, CompressionEncoding::Brotli) { Some(br) } else { None },
                    )
                };

                if can_use_static_fast_cache(cookie_header) {
                    let fast_key = response::ResponseCache::generate_static_fast_cache_key(
                        path,
                        query_params_ref,
                        None,
                    );
                    state.static_fast_cache.insert(
                        fast_key,
                        Arc::new(response::PrebuiltResponse {
                            identity: body_bytes.clone(),
                            gzip: compressed_gzip.clone(),
                            br: compressed_br.clone(),
                            zstd: compressed_zstd.clone(),
                            etag: etag.clone(),
                            content_type: "text/html; charset=utf-8".to_string(),
                            cache_control: cache_control_value.to_string(),
                            is_not_found: route_match.not_found.is_some(),
                        }),
                    );
                }

                let cached_response = response::CachedResponse {
                    body: body_bytes,
                    headers: response_headers,
                    metadata: response::CacheMetadata {
                        cached_at: Instant::now(),
                        ttl: cache_policy.ttl,
                        etag: Some(etag.clone()),
                        tags: response_cache_tags,
                    },
                    compressed_zstd,
                    compressed_br,
                    compressed_gzip,
                };

                state.response_cache.set(cache_key, cached_response).await;
            }

            {
                use crate::server::compression::{CompressionEncoding, compress_body};
                let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
                let (body_bytes, actual_encoding) =
                    compress_body(Bytes::from(final_html), encoding).await;

                if let Some(encoding_header) = actual_encoding.as_header_value() {
                    response_builder = response_builder.header("content-encoding", encoding_header);
                }

                #[expect(
                    clippy::expect_used,
                    reason = "Response::builder() with valid components never fails"
                )]
                Ok(response_builder.body(Body::from(body_bytes)).expect("Valid HTML response"))
            }
        }
    }
}
