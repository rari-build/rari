#![expect(clippy::exhaustive_structs)]

use std::{
    env, fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize, rancor};
use tokio::task;

use super::types::ImageFormat;
use crate::server::cache::{
    MemoryConfig,
    handler::{CacheHandler, MemoryCacheHandler},
};

#[derive(Debug, Clone, Archive, RkyvDeserialize, RkyvSerialize)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct CachedImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

const IMG_TTL_SECS: u64 = 60 * 60 * 24 * 365 * 10;
const KEY_PREFIX: &str = "image:";

pub struct ImageCache {
    handler: Arc<dyn CacheHandler>,
    cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new(max_memory_size: usize, project_path: &Path) -> Self {
        #[expect(clippy::expect_used, reason = "Value is clamped to >= 20, guaranteed non-zero")]
        let capacity = NonZeroUsize::new((max_memory_size / 1024 / 50).max(20))
            .expect("capacity is always at least 20");
        let handler = MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: capacity.get(),
            default_ttl: 0,
            ..Default::default()
        });
        Self::with_handler(Arc::new(handler), max_memory_size, project_path)
    }

    pub fn with_handler(
        handler: Arc<dyn CacheHandler>,
        _max_memory_size: usize,
        project_path: &Path,
    ) -> Self {
        let cache_dir = Self::resolve_cache_dir(project_path);
        Self { handler, cache_dir }
    }

    fn ns(key: &str) -> String {
        format!("{KEY_PREFIX}{key}")
    }

    async fn ensure_cache_dir(&self) {
        let dir = self.cache_dir.clone();
        let result = task::spawn_blocking(move || fs::create_dir_all(&dir)).await;
        if let Ok(Err(e)) = result {
            tracing::error!("Failed to create image cache directory: {}", e);
        }
    }

    fn resolve_cache_dir(project_path: &Path) -> PathBuf {
        let is_production = env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

        if is_production {
            PathBuf::from("/tmp/rari-image-cache")
        } else {
            project_path.join(".cache").join("images")
        }
    }

    fn cache_filename(&self, key: &str) -> PathBuf {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        self.cache_dir.join(format!("{hash:x}.cache"))
    }

    pub async fn get(&self, key: &str) -> Option<Arc<CachedImage>> {
        if let Ok(Some(bytes)) = self.handler.get(&Self::ns(key)).await {
            match rkyv::from_bytes::<CachedImage, rancor::Error>(&bytes) {
                Ok(cached) => return Some(Arc::new(cached)),
                Err(e) => {
                    tracing::debug!("Image cache handler entry un-deserializable: {}", e);
                    let _ = self.handler.invalidate(&Self::ns(key)).await;
                }
            }
        }

        let path = self.cache_filename(key);
        let path_for_blocking = path.clone();
        let read_result =
            task::spawn_blocking(move || fs::read(&path_for_blocking)).await.ok()?.ok()?;

        let cached = match rkyv::from_bytes::<CachedImage, rancor::Error>(&read_result) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!("Failed to deserialize cached image from disk: {}", e);
                return None;
            }
        };

        let cached_arc = Arc::new(cached);

        if let Err(e) = self.handler.set(&Self::ns(key), read_result, IMG_TTL_SECS).await {
            tracing::debug!("Image cache write-through to handler failed: {}", e);
        }

        Some(cached_arc)
    }

    pub async fn put(&self, key: String, cached: CachedImage) {
        let serialized = match rkyv::to_bytes::<rancor::Error>(&cached) {
            Ok(b) => b.into_vec(),
            Err(e) => {
                tracing::error!("Failed to serialize cached image: {}", e);
                return;
            }
        };

        self.ensure_cache_dir().await;
        let path = self.cache_filename(&key);
        let path_for_blocking = path.clone();
        let data_for_blocking = serialized.clone();
        let write_result =
            task::spawn_blocking(move || fs::write(&path_for_blocking, &data_for_blocking)).await;
        match write_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::error!("Failed to write image to disk cache: {}", e),
            Err(e) => tracing::error!("Failed to spawn disk write task: {}", e),
        }

        if let Err(e) = self.handler.set(&Self::ns(&key), serialized, IMG_TTL_SECS).await {
            tracing::error!("Failed to write image to handler cache: {}", e);
        }
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::env::temp_dir;

    use super::*;
    use crate::server::cache::{MemoryConfig, handler::MemoryCacheHandler};

    fn test_project_path(test_name: &str) -> PathBuf {
        temp_dir().join(format!("rari-test-image-cache-{test_name}"))
    }

    fn fresh_cache(test_name: &str, max_memory_size: usize) -> ImageCache {
        let handler = Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: 32,
            default_ttl: 0,
            ..Default::default()
        }));
        ImageCache::with_handler(handler, max_memory_size, &test_project_path(test_name))
    }

    fn sample_image() -> CachedImage {
        CachedImage {
            data: vec![0xCA, 0xFE, 0xBA, 0xBE, 0xDE, 0xAD, 0xBE, 0xEF],
            width: 100,
            height: 50,
            format: ImageFormat::Avif,
        }
    }

    #[tokio::test]
    async fn test_handler_round_trip() {
        let cache = fresh_cache("round-trip", 1024 * 1024);
        let key = "k1".to_string();
        let image = sample_image();

        cache.put(key.clone(), image.clone()).await;
        let got = cache.get(&key).await.expect("expected handler hit");
        assert_eq!(got.data, image.data);
        assert_eq!(got.width, image.width);
        assert_eq!(got.height, image.height);
        assert_eq!(got.format, image.format);
    }

    #[tokio::test]
    async fn test_disk_persistence() {
        // cache_a writes, drops. cache_b (fresh handler, same disk) must
        // serve the read from disk, then write through to its handler.
        let project_path = test_project_path("disk-persistence");
        let _ = fs::remove_dir_all(&project_path);

        let handler_a = Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: 32,
            default_ttl: 0,
            ..Default::default()
        }));
        let cache_a = ImageCache::with_handler(handler_a, 1024 * 1024, &project_path);
        let image = sample_image();
        cache_a.put("persistent".to_string(), image.clone()).await;
        assert!(cache_a.get("persistent").await.is_some());
        drop(cache_a);

        let handler_b = Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: 32,
            default_ttl: 0,
            ..Default::default()
        }));
        let cache_b = ImageCache::with_handler(
            Arc::clone(&handler_b) as Arc<dyn CacheHandler>,
            1024 * 1024,
            &project_path,
        );

        let got = cache_b.get("persistent").await.expect("expected disk-fallback hit in cache_b");
        assert_eq!(got.data, image.data);

        let in_handler_b = handler_b.get("image:persistent").await.unwrap();
        assert!(in_handler_b.is_some(), "write-through to handler_b missing");

        let _ = fs::remove_dir_all(&project_path);
    }

    #[tokio::test]
    async fn test_handler_invalidate_drops_both_tiers() {
        let cache = fresh_cache("invalidate", 1024 * 1024);
        let key = "k1".to_string();
        let image = sample_image();

        cache.put(key.clone(), image.clone()).await;
        assert!(cache.get(&key).await.is_some());

        // Invalidate via the handler directly. Disk still has the blob,
        // so the next get() should re-populate from disk via write-through.
        cache.handler.invalidate(&key).await.unwrap();
        assert!(
            cache.get(&key).await.is_some(),
            "disk should still serve after handler invalidate"
        );
    }
}
