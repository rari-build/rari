use super::config::CacheStats;
use crate::error::RariError;
use crate::server::cache::handler::CacheHandler;
use crate::server::cache::{CacheHandlerRegistry, MemoryCacheHandler, MemoryConfig};
use crate::server::config::CacheLayerConfig;
use dashmap::DashMap;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ModuleCaching {
    handler: Arc<dyn CacheHandler>,
    max_age_secs: u64,
    hit_count: AtomicUsize,
    miss_count: AtomicUsize,
    eviction_count: AtomicUsize,
    size: AtomicUsize,
    component_source_paths: DashMap<String, String>,
}

impl std::fmt::Debug for ModuleCaching {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleCaching")
            .field("max_age_secs", &self.max_age_secs)
            .field("hit_count", &self.hit_count)
            .field("miss_count", &self.miss_count)
            .field("eviction_count", &self.eviction_count)
            .field("size", &self.size)
            .field("component_source_paths", &self.component_source_paths)
            .finish_non_exhaustive()
    }
}

pub const DEFAULT_TTL_SECS: u64 = 3600;
const DEFAULT_CACHE_SIZE: usize = 5000;

impl Default for ModuleCaching {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_SIZE, DEFAULT_TTL_SECS)
    }
}

impl ModuleCaching {
    pub fn new(cache_size: usize, ttl_secs: u64) -> Self {
        Self::with_handler(
            cache_size,
            ttl_secs,
            Arc::new(MemoryCacheHandler::with_config(MemoryConfig {
                max_entries: cache_size.max(1),
                default_ttl: ttl_secs,
            })),
        )
    }

