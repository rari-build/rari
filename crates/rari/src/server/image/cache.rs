use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ImageCache {
    memory_cache: Mutex<LruCache<String, Arc<Vec<u8>>>>,
    cache_dir: PathBuf,
    max_memory_size: usize,
    current_memory_size: Mutex<usize>,
}

impl ImageCache {
    pub fn new(max_memory_size: usize, project_path: &Path) -> Self {
        let capacity = NonZeroUsize::new((max_memory_size / 1024 / 50).max(20))
            .expect("capacity is always at least 20");

        let cache_dir = Self::resolve_cache_dir(project_path);

        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::error!("Failed to create image cache directory: {}", e);
        }

        Self {
            memory_cache: Mutex::new(LruCache::new(capacity)),
            cache_dir,
            max_memory_size,
            current_memory_size: Mutex::new(0),
        }
    }

    fn resolve_cache_dir(project_path: &Path) -> PathBuf {
        let is_production = std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

        if is_production {
            PathBuf::from("/tmp/rari-image-cache")
        } else {
            project_path.join("dist").join("cache").join("images")
        }
    }

    fn cache_filename(&self, key: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        let ext = if key.contains("webp") {
            "webp"
        } else if key.contains("avif") {
            "avif"
        } else if key.contains("png") {
            "png"
        } else {
            "img"
        };

        self.cache_dir.join(format!("{:x}.{}", hash, ext))
    }

    pub fn get(&self, key: &str) -> Option<Arc<Vec<u8>>> {
        if let Some(data) = self.memory_cache.lock().get(key).cloned() {
            return Some(data);
        }

        let path = self.cache_filename(key);
        if let Ok(data) = std::fs::read(&path) {
            let data = Arc::new(data);

            let data_size = data.len();
            if *self.current_memory_size.lock() + data_size <= self.max_memory_size {
                self.memory_cache.lock().put(key.to_string(), data.clone());
                *self.current_memory_size.lock() += data_size;
            }

            return Some(data);
        }

        None
    }

    pub fn put(&self, key: String, data: Vec<u8>) {
        let data_size = data.len();

        let path = self.cache_filename(&key);
        if let Err(e) = std::fs::write(&path, &data) {
            tracing::error!("Failed to write image to disk cache: {}", e);
        }

        let data = Arc::new(data);

        while *self.current_memory_size.lock() + data_size > self.max_memory_size {
            let mut cache = self.memory_cache.lock();
            if let Some((_, evicted)) = cache.pop_lru() {
                let mut size = self.current_memory_size.lock();
                *size = size.saturating_sub(evicted.len());
            } else {
                break;
            }
        }

        self.memory_cache.lock().put(key, data);
        *self.current_memory_size.lock() += data_size;
    }
}
