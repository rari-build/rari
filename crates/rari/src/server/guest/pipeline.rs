//! The host's request pipeline for guest frameworks.
//!
//! Everything a page request goes through *around* the framework lives here,
//! once, for every guest: public files, the host's route decision, the static
//! fast tier and the response cache, the request descriptor handed to V8, and
//! streaming or buffering the guest's response back out. A [`GuestRenderer`]
//! only contributes the framework-specific pieces (bundle loading, URL
//! normalisation, the render call), so a Solid or Svelte adapter gets the same
//! behaviour, and the same `x-rari-route` / `x-cache` semantics, as the Qwik
//! one without copying any of this.

use std::{
    env, io,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    body::{Body, Bytes, to_bytes},
    http::{
        HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode,
        request::Parts,
        header::{
            ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, ETAG,
            HOST, IF_NONE_MATCH, SET_COOKIE, VARY,
        },
    },
    response::{IntoResponse, Response},
};
use base64::prelude::*;
use rustc_hash::FxHashMap;
use serde::Serialize;
use tokio::time;
use url::form_urlencoded;

use super::{GuestRenderer, serve_mounted_file, stream::next_stream_id};
use crate::server::{
    ServerState,
    cache::response::{
        CacheMetadata, CachedResponse, PrebuiltResponse, ResponseCache, RouteCachePolicy,
        insert_static_fast_cache,
    },
    compression::{CompressionEncoding, compress_all_encodings},
    error_response,
    host::utils::http::merge_vary_with_accept,
    routing::{
        AppRouter,
        app::cache::{
            can_use_static_fast_cache, insert_response_cache_vary_header, request_cookie_header,
            response_cache_key, route_query_params_for_cache, static_html_vary_header,
        },
        types::ParamValue,
    },
};

/// Request bodies above this are rejected before reaching V8 (form posts and
/// RPC payloads are small; uploads belong on an API route).
const MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;
/// Headers that describe the transport, not the response; never copied from
/// the guest's frame onto the host response.
const HOP_BY_HOP_HEADERS: &[&str] =
    &["content-length", "transfer-encoding", "connection", "keep-alive"];
/// Every guest response carries the host's route decision.
const ROUTE_HEADER: &str = "x-rari-route";

/// Route decision the host hands to the guest: rari matched the URL against
/// `dist/server/routes.json`, so the framework does not have to decide what a
/// URL *is*; it only renders what the host resolved. Adapters expose it to app
/// code (Qwik: `platform.rari.route`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HostRoute {
    /// Matched route pattern (`/blog/[slug]`), or the request path for a 404 page.
    pub path: String,
    /// Route params as plain values (strings, or arrays for catch-alls).
    pub params: serde_json::Map<String, serde_json::Value>,
    /// Layout chain, outermost first, as manifest file paths. Resolved with
    /// rari's route-file grammar (`page@name`, `page!`, `layout-name`, `layout!`).
    pub layouts: Vec<String>,
    /// The host matched nothing and resolved the app's not-found page instead.
    pub not_found: bool,
}

/// What the host hands a guest per request. Serialised as the JSON argument of
/// the guest's handler; `@rari/core/guest` holds the TypeScript twin.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct GuestRequest {
    /// Stream id for the runtime's chunk ops (see [`super::stream`]).
    pub stream_id: String,
    /// Absolute request URL.
    pub url: String,
    pub method: String,
    /// Request headers as name/value pairs (repeated names appear repeatedly).
    pub headers: Vec<(String, String)>,
    /// Base64-encoded body for methods that carry one.
    pub body_base64: Option<String>,
    pub client_ip: Option<String>,
    /// The host's route decision; `None` when the host has no manifest.
    pub route: Option<HostRoute>,
}

impl GuestRequest {
    /// The descriptor as a JSON literal, ready to splice into a script.
    ///
    /// # Errors
    ///
    /// Fails only if serialisation fails, which for this plain struct it does not.
    pub fn to_json(&self) -> Result<String, rari_error::RariError> {
        serde_json::to_string(self).map_err(|err| {
            rari_error::RariError::serialization(format!("guest request: {err}"))
        })
    }
}

