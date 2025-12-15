use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;

pub struct ImageCache {
    cache: Mutex<LruCache<String, Arc<Vec<u8>>>>,
    max_size: usize,
    current_size: Mutex<usize>,
}

impl ImageCache {
    pub fn new(max_size: usize) -> Self {
        let capacity = NonZeroUsize::new((max_size / 1024 / 10).max(10))
            .expect("capacity is always at least 10, so NonZeroUsize creation cannot fail");
        Self { cache: Mutex::new(LruCache::new(capacity)), max_size, current_size: Mutex::new(0) }
    }

    pub fn get(&self, key: &str) -> Option<Arc<Vec<u8>>> {
        self.cache.lock().get(key).cloned()
    }

    pub fn put(&self, key: String, data: Vec<u8>) {
        let data_size = data.len();
        let data = Arc::new(data);

        while *self.current_size.lock() + data_size > self.max_size {
            let mut cache = self.cache.lock();
            if let Some((_, evicted)) = cache.pop_lru() {
                let mut size = self.current_size.lock();
                *size = size.saturating_sub(evicted.len());
            } else {
                break;
            }
        }

        self.cache.lock().put(key, data);
        *self.current_size.lock() += data_size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = ImageCache::new(1024 * 1024);
        let key = "test".to_string();
        let data = vec![1, 2, 3, 4, 5];

        cache.put(key.clone(), data.clone());
        let retrieved = cache.get(&key).unwrap();
        assert_eq!(*retrieved, data);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = ImageCache::new(100);
        cache.put("key1".to_string(), vec![0; 60]);
        cache.put("key2".to_string(), vec![0; 60]);

        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
    }
}