    pub fn with_handler(
        cache_size: usize,
        max_age_secs: u64,
        handler: Arc<dyn CacheHandler>,
    ) -> Self {
        let _ = (cache_size, max_age_secs);
        Self {
            handler,
            max_age_secs,
            hit_count: AtomicUsize::new(0),
            miss_count: AtomicUsize::new(0),
            eviction_count: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
            component_source_paths: DashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_config(layer: &CacheLayerConfig, registry: &CacheHandlerRegistry) -> Self {
        let handler =
            registry.get(&layer.handler).or_else(|| registry.get("memory")).unwrap_or_else(|| {
                let mem: Arc<dyn CacheHandler> = Arc::new(MemoryCacheHandler::default());
                registry.register("memory", Arc::clone(&mem));
                mem
            });

        Self::with_handler(layer.max_entries, layer.default_ttl_secs, handler)
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let size = self.size.load(Ordering::Relaxed);
        CacheStats {
            hits: self.hit_count.load(Ordering::Relaxed),
            misses: self.miss_count.load(Ordering::Relaxed),
            evictions: self.eviction_count.load(Ordering::Relaxed),
            size,
            memory_bytes: size * 64,
        }
    }

    pub async fn get(&self, key: &str) -> Option<JsonValue> {
        let bytes = match self.handler.get(key).await {
            Ok(Some(b)) => b,
            Ok(None) => {
                self.miss_count.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            Err(_) => {
                self.miss_count.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };
        match serde_json::from_slice(&bytes) {
            Ok(v) => {
                self.hit_count.fetch_add(1, Ordering::Relaxed);
                Some(v)
            }
            Err(_) => {
                self.miss_count.fetch_add(1, Ordering::Relaxed);
                let _ = self.handler.invalidate(key).await;
                None
            }
        }
    }

    pub async fn insert(&self, key: String, value: JsonValue) -> Result<(), RariError> {
        let bytes = serde_json::to_vec(&value)
            .map_err(|e| RariError::cache(format!("json serialize: {e}")))?;
        let outcome = self
            .handler
            .set(&key, bytes, self.max_age_secs)
            .await
            .map_err(|e| RariError::cache(format!("cache set: {e}")))?;
        if !outcome.replaced {
            self.size.fetch_add(1, Ordering::Relaxed);
        }
        if outcome.evicted > 0 {
            self.size.fetch_sub(outcome.evicted, Ordering::Relaxed);
        }
        Ok(())
    }

    pub async fn invalidate(&self, key: &str) {
        match self.handler.invalidate(key).await {
            Ok(true) => {
                self.size.fetch_sub(1, Ordering::Relaxed);
            }
            Ok(false) => {}
            Err(e) => {
                tracing::debug!("module_caching.invalidate({}) failed: {}", key, e);
            }
        }
    }

    #[cfg(test)]
    pub fn set_component_source_path(&self, component_id: String, path: String) {
        self.component_source_paths.insert(component_id, path);
    }

    pub fn get_component_source_path(&self, component_id: &str) -> Option<String> {
        self.component_source_paths.get(component_id).map(|entry| entry.value().clone())
    }

    pub fn remove_component_source_path(&self, component_id: &str) {
        self.component_source_paths.remove(component_id);
    }

    pub async fn clear(&self) {
        if let Err(e) = self.handler.clear().await {
            tracing::debug!("module_caching.clear failed: {}", e);
        }
        self.size.store(0, Ordering::Relaxed);
        self.component_source_paths.clear();
    }

    pub fn clear_component(&self, component_id: &str) {
        self.component_source_paths.remove(component_id);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = ModuleCaching::new(10, DEFAULT_TTL_SECS);

        cache.insert("key1".to_string(), serde_json::json!({"value": 1})).await.unwrap();
        assert_eq!(cache.get("key1").await, Some(serde_json::json!({"value": 1})));
        assert!(cache.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        let cache = ModuleCaching::new(2, DEFAULT_TTL_SECS);

        cache.insert("key1".to_string(), serde_json::json!(1)).await.unwrap();
        cache.insert("key2".to_string(), serde_json::json!(2)).await.unwrap();
        cache.insert("key3".to_string(), serde_json::json!(3)).await.unwrap();

        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_some());
        assert!(cache.get("key3").await.is_some());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = ModuleCaching::new(10, DEFAULT_TTL_SECS);

        cache.insert("key1".to_string(), serde_json::json!(1)).await.unwrap();
        cache.get("key1").await;
        cache.get("key2").await;

        let stats = cache.get_cache_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_module_caching() {
        let caching = ModuleCaching::new(10, DEFAULT_TTL_SECS);

        caching.set_component_source_path("comp1".to_string(), "/path/to/comp1".to_string());
        assert_eq!(caching.get_component_source_path("comp1"), Some("/path/to/comp1".to_string()));

        caching.insert("module1".to_string(), serde_json::json!({"data": "test"})).await.unwrap();
        assert!(caching.get("module1").await.is_some());
    }

    #[tokio::test]
    async fn test_handler_round_trip() {
        let cache = ModuleCaching::new(10, DEFAULT_TTL_SECS);
        let value = serde_json::json!({"nested": {"a": [1, 2, 3], "b": "x"}});

        cache.insert("k".to_string(), value.clone()).await.unwrap();
        assert_eq!(cache.get("k").await, Some(value));

        let stats = cache.get_cache_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[tokio::test]
    async fn test_invalidate_via_custom_handler() {
        let handler = Arc::new(MemoryCacheHandler::default());
        let cache = ModuleCaching::with_handler(4, 60, handler.clone());

        cache.insert("k".to_string(), serde_json::json!({"v": 1})).await.unwrap();
        assert!(cache.get("k").await.is_some());

        cache.invalidate("k").await;

        assert!(cache.get("k").await.is_none());
        assert_eq!(cache.get_cache_stats().size, 0);
    }

    #[tokio::test]
    async fn test_size_counter_survives_replace_and_double_invalidate() {
        // Regression test: previously `insert` did an unconditional +1 on
        // size even when the key already existed (replace), and `invalidate`
        // did an unconditional -1 even when the key was not present. The
        // combination of replace + repeated invalidate on a non-existent
        // key would wrap size to usize::MAX.
        let cache = ModuleCaching::new(8, DEFAULT_TTL_SECS);

        cache.insert("k".to_string(), serde_json::json!(1)).await.unwrap();
        assert_eq!(cache.get_cache_stats().size, 1);

        cache.insert("k".to_string(), serde_json::json!(2)).await.unwrap();
        assert_eq!(cache.get_cache_stats().size, 1, "replace must not inflate size");

        cache.invalidate("k").await;
        assert_eq!(cache.get_cache_stats().size, 0);

        cache.invalidate("k").await;
        cache.invalidate("k").await;
        assert_eq!(
            cache.get_cache_stats().size,
            0,
            "repeated invalidate of absent key must not underflow"
        );
    }

    #[tokio::test]
    async fn test_size_counter_tracks_lru_eviction() {
        // Regression test: previously an `insert` that triggered LRU
        // eviction in the underlying handler still did +1 on size, while
        // the evicted entry silently disappeared from the cache. The size
        // would drift up over time, and combined with the invalidate bug
        // could eventually underflow to usize::MAX.
        let cache = ModuleCaching::new(2, DEFAULT_TTL_SECS);

        cache.insert("a".to_string(), serde_json::json!(1)).await.unwrap();
        cache.insert("b".to_string(), serde_json::json!(2)).await.unwrap();
        assert_eq!(cache.get_cache_stats().size, 2);

        cache.insert("c".to_string(), serde_json::json!(3)).await.unwrap();
        assert_eq!(cache.get_cache_stats().size, 2, "evicted entry must decrement size");

        cache.invalidate("a").await;
        cache.invalidate("b").await;
        cache.invalidate("c").await;
        assert_eq!(
            cache.get_cache_stats().size,
            0,
            "invalidate of evicted keys must not underflow"
        );
    }
}
