use crate::error::RariError;
use axum::http::HeaderMap;
use bytes::Bytes;
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CachedFetchResult {
    pub body: Bytes,
    pub status: u16,
    pub headers: HeaderMap,
    pub cached_at: Instant,
}
type InFlightFetches =
    Arc<DashMap<String, Arc<Mutex<Option<Result<CachedFetchResult, RariError>>>>>>;

pub struct RequestContext {
    fetch_cache: Arc<DashMap<String, CachedFetchResult>>,
    in_flight_fetches: InFlightFetches,
    request_id: String,
    start_time: Instant,
    route_path: String,
}

impl RequestContext {
    pub fn new(route_path: String) -> Self {
        Self {
            fetch_cache: Arc::new(DashMap::new()),
            in_flight_fetches: Arc::new(DashMap::new()),
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

    pub fn fetch_cache(&self) -> &Arc<DashMap<String, CachedFetchResult>> {
        &self.fetch_cache
    }

    fn generate_cache_key(url: &str, options: &FxHashMap<String, String>) -> String {
        if options.is_empty() {
            url.to_string()
        } else {
            let mut sorted_opts: Vec<_> = options.iter().collect();
            sorted_opts.sort_by_key(|(k, _)| *k);

            let opts_str = sorted_opts
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
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

        if let Some(cached) = self.fetch_cache.get(&cache_key) {
            return Ok(cached.value().clone());
        }

        let fetch_lock = {
            let entry = self.in_flight_fetches.entry(cache_key.clone());
            entry.or_insert_with(|| Arc::new(Mutex::new(None))).clone()
        };

        let mut guard = fetch_lock.lock().await;

        if let Some(result) = guard.as_ref() {
            return result.clone();
        }

        let fetch_result = self.perform_fetch(url, &options).await;

        *guard = Some(fetch_result.clone());

        if let Ok(ref cached_result) = fetch_result {
            self.fetch_cache.insert(cache_key.clone(), cached_result.clone());
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
        let client = reqwest::Client::new();
        let mut request = client.get(url);

        if let Some(headers_str) = options.get("headers") {
            for header_pair in headers_str.split(',') {
                if let Some((key, value)) = header_pair.split_once(':') {
                    request = request.header(key.trim(), value.trim());
                }
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

        Ok(CachedFetchResult { body, status, headers, cached_at: Instant::now() })
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

        assert_eq!(cache.len(), 0);

        let result = CachedFetchResult {
            body: Bytes::from("test"),
            status: 200,
            headers: HeaderMap::new(),
            cached_at: Instant::now(),
        };

        cache.insert("https://example.com".to_string(), result);

        assert_eq!(cache.len(), 1);
        assert!(cache.contains_key("https://example.com"));
    }
}
