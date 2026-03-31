use crate::error::RariError;
use crate::server::http_client::get_http_client;
use axum::http::HeaderMap;
use bytes::Bytes;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock};
use std::time::Instant;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

#[derive(Clone, Debug)]
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
pub struct PendingCookieKey {
    pub name: String,
    pub path: Option<String>,
    pub domain: Option<String>,
}

impl PendingCookieKey {
    pub fn new(name: &str, path: Option<&str>, domain: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            path: path.map(|s| s.to_string()),
            domain: domain.map(|s| s.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CachedFetchResult {
    pub body: Bytes,
    pub status: u16,
    pub headers: HeaderMap,
    pub cached_at: Instant,
    pub was_cached: bool,
    pub tags: Vec<String>,
}
type InFlightFetches =
    Arc<DashMap<String, Arc<TokioMutex<Option<Result<CachedFetchResult, RariError>>>>>>;

const MAX_CACHE_ENTRIES: usize = 1000;

static GLOBAL_FETCH_CACHE: LazyLock<Arc<Mutex<LruCache<String, CachedFetchResult>>>> =
    LazyLock::new(|| {
        Arc::new(Mutex::new(LruCache::new(
            NonZeroUsize::new(MAX_CACHE_ENTRIES).expect("MAX_CACHE_ENTRIES must be non-zero"),
        )))
    });

static GLOBAL_IN_FLIGHT_FETCHES: LazyLock<InFlightFetches> =
    LazyLock::new(|| Arc::new(DashMap::new()));

pub struct RequestContext {
    fetch_cache: Arc<Mutex<LruCache<String, CachedFetchResult>>>,
    in_flight_fetches: InFlightFetches,
    request_id: String,
    start_time: Instant,
    route_path: String,
    pub cookie_header: Option<String>,
    pub pending_cookies: Arc<DashMap<PendingCookieKey, PendingCookie>>,
}

impl RequestContext {
    pub fn new(route_path: String) -> Self {
        Self {
            fetch_cache: GLOBAL_FETCH_CACHE.clone(),
            in_flight_fetches: GLOBAL_IN_FLIGHT_FETCHES.clone(),
            request_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
            route_path,
            cookie_header: None,
            pending_cookies: Arc::new(DashMap::new()),
        }
    }

    pub fn with_cookies(mut self, cookie_header: Option<String>) -> Self {
        self.cookie_header = cookie_header;
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

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn fetch_cache(&self) -> &Arc<Mutex<LruCache<String, CachedFetchResult>>> {
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
                        format!("{}={}", k, v)
                    }
                })
                .collect::<Vec<_>>()
                .join("&");

            format!("{}?{}", url, opts_str)
        }
    }

    pub async fn fetch_with_cache(
        &self,
        url: &str,
        options: FxHashMap<String, String>,
    ) -> Result<CachedFetchResult, RariError> {
        let cache_key = Self::generate_cache_key(url, &options);

        let tags: Vec<String> =
            options.get("tags").and_then(|t| serde_json::from_str(t).ok()).unwrap_or_default();

        {
            let mut cache = self.fetch_cache.lock();
            if let Some(cached) = cache.get(&cache_key) {
                let ttl_ms =
                    options.get("cacheTTLMs").and_then(|t| t.parse::<u64>().ok()).unwrap_or(60_000);

                let elapsed_ms = cached.cached_at.elapsed().as_millis();

                if elapsed_ms < ttl_ms as u128 {
                    let mut result = cached.clone();
                    result.was_cached = true;
                    result.tags = Self::merge_and_sort_tags(result.tags, tags);

                    cache.put(cache_key.clone(), result.clone());

                    return Ok(result);
                }
                cache.pop(&cache_key);
            }
        }

        let fetch_lock = {
            let entry = self.in_flight_fetches.entry(cache_key.clone());
            entry.or_insert_with(|| Arc::new(TokioMutex::new(None))).clone()
        };

        let mut guard = fetch_lock.lock().await;

        if let Some(result) = guard.as_ref() {
            let mut cloned_result = result.clone()?;
            cloned_result.tags = Self::merge_and_sort_tags(cloned_result.tags, tags);

            {
                let mut cache = self.fetch_cache.lock();
                cache.put(cache_key.clone(), cloned_result.clone());
            }

            *guard = Some(Ok(cloned_result.clone()));

            return Ok(cloned_result);
        }

        struct CleanupGuard<'a> {
            in_flight_fetches: &'a InFlightFetches,
            cache_key: String,
        }

        impl<'a> Drop for CleanupGuard<'a> {
            fn drop(&mut self) {
                self.in_flight_fetches.remove(&self.cache_key);
            }
        }

        let _cleanup = CleanupGuard {
            in_flight_fetches: &self.in_flight_fetches,
            cache_key: cache_key.clone(),
        };

        let mut fetch_result = self.perform_fetch(url, &options).await;

        if let Ok(ref mut result) = fetch_result {
            result.tags = Self::merge_and_sort_tags(std::mem::take(&mut result.tags), tags);
        }

        *guard = Some(fetch_result.clone());

        if let Ok(ref cached_result) = fetch_result {
            let mut cache = self.fetch_cache.lock();
            cache.put(cache_key.clone(), cached_result.clone());
        }

        drop(guard);

        fetch_result
    }

    async fn perform_fetch(
        &self,
        url: &str,
        options: &FxHashMap<String, String>,
    ) -> Result<CachedFetchResult, RariError> {
        let client = get_http_client()
            .map_err(|e| RariError::network(format!("HTTP client initialization failed: {}", e)))?;
        let mut request = client.get(url);

        if let Some(headers_str) = options.get("headers")
            && let Ok(pairs) = serde_json::from_str::<Vec<(String, String)>>(headers_str)
        {
            for (key, value) in pairs {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        let timeout = options.get("timeout").and_then(|t| t.parse::<u64>().ok()).unwrap_or(5000);

        request = request.timeout(std::time::Duration::from_millis(timeout));

        let response = request
            .send()
            .await
            .map_err(|e| RariError::network(format!("Fetch failed for {}: {}", url, e)))?;

        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response
            .bytes()
            .await
            .map_err(|e| RariError::network(format!("Failed to read response body: {}", e)))?;

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
        assert!(!ctx.request_id().is_empty());
        assert!(ctx.elapsed().as_millis() < 100);
    }

    #[test]
    fn test_request_context_fetch_cache() {
        let ctx = RequestContext::new("/test".to_string());
        let cache = ctx.fetch_cache();

        let initial_len = cache.lock().len();

        let result = CachedFetchResult {
            body: Bytes::from("test"),
            status: 200,
            headers: HeaderMap::new(),
            cached_at: Instant::now(),
            was_cached: false,
            tags: Vec::new(),
        };

        let test_key = format!("https://test-{}.example.com", uuid::Uuid::new_v4());
        cache.lock().put(test_key.clone(), result);

        let new_len = cache.lock().len();
        assert!(
            new_len == initial_len + 1 || new_len == initial_len,
            "Cache should grow by 1 or stay at capacity"
        );

        assert!(cache.lock().contains(&test_key));
    }
}
