//! Pluggable cache handler abstraction.
//!
//! Handlers are byte-agnostic: the trait operates on `Vec<u8>`. Typed
//! wrappers (e.g. `ResponseCache`) serialize their domain value to
//! bytes before calling the handler. Tags are first-class — handlers
//! that don't support tags can no-op `set_with_tags` /
//! `invalidate_by_tag`.
//!
//! Trait is `async_trait`-based (so `Arc<dyn CacheHandler>` works).
//! Native `async fn in dyn trait` is not yet stable; when it lands
//! the macro can be dropped.

use std::{
    fmt,
    io::Error,
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, Instant},
};

pub use async_trait::async_trait;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CacheError {
    #[error("io error: {0}")]
    Io(#[from] Error),

    #[error("serialization error: {0}")]
    Serialize(String),

    #[error("deserialization error: {0}")]
    Deserialize(String),

    #[error("backend error: {0}")]
    Backend(String),
}

#[async_trait::async_trait]
pub trait CacheHandler: Send + Sync + fmt::Debug {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;

    async fn set(&self, key: &str, value: Vec<u8>, ttl_secs: u64)
    -> Result<SetOutcome, CacheError>;

    async fn set_with_tags(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl_secs: u64,
        tags: &[String],
    ) -> Result<SetOutcome, CacheError>;

    async fn invalidate(&self, key: &str) -> Result<bool, CacheError>;

    async fn invalidate_by_tag(&self, tag: &str) -> Result<(), CacheError>;

    async fn clear(&self) -> Result<(), CacheError>;

    async fn clear_prefix(&self, prefix: &str) -> Result<usize, CacheError> {
        let keys = self.get_all_keys();
        let mut removed = 0usize;
        for key in keys {
            if key.starts_with(prefix) && self.invalidate(&key).await? {
                removed += 1;
            }
        }
        Ok(removed)
    }

    fn get_all_keys(&self) -> Vec<String>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate boundary type: backends construct this exhaustively; in-tree so no semver isolation"
)]
pub struct SetOutcome {
    pub replaced: bool,
    pub evicted: usize,
    pub evicted_bytes: usize,
}

#[derive(Debug)]
struct MemEntry {
    bytes: Vec<u8>,
    expires_at: Option<Instant>,
    tags: Vec<String>,
}

#[derive(Clone, Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate boundary type: backends construct this exhaustively; in-tree so no semver isolation"
)]
pub struct MemoryConfig {
    pub max_entries: usize,
    pub default_ttl: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { max_entries: 1000, default_ttl: 31_536_000 }
    }
}

#[derive(Debug)]
pub struct MemoryCacheHandler {
    cache: DashMap<String, MemEntry>,
    lru: Mutex<LruCache<String, ()>>,
    tag_index: DashMap<String, Vec<String>>,
    max_entries: usize,
}

impl MemoryCacheHandler {
    pub fn with_config(config: &MemoryConfig) -> Self {
        #[expect(clippy::expect_used, reason = "Value is clamped to >= 1, guaranteed non-zero")]
        let max_entries = NonZeroUsize::new(config.max_entries.max(1)).expect("clamped to >= 1");
        tracing::debug!(
            max_entries = max_entries.get(),
            default_ttl_secs = config.default_ttl,
            "memory cache handler initialized"
        );
        Self {
            cache: DashMap::new(),
            lru: Mutex::new(LruCache::new(max_entries)),
            tag_index: DashMap::new(),
            max_entries: max_entries.get(),
        }
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    fn expires_at(ttl_secs: u64) -> Option<Instant> {
        if ttl_secs == 0 {
            Some(Instant::now())
        } else {
            Instant::now().checked_add(Duration::from_secs(ttl_secs))
        }
    }

    fn entry_is_expired(entry: &MemEntry) -> bool {
        entry.expires_at.is_some_and(|expires_at| Instant::now() >= expires_at)
    }

    fn touch_lru(&self, key: &str) {
        self.lru.lock().promote(key);
    }

    fn remove_entry(&self, key: &str) -> Option<MemEntry> {
        let (_key, entry) = self.cache.remove(key)?;
        self.remove_from_tag_index(key, &entry.tags);
        self.lru.lock().pop(key);
        Some(entry)
    }

    fn evict_lru_with_entry(&self) -> Option<MemEntry> {
        let key = {
            let mut lru = self.lru.lock();
            lru.pop_lru().map(|(k, ())| k)
        }?;
        let entry = self.remove_entry(&key);
        tracing::debug!(key = %key, "memory cache LRU eviction");
        entry
    }

    fn insert_tag_index(&self, key: &str, tags: &[String]) {
        for tag in tags {
            self.tag_index.entry(tag.clone()).or_default().push(key.to_string());
        }
    }

    fn remove_from_tag_index(&self, key: &str, tags: &[String]) {
        for tag in tags {
            if let Some(mut keys) = self.tag_index.get_mut(tag) {
                keys.retain(|k| k != key);
                if keys.is_empty() {
                    drop(keys);
                    self.tag_index.remove(tag);
                }
            }
        }
    }
}

impl Default for MemoryCacheHandler {
    fn default() -> Self {
        Self::with_config(&MemoryConfig::default())
    }
}

#[async_trait::async_trait]
impl CacheHandler for MemoryCacheHandler {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let (snapshot, observed_expires_at) = match self.cache.get(key) {
            Some(entry) => {
                let expired = Self::entry_is_expired(&entry);
                let bytes = if expired { None } else { Some(entry.bytes.clone()) };
                (bytes, Some((entry.expires_at, expired)))
            }
            None => (None, None),
        };

