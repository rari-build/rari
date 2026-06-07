use crate::error::RariError;
use crate::runtime::module_loader::config::CacheStats;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use serde_json::Value as JsonValue;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct TimedEntry {
    value: JsonValue,
    last_accessed: Instant,
}

#[derive(Debug)]
pub struct ModuleCaching {
    cache: Mutex<LruCache<String, TimedEntry>>,
    max_age_secs: u64,
    hit_count: AtomicUsize,
    miss_count: AtomicUsize,
    eviction_count: AtomicUsize,
    component_source_paths: DashMap<String, String>,
}

impl std::fmt::Debug for TimedEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimedEntry")
            .field("last_accessed", &self.last_accessed)
            .finish_non_exhaustive()
    }
}

impl ModuleCaching {
    pub fn new(cache_size: usize) -> Self {
        let cap = NonZeroUsize::new(cache_size)
            .unwrap_or(NonZeroUsize::new(5000).expect("5000 is non-zero"));
        Self {
            cache: Mutex::new(LruCache::new(cap)),
            max_age_secs: 3600,
            hit_count: AtomicUsize::new(0),
            miss_count: AtomicUsize::new(0),
            eviction_count: AtomicUsize::new(0),
            component_source_paths: DashMap::new(),
        }
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let cache = self.cache.lock();
        CacheStats {
            hits: self.hit_count.load(Ordering::Relaxed),
            misses: self.miss_count.load(Ordering::Relaxed),
            evictions: self.eviction_count.load(Ordering::Relaxed),
            size: cache.len(),
            memory_bytes: cache.len() * std::mem::size_of::<TimedEntry>(),
        }
    }

    pub fn get(&self, key: &str) -> Option<JsonValue> {
        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(key) {
            if entry.last_accessed.elapsed().as_secs() > self.max_age_secs {
                cache.pop(key);
                self.miss_count.fetch_add(1, Ordering::Relaxed);
                self.eviction_count.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            let value = entry.value.clone();
            if let Some(entry) = cache.get_mut(key) {
                entry.last_accessed = Instant::now();
            }
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            Some(value)
        } else {
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn insert(&self, key: String, value: JsonValue) -> Result<(), RariError> {
        let mut cache = self.cache.lock();
        let was_full = cache.len() == cache.cap().get() && cache.peek(&key).is_none();
        cache.push(key, TimedEntry { value, last_accessed: Instant::now() });
        if was_full {
            self.eviction_count.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
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

    pub fn clear(&self) {
        self.cache.lock().clear();
        self.component_source_paths.clear();
    }

    pub fn clear_component(&self, component_id: &str) {
        self.component_source_paths.remove(component_id);
    }
}

impl Default for ModuleCaching {
    fn default() -> Self {
        Self::new(5000)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = ModuleCaching::new(10);

        cache.insert("key1".to_string(), serde_json::json!({"value": 1})).unwrap();
        let result = cache.get("key1");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), serde_json::json!({"value": 1}));

        let result = cache.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = ModuleCaching::new(2);

        cache.insert("key1".to_string(), serde_json::json!(1)).unwrap();
        cache.insert("key2".to_string(), serde_json::json!(2)).unwrap();
        cache.insert("key3".to_string(), serde_json::json!(3)).unwrap();

        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ModuleCaching::new(10);

        cache.insert("key1".to_string(), serde_json::json!(1)).unwrap();

        cache.get("key1");
        cache.get("key2");

        let stats = cache.get_cache_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_module_caching() {
        let caching = ModuleCaching::new(10);

        caching.set_component_source_path("comp1".to_string(), "/path/to/comp1".to_string());
        assert_eq!(caching.get_component_source_path("comp1"), Some("/path/to/comp1".to_string()));

        caching.insert("module1".to_string(), serde_json::json!({"data": "test"})).unwrap();
        assert!(caching.get("module1").is_some());
    }
}
