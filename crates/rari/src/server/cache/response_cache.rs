use axum::http::HeaderMap;
use bytes::Bytes;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct CacheMetadata {
    pub cached_at: Instant,
    pub ttl: u64,
    pub etag: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CachedResponse {
    pub body: Bytes,
    pub headers: HeaderMap,
    pub metadata: CacheMetadata,
}

impl CachedResponse {
    pub fn is_valid(&self) -> bool {
        let elapsed = self.metadata.cached_at.elapsed().as_secs();
        elapsed < self.metadata.ttl
    }
}

#[derive(Clone, Debug)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub default_ttl: u64,
    pub enabled: bool,
}

impl CacheConfig {
    pub fn default_ttl(&self) -> u64 {
        self.default_ttl
    }

    pub fn from_env(is_production: bool) -> Self {
        Self {
            max_entries: std::env::var("RARI_CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            default_ttl: std::env::var("RARI_CACHE_DEFAULT_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(31536000),
            enabled: std::env::var("RARI_CACHE_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(is_production),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RouteCachePolicy {
    pub ttl: u64,
    pub enabled: bool,
    pub tags: Vec<String>,
}

impl Default for RouteCachePolicy {
    fn default() -> Self {
        Self { ttl: 60, enabled: true, tags: Vec::new() }
    }
}

impl RouteCachePolicy {
    pub fn new(ttl: u64, enabled: bool, tags: Vec<String>) -> Self {
        Self { ttl, enabled, tags }
    }

    pub fn from_cache_control(cache_control: &str, route_path: &str) -> Self {
        let mut policy = Self::default();
        policy.tags.push(route_path.to_string());

        for directive in cache_control.split(',') {
            let directive = directive.trim();

            if directive == "no-store" || directive == "no-cache" {
                policy.enabled = false;
                return policy;
            }

            if let Some(max_age_str) = directive.strip_prefix("max-age=")
                && let Ok(max_age) = max_age_str.trim().parse::<u64>()
            {
                policy.ttl = max_age;
            }
        }

        policy
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self { max_entries: 1000, default_ttl: 60, enabled: true }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CacheMetrics {
    pub total_entries: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub hit_rate: f64,
    pub memory_usage_bytes: usize,
}

pub struct ResponseCache {
    cache: Arc<DashMap<String, CachedResponse>>,
    lru: Arc<Mutex<LruCache<String, ()>>>,
    pub config: CacheConfig,
    metrics: Arc<Mutex<CacheMetrics>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
}

impl ResponseCache {
    pub fn new(config: CacheConfig) -> Self {
        let max_entries = NonZeroUsize::new(config.max_entries)
            .unwrap_or(NonZeroUsize::new(1000).expect("1000 is non-zero"));

        Self {
            cache: Arc::new(DashMap::new()),
            lru: Arc::new(Mutex::new(LruCache::new(max_entries))),
            config,
            metrics: Arc::new(Mutex::new(CacheMetrics::default())),
            tag_index: Arc::new(DashMap::new()),
        }
    }

    pub fn generate_cache_key(
        route: &str,
        params: Option<&rustc_hash::FxHashMap<String, String>>,
    ) -> String {
        if let Some(params) = params {
            if params.is_empty() {
                return route.to_string();
            }

            let mut sorted_params: Vec<_> = params.iter().collect();
            sorted_params.sort_by_key(|(k, _)| *k);

            let params_str = sorted_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");

            format!("{}?{}", route, params_str)
        } else {
            route.to_string()
        }
    }

    pub fn generate_etag(content: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        format!("W/\"{:x}\"", hash)
    }

    pub async fn get(&self, key: &str) -> Option<CachedResponse> {
        if !self.config.enabled {
            return None;
        }

        let entry = match self.cache.get(key) {
            Some(e) => e,
            None => {
                self.record_miss();
                return None;
            }
        };

        let cached_response = entry.value().clone();
        drop(entry);

        if !cached_response.is_valid() {
            self.invalidate(key).await;
            self.record_miss();
            return None;
        }

        {
            let mut lru = self.lru.lock();
            lru.promote(key);
        }

        self.record_hit();
        Some(cached_response)
    }

    pub async fn set(&self, key: String, response: CachedResponse) {
        if !self.config.enabled {
            return;
        }

        let should_evict = {
            let lru = self.lru.lock();
            lru.len() >= self.config.max_entries && !self.cache.contains_key(&key)
        };

        if should_evict {
            self.evict_lru().await;
        }

        for tag in &response.metadata.tags {
            self.tag_index.entry(tag.clone()).or_insert_with(Vec::new).push(key.clone());
        }

        self.cache.insert(key.clone(), response);

        {
            let mut lru = self.lru.lock();
            lru.put(key, ());
        }

        self.update_entry_count();
    }

    pub async fn invalidate(&self, key: &str) {
        if let Some((_, response)) = self.cache.remove(key) {
            for tag in &response.metadata.tags {
                if let Some(mut keys) = self.tag_index.get_mut(tag) {
                    keys.retain(|k| k != key);
                }
            }

            {
                let mut lru = self.lru.lock();
                lru.pop(key);
            }

            self.update_entry_count();
        }
    }

    pub async fn invalidate_by_tag(&self, tag: &str) {
        if let Some(keys_entry) = self.tag_index.get(tag) {
            let keys = keys_entry.value().clone();
            drop(keys_entry);

            for key in keys {
                self.invalidate(&key).await;
            }
        }

        self.tag_index.remove(tag);
    }

    pub async fn clear(&self) {
        self.cache.clear();
        {
            let mut lru = self.lru.lock();
            lru.clear();
        }
        self.tag_index.clear();

        let mut metrics = self.metrics.lock();
        metrics.total_entries = 0;
        metrics.memory_usage_bytes = 0;
    }

    pub async fn clear_percentage(&self, percentage: f64) {
        let percentage = percentage.clamp(0.0, 1.0);
        let current_size = self.cache.len();
        let entries_to_remove = (current_size as f64 * percentage).ceil() as usize;

        for _ in 0..entries_to_remove {
            let key_to_evict = {
                let mut lru = self.lru.lock();
                lru.pop_lru().map(|(k, _)| k)
            };

            if let Some(key) = key_to_evict {
                self.cache.remove(&key);

                let mut metrics = self.metrics.lock();
                metrics.evictions += 1;
            } else {
                break;
            }
        }

        self.update_entry_count();
    }

    pub fn should_clear_on_memory_pressure(&self) -> bool {
        let current_size = self.cache.len();
        let threshold = (self.config.max_entries as f64 * 0.9) as usize;
        current_size >= threshold
    }

    async fn evict_lru(&self) {
        let key_to_evict = {
            let mut lru = self.lru.lock();
            lru.pop_lru().map(|(k, _)| k)
        };

        if let Some(key) = key_to_evict {
            self.cache.remove(&key);

            let mut metrics = self.metrics.lock();
            metrics.evictions += 1;
        }
    }

    pub fn get_metrics(&self) -> CacheMetrics {
        let metrics = self.metrics.lock();
        metrics.clone()
    }

    pub fn get_all_keys(&self) -> Vec<String> {
        self.cache.iter().map(|entry| entry.key().clone()).collect()
    }

    fn record_hit(&self) {
        let mut metrics = self.metrics.lock();
        metrics.cache_hits += 1;
        self.update_hit_rate(&mut metrics);
    }

    fn record_miss(&self) {
        let mut metrics = self.metrics.lock();
        metrics.cache_misses += 1;
        self.update_hit_rate(&mut metrics);
    }

    fn update_hit_rate(&self, metrics: &mut CacheMetrics) {
        let total = metrics.cache_hits + metrics.cache_misses;
        if total > 0 {
            metrics.hit_rate = metrics.cache_hits as f64 / total as f64;
        }
    }

    fn update_entry_count(&self) {
        let mut metrics = self.metrics.lock();
        metrics.total_entries = self.cache.len();

        metrics.memory_usage_bytes = metrics.total_entries * 10_000;
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn create_test_response(body: &str, ttl: u64) -> CachedResponse {
        CachedResponse {
            body: Bytes::from(body.to_string()),
            headers: HeaderMap::new(),
            metadata: CacheMetadata {
                cached_at: Instant::now(),
                ttl,
                etag: Some("test-etag".to_string()),
                tags: vec!["test-tag".to_string()],
            },
        }
    }

    #[test]
    fn test_generate_cache_key_without_params() {
        let key = ResponseCache::generate_cache_key("/blog/post", None);
        assert_eq!(key, "/blog/post");
    }

    #[test]
    fn test_generate_cache_key_with_params() {
        let mut params = rustc_hash::FxHashMap::default();
        params.insert("page".to_string(), "1".to_string());
        params.insert("sort".to_string(), "date".to_string());

        let key = ResponseCache::generate_cache_key("/blog", Some(&params));

        assert_eq!(key, "/blog?page=1&sort=date");
    }

    #[test]
    fn test_generate_cache_key_consistency() {
        let mut params1 = rustc_hash::FxHashMap::default();
        params1.insert("b".to_string(), "2".to_string());
        params1.insert("a".to_string(), "1".to_string());

        let mut params2 = rustc_hash::FxHashMap::default();
        params2.insert("a".to_string(), "1".to_string());
        params2.insert("b".to_string(), "2".to_string());

        let key1 = ResponseCache::generate_cache_key("/test", Some(&params1));
        let key2 = ResponseCache::generate_cache_key("/test", Some(&params2));

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_generate_etag() {
        let content = b"Hello, World!";
        let etag = ResponseCache::generate_etag(content);

        assert!(etag.starts_with("W/\""));
        assert!(etag.ends_with("\""));

        let etag2 = ResponseCache::generate_etag(content);
        assert_eq!(etag, etag2);

        let different_content = b"Different content";
        let different_etag = ResponseCache::generate_etag(different_content);
        assert_ne!(etag, different_etag);
    }

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        assert!(cache.get("test-key").await.is_none());

        let response = create_test_response("test body", 60);
        cache.set("test-key".to_string(), response).await;

        let retrieved = cache.get("test-key").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().body, Bytes::from("test body"));

        let metrics = cache.get_metrics();
        assert_eq!(metrics.total_entries, 1);
        assert_eq!(metrics.cache_hits, 1);
        assert_eq!(metrics.cache_misses, 1);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        let response = create_test_response("test body", 0);
        cache.set("test-key".to_string(), response).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert!(cache.get("test-key").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        let response = create_test_response("test body", 60);
        cache.set("test-key".to_string(), response).await;

        assert!(cache.get("test-key").await.is_some());

        cache.invalidate("test-key").await;

        assert!(cache.get("test-key").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_tag_invalidation() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        let mut response1 = create_test_response("body1", 60);
        response1.metadata.tags = vec!["shared-tag".to_string()];

        let mut response2 = create_test_response("body2", 60);
        response2.metadata.tags = vec!["shared-tag".to_string()];

        cache.set("key1".to_string(), response1).await;
        cache.set("key2".to_string(), response2).await;

        assert!(cache.get("key1").await.is_some());
        assert!(cache.get("key2").await.is_some());

        cache.invalidate_by_tag("shared-tag").await;

        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        let config = CacheConfig { max_entries: 2, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        cache.set("key1".to_string(), create_test_response("body1", 60)).await;
        cache.set("key2".to_string(), create_test_response("body2", 60)).await;

        assert!(cache.get("key1").await.is_some());
        assert!(cache.get("key2").await.is_some());

        cache.set("key3".to_string(), create_test_response("body3", 60)).await;

        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_some());
        assert!(cache.get("key3").await.is_some());
    }

    #[tokio::test]
    async fn test_cache_disabled() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: false };
        let cache = ResponseCache::new(config);

        let response = create_test_response("test body", 60);
        cache.set("test-key".to_string(), response).await;

        assert!(cache.get("test-key").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        cache.set("key1".to_string(), create_test_response("body1", 60)).await;
        cache.set("key2".to_string(), create_test_response("body2", 60)).await;

        assert_eq!(cache.get_metrics().total_entries, 2);

        cache.clear().await;

        assert_eq!(cache.get_metrics().total_entries, 0);
        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_none());
    }

    #[tokio::test]
    async fn test_clear_percentage() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        for i in 0..10 {
            cache.set(format!("key{}", i), create_test_response(&format!("body{}", i), 60)).await;
        }

        assert_eq!(cache.get_metrics().total_entries, 10);

        cache.clear_percentage(0.5).await;

        let metrics = cache.get_metrics();
        assert_eq!(metrics.total_entries, 5);
        assert_eq!(metrics.evictions, 5);
    }

    #[tokio::test]
    async fn test_memory_pressure_detection() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        for i in 0..8 {
            cache.set(format!("key{}", i), create_test_response(&format!("body{}", i), 60)).await;
        }

        assert!(!cache.should_clear_on_memory_pressure());

        cache.set("key8".to_string(), create_test_response("body8", 60)).await;

        assert!(cache.should_clear_on_memory_pressure());
    }

    #[test]
    fn test_route_cache_policy_from_cache_control() {
        let policy = RouteCachePolicy::from_cache_control("max-age=3600, public", "/test");
        assert_eq!(policy.ttl, 3600);
        assert!(policy.enabled);
        assert_eq!(policy.tags, vec!["/test".to_string()]);
    }

    #[test]
    fn test_route_cache_policy_no_store() {
        let policy = RouteCachePolicy::from_cache_control("no-store", "/test");
        assert!(!policy.enabled);
    }

    #[test]
    fn test_route_cache_policy_no_cache() {
        let policy = RouteCachePolicy::from_cache_control("no-cache", "/test");
        assert!(!policy.enabled);
    }

    #[test]
    fn test_route_cache_policy_default() {
        let policy = RouteCachePolicy::default();
        assert_eq!(policy.ttl, 60);
        assert!(policy.enabled);
        assert!(policy.tags.is_empty());
    }

    #[test]
    fn test_cache_config_from_env() {
        let config = CacheConfig::from_env(true);
        assert!(config.enabled);
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.default_ttl, 31536000);

        let config = CacheConfig::from_env(false);
        assert!(!config.enabled);
    }
}
