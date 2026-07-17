use std::{
    fmt::{Debug, Formatter, Result},
    result,
    sync::Arc,
};

use dashmap::DashMap;
use rari_error::RariError;
use serde_json::Value;

use crate::{
    cache::{CacheHandlerRegistry, MemoryCacheHandler, MemoryConfig, handler::CacheHandler},
    config::CacheLayerConfig,
};

pub struct ModuleCaching {
    handler: Arc<dyn CacheHandler>,
    max_age_secs: u64,
    component_source_paths: DashMap<String, String>,
}

impl Debug for ModuleCaching {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ModuleCaching")
            .field("max_age_secs", &self.max_age_secs)
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
            Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
                max_entries: cache_size.max(1),
                default_ttl: ttl_secs,
            })),
        )
    }

    pub fn with_handler(
        _cache_size: usize,
        max_age_secs: u64,
        handler: Arc<dyn CacheHandler>,
    ) -> Self {
        Self { handler, max_age_secs, component_source_paths: DashMap::new() }
    }

    pub fn from_config(layer: &CacheLayerConfig, registry: &CacheHandlerRegistry) -> Self {
        let handler = registry.resolve(&layer.handler);
        Self::with_handler(layer.max_entries, layer.default_ttl_secs, handler)
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        let Ok(Some(bytes)) = self.handler.get(key).await else {
            return None;
        };
        match serde_json::from_slice(&bytes) {
            Ok(v) => Some(v),
            Err(_) => {
                self.invalidate(key).await;
                None
            }
        }
    }

    #[expect(clippy::missing_errors_doc)]
    pub async fn insert(&self, key: String, value: Value) -> result::Result<(), RariError> {
        let bytes = serde_json::to_vec(&value)
            .map_err(|e| RariError::cache(format!("json serialize: {e}")))?;
        self.handler
            .set(&key, bytes, self.max_age_secs)
            .await
            .map_err(|e| RariError::cache(format!("cache set: {e}")))?;
        Ok(())
    }

    pub async fn invalidate(&self, key: &str) {
        if let Err(e) = self.handler.invalidate(key).await {
            tracing::debug!("module_caching.invalidate({}) failed: {}", key, e);
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
        if self.handler.clear().await.is_ok() {
            self.component_source_paths.clear();
        } else {
            tracing::debug!("module_caching.clear failed; local state preserved");
        }
    }

    pub fn clear_component(&self, component_id: &str) {
        self.component_source_paths.remove(component_id);
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, clippy::clone_on_ref_ptr)]
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
    }

    #[tokio::test]
    async fn test_invalidate() {
        let handler = Arc::new(MemoryCacheHandler::default());
        let cache = ModuleCaching::with_handler(4, 60, handler.clone());

        cache.insert("k".to_string(), serde_json::json!({"v": 1})).await.unwrap();
        assert!(cache.get("k").await.is_some());

        cache.invalidate("k").await;
        assert!(cache.get("k").await.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = ModuleCaching::new(10, DEFAULT_TTL_SECS);

        cache.insert("a".to_string(), serde_json::json!(1)).await.unwrap();
        cache.insert("b".to_string(), serde_json::json!(2)).await.unwrap();
        cache.set_component_source_path("comp".to_string(), "/path".to_string());

        cache.clear().await;

        assert!(cache.get("a").await.is_none());
        assert!(cache.get("b").await.is_none());
        assert!(cache.get_component_source_path("comp").is_none());
    }

    #[tokio::test]
    async fn test_corrupt_data_evicts_key() {
        use crate::cache::handler::CacheHandler;

        let handler = Arc::new(MemoryCacheHandler::default());
        let cache = ModuleCaching::with_handler(4, DEFAULT_TTL_SECS, handler.clone());

        handler.set("corrupt", b"not-json{".to_vec(), DEFAULT_TTL_SECS).await.unwrap();

        let got = cache.get("corrupt").await;
        assert!(got.is_none());
        // Key should have been invalidated on deserialization failure
        assert!(handler.get("corrupt").await.unwrap().is_none());
    }
}
