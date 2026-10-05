//! Qwik guest renderer.
//!
//! `@rari/qwik`'s build emits `dist/server/qwik-server-entry.mjs`: a
//! self-contained ESM bundle of the app, Qwik Router's `requestHandler`, and the
//! rari platform shim. Evaluating it installs `globalThis.__rariQwikHandle`, an
//! async function taking the host's request descriptor and speaking the
//! protocol in [`super::stream`]. Qwik owns loaders, actions, `server$`,
//! cookies, redirects and rendering; rari owns everything around the request.

use std::{
    env, io,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    body::{Body, Bytes, to_bytes},
    http::{
        HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode,
        header::{
            ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, ETAG,
            HOST, IF_NONE_MATCH, SET_COOKIE, VARY,
        },
    },
    response::{IntoResponse, Response},
};
use base64::prelude::*;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use tokio::{fs, time};
use url::form_urlencoded;

use super::{
    GuestRenderer, StaticMount, serve_mounted_file,
    stream::{next_stream_id, run_guest_script},
};
use crate::{
    async_trait::async_trait,
    runtime::{JsExecutionRuntime, factory::JsRuntimeInterface},
    server::{
        ServerState,
        cache::response::{CacheMetadata, CachedResponse, ResponseCache, RouteCachePolicy},
        compression::{CompressionEncoding, compress_all_encodings},
        config::Framework,
        error_response,
        host::utils::http::merge_vary_with_accept,
        routing::app::cache::{
            insert_response_cache_vary_header, request_cookie_header, response_cache_key,
            route_query_params_for_cache,
        },
    },
};

/// Server bundle the Qwik build writes, relative to the project root.
pub const SERVER_ENTRY: &str = "dist/server/qwik-server-entry.mjs";
/// Client output the Qwik build writes, relative to the project root.
pub const CLIENT_DIR: &str = "dist/client";
/// Global the server bundle installs.
const HANDLER_GLOBAL: &str = "__rariQwikHandle";
/// Request bodies above this are rejected before reaching V8 (form posts and
/// `server$` payloads are small; uploads belong on an API route).
const MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;
/// Headers that describe the transport, not the response; never copied from
/// the guest's frame onto the host response.
const HOP_BY_HOP_HEADERS: &[&str] =
    &["content-length", "transfer-encoding", "connection", "keep-alive"];

pub struct QwikGuest {
    runtime: Arc<JsExecutionRuntime>,
    /// `file://` URL the bundle is registered under in the module loader.
    entry_specifier: String,
    /// The loader keys registered modules by a component id derived from the
    /// specifier (for `dist/server/<file>` that is the file name); loading and
    /// evaluating goes through that id, as the API-route loader does.
    entry_component_id: String,
    entry_code: String,
    client_dir: PathBuf,
}

impl QwikGuest {
    /// Load the server bundle into every isolate of the pool.
    ///
    /// # Errors
    ///
    /// Fails when the bundle is missing or unreadable, or when evaluating it in
    /// the runtime fails (including when it does not install the handler).
    pub async fn load(
        runtime: Arc<JsExecutionRuntime>,
        project_root: &Path,
    ) -> Result<Self, RariError> {
        let entry = project_root.join(SERVER_ENTRY);
        let canonical = fs::canonicalize(&entry).await.map_err(|err| {
            RariError::configuration(format!(
                "Qwik server entry {} not found ({err}). Build the app with @rari/qwik first.",
                entry.display()
            ))
        })?;
        let entry_code = fs::read_to_string(&canonical).await.map_err(|err| {
            RariError::io(format!(
                "Failed to read Qwik server entry {}: {err}",
                canonical.display()
            ))
        })?;
        let entry_specifier = url::Url::from_file_path(&canonical)
            .map_err(|()| {
                RariError::configuration(format!(
                    "Failed to create a file URL for {}",
                    canonical.display()
                ))
            })?
            .to_string();

        let entry_component_id =
            canonical.file_name().and_then(|name| name.to_str()).map(str::to_string).ok_or_else(
                || {
                    RariError::configuration(format!(
                        "Qwik server entry has no file name: {}",
                        canonical.display()
                    ))
                },
            )?;

        runtime.add_module_to_loader(&entry_specifier, entry_code.clone()).await?;
        runtime.load_and_evaluate_module(&entry_component_id).await?;
        runtime
            .broadcast_script(
                "qwik_guest_check",
                &format!(
                    "if (typeof globalThis.{HANDLER_GLOBAL} !== 'function') \
                     throw new Error('Qwik server entry did not install globalThis.{HANDLER_GLOBAL}')"
                ),
            )
            .await?;

        Ok(Self {
            runtime,
            entry_specifier,
            entry_component_id,
            entry_code,
            client_dir: project_root.join(CLIENT_DIR),
        })
    }

