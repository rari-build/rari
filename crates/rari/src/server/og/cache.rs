use lru::LruCache;
use parking_lot::RwLock;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

pub struct OgImageCache {
    memory_cache: RwLock<LruCache<String, Vec<u8>>>,
    cache_dir: PathBuf,
}

impl OgImageCache {
    pub fn new(memory_capacity: usize, project_path: &Path) -> Self {
        let capacity = NonZeroUsize::new(memory_capacity)
            .unwrap_or_else(|| NonZeroUsize::new(20).expect("20 is non-zero"));

        let cache_dir = Self::resolve_cache_dir(project_path);

        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::error!("Failed to create OG cache directory: {}", e);
        }

        Self { memory_cache: RwLock::new(LruCache::new(capacity)), cache_dir }
    }

    fn resolve_cache_dir(project_path: &Path) -> PathBuf {
        let is_production = std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

        if is_production {
            PathBuf::from("/tmp/rari-og-cache")
        } else {
            project_path.join("dist").join("cache").join("og")
        }
    }

    fn cache_filename(&self, key: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        self.cache_dir.join(format!("{:x}.webp", hash))
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(data) = self.memory_cache.write().get(key).cloned() {
            return Some(data);
        }

        let path = self.cache_filename(key);
        if let Ok(data) = std::fs::read(&path) {
            self.memory_cache.write().put(key.to_string(), data.clone());
            return Some(data);
        }

        None
    }

    pub fn insert(&self, key: String, value: Vec<u8>) {
        let path = self.cache_filename(&key);
        if let Err(e) = std::fs::write(&path, &value) {
            tracing::error!("Failed to write OG image to disk cache: {}", e);
        }

        self.memory_cache.write().put(key, value);
    }

    pub fn remove(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.cache_filename(key);
        let _ = std::fs::remove_file(&path);

        self.memory_cache.write().pop(key)
    }

    pub fn clear(&self) {
        self.memory_cache.write().clear();

        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map(|e| e == "webp").unwrap_or(false) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    fn test_project_path(test_name: &str) -> PathBuf {
        temp_dir().join(format!("rari-test-og-cache-{}", test_name))
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache = OgImageCache::new(5, &test_project_path("basic"));
        let data = vec![1, 2, 3, 4, 5];

        cache.insert("/test/route".to_string(), data.clone());

        let retrieved = cache.get("/test/route");
        assert_eq!(retrieved, Some(data));

        cache.clear();
    }

    #[test]
    fn test_cache_remove() {
        let cache = OgImageCache::new(5, &test_project_path("remove"));
        let data = vec![1, 2, 3, 4, 5];

        cache.insert("/test/route".to_string(), data.clone());
        let removed = cache.remove("/test/route");

        assert_eq!(removed, Some(data));
        assert!(cache.get("/test/route").is_none());
    }

    #[test]
    fn test_disk_persistence() {
        let cache = OgImageCache::new(1, &test_project_path("persistence"));
        let data = vec![10, 20, 30, 40, 50];

        cache.insert("/route1".to_string(), data.clone());
        cache.insert("/route2".to_string(), vec![1, 2, 3]);

        let retrieved = cache.get("/route1");
        assert_eq!(retrieved, Some(data));

        cache.clear();
    }
}