/// The default [`GuestRenderer::page_pathname`]: the path without a trailing
/// slash (`/about/` and `/about` are the same page).
#[must_use]
pub fn default_page_pathname(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() { "/".to_string() } else { trimmed.to_string() }
}

fn param_value_json(value: &ParamValue) -> serde_json::Value {
    match value {
        ParamValue::Single(value) => serde_json::Value::String(value.clone()),
        ParamValue::Multiple(values) => serde_json::Value::Array(
            values.iter().cloned().map(serde_json::Value::String).collect(),
        ),
    }
}

/// Resolve the host's route decision for a page pathname: the matched route,
/// the app's not-found page when nothing matched, or `None` when the host has
/// no page to offer (a host-level 404).
#[must_use]
pub fn host_route_for(router: &AppRouter, pathname: &str) -> Option<HostRoute> {
    if let Ok(matched) = router.match_route(pathname) {
        let params = matched
            .params
            .iter()
            .map(|(name, value)| (name.clone(), param_value_json(value)))
            .collect();
        return Some(HostRoute {
            path: matched.route.path,
            params,
            layouts: matched.layouts.into_iter().map(|layout| layout.file_path).collect(),
            not_found: false,
        });
    }
    let not_found = router.create_not_found_match(pathname)?;
    Some(HostRoute {
        path: not_found.pathname,
        params: serde_json::Map::new(),
        layouts: not_found.layouts.into_iter().map(|layout| layout.file_path).collect(),
        not_found: true,
    })
}