    /// Serve a file from the client output directory for extension-bearing
    /// paths (favicons, copied `public/` files). Returns `None` when the path
    /// is not a file there, so the request falls through to Qwik (which may own
    /// routes such as `/sitemap.xml`).
    async fn try_serve_client_file(&self, state: &ServerState, path: &str) -> Option<Response> {
        let relative = path.trim_start_matches('/');
        if relative.is_empty()
            || !relative.rsplit('/').next().is_some_and(|name| name.contains('.'))
        {
            return None;
        }
        serve_mounted_file(&self.client_dir, relative, false, &state.config.caching.static_files)
            .await
    }
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

#[async_trait]
impl GuestRenderer for QwikGuest {
    fn framework(&self) -> Framework {
        Framework::Qwik
    }

    fn static_mounts(&self) -> Vec<StaticMount> {
        vec![
            StaticMount::new("/build", self.client_dir.join("build"), true),
            StaticMount::new("/assets", self.client_dir.join("assets"), true),
        ]
    }

    async fn resync_slot(&self, runtime: &Arc<dyn JsRuntimeInterface>) -> Result<(), RariError> {
        runtime.add_module_to_loader(&self.entry_specifier, self.entry_code.clone()).await?;
        let module_id = runtime.load_es_module(&self.entry_component_id).await?;
        runtime.evaluate_module(module_id).await?;
        Ok(())
    }

    #[expect(clippy::too_many_lines)]
    async fn handle(
        &self,
        state: &ServerState,
        req: Request<Body>,
        client_addr: Option<SocketAddr>,
    ) -> Response {
        let (parts, body) = req.into_parts();
        let path = parts.uri.path().to_string();

        if let Some(response) = self.try_serve_client_file(state, &path).await {
            return response;
        }

        let is_get = parts.method == Method::GET;
        let path_and_query = parts.uri.path_and_query().map_or("/", |pq| pq.as_str());
        let url = format!("{}{}", request_origin(state, &parts.headers), path_and_query);

        let headers: Vec<(String, String)> = parts
            .headers
            .iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|v| (name.as_str().to_string(), v.to_string()))
            })
            .collect();

        let body_base64 = if is_get || parts.method == Method::HEAD {
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

        // Host-owned response cache for anonymous page GETs. The key partitions
        // on the same cookie rules as the RSC path so a cookie-bearing request
        // never reads another visitor's page.
        let cookie_header = request_cookie_header(&parts.headers);
        let query = query_params(parts.uri.query());
        let query_for_cache = route_query_params_for_cache(&query);
        let cache_enabled = state.response_cache.config.enabled;
        let cacheable_request = is_get && cookie_header.is_none() && cache_enabled;
        let cache_key =
            response_cache_key(&path, query_for_cache.as_ref(), Some("guest"), cookie_header);

        if cacheable_request && let Some(cached) = state.response_cache.get(&cache_key).await {
            return cached_hit_response(&cached, &parts.headers);
        }

        let stream_id = next_stream_id();
        let req_init = serde_json::json!({
            "streamId": stream_id,
            "url": url,
            "method": parts.method.as_str(),
            "headers": headers,
            "bodyBase64": body_base64,
            "clientIp": client_addr.map(|addr| addr.ip().to_string()),
        });
        let script = format!("globalThis.{HANDLER_GLOBAL}({req_init})");

        let mut stream =
            match run_guest_script(&self.runtime, stream_id, "qwik_render", script).await {
                Ok(stream) => stream,
                Err(err) => {
                    tracing::error!("Qwik render failed for {path}: {err}");
                    return error_response::status(&err).into_response();
                }
            };

        let status =
            StatusCode::from_u16(stream.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut headers = frame_headers(&stream.headers);
        let is_html = headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("text/html"));

        let store = cacheable_request
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
                        tracing::error!("Qwik render failed mid-stream for {path}: {err}");
                        failed = true;
                        break;
                    }
                }
            }
            let body = Bytes::from(full);

            if !failed {
                let policy = RouteCachePolicy::from_cache_control(
                    state.config.get_cache_control_for_route(&path),
                    &path,
                );
                if policy.enabled {
                    let mut cache_headers = headers.clone();
                    insert_response_cache_vary_header(&mut cache_headers, cookie_header, true);
                    // Compress once at store time so hits serve precompressed bytes
                    // instead of paying for compression per request (RSC parity).
                    let (compressed_gzip, compressed_zstd, compressed_br) =
                        compress_all_encodings(body.clone()).await;
                    state
                        .response_cache
                        .set(
                            cache_key,
                            CachedResponse {
                                body: body.clone(),
                                headers: cache_headers,
                                metadata: CacheMetadata {
                                    cached_at: Instant::now(),
                                    ttl: policy.ttl,
                                    etag: Some(ResponseCache::generate_etag(&body)),
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
                        tracing::error!("Qwik stream error for {path}: {err}");
                        yield Err(io::Error::other(err.to_string()));
                        break;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        tracing::error!(
                            "Qwik stream stalled for {path}: no chunk within {} ms",
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
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;

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
    fn query_string_parses_into_params() {
        let params = query_params(Some("a=1&b=two%20words&utm_source=x"));
        assert_eq!(params.get("a").map(String::as_str), Some("1"));
        assert_eq!(params.get("b").map(String::as_str), Some("two words"));
        assert!(!route_query_params_for_cache(&params).unwrap().contains_key("utm_source"));
    }
}
