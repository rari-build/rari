use std::{
    mem,
    num::NonZeroUsize,
    string::ToString,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use axum::http::HeaderMap;
use bytes::Bytes;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use rari_error::RariError;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

use crate::server::{
    core::utils::{client::get_http_client, http},
    middleware::request::X_RARI_CSP_NONCE,
};

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PendingCookie {
    pub name: String,
    pub value: String,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub expires: Option<String>,
    pub max_age: Option<i64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
    pub priority: Option<String>,
    pub partitioned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PendingCookieKey {
    pub name: String,
    pub path: Option<String>,
    pub domain: Option<String>,
}

impl PendingCookieKey {
    pub fn new(name: &str, path: Option<&str>, domain: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            path: path.map(ToString::to_string),
            domain: domain.map(ToString::to_string),
        }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CachedFetchResult {
    pub body: Bytes,
    pub status: u16,
    pub headers: HeaderMap,
    pub cached_at: Instant,
    pub was_cached: bool,
    pub tags: Vec<String>,
}

impl CachedFetchResult {
    /// 5xx responses are never cached: a transient backend error cached here
    /// is replayed to every render for the TTL (default 60s), turning a blip
    /// into a minute of failures. 4xx stays cacheable, a missing article is
    /// stable data worth deduplicating.
    fn is_cacheable(&self) -> bool {
        self.status < 500
    }
}

type InFlightFetches =
    Arc<DashMap<String, Arc<TokioMutex<Option<Result<CachedFetchResult, RariError>>>>>>;

const DEFAULT_FETCH_CACHE_ENTRIES: usize = 1000;

pub struct GlobalFetchCache {
    state: Mutex<FetchLruState>,
    max_bytes: usize,
}

struct FetchLruState {
    lru: LruCache<String, CachedFetchResult>,
    bytes: usize,
}

static GLOBAL_FETCH_CACHE: OnceLock<Arc<GlobalFetchCache>> = OnceLock::new();

static GLOBAL_IN_FLIGHT_FETCHES: OnceLock<InFlightFetches> = OnceLock::new();

fn fetch_cache_capacity(max_entries: usize) -> NonZeroUsize {
    #[expect(
        clippy::expect_used,
        reason = "max_entries.max(1) is always >= 1, so NonZeroUsize::new never fails"
    )]
    NonZeroUsize::new(max_entries.max(1)).expect("fetch cache capacity is always non-zero")
}

/// Install the process-wide fetch LRU from `cache.layers.fetch`.
/// Must run once at server boot before any `RequestContext` is created.
///
/// # Errors
/// Returns an error if the fetch cache was already initialized, so configured
/// `max_entries` / `max_bytes` are never silently ignored.
pub fn init_global_fetch_cache(max_entries: usize, max_bytes: usize) -> Result<(), RariError> {
    let cache = Arc::new(GlobalFetchCache {
        state: Mutex::new(FetchLruState {
            lru: LruCache::new(fetch_cache_capacity(max_entries)),
            bytes: 0,
        }),
        max_bytes,
    });
    GLOBAL_FETCH_CACHE.set(cache).map_err(|_| {
        RariError::configuration(
            "fetch cache already initialized before server startup; configured cache.layers.fetch limits were not applied"
                .to_string(),
        )
    })
}

fn global_fetch_cache() -> Arc<GlobalFetchCache> {
    Arc::clone(GLOBAL_FETCH_CACHE.get_or_init(|| {
        Arc::new(GlobalFetchCache {
            state: Mutex::new(FetchLruState {
                lru: LruCache::new(fetch_cache_capacity(DEFAULT_FETCH_CACHE_ENTRIES)),
                bytes: 0,
            }),
            max_bytes: 0,
        })
    }))
}

fn global_in_flight_fetches() -> InFlightFetches {
    Arc::clone(GLOBAL_IN_FLIGHT_FETCHES.get_or_init(|| Arc::new(DashMap::new())))
}

fn put_fetch_cache_entry(cache: &GlobalFetchCache, key: String, result: CachedFetchResult) {
    let max_bytes = cache.max_bytes;
    let new_len = result.body.len();
    if max_bytes > 0 && new_len > max_bytes {
        return;
    }

    let mut state = cache.state.lock();
    if let Some((_, evicted)) = state.lru.push(key, result) {
        state.bytes = state.bytes.saturating_sub(evicted.body.len());
    }
    state.bytes = state.bytes.saturating_add(new_len);

    if max_bytes == 0 {
        return;
    }

    while state.bytes > max_bytes {
        let Some((_, evicted)) = state.lru.pop_lru() else {
            break;
        };
        state.bytes = state.bytes.saturating_sub(evicted.body.len());
    }
}

struct FetchCleanupGuard {
    in_flight_fetches: InFlightFetches,
    cache_key: String,
    armed: bool,
}

impl FetchCleanupGuard {
    fn new(in_flight_fetches: InFlightFetches, cache_key: String) -> Self {
        Self { in_flight_fetches, cache_key, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for FetchCleanupGuard {
    fn drop(&mut self) {
        if self.armed {
            self.in_flight_fetches.remove(&self.cache_key);
        }
    }
}

pub struct RequestContext {
    fetch_cache: Arc<GlobalFetchCache>,
    in_flight_fetches: InFlightFetches,
    request_id: String,
    start_time: Instant,
    route_path: String,
    pub cookie_header: Option<String>,
    pub request_headers: FxHashMap<String, String>,
    pub skip_layout_html_cache: bool,
    pub pending_cookies: Arc<DashMap<PendingCookieKey, PendingCookie>>,
    pub function_cache: Arc<DashMap<String, Value>>,
    pub action_form_state: Option<Value>,
    pub csp_nonce: Option<String>,
}

impl RequestContext {
    pub fn new(route_path: String) -> Self {
        Self {
            fetch_cache: global_fetch_cache(),
            in_flight_fetches: global_in_flight_fetches(),
            request_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
            route_path,
            cookie_header: None,
            request_headers: FxHashMap::default(),
            skip_layout_html_cache: false,
            pending_cookies: Arc::new(DashMap::new()),
            function_cache: Arc::new(DashMap::new()),
            action_form_state: None,
            csp_nonce: None,
        }
    }

    #[must_use]
    pub fn with_cookies(mut self, cookie_header: Option<String>) -> Self {
        self.cookie_header = cookie_header;
        self
    }

    #[must_use]
    pub fn with_http_headers(mut self, mut headers: FxHashMap<String, String>) -> Self {
        self.cookie_header = headers.get("cookie").cloned();
        self.csp_nonce = headers.remove(X_RARI_CSP_NONCE);
        self.request_headers = http::filter_headers_for_components(headers);
        self
    }

    #[must_use]
    pub fn without_layout_html_cache(mut self) -> Self {
        self.skip_layout_html_cache = true;
        self
    }

    #[must_use]
    pub fn with_action_form_state(mut self, form_state: Option<Value>) -> Self {
        self.action_form_state = form_state;
        self
    }

    #[must_use]
    pub fn with_csp_nonce(mut self, nonce: Option<String>) -> Self {
        self.csp_nonce = nonce;
        self
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn route_path(&self) -> &str {
        &self.route_path
    }

    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn fetch_cache(&self) -> &Arc<GlobalFetchCache> {
        &self.fetch_cache
    }

    fn merge_and_sort_tags(
        existing: impl IntoIterator<Item = String>,
        extra: impl IntoIterator<Item = String>,
    ) -> Vec<String> {
        let mut tag_set: FxHashSet<String> = existing.into_iter().collect();
        tag_set.extend(extra);
        let mut merged: Vec<String> = tag_set.into_iter().collect();
        merged.sort();
        merged
    }

    fn generate_cache_key(url: &str, options: &FxHashMap<String, String>) -> String {
        let cache_relevant_options: FxHashMap<_, _> = options
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "cacheTTLMs" | "timeout" | "tags"))
            .collect();

        if cache_relevant_options.is_empty() {
            url.to_string()
        } else {
            let mut sorted_opts: Vec<_> = cache_relevant_options.iter().collect();
            sorted_opts.sort_by_key(|(k, _)| *k);

            let opts_str = sorted_opts
                .iter()
                .map(|(k, v)| {
                    if k.as_str() == "headers" && v.len() > 100 {
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(v.as_bytes());
                        let hash = hasher.finalize();
                        format!("{}=h:{}", k, hex::encode(&hash[..8]))
                    } else {
                        format!("{k}={v}")
                    }
                })
                .collect::<Vec<_>>()
                .join("&");

            format!("{url}?{opts_str}")
        }
    }

    #[expect(clippy::missing_errors_doc)]
    pub async fn fetch_with_cache(
        &self,
        url: &str,
        options: FxHashMap<String, String>,
    ) -> Result<CachedFetchResult, RariError> {
        let cache_key = Self::generate_cache_key(url, &options);

        let tags: Vec<String> =
            options.get("tags").and_then(|t| serde_json::from_str(t).ok()).unwrap_or_default();

        {
            let mut state = self.fetch_cache.state.lock();
            if let Some(cached) = state.lru.get(&cache_key) {
                let ttl_ms =
                    options.get("cacheTTLMs").and_then(|t| t.parse::<u64>().ok()).unwrap_or(60_000);

                let elapsed_ms = cached.cached_at.elapsed().as_millis();

                if elapsed_ms < u128::from(ttl_ms) {
                    let mut result = cached.clone();
                    result.was_cached = true;
                    result.tags = Self::merge_and_sort_tags(result.tags, tags);

                    state.lru.put(cache_key.clone(), result.clone());

                    return Ok(result);
                }
                if let Some(evicted) = state.lru.pop(&cache_key) {
                    state.bytes = state.bytes.saturating_sub(evicted.body.len());
                }
            }
        }

        let fetch_lock = {
            let entry = self.in_flight_fetches.entry(cache_key.clone());
            entry.or_insert_with(|| Arc::new(TokioMutex::new(None))).clone()
        };

        let url = url.to_string();
        let fetch_cache = Arc::clone(&self.fetch_cache);
        let in_flight_fetches = Arc::clone(&self.in_flight_fetches);
        let cache_key_for_task = cache_key.clone();

        tokio::spawn(async move {
            let mut cleanup =
                FetchCleanupGuard::new(Arc::clone(&in_flight_fetches), cache_key_for_task.clone());
            let mut guard = fetch_lock.lock().await;

            if let Some(result) = guard.as_ref() {
                cleanup.disarm();
                let mut result = result.clone();
                if let Ok(ref mut cached) = result {
                    cached.tags = Self::merge_and_sort_tags(mem::take(&mut cached.tags), tags);
                    *guard = Some(Ok(cached.clone()));
                    if cached.is_cacheable() {
                        put_fetch_cache_entry(&fetch_cache, cache_key_for_task, cached.clone());
                    }
                }
                return result;
            }

            let mut fetch_result = Self::perform_fetch_standalone(&url, &options).await;

            if let Ok(ref mut result) = fetch_result {
                result.tags = Self::merge_and_sort_tags(mem::take(&mut result.tags), tags);
            }

            *guard = Some(fetch_result.clone());

            if let Ok(ref cached_result) = fetch_result
                && cached_result.is_cacheable()
            {
                put_fetch_cache_entry(&fetch_cache, cache_key_for_task, cached_result.clone());
            }

            drop(guard);
            fetch_result
        })
        .await
        .map_err(|e| RariError::internal(format!("fetch singleflight join failed: {e}")))?
    }

    async fn perform_fetch_standalone(
        url: &str,
        options: &FxHashMap<String, String>,
    ) -> Result<CachedFetchResult, RariError> {
        let client = get_http_client()?;
        let mut request = client.get(url);

        if let Some(headers_str) = options.get("headers")
            && let Ok(pairs) = serde_json::from_str::<Vec<(String, String)>>(headers_str)
        {
            for (key, value) in pairs {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        let timeout = options.get("timeout").and_then(|t| t.parse::<u64>().ok()).unwrap_or(5000);

        request = request.timeout(Duration::from_millis(timeout));

        let response = request
            .send()
            .await
            .map_err(|e| RariError::network(format!("Fetch failed for {url}: {e}")))?;

        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response
            .bytes()
            .await
            .map_err(|e| RariError::network(format!("Failed to read response body: {e}")))?;

        Ok(CachedFetchResult {
            body,
            status,
            headers,
            cached_at: Instant::now(),
            was_cached: false,
            tags: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_context_creation() {
        let ctx = RequestContext::new("/test".to_string());

        assert_eq!(ctx.route_path(), "/test");
        assert_ne!(ctx.request_id(), "");
        assert!(ctx.elapsed().as_millis() < 100);
    }

    #[test]
    fn test_request_context_fetch_cache() {
        let ctx = RequestContext::new("/test".to_string());
        let cache = ctx.fetch_cache();

        let initial_len = cache.state.lock().lru.len();

        let result = CachedFetchResult {
            body: Bytes::from("test"),
            status: 200,
            headers: HeaderMap::new(),
            cached_at: Instant::now(),
            was_cached: false,
            tags: Vec::new(),
        };

        let test_key = format!("https://test-{}.example.com", uuid::Uuid::new_v4());
        put_fetch_cache_entry(cache, test_key.clone(), result);

        let new_len = cache.state.lock().lru.len();
        assert!(
            new_len == initial_len + 1 || new_len == initial_len,
            "Cache should grow by 1 or stay at capacity"
        );

        assert!(cache.state.lock().lru.contains(&test_key));
    }

    #[test]
    fn test_fetch_cache_respects_max_bytes() {
        let cache = GlobalFetchCache {
            state: Mutex::new(FetchLruState {
                lru: LruCache::new(fetch_cache_capacity(10)),
                bytes: 0,
            }),
            max_bytes: 8,
        };

        put_fetch_cache_entry(
            &cache,
            "a".into(),
            CachedFetchResult {
                body: Bytes::from("12345"),
                status: 200,
                headers: HeaderMap::new(),
                cached_at: Instant::now(),
                was_cached: false,
                tags: Vec::new(),
            },
        );
        put_fetch_cache_entry(
            &cache,
            "b".into(),
            CachedFetchResult {
                body: Bytes::from("12345"),
                status: 200,
                headers: HeaderMap::new(),
                cached_at: Instant::now(),
                was_cached: false,
                tags: Vec::new(),
            },
        );

        let guard = cache.state.lock();
        assert_eq!(guard.lru.len(), 1);
        assert!(guard.lru.contains("b"));
        assert!(!guard.lru.contains("a"));
        assert_eq!(guard.bytes, 5);
    }

    #[test]
    fn test_fetch_cache_rejects_oversized_entry() {
        let cache = GlobalFetchCache {
            state: Mutex::new(FetchLruState {
                lru: LruCache::new(fetch_cache_capacity(10)),
                bytes: 0,
            }),
            max_bytes: 4,
        };

        put_fetch_cache_entry(
            &cache,
            "big".into(),
            CachedFetchResult {
                body: Bytes::from("12345"),
                status: 200,
                headers: HeaderMap::new(),
                cached_at: Instant::now(),
                was_cached: false,
                tags: Vec::new(),
            },
        );

        assert!(cache.state.lock().lru.is_empty());
        assert_eq!(cache.state.lock().bytes, 0);
    }

    #[test]
    fn test_fetch_cache_byte_accounting_on_capacity_eviction() {
        let cache = GlobalFetchCache {
            state: Mutex::new(FetchLruState {
                lru: LruCache::new(fetch_cache_capacity(2)),
                bytes: 0,
            }),
            max_bytes: 10,
        };

        let entry = |body: &'static str| CachedFetchResult {
            body: Bytes::from(body),
            status: 200,
            headers: HeaderMap::new(),
            cached_at: Instant::now(),
            was_cached: false,
            tags: Vec::new(),
        };

        put_fetch_cache_entry(&cache, "a".into(), entry("12345"));
        put_fetch_cache_entry(&cache, "b".into(), entry("12345"));
        put_fetch_cache_entry(&cache, "c".into(), entry("12345"));

        let guard = cache.state.lock();
        assert_eq!(guard.lru.len(), 2);
        assert!(!guard.lru.contains("a"));
        assert!(guard.lru.contains("b"));
        assert!(guard.lru.contains("c"));
        assert_eq!(guard.bytes, 10);
    }
}
