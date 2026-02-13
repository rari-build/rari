use crate::error::RariError;
use crate::server::http_client::get_http_client;
use axum::http::HeaderMap;
use bytes::Bytes;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock};
use std::time::Instant;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CachedFetchResult {
    pub body: Bytes,
    pub status: u16,
    pub headers: HeaderMap,
    pub cached_at: Instant,
    pub was_cached: bool,
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
}

impl RequestContext {
    pub fn new(route_path: String) -> Self {
        Self {
            fetch_cache: GLOBAL_FETCH_CACHE.clone(),
            in_flight_fetches: GLOBAL_IN_FLIGHT_FETCHES.clone(),
            request_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
            route_path,
        }
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

    fn generate_cache_key(url: &str, options: &FxHashMap<String, String>) -> String {
        let cache_relevant_options: FxHashMap<_, _> = options
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "cacheTTLMs" | "timeout"))
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
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        v.hash(&mut hasher);
                        format!("{}=h:{:x}", k, hasher.finish())
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

        {
            let mut cache = self.fetch_cache.lock();
            if let Some(cached) = cache.get(&cache_key) {
                let ttl_seconds = options
                    .get("cacheTTLMs")
                    .and_then(|t| t.parse::<u64>().ok())
                    .map(|ms| ms.div_ceil(1000))
                    .unwrap_or(60);

                let elapsed = cached.cached_at.elapsed().as_secs();

                if elapsed < ttl_seconds {
                    let mut result = cached.clone();
                    result.was_cached = true;
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
            return result.clone();
        }

        let fetch_result = self.perform_fetch(url, &options).await;

        *guard = Some(fetch_result.clone());

        if let Ok(ref cached_result) = fetch_result {
            let mut cache = self.fetch_cache.lock();
            cache.put(cache_key.clone(), cached_result.clone());
        }

        drop(guard);
        self.in_flight_fetches.remove(&cache_key);

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