        match snapshot {
            Some(bytes) => {
                self.touch_lru(key);
                tracing::debug!(key = %key, size_bytes = bytes.len(), "memory cache hit");
                Ok(Some(bytes))
            }
            None => {
                let existed = observed_expires_at.is_some();
                let msg = if existed {
                    "memory cache miss (expired); evicting"
                } else {
                    "memory cache miss (not present)"
                };
                tracing::debug!(key = %key, msg);

                if let Some((observed_expires, observed_was_expired)) = observed_expires_at {
                    let still_same = match self.cache.get(key) {
                        Some(current) => current.expires_at == observed_expires,
                        None => false,
                    };
                    if !still_same {
                        tracing::warn!(
                            key = %key,
                            "memory cache entry changed under us; skipping invalidate"
                        );
                    } else if !observed_was_expired {
                        tracing::warn!(
                            key = %key,
                            "memory cache entry no longer expired; skipping invalidate"
                        );
                    } else {
                        self.invalidate(key).await?;
                    }
                }
                Ok(None)
            }
        }
    }

    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl_secs: u64,
    ) -> Result<SetOutcome, CacheError> {
        self.set_with_tags(key, value, ttl_secs, &[]).await
    }

    async fn set_with_tags(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl_secs: u64,
        tags: &[String],
    ) -> Result<SetOutcome, CacheError> {
        tracing::debug!(
            key = %key,
            size_bytes = value.len(),
            ttl_secs,
            tag_count = tags.len(),
            "memory cache set_with_tags"
        );
        let replaced = self.remove_entry(key).is_some();
        let mut evicted = 0usize;
        let mut evicted_bytes = 0usize;

        let mut lru = self.lru.lock();
        if !replaced && lru.len() >= self.max_entries {
            drop(lru);
            if let Some(entry) = self.evict_lru_with_entry() {
                evicted += 1;
                evicted_bytes = evicted_bytes.saturating_add(entry.bytes.len());
            }
            lru = self.lru.lock();
        }

        let entry =
            MemEntry { bytes: value, expires_at: Self::expires_at(ttl_secs), tags: tags.to_vec() };
        self.cache.insert(key.to_string(), entry);
        self.insert_tag_index(key, tags);

        if let Some((evicted_key, ())) = lru.push(key.to_string(), ()) {
            drop(lru);
            if let Some(prev) = self.remove_entry(&evicted_key) {
                evicted += 1;
                evicted_bytes = evicted_bytes.saturating_add(prev.bytes.len());
            }
        }

        Ok(SetOutcome { replaced, evicted, evicted_bytes })
    }

    async fn invalidate(&self, key: &str) -> Result<bool, CacheError> {
        if self.remove_entry(key).is_none() {
            tracing::debug!(key = %key, "memory cache invalidate (no-op, not present)");
            return Ok(false);
        }

        tracing::debug!(key = %key, "memory cache invalidate");

        Ok(true)
    }

    async fn invalidate_by_tag(&self, tag: &str) -> Result<(), CacheError> {
        let keys: Vec<String> =
            self.tag_index.get(tag).map(|e| e.value().clone()).unwrap_or_default();
        tracing::debug!(tag = %tag, key_count = keys.len(), "memory cache invalidate_by_tag");
        for key in keys {
            self.invalidate(&key).await?;
        }
        let empty = self.tag_index.get(tag).map(|e| e.value().is_empty()).unwrap_or(true);
        if empty {
            self.tag_index.remove(tag);
        }
        Ok(())
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let n = self.cache.len();
        self.cache.clear();
        self.tag_index.clear();
        let mut lru = self.lru.lock();
        lru.clear();
        tracing::debug!(cleared_entries = n, "memory cache clear");
        Ok(())
    }

    async fn clear_prefix(&self, prefix: &str) -> Result<usize, CacheError> {
        let prefix = prefix.to_owned();
        let mut removed_entries: Vec<(String, Vec<String>)> = Vec::new();
        self.cache.retain(|key, entry| {
            if key.starts_with(&prefix) {
                removed_entries.push((key.clone(), entry.tags.clone()));
                false
            } else {
                true
            }
        });
        let removed = removed_entries.len();
        {
            let mut lru = self.lru.lock();
            for (key, _) in &removed_entries {
                lru.pop(key);
            }
        }
        for (key, tags) in &removed_entries {
            self.remove_from_tag_index(key, tags);
        }
        if removed > 0 {
            tracing::debug!(prefix = %prefix, removed, "memory cache clear_prefix");
        }
        Ok(removed)
    }

    fn get_all_keys(&self) -> Vec<String> {
        self.cache.iter().map(|e| e.key().clone()).collect()
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate boundary type: backends construct this exhaustively; in-tree so no semver isolation"
)]
pub struct NoOpCacheHandler;

