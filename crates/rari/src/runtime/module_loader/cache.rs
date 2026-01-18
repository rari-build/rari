use crate::error::RariError;
use crate::runtime::module_loader::config::CacheStats;
use dashmap::DashMap;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::Instant;

#[derive(Debug, Clone)]
struct CacheEntry {
    value: JsonValue,
    last_accessed: Instant,
    access_count: usize,
}

#[derive(Debug)]
struct ThreadSafeCache {
    entries: Arc<RwLock<FxHashMap<String, CacheEntry>>>,
    access_order: Arc<RwLock<VecDeque<String>>>,
    max_size: usize,
    hit_count: AtomicUsize,
    miss_count: AtomicUsize,
    eviction_count: AtomicUsize,
}

impl ThreadSafeCache {
    fn new(max_size: usize) -> Self {
        let cache = Self {
            entries: Arc::new(RwLock::new(FxHashMap::default())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            hit_count: AtomicUsize::new(0),
            miss_count: AtomicUsize::new(0),
            eviction_count: AtomicUsize::new(0),
        };

        cache.start_cleanup_task();
        cache
    }

    fn start_cleanup_task(&self) {
        let entries_clone = Arc::clone(&self.entries);
        let access_order_clone = Arc::clone(&self.access_order);

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(300));

                let max_age = std::time::Duration::from_secs(3600);
                let now = Instant::now();

                let mut entries = entries_clone.write();
                let mut order = access_order_clone.write();
                {
                    let mut to_remove = Vec::new();

                    for (key, entry) in entries.iter() {
                        if now.duration_since(entry.last_accessed) > max_age {
                            to_remove.push(key.clone());
                        }
                    }

                    for key in &to_remove {
                        entries.remove(key);
                        if let Some(pos) = order.iter().position(|x| x == key) {
                            order.remove(pos);
                        }
                    }
                }
            }
        });
    }

    pub fn get(&self, key: &str) -> Option<JsonValue> {
        let now = Instant::now();

        {
            let mut entries = self.entries.write();
            if let Some(entry) = entries.get_mut(key) {
                entry.last_accessed = now;
                entry.access_count += 1;
                self.hit_count.fetch_add(1, Ordering::Relaxed);

                let mut order = self.access_order.write();
                order.retain(|k| k != key);
                order.push_back(key.to_string());

                return Some(entry.value.clone());
            }
        }

        self.miss_count.fetch_add(1, Ordering::Relaxed);
        None
    }

    fn insert(&self, key: String, value: JsonValue) -> Result<(), RariError> {
        let now = Instant::now();

        let mut entries = self.entries.write();
        let mut order = self.access_order.write();

        if entries.len() >= self.max_size && !entries.contains_key(&key) {
            self.evict_lru(&mut entries, &mut order);
        }

        let entry = CacheEntry { value, last_accessed: now, access_count: 1 };

        entries.insert(key.clone(), entry);

        order.retain(|k| k != &key);
        order.push_back(key);

        Ok(())
    }

    fn evict_lru(&self, entries: &mut FxHashMap<String, CacheEntry>, order: &mut VecDeque<String>) {
        if let Some(lru_key) = order.pop_front() {
            entries.remove(&lru_key);
            self.eviction_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn stats(&self) -> CacheStats {
        let entries = self.entries.read();
        CacheStats {
            hits: self.hit_count.load(Ordering::Relaxed),
            misses: self.miss_count.load(Ordering::Relaxed),
            evictions: self.eviction_count.load(Ordering::Relaxed),
            size: entries.len(),
            memory_bytes: entries.len() * std::mem::size_of::<CacheEntry>(),
        }
    }

    fn clear(&self) {
        let mut entries = self.entries.write();
        let mut order = self.access_order.write();
        entries.clear();
        order.clear();
    }
}

#[derive(Debug)]
pub struct ModuleCaching {
    cache: ThreadSafeCache,
    component_source_paths: DashMap<String, String>,
}

impl ModuleCaching {
    pub fn new(cache_size: usize) -> Self {
        Self { cache: ThreadSafeCache::new(cache_size), component_source_paths: DashMap::new() }
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    pub fn get(&self, key: &str) -> Option<JsonValue> {
        self.cache.get(key)
    }

    pub fn insert(&self, key: String, value: JsonValue) -> Result<(), RariError> {
        self.cache.insert(key, value)
    }

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
        self.cache.clear();
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
        let cache = ThreadSafeCache::new(10);

        cache.insert("key1".to_string(), serde_json::json!({"value": 1})).unwrap();
        let result = cache.get("key1");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), serde_json::json!({"value": 1}));

        let result = cache.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = ThreadSafeCache::new(2);

        cache.insert("key1".to_string(), serde_json::json!(1)).unwrap();
        cache.insert("key2".to_string(), serde_json::json!(2)).unwrap();
        cache.insert("key3".to_string(), serde_json::json!(3)).unwrap();

        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ThreadSafeCache::new(10);

        cache.insert("key1".to_string(), serde_json::json!(1)).unwrap();

        cache.get("key1");
        cache.get("key2");

        let stats = cache.stats();
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
