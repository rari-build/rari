#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{string::String, sync::Arc, time::Instant};

use axum::{
    body,
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header::CACHE_CONTROL},
    response::Response,
};
use bytes::Bytes;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use tokio::fs;

use super::{
    cache::{
        can_use_static_fast_cache, decompress_bytes, insert_response_cache_vary_header,
        request_cookie_header, response_cache_key, route_query_params_for_cache, rsc_vary_header,
        should_store_response_cache, static_html_vary_header,
    },
    render::{
        collect_page_metadata, mark_route_not_found_if_signaled, render_fallback_html,
        render_rsc_navigation_streaming, render_with_fallback, resolve_not_found_entry,
        should_use_streaming, wrap_html_with_metadata,
    },
};
use crate::{
    rendering::layout::{
        ChunkedContentType, LayoutRenderer, RenderResult, create_layout_context,
        drain_chunked_stream, is_rari_page_not_found, pathname_from_router_state_header,
        router_state_from_headers, shared_layout_paths_for_navigation,
        tree_from_router_state_header,
    },
    server::{
        ServerState,
        actions::parse_action_form_state_from_cookie,
        cache::{merge_page_cache_tags, response},
        compression::{CompressionEncoding, compress_all_encodings, compress_body},
        document::{metadata::apply_page_metadata, utils::inject_assets_into_html},
        error_response,
        host::{
            types::request::{RenderMode, detect_render_mode},
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
    },
};

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

            if state.config.is_development()
                && let Some(icon) = state
                    .app_icons
                    .iter()
                    .find(|icon| icon.url.trim_start_matches('/') == path_without_leading_slash)
            {
                let app_dir = state.project_root.join("src").join("app");
                if let Ok(file_path) = validate_safe_path(&app_dir, &icon.file_path).await
                    && let Ok(metadata) = fs::metadata(&file_path).await
                    && metadata.is_file()
                {
                    match fs::read(&file_path).await {
                        Ok(content) => {
                            let cache_control = &state.config.caching.static_files;
                            #[expect(
                                clippy::expect_used,
                                reason = "Response::builder() with valid components never fails"
                            )]
                            return Ok(Response::builder()
                                .header("content-type", icon.content_type.as_str())
                                .header("cache-control", cache_control)
                                .body(Body::from(content))
                                .expect("Valid app icon response"));
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to read app icon {}: {}",
                                file_path.display(),
                                e
                            );
                        }
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
        RequestContext::new(path.to_string())
            .with_http_headers(extract_headers(&headers))
            .with_action_form_state(parse_action_form_state_from_cookie(request_cookie_header(
                &headers,
            ))),
    );

    let render_mode = detect_render_mode(&headers);
    let accept_encoding = headers.get("accept-encoding").and_then(|v| v.to_str().ok());

    let cookie_header = request_cookie_header(&headers);
    let query_params_for_cache = route_query_params_for_cache(&query_params);
    let query_params_ref = query_params_for_cache.as_ref();

    if matches!(render_mode, RenderMode::Ssr) && can_use_static_fast_cache(cookie_header) {
        let fast_key =
            response::ResponseCache::generate_static_fast_cache_key(path, query_params_ref, None);

        if let Some(prebuilt) = state.static_fast_cache.get(&fast_key) {
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
    let search_params = extract_search_params(query_params_for_cache.clone().unwrap_or_default());

    let request_headers = extract_headers(&headers);

    let mut context = create_layout_context(
        route_match.params.clone(),
        search_params.clone(),
        request_headers,
        route_match.pathname.clone(),
    );
    context.template_navigation_id = utils::http::parse_navigation_id(&context.headers);

    if matches!(render_mode, RenderMode::RscNavigation)
        && context.template_navigation_id.is_some()
        && let Some(state_header) = router_state_from_headers(&context.headers)
        && let Some(from_pathname) = pathname_from_router_state_header(&state_header)
    {
        let client_tree = tree_from_router_state_header(&state_header);
        context.reuse_layout_paths = shared_layout_paths_for_navigation(
            app_router,
            &from_pathname,
            &context.pathname,
            client_tree.as_ref(),
        );
    }

    let layout_renderer = LayoutRenderer::with_shared_cache(
        Arc::clone(&state.renderer),
        Arc::clone(&state.layout_html_cache),
    );

    if route_match.not_found.is_none() && route_match.route.is_dynamic {
        match layout_renderer.check_page_not_found(&route_match, &context).await {
            Ok(true) => {
                if let Some(not_found_entry) = resolve_not_found_entry(app_router, &route_match) {
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
                    request_context,
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

            let rsc_result = layout_renderer
                .render_route(&route_match, &context, Some(Arc::clone(&request_context)))
                .await;

            let rsc_flight_protocol = match rsc_result {
                Ok(payload) => payload,
                Err(e) => {
                    if let Some(app_router) = state.app_router.as_ref()
                        && mark_route_not_found_if_signaled(&e, &mut route_match, app_router)
                    {
                        context.metadata =
                            collect_page_metadata(&state, &route_match, &context).await;
                        match layout_renderer
                            .render_route(
                                &route_match,
                                &context,
                                Some(Arc::clone(&request_context)),
                            )
                            .await
                        {
                            Ok(payload) => payload,
                            Err(retry_err) => {
                                tracing::error!("Failed to render RSC: {}", retry_err);
                                return Err(error_response::status(&retry_err));
                            }
                        }
                    } else {
                        tracing::error!("Failed to render RSC: {}", e);
                        return Err(error_response::status(&e));
                    }
                }
            };

            {
                let status_code = if route_match.not_found.is_some() {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::OK
                };

                let router_state_sensitive = context.template_navigation_id.is_some()
                    || !context.reuse_layout_paths.is_empty();

                let mut response_builder = Response::builder()
                    .status(status_code)
                    .header("content-type", "text/x-component")
                    .header("vary", rsc_vary_header(cookie_header, router_state_sensitive))
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

                let can_cache_rsc = !router_state_sensitive
                    && route_match.not_found.is_none()
                    && should_store_response_cache(&state, &cache_policy).await;
                if can_cache_rsc {
                    let response_cache_tags =
                        merge_page_cache_tags(&state, cache_policy.tags.clone()).await;
                    if cookie_header.is_some() {
                        insert_response_cache_vary_header(&mut cache_headers, cookie_header, false);
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

            let use_streaming = should_use_streaming(&route_match, &state.config);

            if use_streaming {
                let mut context = context.clone();
                let metadata = collect_page_metadata(&state, &route_match, &context).await;
                apply_page_metadata(&mut context, metadata);
                let response = render_with_fallback(
                    Arc::new(state.clone()),
                    route_match.clone(),
                    context,
                    accept_encoding,
                    Arc::clone(&request_context),
                )
                .await?;

                if response.status() == StatusCode::OK
                    && let Some(render_mode) = response.headers().get("x-render-mode")
                    && render_mode == "static"
                {
                    let (parts, body) = response.into_parts();
                    let body_bytes = body::to_bytes(body, usize::MAX).await.map_err(|e| {
                        tracing::error!("Failed to read response body for cache: {}", e);
                        error_response::status(&RariError::internal(format!(
                            "Failed to read response body for cache: {e}"
                        )))
                    })?;

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
                        insert_response_cache_vary_header(
                            &mut response_headers,
                            cookie_header,
                            true,
                        );

                        let merged_tags =
                            merge_page_cache_tags(&state, cache_policy.tags.clone()).await;

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

                        let merged_vary = static_html_vary_header(cookie_header);

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

            let metadata = collect_page_metadata(&state, &route_match, &context).await;
            context.metadata = metadata;

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
                        context.metadata =
                            collect_page_metadata(&state, &route_match, &context).await;
                        match layout_renderer
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
                            Err(retry_err) => {
                                tracing::error!(
                                    "Direct HTML rendering failed after not-found retry: {}",
                                    retry_err
                                );
                                return render_fallback_html(
                                    &state,
                                    route_match.not_found.is_some()
                                        || is_rari_page_not_found(&retry_err),
                                )
                                .await;
                            }
                        }
                    } else {
                        tracing::error!(
                            "Direct HTML rendering failed: {}, falling back to shell",
                            e
                        );
                        return render_fallback_html(
                            &state,
                            route_match.not_found.is_some() || is_rari_page_not_found(&e),
                        )
                        .await;
                    }
                }
            };

            let cache_control_value = state.config.get_cache_control_for_route(path);
            let cache_policy =
                response::RouteCachePolicy::from_cache_control(cache_control_value, path);
            let mut for_response_cache = should_store_response_cache(&state, &cache_policy).await;
            if route_match.not_found.is_some() {
                for_response_cache = false;
            }

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

                    let final_html = wrap_html_with_metadata(html_with_assets, &state);

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
                            return render_fallback_html(&state, route_match.not_found.is_some())
                                .await;
                        }
                    };
                    let final_html = wrap_html_with_metadata(html, &state);
                    let etag = response::ResponseCache::generate_etag(final_html.as_bytes());
                    (final_html, etag)
                }
                RenderResult::StaticBinary(_bytes) => {
                    tracing::error!("StaticBinary not supported in build mode");
                    return render_fallback_html(&state, route_match.not_found.is_some()).await;
                }
                RenderResult::Chunked { content_type: ChunkedContentType::RscFlight, .. } => {
                    tracing::error!(
                        "RSC chunked render not supported in build mode HTML rendering"
                    );
                    return render_fallback_html(&state, route_match.not_found.is_some()).await;
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
                .header("vary", static_html_vary_header(cookie_header))
                .header("x-cache", "MISS");

            let mut response_headers = HeaderMap::new();

            response_builder = response_builder.header("cache-control", cache_control_value);
            if let Ok(header_value) = HeaderValue::from_str(cache_control_value) {
                response_headers.insert(CACHE_CONTROL, header_value);
            }
            insert_response_cache_vary_header(&mut response_headers, cookie_header, true);

            // One refcounted buffer serves the response cache, the fast
            // cache, and the response body, the page was previously cloned
            // in full for the cache and again for the body.
            let final_html = Bytes::from(final_html);

            if for_response_cache {
                let response_cache_tags =
                    merge_page_cache_tags(&state, cache_policy.tags.clone()).await;
                let body_bytes = final_html.clone();

                let (compressed_gzip, compressed_zstd, compressed_br) =
                    compress_all_encodings(body_bytes.clone()).await;

                if can_use_static_fast_cache(cookie_header) {
                    let fast_key = response::ResponseCache::generate_static_fast_cache_key(
                        path,
                        query_params_ref,
                        None,
                    );
                    response::insert_static_fast_cache(
                        &state.static_fast_cache,
                        &fast_key,
                        Arc::new(response::PrebuiltResponse {
                            identity: body_bytes.clone(),
                            gzip: compressed_gzip.clone(),
                            br: compressed_br.clone(),
                            zstd: compressed_zstd.clone(),
                            etag: etag.clone(),
                            content_type: "text/html; charset=utf-8".to_string(),
                            cache_control: cache_control_value.to_string(),
                            is_not_found: route_match.not_found.is_some(),
                            cached_at: Instant::now(),
                        }),
                        state.response_cache.config.max_entries,
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
                let (body_bytes, actual_encoding) = compress_body(final_html, encoding).await;

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
