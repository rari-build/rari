use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use axum::http::HeaderMap;
use bytes::Bytes;
use dashmap::DashMap;
use parking_lot::Mutex;

use crate::{
    server::{
        cache::handler::{CacheHandler, MemoryCacheHandler, MemoryConfig},
        compression::CompressionEncoding,
        config::CacheLayerConfig,
    },
    utils::float,
};

#[derive(Clone)]
#[non_exhaustive]
pub struct PrebuiltResponse {
    pub identity: Bytes,
    pub gzip: Option<Bytes>,
    pub br: Option<Bytes>,
    pub zstd: Option<Bytes>,
    pub etag: String,
    pub content_type: String,
    pub cache_control: String,
    pub is_not_found: bool,
}

impl PrebuiltResponse {
    pub fn body_for(&self, encoding: CompressionEncoding) -> (Bytes, Option<&'static str>) {
        match encoding {
            CompressionEncoding::Zstd => match &self.zstd {
                Some(b) => (b.clone(), Some("zstd")),
                None => (self.identity.clone(), None),
            },
            CompressionEncoding::Brotli => match &self.br {
                Some(b) => (b.clone(), Some("br")),
                None => (self.identity.clone(), None),
            },
            CompressionEncoding::Gzip => match &self.gzip {
                Some(b) => (b.clone(), Some("gzip")),
                None => (self.identity.clone(), None),
            },
            CompressionEncoding::Identity => (self.identity.clone(), None),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CacheMetadata {
    #[serde(with = "instant_serde")]
    pub cached_at: Instant,
    pub ttl: u64,
    pub etag: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CachedResponse {
    pub body: Bytes,
    #[serde(with = "header_map_serde")]
    pub headers: HeaderMap,
    pub metadata: CacheMetadata,
    pub compressed_zstd: Option<Bytes>,
    pub compressed_br: Option<Bytes>,
    pub compressed_gzip: Option<Bytes>,
}

impl CachedResponse {
    pub fn is_valid(&self) -> bool {
        let elapsed = self.metadata.cached_at.elapsed().as_secs();
        elapsed < self.metadata.ttl
    }

    pub fn get_compressed(&self, encoding: &CompressionEncoding) -> Option<&Bytes> {
        match encoding {
            CompressionEncoding::Zstd => self.compressed_zstd.as_ref(),
            CompressionEncoding::Brotli => self.compressed_br.as_ref(),
            CompressionEncoding::Gzip => self.compressed_gzip.as_ref(),
            CompressionEncoding::Identity => None,
        }
    }
}

pub struct StaticFastCache {
    map: DashMap<String, Arc<PrebuiltResponse>>,
    insert_lock: Mutex<()>,
    entry_count: AtomicUsize,
}

impl StaticFastCache {
    pub fn new() -> Self {
        Self { map: DashMap::new(), insert_lock: Mutex::new(()), entry_count: AtomicUsize::new(0) }
    }

    pub fn entry_count(&self) -> usize {
        self.entry_count.load(Ordering::Relaxed)
    }

    pub fn get(&self, key: &str) -> Option<Arc<PrebuiltResponse>> {
        self.map.get(key).map(|entry| Arc::clone(entry.value()))
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }
}

impl Default for StaticFastCache {
    fn default() -> Self {
        Self::new()
    }
}

pub fn invalidate_static_fast_cache_for_path(cache: &StaticFastCache, path: &str) {
    let _guard = cache.insert_lock.lock();
    if cache.map.remove(path).is_some() {
        cache.entry_count.fetch_sub(1, Ordering::Relaxed);
    }
    let query_prefix = format!("{path}?");
    let hash_prefix = format!("{path}#");
    let keys: Vec<String> = cache
        .map
        .iter()
        .filter(|entry| {
            let key = entry.key();
            key.starts_with(&query_prefix) || key.starts_with(&hash_prefix)
        })
        .map(|entry| entry.key().clone())
        .collect();
    for key in keys {
        if cache.map.remove(&key).is_some() {
            cache.entry_count.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

pub fn insert_static_fast_cache(
    cache: &StaticFastCache,
    key: &str,
    value: Arc<PrebuiltResponse>,
    max_entries: usize,
) {
    if max_entries == 0 {
        return;
    }

    let _guard = cache.insert_lock.lock();
    let replacing = cache.map.insert(key.to_string(), value).is_some();
    if !replacing {
        cache.entry_count.fetch_add(1, Ordering::Relaxed);
    }
    if replacing {
        return;
    }

    while cache.entry_count.load(Ordering::Relaxed) > max_entries {
        let victim = cache
            .map
            .iter()
            .find(|entry| entry.key().as_str() != key && entry.key().contains('?'))
            .map(|entry| entry.key().clone())
            .or_else(|| {
                cache
                    .map
                    .iter()
                    .find(|entry| entry.key().as_str() != key)
                    .map(|entry| entry.key().clone())
            });
        match victim {
            Some(victim_key) => {
                if cache.map.remove(&victim_key).is_some() {
                    cache.entry_count.fetch_sub(1, Ordering::Relaxed);
                }
            }
            None => break,
        }
    }

    debug_assert!(cache.entry_count.load(Ordering::Relaxed) <= max_entries);
}

#[derive(Clone, Debug)]
#[non_exhaustive]
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
            max_entries: env::var("RARI_CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            default_ttl: env::var("RARI_CACHE_DEFAULT_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(31_536_000),
            enabled: env::var("RARI_CACHE_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(is_production),
        }
    }

    pub fn from_layer(layer: &CacheLayerConfig, is_production: bool) -> Self {
        Self {
            max_entries: env::var("RARI_CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(layer.max_entries),
            default_ttl: env::var("RARI_CACHE_DEFAULT_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(layer.default_ttl_secs),
            enabled: env::var("RARI_CACHE_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(is_production),
        }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
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

    pub fn merge_cache_tags(mut base: Vec<String>, extra: &[String]) -> Vec<String> {
        for tag in extra {
            if !base.iter().any(|existing| existing == tag) {
                base.push(tag.clone());
            }
        }
        base
    }

    /// The shared-cache freshness lifetime declared in a Cache-Control header,
    /// if any. `s-maxage` takes precedence over `max-age` (RFC 9111 §5.2.2.10):
    /// the server-side cache is a shared cache, and `max-age=0, s-maxage=3600`
    /// means "browsers revalidate, shared caches hold for an hour" -- reading
    /// only `max-age` there would disable caching the route asked for.
    pub fn max_age_from_cache_control(cache_control: &str) -> Option<u64> {
        let directive_value = |prefix: &str| {
            cache_control.split(',').find_map(|directive| {
                directive.trim().strip_prefix(prefix).and_then(|v| v.trim().parse().ok())
            })
        };
        directive_value("s-maxage=").or_else(|| directive_value("max-age="))
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
#[non_exhaustive]
pub struct CacheMetrics {
    pub total_entries: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub hit_rate: f64,
    pub memory_usage_bytes: usize,
}

pub struct ResponseCache {
    handler: Arc<dyn CacheHandler>,
    pub config: CacheConfig,
    metrics: Arc<Mutex<CacheMetrics>>,
    entry_count: Arc<AtomicUsize>,
}

impl ResponseCache {
    const KEY_PREFIX: &'static str = "response:";

    fn ns(key: &str) -> String {
        let prefix = Self::KEY_PREFIX;
        format!("{prefix}{key}")
    }

    pub fn new(config: CacheConfig) -> Self {
        let handler = MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: config.max_entries,
            default_ttl: config.default_ttl,
        });
        Self::new_with_handler(config, Arc::new(handler))
    }

    pub fn new_with_handler(config: CacheConfig, handler: Arc<dyn CacheHandler>) -> Self {
        Self {
            handler,
            config,
            metrics: Arc::new(Mutex::new(CacheMetrics::default())),
            entry_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn generate_cache_key(
        route: &str,
        params: Option<&rustc_hash::FxHashMap<String, String>>,
    ) -> String {
        Self::generate_cache_key_with_mode(route, params, None, None)
    }

    pub fn generate_cache_key_with_mode(
        route: &str,
        params: Option<&rustc_hash::FxHashMap<String, String>>,
        render_mode: Option<&str>,
        cookie_header: Option<&str>,
    ) -> String {
        let base = Self::route_cache_base_key(route, params);

        let with_mode = match render_mode {
            Some(mode) => format!("{base}#:{mode}"),
            None => base,
        };

        Self::append_cookie_partition(with_mode, cookie_header)
    }

    fn route_cache_base_key(
        route: &str,
        params: Option<&rustc_hash::FxHashMap<String, String>>,
    ) -> String {
        if let Some(params) = params {
            if params.is_empty() {
                route.to_string()
            } else {
                let mut sorted_params: Vec<_> = params.iter().collect();
                sorted_params.sort_by_key(|(k, _)| *k);

                let params_str = sorted_params
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("&");

                format!("{route}?{params_str}")
            }
        } else {
            route.to_string()
        }
    }

    fn append_cookie_partition(base_key: String, cookie_header: Option<&str>) -> String {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        let Some(cookie_header) = cookie_header.filter(|value| !value.is_empty()) else {
            return base_key;
        };

        let mut hasher = DefaultHasher::new();
        cookie_header.hash(&mut hasher);
        format!("{base_key}#cookie:{:x}", hasher.finish())
    }

    pub fn generate_static_fast_cache_key(
        route: &str,
        params: Option<&rustc_hash::FxHashMap<String, String>>,
        cookie_header: Option<&str>,
    ) -> String {
        Self::generate_cache_key_with_mode(route, params, None, cookie_header)
    }

    pub fn cache_key_matches_route(cache_key: &str, route: &str) -> bool {
        cache_key == route
            || cache_key.starts_with(&format!("{route}?"))
            || cache_key.starts_with(&format!("{route}#"))
    }

    pub fn generate_etag(content: &[u8]) -> String {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        format!("W/\"{hash:x}\"")
    }

    pub async fn get(&self, key: &str) -> Option<CachedResponse> {
        if !self.config.enabled {
            return None;
        }

        let bytes = match self.handler.get(&Self::ns(key)).await {
            Ok(Some(b)) => b,
            Ok(None) => {
                self.record_miss();
                return None;
            }
            Err(e) => {
                tracing::debug!(error = %e, key = %key, "cache get failed");
                self.record_miss();
                return None;
            }
        };

        let response: CachedResponse = match serde_json::from_slice(&bytes) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "cache deserialize failed; evicting");
                let _ = self.handler.invalidate(&Self::ns(key)).await;
                self.resync_entry_count();
                self.record_miss();
                return None;
            }
        };

        if !response.is_valid() {
            let _ = self.handler.invalidate(&Self::ns(key)).await;
            self.resync_entry_count();
            self.record_miss();
            return None;
        }

        self.record_hit();
        Some(response)
    }

    pub async fn set(&self, key: String, response: CachedResponse) {
        if !self.config.enabled {
            return;
        }

        let bytes = match serde_json::to_vec(&response) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = %e, "CachedResponse serialize failed; skipping set");
                return;
            }
        };

        let ttl = response.metadata.ttl;
        let tags = response.metadata.tags.clone();

        let ns_key = Self::ns(&key);
        let result = if tags.is_empty() {
            self.handler.set(&ns_key, bytes, ttl).await
        } else {
            self.handler.set_with_tags(&ns_key, bytes, ttl, &tags).await
        };

        if let Err(e) = result {
            tracing::warn!(error = %e, key = %key, "cache set failed");
            return;
        }

        self.resync_entry_count();
    }

    pub async fn update_in_place(&self, key: &str, response: CachedResponse) {
        let ns_key = Self::ns(key);
        if !self.config.enabled || self.handler.get(&ns_key).await.ok().flatten().is_none() {
            return;
        }

        let bytes = match serde_json::to_vec(&response) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, "CachedResponse serialize failed; skipping update_in_place");
                return;
            }
        };
        let ttl = response.metadata.ttl;
        let tags = response.metadata.tags.clone();
        let result = if tags.is_empty() {
            self.handler.set(&ns_key, bytes, ttl).await
        } else {
            self.handler.set_with_tags(&ns_key, bytes, ttl, &tags).await
        };
        if let Err(e) = result {
            tracing::warn!(error = %e, key = %key, "cache update_in_place failed");
        }
        self.resync_entry_count();
    }

    pub async fn invalidate(&self, key: &str) {
        if let Err(e) = self.handler.invalidate(&Self::ns(key)).await {
            tracing::debug!(error = %e, key = %key, "cache invalidate failed");
        }
        self.resync_entry_count();
    }

    pub async fn invalidate_by_tag(&self, tag: &str) {
        if let Err(e) = self.handler.invalidate_by_tag(tag).await {
            tracing::debug!(error = %e, tag = %tag, "cache invalidate_by_tag failed");
        }
        self.resync_entry_count();
    }

    pub async fn clear(&self) {
        if let Err(e) = self.handler.clear_prefix(Self::KEY_PREFIX).await {
            tracing::warn!(error = %e, "cache clear failed");
        }
        self.resync_entry_count();
    }

    #[cfg(test)]
    pub async fn clear_percentage(&self, percentage: f64) {
        use crate::utils::cast;

        let percentage = percentage.clamp(0.0, 1.0);
        let current_size = self.entry_count.load(Ordering::Relaxed);
        let entries_to_remove = cast::usize_fraction_ceil(current_size, percentage);

        let keys: Vec<String> = self
            .handler
            .get_all_keys()
            .into_iter()
            .filter(|k| k.starts_with(Self::KEY_PREFIX))
            .collect();
        for key in keys.into_iter().take(entries_to_remove) {
            if self.handler.invalidate(&key).await.is_ok() {
                let mut metrics = self.metrics.lock();
                metrics.evictions += 1;
            }
        }

        self.resync_entry_count();
    }

    #[cfg(test)]
    pub fn should_clear_on_memory_pressure(&self) -> bool {
        let current_size = self.entry_count.load(Ordering::Relaxed);
        let threshold = self.config.max_entries * 9 / 10;
        current_size >= threshold
    }

    pub fn get_metrics(&self) -> CacheMetrics {
        let metrics = self.metrics.lock();
        metrics.clone()
    }

    pub fn get_all_keys(&self) -> Vec<String> {
        self.handler
            .get_all_keys()
            .into_iter()
            .filter(|k| k.starts_with(Self::KEY_PREFIX))
            .map(|k| k.strip_prefix(Self::KEY_PREFIX).unwrap_or(&k).to_string())
            .collect()
    }

    fn record_hit(&self) {
        let mut metrics = self.metrics.lock();
        metrics.cache_hits += 1;
        Self::update_hit_rate(&mut metrics);
    }

    fn record_miss(&self) {
        let mut metrics = self.metrics.lock();
        metrics.cache_misses += 1;
        Self::update_hit_rate(&mut metrics);
    }

    fn update_hit_rate(metrics: &mut CacheMetrics) {
        let total = metrics.cache_hits + metrics.cache_misses;
        if total > 0 {
            metrics.hit_rate = float::u64_ratio(metrics.cache_hits, total);
        }
    }

    fn update_entry_count_metrics(&self) {
        let n = self.entry_count.load(Ordering::Relaxed);
        let mut metrics = self.metrics.lock();
        metrics.total_entries = n;
        metrics.memory_usage_bytes = n * 10_000;
    }

    fn resync_entry_count(&self) {
        let live =
            self.handler.get_all_keys().iter().filter(|k| k.starts_with(Self::KEY_PREFIX)).count();
        self.entry_count.store(live, Ordering::Relaxed);
        self.update_entry_count_metrics();
    }
}

mod instant_serde {
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::utils::cast;

    pub fn serialize<S: Serializer>(instant: &Instant, s: S) -> Result<S::Ok, S::Error> {
        let now_mono = Instant::now();
        let now_wall = SystemTime::now();
        let elapsed = now_mono.saturating_duration_since(*instant);
        let stored = now_wall.checked_sub(elapsed).unwrap_or(now_wall);
        let stored_ms =
            cast::duration_millis_u64(stored.duration_since(UNIX_EPOCH).unwrap_or_default());
        stored_ms.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Instant, D::Error> {
        let ms = u64::deserialize(d)?;
        let stored = UNIX_EPOCH + Duration::from_millis(ms);
        let elapsed_since_stored = SystemTime::now().duration_since(stored).unwrap_or_default();
        Ok(Instant::now().checked_sub(elapsed_since_stored).unwrap_or_else(Instant::now))
    }
}

mod header_map_serde {
    use axum::http::{HeaderMap, HeaderName, HeaderValue};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(map: &HeaderMap, s: S) -> Result<S::Ok, S::Error> {
        let pairs: Vec<(String, Vec<u8>)> = map
            .iter()
            .map(|(name, value)| (name.as_str().to_string(), value.as_bytes().to_vec()))
            .collect();
        pairs.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<HeaderMap, D::Error> {
        let pairs: Vec<(String, Vec<u8>)> = Vec::deserialize(d)?;
        let mut map = HeaderMap::with_capacity(pairs.len());
        for (name, value) in pairs {
            if let (Ok(n), Ok(v)) =
                (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_bytes(&value))
            {
                map.append(n, v);
            }
        }
        Ok(map)
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used, clippy::clone_on_ref_ptr)]
mod tests {
    use std::{sync::Arc, thread, time::Duration};

    use parking_lot::Mutex as PMutex;
    use rustc_hash::FxHashMap;
    use tokio::time;

    use super::*;
    use crate::server::cache::handler::{CacheError, SetOutcome};

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
            compressed_zstd: None,
            compressed_br: None,
            compressed_gzip: None,
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
        assert!(etag.ends_with('"'));

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

        time::sleep(time::Duration::from_millis(10)).await;

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
            cache.set(format!("key{i}"), create_test_response(&format!("body{i}"), 60)).await;
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
            cache.set(format!("key{i}"), create_test_response(&format!("body{i}"), 60)).await;
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
    fn test_max_age_from_cache_control() {
        let max_age = RouteCachePolicy::max_age_from_cache_control;
        assert_eq!(max_age("public, max-age=60"), Some(60));
        assert_eq!(max_age("max-age=3600, stale-while-revalidate=86400"), Some(3600));
        assert_eq!(max_age("max-age=0, s-maxage=3600"), Some(3600));
        assert_eq!(max_age("s-maxage=60"), Some(60));
        assert_eq!(max_age("s-maxage=garbage, max-age=60"), Some(60));
        assert_eq!(max_age("public"), None);
        assert_eq!(max_age("max-age=garbage"), None);
        assert_eq!(max_age(""), None);
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
        assert_eq!(config.default_ttl, 31_536_000);

        let config = CacheConfig::from_env(false);
        assert!(!config.enabled);
    }

    #[test]
    fn test_generate_cache_key_with_mode() {
        let key =
            ResponseCache::generate_cache_key_with_mode("/blog/post", None, Some("rsc"), None);
        assert_eq!(key, "/blog/post#:rsc");
    }

    #[test]
    fn test_generate_cache_key_with_mode_and_params() {
        let mut params = rustc_hash::FxHashMap::default();
        params.insert("page".to_string(), "1".to_string());

        let key =
            ResponseCache::generate_cache_key_with_mode("/blog", Some(&params), Some("rsc"), None);
        assert_eq!(key, "/blog?page=1#:rsc");
    }

    #[test]
    fn test_generate_cache_key_with_mode_none() {
        let key = ResponseCache::generate_cache_key_with_mode("/blog/post", None, None, None);
        assert_eq!(key, "/blog/post");
    }

    #[test]
    fn test_generate_cache_key_with_cookie_partition() {
        let key = ResponseCache::generate_cache_key_with_mode(
            "/actions",
            None,
            None,
            Some("session=abc"),
        );
        assert!(key.starts_with("/actions#cookie:"));
        assert_ne!(key, ResponseCache::generate_cache_key_with_mode("/actions", None, None, None));
        assert_eq!(
            key,
            ResponseCache::generate_cache_key_with_mode(
                "/actions",
                None,
                None,
                Some("session=abc")
            )
        );
    }

    #[derive(Debug, Default)]
    struct StubHandler {
        map: PMutex<FxHashMap<String, Vec<u8>>>,
        set_calls: PMutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl CacheHandler for StubHandler {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            Ok(self.map.lock().get(key).cloned())
        }
        async fn set(
            &self,
            key: &str,
            value: Vec<u8>,
            _ttl_secs: u64,
        ) -> Result<SetOutcome, CacheError> {
            self.set_calls.lock().push(key.to_string());
            let replaced = self.map.lock().insert(key.to_string(), value).is_some();
            Ok(SetOutcome { replaced, evicted: 0, evicted_bytes: 0 })
        }
        async fn set_with_tags(
            &self,
            key: &str,
            value: Vec<u8>,
            ttl_secs: u64,
            _tags: &[String],
        ) -> Result<SetOutcome, CacheError> {
            self.set(key, value, ttl_secs).await
        }
        async fn invalidate(&self, key: &str) -> Result<bool, CacheError> {
            Ok(self.map.lock().remove(key).is_some())
        }
        async fn invalidate_by_tag(&self, _tag: &str) -> Result<(), CacheError> {
            self.map.lock().clear();
            Ok(())
        }
        async fn clear(&self) -> Result<(), CacheError> {
            self.map.lock().clear();
            Ok(())
        }
        fn get_all_keys(&self) -> Vec<String> {
            self.map.lock().keys().cloned().collect()
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_handler_is_memory_by_default() {
        let config = CacheConfig { max_entries: 2, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        cache.set("a".to_string(), create_test_response("body-a", 60)).await;
        cache.set("b".to_string(), create_test_response("body-b", 60)).await;
        cache.set("c".to_string(), create_test_response("body-c", 60)).await;

        assert_eq!(cache.get_all_keys().len(), 2, "MemoryCacheHandler must cap at max_entries");
        assert_eq!(cache.get_metrics().total_entries, 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_new_with_custom_handler() {
        let stub = Arc::new(StubHandler::default());
        let config = CacheConfig { max_entries: 100, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new_with_handler(config, stub.clone());

        cache.set("k".to_string(), create_test_response("v", 60)).await;
        assert_eq!(*stub.set_calls.lock(), vec!["response:k".to_string()]);

        let got = cache.get("k").await.expect("get should round-trip via stub");
        assert_eq!(got.body, Bytes::from("v"));

        cache.invalidate("k").await;
        assert!(stub.map.lock().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_namespace_isolates_response_cache_from_og_and_image_layers() {
        use crate::server::og::OgImageCache;

        let shared: Arc<dyn CacheHandler> =
            Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
                max_entries: 32,
                default_ttl: 60,
            }));

        let response_cache = ResponseCache::new_with_handler(
            CacheConfig { max_entries: 32, default_ttl: 60, enabled: true },
            shared.clone(),
        );
        let test_dir = env::temp_dir().join("rari-test-cache-namespace");
        let og_cache = OgImageCache::with_handler(shared.clone(), &test_dir);

        response_cache.set("/about".to_string(), create_test_response("response-body", 60)).await;
        let og_payload = vec![0x52, 0x49, 0x46, 0x46];
        og_cache.insert("/about".to_string(), og_payload.clone()).await.expect("og insert");

        let response_got = response_cache.get("/about").await;
        assert!(response_got.is_some(), "response cache must not be polluted by og write");
        assert_eq!(response_got.unwrap().body, Bytes::from("response-body"));

        let og_got = og_cache.get("/about").await;
        assert_eq!(
            og_got,
            Some(og_payload.clone()),
            "og cache must not be polluted by response write"
        );

        response_cache.invalidate("/about").await;
        let og_after = og_cache.get("/about").await;
        assert_eq!(
            og_after,
            Some(og_payload),
            "og cache must survive response-cache invalidation under shared handler"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_serialized_response_round_trip() {
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html; charset=utf-8".parse().unwrap());
        headers.insert("cache-control", "max-age=60".parse().unwrap());
        headers.insert("x-rari-tag", "alpha".parse().unwrap());

        let original = CachedResponse {
            body: Bytes::from_static(b"hello world, this is the body"),
            headers,
            metadata: CacheMetadata {
                cached_at: Instant::now(),
                ttl: 120,
                etag: Some("W/\"deadbeef\"".to_string()),
                tags: vec!["alpha".to_string(), "beta".to_string()],
            },
            compressed_zstd: Some(Bytes::from_static(b"zstd-bytes")),
            compressed_br: Some(Bytes::from_static(b"br-bytes")),
            compressed_gzip: Some(Bytes::from_static(b"gzip-bytes")),
        };

        cache.set("k".to_string(), original.clone()).await;
        let got = cache.get("k").await.expect("get after set");

        assert_eq!(got.body, original.body);
        assert_eq!(got.headers, original.headers);
        assert_eq!(got.metadata.etag, original.metadata.etag);
        assert_eq!(got.metadata.ttl, original.metadata.ttl);
        assert_eq!(got.metadata.tags, original.metadata.tags);
        assert_eq!(got.compressed_zstd, original.compressed_zstd);
        assert_eq!(got.compressed_br, original.compressed_br);
        assert_eq!(got.compressed_gzip, original.compressed_gzip);
        // cached_at was reconstructed from elapsed ms -- still recent.
        assert!(got.metadata.cached_at.elapsed().as_secs() <= 1);
        assert!(got.is_valid());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_serialize_preserves_cached_at_across_reserialize() {
        // Regression test: `instant_serde::serialize` must encode the *given*
        // `Instant` (mapped to wall time), not `SystemTime::now()`. Otherwise
        // paths like `update_in_place` that re-serialize an existing
        // `CachedResponse` would silently reset `cached_at` to the moment of
        // the call, extending the entry's effective TTL.
        let config = CacheConfig { max_entries: 10, default_ttl: 60, enabled: true };
        let cache = ResponseCache::new(config);

        // Backdate `cached_at` so the age is clearly distinguishable from "now".
        // Must stay < ttl, otherwise `get` evicts the entry as expired and the
        // assertion below never runs.
        let original_age = Duration::from_secs(10);
        let original_cached_at = Instant::now().checked_sub(original_age).expect("monotonic clock");

        let mut response = create_test_response("body", 60);
        response.metadata.cached_at = original_cached_at;

        // First write via set, then re-serialize through update_in_place using a
        // response that still carries the original (backdated) `cached_at`.
        // This is exactly the shape the production path takes.
        cache.set("k".to_string(), response.clone()).await;
        cache.update_in_place("k", response).await;

        let got = cache.get("k").await.expect("get after re-serialize");

        // The reconstructed `cached_at` must reflect the *original* moment,
        // not the wall-clock time of the re-serialize. Allow a small slack
        // for scheduling jitter between the two operations.
        let reconstructed_age = got.metadata.cached_at.elapsed();
        assert!(
            reconstructed_age >= original_age.saturating_sub(Duration::from_secs(1)),
            "cached_at was reset on re-serialize: original age = {original_age:?}, \
             reconstructed age = {reconstructed_age:?}",
        );
    }

    #[test]
    fn test_invalidate_static_fast_cache_for_path() {
        let cache = StaticFastCache::new();
        let body = Bytes::from("html");
        let make_entry = || {
            Arc::new(PrebuiltResponse {
                identity: body.clone(),
                gzip: None,
                br: None,
                zstd: None,
                etag: "W/\"1\"".to_string(),
                content_type: "text/html; charset=utf-8".to_string(),
                cache_control: "public".to_string(),
                is_not_found: false,
            })
        };

        insert_static_fast_cache(&cache, "/about", make_entry(), 10);
        insert_static_fast_cache(&cache, "/about?tab=1", make_entry(), 10);
        insert_static_fast_cache(&cache, "/about#cookie:abc123", make_entry(), 10);
        insert_static_fast_cache(&cache, "/other", make_entry(), 10);

        invalidate_static_fast_cache_for_path(&cache, "/about");

        assert!(!cache.contains_key("/about"));
        assert!(!cache.contains_key("/about?tab=1"));
        assert!(!cache.contains_key("/about#cookie:abc123"));
        assert!(cache.contains_key("/other"));
        assert_eq!(cache.entry_count(), 1);
    }

    #[test]
    fn test_insert_static_fast_cache_caps_and_prefers_query_eviction() {
        let cache = StaticFastCache::new();
        let body = Bytes::from("html");
        let make_entry = || {
            Arc::new(PrebuiltResponse {
                identity: body.clone(),
                gzip: None,
                br: None,
                zstd: None,
                etag: "W/\"1\"".to_string(),
                content_type: "text/html; charset=utf-8".to_string(),
                cache_control: "public".to_string(),
                is_not_found: false,
            })
        };

        insert_static_fast_cache(&cache, "/", make_entry(), 2);
        insert_static_fast_cache(&cache, "/?utm=1", make_entry(), 2);
        assert_eq!(cache.entry_count(), 2);

        insert_static_fast_cache(&cache, "/blog", make_entry(), 2);
        assert_eq!(cache.entry_count(), 2);
        assert!(cache.contains_key("/"));
        assert!(cache.contains_key("/blog"));
        assert!(!cache.contains_key("/?utm=1"));

        insert_static_fast_cache(&cache, "/", make_entry(), 2);
        assert_eq!(cache.entry_count(), 2);
        assert!(cache.contains_key("/"));
        assert!(cache.contains_key("/blog"));

        insert_static_fast_cache(&cache, "/x", make_entry(), 0);
        assert!(!cache.contains_key("/x"));
        assert_eq!(cache.entry_count(), 2);
    }

    #[test]
    fn test_insert_static_fast_cache_replace_updates_value_not_count() {
        let cache = StaticFastCache::new();
        let make_entry = |body: &'static str, etag: &'static str| {
            Arc::new(PrebuiltResponse {
                identity: Bytes::from(body),
                gzip: None,
                br: None,
                zstd: None,
                etag: etag.to_string(),
                content_type: "text/html; charset=utf-8".to_string(),
                cache_control: "public".to_string(),
                is_not_found: false,
            })
        };

        insert_static_fast_cache(&cache, "/", make_entry("v1", "W/\"1\""), 2);
        insert_static_fast_cache(&cache, "/other", make_entry("other", "W/\"o\""), 2);
        assert_eq!(cache.entry_count(), 2);

        insert_static_fast_cache(&cache, "/", make_entry("v2", "W/\"2\""), 2);
        assert_eq!(cache.entry_count(), 2);
        assert!(cache.contains_key("/other"));

        let stored = cache.get("/").expect("replaced entry");
        assert_eq!(stored.identity.as_ref(), b"v2");
        assert_eq!(stored.etag, "W/\"2\"");
    }

    #[test]
    fn test_insert_static_fast_cache_concurrent_never_exceeds_max() {
        let cache = Arc::new(StaticFastCache::new());
        let max_entries = 32usize;
        let threads = 8usize;
        let per_thread = 200usize;
        let body = Bytes::from("html");

        let mut handles = Vec::with_capacity(threads);
        for thread_id in 0..threads {
            let cache = Arc::clone(&cache);
            let body = body.clone();
            handles.push(thread::spawn(move || {
                for i in 0..per_thread {
                    let key = format!("/t{thread_id}?n={i}");
                    let entry = Arc::new(PrebuiltResponse {
                        identity: body.clone(),
                        gzip: None,
                        br: None,
                        zstd: None,
                        etag: "W/\"1\"".to_string(),
                        content_type: "text/html; charset=utf-8".to_string(),
                        cache_control: "public".to_string(),
                        is_not_found: false,
                    });
                    insert_static_fast_cache(&cache, &key, entry, max_entries);
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker thread panicked");
        }

        assert!(cache.entry_count() <= max_entries);
        assert_eq!(cache.entry_count(), max_entries);
    }
}
