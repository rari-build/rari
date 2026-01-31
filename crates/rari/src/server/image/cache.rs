use lru::LruCache;
use parking_lot::Mutex;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::types::ImageFormat;

#[derive(Debug, Clone, Archive, RkyvDeserialize, RkyvSerialize)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct CachedImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

pub struct ImageCache {
    memory_cache: Mutex<LruCache<String, Arc<CachedImage>>>,
    cache_dir: PathBuf,
    max_memory_size: usize,
    current_memory_size: Mutex<usize>,
}

impl ImageCache {
    pub fn new(max_memory_size: usize, project_path: &Path) -> Self {
        let capacity = NonZeroUsize::new((max_memory_size / 1024 / 50).max(20))
            .expect("capacity is always at least 20");

        let cache_dir = Self::resolve_cache_dir(project_path);

        Self {
            memory_cache: Mutex::new(LruCache::new(capacity)),
            cache_dir,
            max_memory_size,
            current_memory_size: Mutex::new(0),
        }
    }

    fn ensure_cache_dir(&self) {
        if let Err(e) = std::fs::create_dir_all(&self.cache_dir) {
            tracing::error!("Failed to create image cache directory: {}", e);
        }
    }

    fn resolve_cache_dir(project_path: &Path) -> PathBuf {
        let is_production = std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

        if is_production {
            PathBuf::from("/tmp/rari-image-cache")
        } else {
            project_path.join(".cache").join("images")
        }
    }

    fn cache_filename(&self, key: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        self.cache_dir.join(format!("{:x}.cache", hash))
    }

    pub async fn get(&self, key: &str) -> Option<Arc<CachedImage>> {
        if let Some(cached) = self.memory_cache.lock().get(key).cloned() {
            return Some(cached);
        }

        let path = self.cache_filename(key);
        let data = tokio::task::spawn_blocking(move || std::fs::read(&path)).await.ok()?.ok()?;

        match rkyv::from_bytes::<CachedImage, rkyv::rancor::Error>(&data) {
            Ok(cached) => {
                let cached = Arc::new(cached);
                let data_size = cached.data.len();

                let mut cache = self.memory_cache.lock();
                let mut size = self.current_memory_size.lock();

                if *size + data_size <= self.max_memory_size {
                    cache.put(key.to_string(), cached.clone());
                    *size += data_size;
                }

                Some(cached)
            }
            Err(e) => {
                tracing::debug!("Failed to deserialize cached image: {}", e);
                None
            }
        }
    }

    pub async fn put(&self, key: String, cached: CachedImage) {
        let data_size = cached.data.len();

        let serialized = rkyv::to_bytes::<rkyv::rancor::Error>(&cached)
            .map_err(|e| tracing::error!("Failed to serialize cached image: {}", e))
            .ok();

        if let Some(serialized) = serialized {
            self.ensure_cache_dir();

            let path = self.cache_filename(&key);
            let data = serialized.into_vec();

            let write_result =
                tokio::task::spawn_blocking(move || std::fs::write(&path, &data)).await;

            match write_result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::error!("Failed to write image to disk cache: {}", e);
                }
                Err(e) => {
                    tracing::error!("Failed to spawn disk write task: {}", e);
                }
            }
        }

        let cached = Arc::new(cached);

        let mut cache = self.memory_cache.lock();
        let mut size = self.current_memory_size.lock();

        while *size + data_size > self.max_memory_size {
            if let Some((_, evicted)) = cache.pop_lru() {
                *size = size.saturating_sub(evicted.data.len());
            } else {
                break;
            }
        }

        cache.put(key, cached);
        *size += data_size;
    }
}