/// The host answered without the guest: nothing in the manifest matches and
/// the app ships no not-found page.
fn host_not_found() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    headers.insert(ROUTE_HEADER, HeaderValue::from_static("miss"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response_with_headers(StatusCode::NOT_FOUND, &headers, Body::from("Not Found"))
}

/// `Cache-Control` directives in a guest response that forbid the host from
/// storing it.
fn cache_control_forbids_store(value: Option<&HeaderValue>) -> bool {
    let Some(value) = value.and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let lower = value.to_ascii_lowercase();
    lower.contains("no-store") || lower.contains("private") || lower.contains("no-cache")
}

fn stall_timeout() -> Duration {
    Duration::from_millis(
        env::var("RARI_STREAMING_STALL_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(60_000),
    )
}

fn query_params(query: Option<&str>) -> FxHashMap<String, String> {
    query.map(|q| form_urlencoded::parse(q.as_bytes()).into_owned().collect()).unwrap_or_default()
}

fn request_origin(state: &ServerState, headers: &HeaderMap) -> String {
    if let Some(origin) = state.config.server.origin.as_deref() {
        return origin.to_string();
    }
    let host = headers.get(HOST).and_then(|h| h.to_str().ok()).unwrap_or("localhost");
    format!("http://{host}")
}

fn frame_headers(frame: &[(String, String)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in frame {
        if HOP_BY_HOP_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
            continue;
        }
        if let (Ok(name), Ok(value)) =
            (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value))
        {
            headers.append(name, value);
        }
    }
    headers
}

/// Serve a page from the static fast tier: an `Arc` clone of a prebuilt,
/// per-encoding compressed response, no deserialisation. Same tier the React
/// SSR path answers cookie-less GETs from.
fn fast_hit_response(prebuilt: &PrebuiltResponse, request_headers: &HeaderMap) -> Response {
    let vary = static_html_vary_header(None);
    if request_headers.get(IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(&prebuilt.etag) {
        let mut headers = HeaderMap::new();
        if let Ok(value) = HeaderValue::from_str(&prebuilt.etag) {
            headers.insert(ETAG, value);
        }
        if let Ok(value) = HeaderValue::from_str(&vary) {
            headers.insert(VARY, value);
        }
        return response_with_headers(StatusCode::NOT_MODIFIED, &headers, Body::empty());
    }

    let accept_encoding = request_headers.get(ACCEPT_ENCODING).and_then(|v| v.to_str().ok());
    let (body, encoding) =
        prebuilt.body_for(CompressionEncoding::from_accept_encoding(accept_encoding));

    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(&prebuilt.content_type) {
        headers.insert(CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str(&prebuilt.cache_control) {
        headers.insert(CACHE_CONTROL, value);
    }
    if let Ok(value) = HeaderValue::from_str(&prebuilt.etag) {
        headers.insert(ETAG, value);
    }
    if let Ok(value) = HeaderValue::from_str(&vary) {
        headers.insert(VARY, value);
    }
    if let Some(value) = encoding {
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static(value));
    }
    if let Some(label) = prebuilt.route.as_deref().and_then(|r| HeaderValue::from_str(r).ok()) {
        headers.insert(ROUTE_HEADER, label);
    }
    headers.insert("x-cache", HeaderValue::from_static("HIT"));
    let status = if prebuilt.is_not_found { StatusCode::NOT_FOUND } else { StatusCode::OK };
    response_with_headers(status, &headers, Body::from(body))
}

/// Serve a stored page: `304` on a matching `If-None-Match`, otherwise the
/// precompressed variant matching `Accept-Encoding` (identity if none).
fn cached_hit_response(cached: &CachedResponse, request_headers: &HeaderMap) -> Response {
    let vary = merge_vary_with_accept(cached.headers.get(VARY));
    let etag = cached.metadata.etag.as_deref();

    if let (Some(etag), Some(client_etag)) =
        (etag, request_headers.get(IF_NONE_MATCH).and_then(|v| v.to_str().ok()))
        && etag == client_etag
    {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        if let Ok(value) = HeaderValue::from_str(etag) {
            response.headers_mut().insert(ETAG, value);
        }
        if let Ok(value) = HeaderValue::from_str(&vary) {
            response.headers_mut().insert(VARY, value);
        }
        return response;
    }

    let accept_encoding = request_headers.get(ACCEPT_ENCODING).and_then(|v| v.to_str().ok());
    let encoding = CompressionEncoding::from_accept_encoding(accept_encoding);
    let (body, encoding) = match cached.get_compressed(&encoding) {
        Some(bytes) => (bytes.clone(), encoding),
        None => (cached.body.clone(), CompressionEncoding::Identity),
    };

    let mut headers = HeaderMap::new();
    for (name, value) in &cached.headers {
        if !matches!(*name, VARY | CONTENT_ENCODING | CONTENT_LENGTH | ETAG) {
            headers.append(name, value.clone());
        }
    }
    if let Ok(value) = HeaderValue::from_str(&vary) {
        headers.insert(VARY, value);
    }
    if let Some(value) = encoding.as_header_value() {
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static(value));
    }
    if let Some(value) = etag.and_then(|e| HeaderValue::from_str(e).ok()) {
        headers.insert(ETAG, value);
    }
    headers.insert("x-cache", HeaderValue::from_static("HIT"));
    response_with_headers(StatusCode::OK, &headers, Body::from(body))
}

fn response_with_headers(status: StatusCode, headers: &HeaderMap, body: Body) -> Response {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers.clone();
    response
}

/// Serve a file from the guest's public directory for extension-bearing paths
/// (favicons, copied `public/` files). `None` when the path is not a file
/// there, so the request falls through to routing (a framework may own
/// routes such as `/sitemap.xml`).
async fn try_serve_public_file(
    guest: &dyn GuestRenderer,
    state: &ServerState,
    path: &str,
) -> Option<Response> {
    let dir = guest.public_dir()?;
    let relative = path.trim_start_matches('/');
    if relative.is_empty() || !relative.rsplit('/').next().is_some_and(|name| name.contains('.')) {
        return None;
    }
    serve_mounted_file(dir, relative, false, &state.config.caching.static_files).await
}

/// The cache side of a request, computed once up front: whether the request
/// may be answered from the host's caches and under which keys.
struct CacheLookup {
    is_get: bool,
    cacheable_request: bool,
    use_fast_tier: bool,
    cache_key: String,
    fast_key: String,
}

impl CacheLookup {
    fn new(state: &ServerState, parts: &Parts, path: &str) -> Self {
        // The key partitions on the same cookie rules as the RSC path so a
        // cookie-bearing request never reads another visitor's page.
        let is_get = parts.method == Method::GET;
        let cookie_header = request_cookie_header(&parts.headers);
        let query = query_params(parts.uri.query());
        let query_for_cache = route_query_params_for_cache(&query);
        let cacheable_request =
            is_get && cookie_header.is_none() && state.response_cache.config.enabled;
        Self {
            is_get,
            cacheable_request,
            // Only cookie-independent renders are stored in the fast tier.
            use_fast_tier: cacheable_request && can_use_static_fast_cache(cookie_header),
            cache_key: response_cache_key(
                path,
                query_for_cache.as_ref(),
                Some("guest"),
                cookie_header,
            ),
            fast_key: ResponseCache::generate_static_fast_cache_key(
                path,
                query_for_cache.as_ref(),
                None,
            ),
        }
    }

    /// Answer from the static fast tier (an `Arc` clone of a prebuilt,
    /// precompressed response) or the response cache, if the page is there.
    async fn hit(&self, state: &ServerState, request_headers: &HeaderMap) -> Option<Response> {
        if self.use_fast_tier && let Some(prebuilt) = state.static_fast_cache.get(&self.fast_key) {
            return Some(fast_hit_response(&prebuilt, request_headers));
        }
        if self.cacheable_request && let Some(cached) = state.response_cache.get(&self.cache_key).await {
            return Some(cached_hit_response(&cached, request_headers));
        }
        None
    }
}

/// Handle a page request end to end for `guest`.
///
/// 1. public files from the guest's build output,
/// 2. for anonymous GETs, the static fast tier and then the response cache,
/// 3. the host's route decision (manifest match, not-found page, or a host 404
///    unless the guest owns the path),
/// 4. the guest renders on a pool slot; cacheable pages are buffered and
///    stored under the TTL from the page's own `Cache-Control` (falling back
///    to the route config), everything else streams as produced.
///
/// Steps 1 and 2 are the hot path and stay in this small future; the render
/// future is built (and boxed) only on a miss, so a cached hit never pays for
/// the state machine of a full render.
pub async fn handle(
    guest: &dyn GuestRenderer,
    state: &ServerState,
    req: Request<Body>,
    client_addr: Option<SocketAddr>,
) -> Response {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path().to_string();

    if let Some(response) = try_serve_public_file(guest, state, &path).await {
        return response;
    }

    let lookup = CacheLookup::new(state, &parts, &path);
    if let Some(response) = lookup.hit(state, &parts.headers).await {
        return response;
    }

    Box::pin(render(guest, state, parts, body, path, lookup, client_addr)).await
}

/// Route, render and respond on a cache miss; see [`handle`].
#[expect(clippy::too_many_lines)]
async fn render(
    guest: &dyn GuestRenderer,
    state: &ServerState,
    parts: Parts,
    body: Body,
    path: String,
    lookup: CacheLookup,
    client_addr: Option<SocketAddr>,
) -> Response {
    // Host-first routing: rari decides what the URL is before the guest runs.
    // No manifest at all means the guest decides everything.
    let host_route = match state.app_router.as_ref() {
        None => None,
        Some(router) => match host_route_for(router, &guest.page_pathname(&path)) {
            Some(route) => Some(route),
            None if guest.owns_unmatched_path(&path) => None,
            None => return host_not_found(),
        },
    };
    let route_label = host_route.as_ref().map_or_else(
        || HeaderValue::from_static("miss"),
        |route| HeaderValue::from_str(&route.path).unwrap_or(HeaderValue::from_static("miss")),
    );

    let path_and_query = parts.uri.path_and_query().map_or("/", |pq| pq.as_str());
    let url = format!("{}{}", request_origin(state, &parts.headers), path_and_query);

    let headers: Vec<(String, String)> = parts
        .headers
        .iter()
        .filter_map(|(name, value)| {
            value.to_str().ok().map(|v| (name.as_str().to_string(), v.to_string()))
        })
        .collect();

    let body_base64 = if lookup.is_get || parts.method == Method::HEAD {
        None
    } else {
        match to_bytes(body, MAX_REQUEST_BODY_BYTES).await {
            Ok(bytes) if bytes.is_empty() => None,
            Ok(bytes) => Some(BASE64_STANDARD.encode(&bytes)),
            Err(err) => {
                tracing::warn!("Rejected request body for {path}: {err}");
                return StatusCode::PAYLOAD_TOO_LARGE.into_response();
            }
        }
    };

    let request = GuestRequest {
        stream_id: next_stream_id(),
        url,
        method: parts.method.as_str().to_string(),
        headers,
        body_base64,
        client_ip: client_addr.map(|addr| addr.ip().to_string()),
        route: host_route,
    };

    let framework = guest.framework();
    let mut stream = match guest.render(&request).await {
        Ok(stream) => stream,
        Err(err) => {
            tracing::error!("{framework} render failed for {path}: {err}");
            return error_response::status(&err).into_response();
        }
    };

    let status = StatusCode::from_u16(stream.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut headers = frame_headers(&stream.headers);
    headers.insert(ROUTE_HEADER, route_label.clone());
    let is_html = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/html"));

    let store = lookup.cacheable_request
        && status == StatusCode::OK
        && is_html
        && !headers.contains_key(SET_COOKIE)
        && !cache_control_forbids_store(headers.get(CACHE_CONTROL));

    if store {
        // Buffer so the page can be stored; repeat hits then skip V8 entirely.
        let mut full = Vec::new();
        let mut failed = false;
        while let Some(chunk) = stream.body.recv().await {
            match chunk {
                Ok(bytes) => full.extend_from_slice(&bytes),
                Err(err) => {
                    tracing::error!("{framework} render failed mid-stream for {path}: {err}");
                    failed = true;
                    break;
                }
            }
        }
        let body = Bytes::from(full);

        if !failed {
            // The page's own Cache-Control sets the host's TTL; the route config
            // is the fallback. `max-age=0` means "don't keep it", not "keep it
            // for the default lifetime".
            let cache_control = headers
                .get(CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_else(|| state.config.get_cache_control_for_route(&path))
                .to_string();
            let policy = RouteCachePolicy::from_cache_control(&cache_control, &path);
            if policy.enabled && policy.ttl > 0 {
                let mut cache_headers = headers.clone();
                let cookie_header = request_cookie_header(&parts.headers);
                insert_response_cache_vary_header(&mut cache_headers, cookie_header, true);
                // Compress once at store time so hits serve precompressed bytes
                // instead of paying for compression per request (RSC parity).
                let (compressed_gzip, compressed_zstd, compressed_br) =
                    compress_all_encodings(body.clone()).await;
                let etag = ResponseCache::generate_etag(&body);

                if lookup.use_fast_tier {
                    let content_type = headers
                        .get(CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("text/html; charset=utf-8")
                        .to_string();
                    insert_static_fast_cache(
                        &state.static_fast_cache,
                        &lookup.fast_key,
                        Arc::new(PrebuiltResponse {
                            identity: body.clone(),
                            gzip: compressed_gzip.clone(),
                            br: compressed_br.clone(),
                            zstd: compressed_zstd.clone(),
                            etag: etag.clone(),
                            content_type,
                            cache_control,
                            is_not_found: false,
                            cached_at: Instant::now(),
                            route: route_label.to_str().ok().map(str::to_string),
                        }),
                        state.response_cache.config.max_entries,
                    );
                }

                state
                    .response_cache
                    .set(
                        lookup.cache_key,
                        CachedResponse {
                            body: body.clone(),
                            headers: cache_headers,
                            metadata: CacheMetadata {
                                cached_at: Instant::now(),
                                ttl: policy.ttl,
                                etag: Some(etag),
                                tags: policy.tags,
                            },
                            compressed_zstd,
                            compressed_br,
                            compressed_gzip,
                        },
                    )
                    .await;
            }
        }

        headers.insert("x-cache", HeaderValue::from_static("MISS"));
        let status = if failed { StatusCode::INTERNAL_SERVER_ERROR } else { status };
        return response_with_headers(status, &headers, Body::from(body));
    }

    // Live stream. `transfer-encoding: chunked` is the host's marker that a
    // response streams (compression is skipped for it, see the host's
    // `NotStreamingResponse` predicate); hyper still owns the framing.
    headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));
    let stall = stall_timeout();
    let body_stream = async_stream::stream! {
        loop {
            match time::timeout(stall, stream.body.recv()).await {
                Ok(Some(Ok(chunk))) => yield Ok::<Bytes, io::Error>(Bytes::from(chunk)),
                Ok(Some(Err(err))) => {
                    tracing::error!("{framework} stream error for {path}: {err}");
                    yield Err(io::Error::other(err.to_string()));
                    break;
                }
                Ok(None) => break,
                Err(_) => {
                    tracing::error!(
                        "{framework} stream stalled for {path}: no chunk within {} ms",
                        stall.as_millis()
                    );
                    yield Err(io::Error::other("guest stream timed out"));
                    break;
                }
            }
        }
    };
    response_with_headers(status, &headers, Body::from_stream(body_stream))
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::server::routing::app_router::AppRouteManifest;

    #[test]
    fn hop_by_hop_headers_are_dropped() {
        let headers = frame_headers(&[
            ("Content-Type".to_string(), "text/html".to_string()),
            ("Content-Length".to_string(), "12".to_string()),
            ("Transfer-Encoding".to_string(), "chunked".to_string()),
            ("Set-Cookie".to_string(), "a=1".to_string()),
            ("Set-Cookie".to_string(), "b=2".to_string()),
        ]);
        assert_eq!(headers.get(CONTENT_TYPE).unwrap(), "text/html");
        assert!(headers.get("content-length").is_none());
        assert!(headers.get("transfer-encoding").is_none());
        assert_eq!(headers.get_all(SET_COOKIE).iter().count(), 2);
    }

    #[test]
    fn private_or_no_store_responses_are_not_cached() {
        for value in ["private", "no-store", "public, max-age=0, no-cache"] {
            assert!(cache_control_forbids_store(Some(&HeaderValue::from_static(value))), "{value}");
        }
        assert!(!cache_control_forbids_store(Some(&HeaderValue::from_static(
            "public, max-age=60"
        ))));
        assert!(!cache_control_forbids_store(None));
    }

    #[tokio::test]
    async fn cache_hit_serves_the_variant_matching_accept_encoding() {
        let mut stored_headers = HeaderMap::new();
        stored_headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));
        let cached = CachedResponse {
            body: Bytes::from_static(b"identity"),
            headers: stored_headers,
            metadata: CacheMetadata {
                cached_at: Instant::now(),
                ttl: 60,
                etag: Some("W/\"abc\"".to_string()),
                tags: Vec::new(),
            },
            compressed_zstd: Some(Bytes::from_static(b"zstd-bytes")),
            compressed_br: Some(Bytes::from_static(b"br-bytes")),
            compressed_gzip: Some(Bytes::from_static(b"gzip-bytes")),
        };

        for (accept, expected_encoding, expected_body) in [
            ("gzip", Some("gzip"), &b"gzip-bytes"[..]),
            ("br", Some("br"), &b"br-bytes"[..]),
            ("zstd", Some("zstd"), &b"zstd-bytes"[..]),
            ("identity", None, &b"identity"[..]),
        ] {
            let mut request_headers = HeaderMap::new();
            request_headers.insert(ACCEPT_ENCODING, HeaderValue::from_static(accept));
            let response = cached_hit_response(&cached, &request_headers);
            assert_eq!(response.status(), StatusCode::OK, "{accept}");
            assert_eq!(
                response.headers().get(CONTENT_ENCODING).and_then(|v| v.to_str().ok()),
                expected_encoding,
                "{accept}"
            );
            assert_eq!(response.headers().get("x-cache").unwrap(), "HIT");
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert_eq!(&body[..], expected_body, "{accept}");
        }

        let mut request_headers = HeaderMap::new();
        request_headers.insert(IF_NONE_MATCH, HeaderValue::from_static("W/\"abc\""));
        assert_eq!(
            cached_hit_response(&cached, &request_headers).status(),
            StatusCode::NOT_MODIFIED
        );
    }

    #[test]
    fn trailing_slashes_map_to_the_page() {
        assert_eq!(default_page_pathname("/"), "/");
        assert_eq!(default_page_pathname(""), "/");
        assert_eq!(default_page_pathname("/about/"), "/about");
        assert_eq!(default_page_pathname("/blog/hello"), "/blog/hello");
    }

    #[test]
    fn host_routes_pages_params_layouts_and_not_found_pages() {
        let manifest: AppRouteManifest = serde_json::from_value(serde_json::json!({
            "routes": [
                { "path": "/", "filePath": "index.tsx", "segments": [], "params": [], "isDynamic": false },
                { "path": "/wide", "filePath": "wide/index@wide.tsx", "layout": "wide",
                  "segments": [{ "type": "static", "value": "wide" }], "params": [], "isDynamic": false },
                { "path": "/blog/[slug]", "filePath": "blog/[slug]/index.tsx",
                  "segments": [
                    { "type": "static", "value": "blog" },
                    { "type": "dynamic", "value": "[slug]", "param": "slug" }
                  ],
                  "params": ["slug"], "isDynamic": true }
            ],
            "layouts": [
                { "path": "/", "filePath": "layout.tsx" },
                { "path": "/", "filePath": "layout-wide.tsx", "name": "wide" }
            ],
            "loading": [], "errors": [], "templates": [], "apiRoutes": [], "ogImages": [], "appIcons": [],
            "notFound": [],
            "generated": "test"
        }))
        .unwrap();
        let router = AppRouter::new(manifest);

        let home = host_route_for(&router, "/").unwrap();
        assert_eq!(home.path, "/");
        assert_eq!(home.layouts, vec!["layout.tsx".to_string()]);
        assert!(!home.not_found);

        let wide = host_route_for(&router, "/wide").unwrap();
        assert_eq!(wide.layouts, vec!["layout-wide.tsx".to_string()]);

        let post = host_route_for(&router, "/blog/hello-world").unwrap();
        assert_eq!(post.path, "/blog/[slug]");
        assert_eq!(post.params.get("slug").and_then(|v| v.as_str()), Some("hello-world"));

        assert!(host_route_for(&router, "/nope").is_none(), "no 404 page: host answers");
    }

    #[test]
    fn query_string_parses_into_params() {
        let params = query_params(Some("a=1&b=two%20words&utm_source=x"));
        assert_eq!(params.get("a").map(String::as_str), Some("1"));
        assert_eq!(params.get("b").map(String::as_str), Some("two words"));
        assert!(!route_query_params_for_cache(&params).unwrap().contains_key("utm_source"));
    }

    #[test]
    fn guest_request_serialises_in_camel_case() {
        let request = GuestRequest {
            stream_id: "guest-1".to_string(),
            url: "http://localhost/".to_string(),
            method: "GET".to_string(),
            headers: vec![("accept".to_string(), "text/html".to_string())],
            body_base64: None,
            client_ip: Some("127.0.0.1".to_string()),
            route: None,
        };
        let json: serde_json::Value = serde_json::from_str(&request.to_json().unwrap()).unwrap();
        assert_eq!(json["streamId"], "guest-1");
        assert_eq!(json["clientIp"], "127.0.0.1");
        assert!(json["bodyBase64"].is_null());
        assert!(json["route"].is_null());
    }
}