#[async_trait::async_trait]
impl CacheHandler for NoOpCacheHandler {
    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        Ok(None)
    }

    async fn set(
        &self,
        _key: &str,
        _value: Vec<u8>,
        _ttl_secs: u64,
    ) -> Result<SetOutcome, CacheError> {
        Ok(SetOutcome::default())
    }

    async fn set_with_tags(
        &self,
        _key: &str,
        _value: Vec<u8>,
        _ttl_secs: u64,
        _tags: &[String],
    ) -> Result<SetOutcome, CacheError> {
        Ok(SetOutcome::default())
    }

    async fn invalidate(&self, _key: &str) -> Result<bool, CacheError> {
        Ok(false)
    }

    async fn invalidate_by_tag(&self, _tag: &str) -> Result<(), CacheError> {
        Ok(())
    }

    async fn clear(&self) -> Result<(), CacheError> {
        Ok(())
    }

    fn get_all_keys(&self) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Debug, Default)]
pub struct CacheHandlerRegistry {
    handlers: DashMap<String, Arc<dyn CacheHandler>>,
}

impl CacheHandlerRegistry {
    pub fn new() -> Self {
        Self { handlers: DashMap::new() }
    }

    pub fn register(&self, name: &str, handler: Arc<dyn CacheHandler>) {
        self.handlers.insert(name.to_string(), handler);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn CacheHandler>> {
        self.handlers.get(name).map(|e| Arc::clone(e.value()))
    }

    pub fn default_with_memory() -> Self {
        let registry = Self::new();
        registry.register("memory", Arc::new(MemoryCacheHandler::default()));
        registry.register("noop", Arc::new(NoOpCacheHandler));
        registry
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn names(&self) -> Vec<String> {
        self.handlers.iter().map(|e| e.key().clone()).collect()
    }

    pub fn from_env() -> Self {
        Self::default_with_memory()
    }

    pub fn resolve(&self, configured_name: &str) -> Arc<dyn CacheHandler> {
        self.get(configured_name).unwrap_or_else(|| {
            tracing::warn!(
                configured = %configured_name,
                "configured cache handler not registered; falling back to memory"
            );
            let entry = self.handlers.entry("memory".to_owned()).or_insert_with(|| {
                Arc::new(MemoryCacheHandler::default()) as Arc<dyn CacheHandler>
            });
            Arc::clone(&entry)
        })
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use tokio::time;

    use super::*;

    #[tokio::test]
    async fn test_memory_get_set() {
        let handler = MemoryCacheHandler::default();
        handler.set("k", b"hello".to_vec(), 60).await.unwrap();
        let got = handler.get("k").await.unwrap();
        assert_eq!(got, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_memory_ttl_expiry() {
        let handler = MemoryCacheHandler::default();
        handler.set("k", b"hello".to_vec(), 0).await.unwrap();
        time::sleep(Duration::from_millis(50)).await;
        assert_eq!(handler.get("k").await.unwrap(), None);
        assert_eq!(handler.get("k").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_memory_lru_eviction() {
        let handler =
            MemoryCacheHandler::with_config(&MemoryConfig { max_entries: 2, default_ttl: 60 });
        handler.set("a", b"1".to_vec(), 60).await.unwrap();
        handler.set("b", b"2".to_vec(), 60).await.unwrap();
        let _ = handler.get("a").await.unwrap();
        handler.set("c", b"3".to_vec(), 60).await.unwrap();
        assert_eq!(handler.get("a").await.unwrap(), Some(b"1".to_vec()));
        assert_eq!(handler.get("b").await.unwrap(), None);
        assert_eq!(handler.get("c").await.unwrap(), Some(b"3".to_vec()));
    }

    #[tokio::test]
    async fn test_memory_invalidate() {
        let handler = MemoryCacheHandler::default();
        handler.set("k", b"v".to_vec(), 60).await.unwrap();
        assert!(handler.get("k").await.unwrap().is_some());
        handler.invalidate("k").await.unwrap();
        assert_eq!(handler.get("k").await.unwrap(), None);
        handler.invalidate("k").await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_invalidate_by_tag() {
        let handler = MemoryCacheHandler::default();
        handler.set_with_tags("k1", b"a".to_vec(), 60, &["t".to_string()]).await.unwrap();
        handler.set_with_tags("k2", b"b".to_vec(), 60, &["t".to_string()]).await.unwrap();
        handler.set("k3", b"c".to_vec(), 60).await.unwrap();

        handler.invalidate_by_tag("t").await.unwrap();

        assert_eq!(handler.get("k1").await.unwrap(), None);
        assert_eq!(handler.get("k2").await.unwrap(), None);
        assert_eq!(handler.get("k3").await.unwrap(), Some(b"c".to_vec()));
    }

    #[tokio::test]
    async fn test_memory_get_all_keys() {
        let handler = MemoryCacheHandler::default();
        handler.set("a", b"1".to_vec(), 60).await.unwrap();
        handler.set("b", b"2".to_vec(), 60).await.unwrap();

        let mut keys = handler.get_all_keys();
        keys.sort();
        assert_eq!(keys, vec!["a".to_string(), "b".to_string()]);

        handler.invalidate("a").await.unwrap();
        let mut keys = handler.get_all_keys();
        keys.sort();
        assert_eq!(keys, vec!["b".to_string()]);
    }

    #[tokio::test]
    async fn test_noop_get_all_keys() {
        let handler = NoOpCacheHandler;
        handler.set("k", b"v".to_vec(), 60).await.unwrap();
        assert!(handler.get_all_keys().is_empty());
    }

    #[tokio::test]
    async fn test_memory_tag_index_cleanup() {
        let handler = MemoryCacheHandler::default();
        handler
            .set_with_tags("k1", b"a".to_vec(), 60, &["alpha".to_string(), "shared".to_string()])
            .await
            .unwrap();

        assert!(handler.tag_index.contains_key("alpha"));
        assert!(handler.tag_index.contains_key("shared"));

        handler.invalidate("k1").await.unwrap();

        assert!(!handler.tag_index.contains_key("alpha"));
        assert!(!handler.tag_index.contains_key("shared"));
    }

    #[tokio::test]
    async fn test_noop_returns_none() {
        let handler = NoOpCacheHandler;
        assert_eq!(handler.get("anything").await.unwrap(), None);
        handler.set("k", b"v".to_vec(), 60).await.unwrap();
        assert_eq!(handler.get("k").await.unwrap(), None);
        handler.set_with_tags("k", b"v".to_vec(), 60, &["t".to_string()]).await.unwrap();
        handler.invalidate("k").await.unwrap();
        handler.invalidate_by_tag("t").await.unwrap();
        handler.clear().await.unwrap();
    }

    #[tokio::test]
    async fn test_registry_get_and_register() {
        let registry = CacheHandlerRegistry::default_with_memory();
        let memory = registry.get("memory").expect("memory must be registered");
        let noop = registry.get("noop").expect("noop must be registered");
        memory.set("k", b"v".to_vec(), 60).await.unwrap();
        assert_eq!(memory.get("k").await.unwrap(), Some(b"v".to_vec()));
        assert_eq!(noop.get("k").await.unwrap(), None);

        let custom: Arc<dyn CacheHandler> = Arc::new(NoOpCacheHandler);
        registry.register("custom", Arc::clone(&custom));
        assert!(registry.get("custom").is_some());
    }

    #[tokio::test]
    async fn test_registry_unknown_returns_none() {
        let registry = CacheHandlerRegistry::new();
        assert!(registry.get("nope").is_none());
    }

    #[tokio::test]
    async fn test_resolve_uses_configured_handler_when_registered() {
        let registry = CacheHandlerRegistry::default_with_memory();
        let handler = registry.resolve("memory");
        handler.set("k", b"v".to_vec(), 60).await.unwrap();
        assert_eq!(handler.get("k").await.unwrap(), Some(b"v".to_vec()));
    }

    #[tokio::test]
    async fn test_resolve_falls_back_to_memory_when_handler_unknown() {
        let registry = CacheHandlerRegistry::default_with_memory();
        let handler = registry.resolve("no-such-handler");
        handler.set("k", b"v".to_vec(), 60).await.unwrap();
        assert_eq!(handler.get("k").await.unwrap(), Some(b"v".to_vec()));
    }

    #[tokio::test]
    async fn test_resolve_picks_registered_custom_handler() {
        let registry = CacheHandlerRegistry::default_with_memory();
        let noop: Arc<dyn CacheHandler> = Arc::new(NoOpCacheHandler);
        registry.register("custom", noop);

        let resolved = registry.resolve("custom");
        resolved.set("k", b"v".to_vec(), 60).await.unwrap();
        assert_eq!(resolved.get("k").await.unwrap(), None);
    }
}
