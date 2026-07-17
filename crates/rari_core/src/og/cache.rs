use std::{
    collections::hash_map::DefaultHasher,
    fs as std_fs,
    hash::{Hash, Hasher},
    io::ErrorKind::NotFound,
    path::{Path, PathBuf},
    sync::Arc,
};

use rari_error::RariError;
use tokio::{fs, task};

use crate::{
    cache::{
        MemoryConfig,
        handler::{CacheError, CacheHandler, MemoryCacheHandler},
    },
    config::Config,
};

const OG_TTL_SECS: u64 = 60 * 60 * 24 * 365 * 10;
const KEY_PREFIX: &str = "og:";

pub struct OgImageCache {
    handler: Arc<dyn CacheHandler>,
    cache_dir: PathBuf,
}

impl OgImageCache {
    pub fn new(memory_capacity: usize, project_path: &Path) -> Self {
        let handler = MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: memory_capacity.max(1),
            default_ttl: 0,
        });
        Self::with_handler(Arc::new(handler), project_path)
    }

    pub fn with_handler(handler: Arc<dyn CacheHandler>, project_path: &Path) -> Self {
        let cache_dir = Self::resolve_cache_dir(project_path);
        Self { handler, cache_dir }
    }

    fn ns(key: &str) -> String {
        format!("{KEY_PREFIX}{key}")
    }

    fn resolve_cache_dir(project_path: &Path) -> PathBuf {
        let is_production = Config::get().map(Config::is_production).unwrap_or(false);

        if is_production {
            PathBuf::from("/tmp/rari-og-cache")
        } else {
            project_path.join(".cache").join("og")
        }
    }

    async fn ensure_cache_dir(&self) {
        let dir = self.cache_dir.clone();
        let result = task::spawn_blocking(move || std_fs::create_dir_all(&dir)).await;
        if let Ok(Err(e)) = result {
            tracing::error!("Failed to create OG cache directory: {}", e);
        }
    }

    fn cache_filename(&self, key: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        self.cache_dir.join(format!("{hash:x}.webp"))
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        if let Ok(Some(bytes)) = self.handler.get(&Self::ns(key)).await {
            return Some(bytes);
        }

        let path = self.cache_filename(key);
        if let Ok(data) = fs::read(&path).await {
            if let Err(e) = self.handler.set(&Self::ns(key), data.clone(), OG_TTL_SECS).await {
                tracing::debug!("OG image cache write-through to handler failed: {}", e);
            }
            return Some(data);
        }

        None
    }

    #[expect(clippy::missing_errors_doc)]
    pub async fn insert(&self, key: String, value: Vec<u8>) -> Result<(), RariError> {
        self.ensure_cache_dir().await;

        let path = self.cache_filename(&key);
        fs::write(&path, &value)
            .await
            .map_err(|e| RariError::io(format!("Failed to write OG cache file: {e}")))?;

        self.handler
            .set(&Self::ns(&key), value, OG_TTL_SECS)
            .await
            .map_err(|e| RariError::cache(format!("Failed to store OG image in cache: {e}")))?;
        Ok(())
    }

    #[expect(clippy::missing_errors_doc)]
    pub async fn remove(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let prev = match self.handler.get(&Self::ns(key)).await {
            Ok(Some(bytes)) => Some(bytes),
            _ => None,
        };

        if let Err(e) = self.handler.invalidate(&Self::ns(key)).await {
            tracing::error!(key = %key, error = %e, "Failed to invalidate OG image in handler");
            return Err(e);
        }

        let path = self.cache_filename(key);
        match fs::remove_file(&path).await {
            Ok(()) => Ok(prev),
            Err(e) if e.kind() == NotFound => Ok(prev),
            Err(e) => {
                tracing::error!(key = %key, path = %path.display(), error = %e, "Failed to remove OG image from disk cache");
                Err(CacheError::Io(e))
            }
        }
    }

    #[expect(clippy::missing_errors_doc)]
    pub async fn clear(&self) -> Result<(), CacheError> {
        self.handler.clear_prefix(KEY_PREFIX).await?;

        let mut entries = match fs::read_dir(&self.cache_dir).await {
            Ok(e) => e,
            Err(e) if e.kind() == NotFound => return Ok(()),
            Err(e) => {
                tracing::error!(path = %self.cache_dir.display(), error = %e, "Failed to read OG cache dir for clear");
                return Err(CacheError::Io(e));
            }
        };

        let mut first_err: Option<CacheError> = None;
        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(e)) => e,
                Ok(None) => break,
                Err(e) => {
                    if first_err.is_none() {
                        first_err = Some(CacheError::Io(e));
                    }
                    break;
                }
            };

            if entry.path().extension().map(|e| e == "webp").unwrap_or(false) {
                let path = entry.path();
                if let Err(e) = fs::remove_file(&path).await
                    && first_err.is_none()
                {
                    first_err = Some(CacheError::Io(e));
                }
            }
        }

        if let Some(e) = first_err { Err(e) } else { Ok(()) }
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::env::temp_dir;

    use super::*;
    use crate::cache::handler::MemoryCacheHandler;

    fn test_project_path(test_name: &str) -> PathBuf {
        temp_dir().join(format!("rari-test-og-cache-{test_name}"))
    }

    fn fresh_cache(test_name: &str, memory_capacity: usize) -> OgImageCache {
        let handler = Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: memory_capacity.max(1),
            default_ttl: 0,
        }));
        OgImageCache::with_handler(handler, &test_project_path(test_name))
    }

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let cache = fresh_cache("basic", 5);
        let data = vec![1, 2, 3, 4, 5];

        cache.insert("/test/route".to_string(), data.clone()).await.expect("insert");
        assert_eq!(cache.get("/test/route").await, Some(data));
        cache.clear().await.expect("clear");
    }

    #[tokio::test]
    async fn test_cache_remove() {
        let cache = fresh_cache("remove", 5);
        let data = vec![1, 2, 3, 4, 5];

        cache.insert("/test/route".to_string(), data.clone()).await.expect("insert");
        assert_eq!(cache.remove("/test/route").await.expect("remove"), Some(data));
        assert!(cache.get("/test/route").await.is_none());
    }

    #[tokio::test]
    async fn test_disk_persistence() {
        let cache = fresh_cache("persistence", 1);
        let data = vec![10, 20, 30, 40, 50];

        cache.insert("/route1".to_string(), data.clone()).await.expect("insert");
        cache.insert("/route2".to_string(), vec![1, 2, 3]).await.expect("insert");

        assert_eq!(cache.get("/route1").await, Some(data));
        cache.clear().await.expect("clear");
    }

    #[tokio::test]
    async fn test_handler_round_trip() {
        let cache = fresh_cache("handler-round-trip", 8);
        let payload = b"webp-bytes".to_vec();

        cache.insert("k1".to_string(), payload.clone()).await.expect("insert");
        assert_eq!(cache.get("k1").await, Some(payload));
        cache.clear().await.expect("clear");
    }

    #[tokio::test]
    async fn test_handler_fallback_to_disk() {
        let project_path = test_project_path("fallback-to-disk");
        let _ = std_fs::remove_dir_all(&project_path);

        let handler_a = Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: 8,
            default_ttl: 0,
        }));
        let cache_a = OgImageCache::with_handler(handler_a, &project_path);

        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        cache_a.insert("/persistent".to_string(), payload.clone()).await.expect("insert");
        cache_a.get("/persistent").await.expect("cache_a in-memory hit");
        drop(cache_a);

        let handler_b = Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: 8,
            default_ttl: 0,
        }));
        let cache_b = OgImageCache::with_handler(
            Arc::clone(&handler_b) as Arc<dyn CacheHandler>,
            &project_path,
        );

        let from_disk = cache_b.get("/persistent").await;
        assert_eq!(from_disk, Some(payload.clone()), "expected disk-fallback hit");

        let in_new_handler = handler_b.get("og:/persistent").await.unwrap();
        assert_eq!(in_new_handler, Some(payload.clone()), "write-through to new handler missing");

        cache_b.clear().await.expect("clear");
        let _ = std_fs::remove_dir_all(&project_path);
    }
}
