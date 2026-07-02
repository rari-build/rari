use std::{
    env,
    io::{Cursor, Error},
    path::{Path, PathBuf},
    string::String,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
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
use cow_utils::CowUtils;
use rari_utils::path_to_file_url;
use rustc_hash::FxHashMap;
use tokio::{
    fs,
    sync::{Mutex, mpsc::Receiver},
    time::{self, Duration},
};
use tracing::error;

use crate::{
    RscHtmlRenderer,
    rendering::{
        layout::{
            LayoutRenderContext, LayoutRenderer, OpenGraphImage, OpenGraphImageDescriptor,
            OpenGraphMetadata, PageMetadata, RenderResult, TwitterMetadata, create_layout_context,
        },
        r#static::RscToHtmlConverter,
        streaming::stream::RscStream,
    },
    server::{
        ServerState,
        cache::response,
        compression::{CompressionEncoding, compress_stream},
        config::Config,
        core::{
            types::request::{RenderMode, RequestTypeDetector},
            utils::{
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
            utils::{
                self, extract_asset_links_from_index_html, extract_body_scripts_from_index_html,
                inject_assets_into_html, inject_vite_client,
            },
        },
        routing::app_router::AppRouteMatch,
    },
};

async fn decompress_bytes(
    data: &bytes::Bytes,
    encoding: CompressionEncoding,
) -> Result<bytes::Bytes, Error> {
    use tokio::io::AsyncReadExt;

    let data = data.clone();
    match encoding {
        CompressionEncoding::Gzip => {
            let mut decoder = GzipDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(bytes::Bytes::from(decompressed))
        }
        CompressionEncoding::Brotli => {
            let mut decoder = BrotliDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(bytes::Bytes::from(decompressed))
        }
        CompressionEncoding::Zstd => {
            let mut decoder = ZstdDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(bytes::Bytes::from(decompressed))
        }
        CompressionEncoding::Identity => Ok(data),
    }
}

fn sort_rsc_rows(wire_format: &str) -> String {
    let mut rows_with_ids: Vec<(u32, String)> = Vec::new();

    for row in wire_format.lines() {
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

pub(crate) async fn collect_page_metadata(
    state: &ServerState,
    route_match: &AppRouteMatch,
    context: &LayoutRenderContext,
) -> Option<PageMetadata> {
    let dist_server_path =
        env::current_dir().ok().map(|p| p.join("dist/server")).and_then(|p| p.canonicalize().ok());

    let base_path = match dist_server_path {
        Some(path) => path,
        None => {
            error!("Could not determine dist/server path for metadata collection");
            return None;
        }
    };

    fn component_dist_path(
        base_path: &Path,
        file_path: &str,
        component_id: Option<&str>,
    ) -> PathBuf {
        if let Some(component_id) = component_id {
            return base_path.join(format!("{component_id}.js"));
        }

        fn convert_route_path_to_dist_path(path: &str) -> String {
            let (base, ext) = if let Some(pos) = path.rfind('.') {
                (&path[..pos], &path[pos..])
            } else {
                (path, "")
            };

            let converted_base =
                base.chars()
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

    let layout_paths: Vec<String> = route_match
        .layouts
        .iter()
        .filter_map(|layout| {
            let file_path =
                component_dist_path(&base_path, &layout.file_path, layout.component_id.as_deref());
            if file_path.exists() { Some(path_to_file_url(&file_path)) } else { None }
        })
        .collect();

    let page_file_path = component_dist_path(
        &base_path,
        &route_match.route.file_path,
        route_match.route.component_id.as_deref(),
    );

    if !page_file_path.exists() {
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
            error!("Streaming render failed, falling back to synchronous: {}", e);
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

    let request_context = Arc::new(RequestContext::new(route_match.route.path.clone()));

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
            error!(
                "Failed to render RSC navigation for streaming '{}': {}",
                route_match.route.path, e
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match render_result {
        RenderResult::Streaming(rsc_stream) => {
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
        RenderResult::FizzHtmlStream { .. } => {
            error!("FizzHtmlStream not supported in RSC-only mode");
            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            Ok(Response::builder()
                .status(status_code)
                .header("content-type", "text/x-component")
                .body(Body::from("0:\"Error: FizzHtmlStream not supported in RSC mode\"\n"))
                .expect("Valid error response"))
        }
        RenderResult::Static(rsc_wire_format) => {
            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };

            let sorted_wire_format = sort_rsc_rows(&rsc_wire_format);

            let final_payload = if sorted_wire_format.ends_with('\n') {
                sorted_wire_format
            } else {
                format!("{sorted_wire_format}\n")
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

async fn render_rsc_streaming_response(
    state: Arc<ServerState>,
    _route_match: AppRouteMatch,
    context: LayoutRenderContext,
    mut rsc_stream: RscStream,
    is_not_found: bool,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let should_continue = Arc::new(AtomicBool::new(true));
    let should_continue_clone = should_continue;

    let rsc_wire_stream = async_stream::stream! {
        while should_continue_clone.load(Relaxed) {
            match rsc_stream.next_chunk().await {
                Some(chunk) => {
                    let data = String::from_utf8_lossy(&chunk.data).to_string();
                    if data.trim() != "STREAM_COMPLETE" {
                        yield Ok::<_, Error>(bytes::Bytes::from(data));
                    }
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
    let cache_control = state.config.get_cache_control_for_route(&context.pathname);

    let mut response_builder = Response::builder()
        .status(status_code)
        .header("content-type", "text/x-component")
        .header("transfer-encoding", "chunked")
        .header("x-render-mode", "streaming")
        .header("cache-control", cache_control)
        .header("vary", "Accept")
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
    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    Ok(response_builder.body(body).expect("Valid RSC streaming response"))
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
    let request_context = Arc::new(RequestContext::new(route_match.route.path.clone()));

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
                            error!("Failed to inject assets into HTML: {}", e);
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
            RenderResult::FizzHtmlStream { shell, closing, chunks } => {
                render_fizz_html_stream(
                    state,
                    route_match,
                    context,
                    shell,
                    closing,
                    chunks,
                    is_not_found,
                    accept_encoding,
                )
                .await
            }
            RenderResult::Streaming(stream) => {
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
        },
        Err(e) => {
            error!("Synchronous rendering failed: {}", e);
            render_fallback_html(&state, &route_match.route.path, is_not_found).await
        }
    }
}

#[expect(clippy::too_many_arguments)]
async fn render_fizz_html_stream(
    state: Arc<ServerState>,
    _route_match: AppRouteMatch,
    context: LayoutRenderContext,
    shell: bytes::Bytes,
    closing: bytes::Bytes,
    mut chunks: Receiver<Result<Vec<u8>, String>>,
    is_not_found: bool,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    use crate::server::compression::compress_stream;

    let stall_timeout = Duration::from_millis(fizz_stream_stall_timeout_ms());

    let fizz_stream = async_stream::stream! {
        yield Ok::<_, Error>(shell);

        loop {
            match time::timeout(stall_timeout, chunks.recv()).await {
                Ok(Some(Ok(chunk_bytes))) => {
                    if !chunk_bytes.is_empty() {
                        yield Ok(bytes::Bytes::from(chunk_bytes));
                    }
                }
                Ok(Some(Err(e))) => {
                    error!("Error in Fizz stream chunk: {}", e);
                    yield Err(Error::other(e));
                    break;
                }
                Ok(None) => break,
                Err(_) => {
                    error!(
                        "Fizz stream stalled: no chunk received within {} ms",
                        stall_timeout.as_millis()
                    );
                    yield Ok(fizz_stream_error_chunk("Stream timed out waiting for content"));
                    break;
                }
            }
        }

        yield Ok(closing);
    };

    let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
    let compressed_stream = compress_stream(fizz_stream, encoding);

    let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
    let cache_control = state.config.get_cache_control_for_route(&context.pathname);

    let mut response_builder = Response::builder()
        .status(status_code)
        .header("content-type", "text/html; charset=utf-8")
        .header("transfer-encoding", "chunked")
        .header("x-content-type-options", "nosniff")
        .header("x-render-mode", "streaming")
        .header("cache-control", cache_control)
        .header("vary", "Accept");

    if let Some(encoding_header) = encoding.as_header_value() {
        response_builder = response_builder.header("content-encoding", encoding_header);
    }

    let body = Body::from_stream(compressed_stream);
    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    Ok(response_builder.body(body).expect("Valid Fizz streaming response"))
}

fn fizz_stream_stall_timeout_ms() -> u64 {
    env::var("RARI_STREAMING_STALL_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(60_000)
}

fn fizz_stream_error_chunk(message: &str) -> bytes::Bytes {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    bytes::Bytes::from(format!(
        r#"<div class="rari-error" style="color: red; border: 1px solid red; padding: 10px; border-radius: 4px; background-color: #fff5f5;"><strong>Error loading content: </strong>{escaped}</div>"#
    ))
}

async fn render_streaming_response(
    state: Arc<ServerState>,
    _route_match: AppRouteMatch,
    context: LayoutRenderContext,
    mut rsc_stream: RscStream,
    is_not_found: bool,
    accept_encoding: Option<&str>,
) -> Result<Response, StatusCode> {
    let asset_links = extract_asset_links_from_index_html().await;
    let body_scripts = extract_body_scripts_from_index_html().await;

    let html_renderer = {
        let renderer = state.renderer.lock().await;
        Arc::new(RscHtmlRenderer::new(Arc::clone(&renderer.runtime)))
    };

    let asset_tags = asset_links.as_deref().unwrap_or("");

    let title = context
        .metadata
        .as_ref()
        .and_then(|m| m.title.as_ref())
        .map(String::as_str)
        .unwrap_or("rari App");

    let base_shell = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    {asset_tags}
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
"#
    );

    let base_shell = if let Some(ref metadata) = context.metadata {
        inject_metadata(&base_shell, metadata, state.image_optimizer.as_deref())
    } else {
        base_shell
    };

    let converter = Arc::new(Mutex::new(RscToHtmlConverter::with_custom_shell(
        base_shell,
        body_scripts,
        html_renderer,
    )));

    let should_continue = Arc::new(AtomicBool::new(true));
    let should_continue_clone = should_continue;

    let html_stream = async_stream::stream! {
        while should_continue_clone.load(Relaxed) {
            match rsc_stream.next_chunk().await {
                Some(chunk) => {
                    let mut conv = converter.lock().await;

                    match conv.convert_chunk(chunk).await {
                        Ok(html_bytes) => {
                            if !html_bytes.is_empty() {
                                yield Ok::<_, Error>(bytes::Bytes::from(html_bytes));
                            }
                        }
                        Err(e) => {
                            if e.to_string().contains("disconnected") || e.to_string().contains("broken pipe") {
                                should_continue_clone.store(false, Relaxed);
                                break;
                            }

                            error!("Error converting RSC chunk to HTML: {}", e);
                            yield Err(Error::other(e.to_string()));
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
    let cache_control = state.config.get_cache_control_for_route(&context.pathname);

    let mut response_builder = Response::builder()
        .status(status_code)
        .header("content-type", "text/html; charset=utf-8")
        .header("transfer-encoding", "chunked")
        .header("x-content-type-options", "nosniff")
        .header("cache-control", cache_control)
        .header("vary", "Accept");

    if let Some(encoding_header) = encoding.as_header_value() {
        response_builder = response_builder.header("content-encoding", encoding_header);
    }

    let body = Body::from_stream(compressed_stream);
    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    Ok(response_builder.body(body).expect("Valid streaming response"))
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

    let request_context = Arc::new(RequestContext::new(route_match.route.path.clone()));

    let render_result = match layout_renderer
        .render_route_with_streaming(&route_match, &context, Some(request_context), false)
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
        RenderResult::Streaming(stream) => stream,
        RenderResult::FizzHtmlStream { shell, closing, chunks } => {
            return render_fizz_html_stream(
                state,
                route_match,
                context,
                shell,
                closing,
                chunks,
                is_not_found,
                accept_encoding,
            )
            .await;
        }
        RenderResult::Static(html) => {
            use crate::server::compression::compress_body;

            let html_with_assets = match inject_assets_into_html(&html, &state.config).await {
                Ok(html) => html,
                Err(e) => {
                    error!("Failed to inject assets into HTML: {}", e);
                    html
                }
            };

            let final_html =
                wrap_html_with_metadata(html_with_assets, context.metadata.as_ref(), &state);

            let status_code = if is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
            let cache_control = state.config.get_cache_control_for_route(&context.pathname);

            let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
            let (body_bytes, actual_encoding) =
                compress_body(bytes::Bytes::from(final_html), encoding).await;

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
            return Ok(response_builder.body(Body::from(body_bytes)).expect("Valid HTML response"));
        }
        RenderResult::StaticBinary(_bytes) => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
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
        let root_index = PathBuf::from("index.html");
        if root_index.exists() { root_index } else { state.config.public_dir().join("index.html") }
    } else {
        state.config.public_dir().join("index.html")
    };

    if index_path.exists() {
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

    fn should_use_streaming(route_match: &AppRouteMatch, config: &Config) -> bool {
        if route_match.not_found.is_some() {
            return false;
        }
        config.loading.enabled && route_match.loading.is_some()
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

            if let Ok(file_path) =
                validate_safe_path(state.config.public_dir(), path_without_leading_slash)
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
                        error!("Failed to read static file {}: {}", file_path.display(), e);
                    }
                }
            }
        }
    }

    let app_router = match &state.app_router {
        Some(router) => router,
        None => {
            tracing::error!(
                "App router not initialized - routes.json may be missing or invalid. Path: {}",
                path
            );
            return Err(StatusCode::NOT_FOUND);
        }
    };

    let mut route_match = match app_router.match_route(path) {
        Ok(m) => m,
        Err(_) => match app_router.create_not_found_match(path) {
            Some(not_found_match) => not_found_match,
            None => return Err(StatusCode::NOT_FOUND),
        },
    };

    let request_context = Arc::new(RequestContext::new(path.to_string()));

    let render_mode = RequestTypeDetector::detect_render_mode(&headers);
    let accept_encoding = headers.get("accept-encoding").and_then(|v| v.to_str().ok());

    let query_params_for_cache = query_params.clone();

    if matches!(render_mode, RenderMode::Ssr) {
        let fast_key = if query_params_for_cache.is_empty() {
            path.to_string()
        } else {
            let mut sorted: Vec<_> = query_params_for_cache.iter().collect();
            sorted.sort_by_key(|(k, _)| *k);
            let qs = sorted.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
            format!("{path}?{qs}")
        };

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
                    .header("vary", "Accept, Accept-Encoding")
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
                .header("vary", "Accept, Accept-Encoding")
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
                error!("Failed to check if page is not found: {}", e);
            }
        }
    }

    match render_mode {
        RenderMode::RscNavigation => {
            let use_streaming = should_use_streaming(&route_match, &state.config);

            if use_streaming {
                context.metadata = collect_page_metadata(&state, &route_match, &context).await;

                let result = render_rsc_navigation_streaming(
                    Arc::new(state),
                    route_match,
                    context,
                    accept_encoding,
                )
                .await;
                return result;
            }
            let cache_key = response::ResponseCache::generate_cache_key_with_mode(
                path,
                if query_params_for_cache.is_empty() {
                    None
                } else {
                    Some(&query_params_for_cache)
                },
                Some("rsc"),
            );

            if let Some(cached) = state.response_cache.get(&cache_key).await {
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
                Ok(rsc_wire_format) => {
                    let status_code = if route_match.not_found.is_some() {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::OK
                    };

                    let mut response_builder = Response::builder()
                        .status(status_code)
                        .header("content-type", "text/x-component")
                        .header("vary", "Accept")
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

                    if cache_policy.enabled && state.response_cache.config.enabled {
                        let cached_response = response::CachedResponse {
                            body: bytes::Bytes::from(rsc_wire_format.clone()),
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

                        state.response_cache.set(cache_key, cached_response).await;
                    }

                    #[expect(
                        clippy::expect_used,
                        reason = "Response::builder() with valid components never fails"
                    )]
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
            let cache_key = response::ResponseCache::generate_cache_key(
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

                use crate::server::compression::compress_body;
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

                    if cache_policy.enabled && state.response_cache.config.enabled {
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

                        let cached_response = response::CachedResponse {
                            body: raw_body,
                            headers: response_headers,
                            metadata: response::CacheMetadata {
                                cached_at: Instant::now(),
                                ttl: cache_policy.ttl,
                                etag: Some(etag.clone()),
                                tags: cache_policy.tags,
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
                    error!("Direct HTML rendering failed: {}, falling back to shell", e);
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
                                error!("Failed to inject assets into HTML: {}", e);
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
                RenderResult::FizzHtmlStream { shell, closing, mut chunks } => {
                    let mut html = String::from_utf8_lossy(&shell).into_owned();

                    while let Some(chunk_result) = chunks.recv().await {
                        match chunk_result {
                            Ok(chunk_data) => {
                                html.push_str(&String::from_utf8_lossy(&chunk_data));
                            }
                            Err(e) => {
                                error!("FizzHtmlStream chunk error in build mode: {e}");
                                break;
                            }
                        }
                    }

                    html.push_str(&String::from_utf8_lossy(&closing));

                    let etag = response::ResponseCache::generate_etag(html.as_bytes());
                    (html, etag)
                }
                RenderResult::Streaming(stream) => {
                    let asset_links = extract_asset_links_from_index_html().await;

                    let html_renderer = {
                        let renderer = state.renderer.lock().await;
                        Arc::new(RscHtmlRenderer::new(Arc::clone(&renderer.runtime)))
                    };

                    let title = context
                        .metadata
                        .as_ref()
                        .and_then(|m| m.title.as_ref())
                        .map(String::as_str)
                        .unwrap_or("rari App");

                    let asset_tags = asset_links.as_deref().unwrap_or("");
                    let base_shell = format!(
                        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    {asset_tags}
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
<div id="root">"#
                    );

                    let base_shell = if let Some(ref metadata) = context.metadata {
                        inject_metadata(&base_shell, metadata, state.image_optimizer.as_deref())
                    } else {
                        base_shell
                    };

                    let body_scripts = utils::extract_body_scripts_from_index_html().await;
                    let mut converter = RscToHtmlConverter::with_custom_shell(
                        base_shell,
                        body_scripts,
                        html_renderer,
                    );

                    let mut rsc_stream = stream;
                    let mut buffered_html = String::new();

                    while let Some(chunk) = rsc_stream.next_chunk().await {
                        match converter.convert_chunk(chunk).await {
                            Ok(html_bytes) => {
                                if !html_bytes.is_empty() {
                                    buffered_html.push_str(&String::from_utf8_lossy(&html_bytes));
                                }
                            }
                            Err(e) => {
                                error!("Error converting RSC chunk to HTML: {}", e);
                                return render_fallback_html(
                                    &state,
                                    path,
                                    route_match.not_found.is_some(),
                                )
                                .await;
                            }
                        }
                    }

                    let final_html = buffered_html;
                    let etag = response::ResponseCache::generate_etag(final_html.as_bytes());

                    (final_html, etag)
                }
                RenderResult::StaticBinary(_bytes) => {
                    error!("StaticBinary not supported in build mode");
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
                .header("vary", "Accept")
                .header("x-cache", "MISS");

            let cache_control_value = state.config.get_cache_control_for_route(path);
            let mut response_headers = HeaderMap::new();

            response_builder = response_builder.header("cache-control", cache_control_value);
            if let Ok(header_value) = HeaderValue::from_str(cache_control_value) {
                response_headers.insert(CACHE_CONTROL, header_value);
            }

            let cache_policy =
                response::RouteCachePolicy::from_cache_control(cache_control_value, path);

            if cache_policy.enabled && state.response_cache.config.enabled {
                let body_bytes = bytes::Bytes::from(final_html.clone());

                use crate::server::compression::{CompressionEncoding, compress_body};

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

                let fast_key = if query_params_for_cache.is_empty() {
                    path.to_string()
                } else {
                    let mut sorted: Vec<_> = query_params_for_cache.iter().collect();
                    sorted.sort_by_key(|(k, _)| *k);
                    let qs = sorted
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join("&");
                    format!("{path}?{qs}")
                };
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

                let cached_response = response::CachedResponse {
                    body: body_bytes,
                    headers: response_headers,
                    metadata: response::CacheMetadata {
                        cached_at: Instant::now(),
                        ttl: cache_policy.ttl,
                        etag: Some(etag.clone()),
                        tags: cache_policy.tags,
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
                    compress_body(bytes::Bytes::from(final_html), encoding).await;

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
