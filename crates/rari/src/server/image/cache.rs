#![expect(clippy::exhaustive_structs)]

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    env, fs,
    fs::OpenOptions,
    io::{self, Write},
    num::NonZeroUsize,
    path::{Component, Path, PathBuf},
    process,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize, rancor};
use tokio::task;

use super::types::ImageFormat;
use crate::server::{
    cache::{
        MemoryConfig,
        handler::{CacheHandler, MemoryCacheHandler},
    },
    core::utils::path_validation::{canonicalize_or_create_dir, resolve_under_base},
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
const DEFAULT_MAX_BYTES: usize = 100 * 1024 * 1024;

pub struct ImageCache {
    handler: Arc<dyn CacheHandler>,
    cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new(project_path: &Path) -> Self {
        #[expect(clippy::expect_used, reason = "Value is clamped to >= 20, guaranteed non-zero")]
        let capacity = NonZeroUsize::new((DEFAULT_MAX_BYTES / 1024 / 50).max(20))
            .expect("capacity is always at least 20");
        let handler = MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: capacity.get(),
            default_ttl: 0,
            max_bytes: DEFAULT_MAX_BYTES,
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

    async fn ensure_cache_dir(&self) {
        let dir = self.cache_dir.clone();
        let result = task::spawn_blocking(move || {
            let canonical_dir = canonicalize_or_create_dir(&dir)
                .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e.to_string()))?;
            fs::create_dir_all(&canonical_dir)?;
            #[cfg(unix)]
            {
                fs::set_permissions(&canonical_dir, fs::Permissions::from_mode(0o700))?;
            }
            Ok::<(), io::Error>(())
        })
        .await;
        if let Ok(Err(e)) = result {
            tracing::error!("Failed to create image cache directory: {}", e);
        }
    }

    fn resolve_cache_dir(project_path: &Path) -> PathBuf {
        let is_production = env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

        if is_production {
            return env::temp_dir().join("rari-image-cache");
        }

        let Some(base) = Self::validated_project_base(project_path) else {
            return env::temp_dir().join("rari-image-cache");
        };

        let cache_dir = base.join(".cache").join("images");
        if !cache_dir.starts_with(&base) {
            return env::temp_dir().join("rari-image-cache");
        }
        cache_dir
    }

    fn validated_project_base(project_path: &Path) -> Option<PathBuf> {
        let as_str = project_path.to_string_lossy();
        if as_str.contains("..") {
            return None;
        }
        if project_path.components().any(|c| matches!(c, Component::ParentDir)) {
            return None;
        }

        match fs::canonicalize(project_path) {
            Ok(canonical) if !canonical.to_string_lossy().contains("..") => Some(canonical),
            _ => None,
        }
    }

    fn cache_leaf_name(key: &str) -> String {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:x}.cache", hasher.finish())
    }

    fn cache_leaf_for_key(key: &str) -> Option<String> {
        let file_name = Self::cache_leaf_name(key);
        if Self::is_safe_cache_leaf(&file_name) { Some(file_name) } else { None }
    }

    fn is_safe_cache_leaf(file_name: &str) -> bool {
        if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
            return false;
        }
        if file_name.contains('\0') || file_name.is_empty() {
            return false;
        }
        let Some((stem, ext)) = file_name.split_once('.') else {
            return false;
        };
        ext == "cache" && !stem.is_empty() && stem.chars().all(|c| c.is_ascii_hexdigit())
    }

    fn resolve_disk_path(cache_root: &Path, leaf: &str) -> io::Result<PathBuf> {
        if !Self::is_safe_cache_leaf(leaf) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "refusing unsafe cache leaf name",
            ));
        }
        resolve_under_base(cache_root, leaf)
            .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e.to_string()))
    }

    fn write_cache_file_atomic(cache_root: &Path, leaf: &str, data: &[u8]) -> io::Result<()> {
        let path = Self::resolve_disk_path(cache_root, leaf)?;
        let canonical_root = canonicalize_or_create_dir(cache_root)
            .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e.to_string()))?;

        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let tmp_name = format!(".{leaf}.tmp-{}-{nanos}", process::id());
        if tmp_name.contains("..")
            || tmp_name.contains('/')
            || tmp_name.contains('\\')
            || tmp_name.contains('\0')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "refusing unsafe cache temp name",
            ));
        }
        let tmp_path = canonical_root.join(&tmp_name);
        if !tmp_path.starts_with(&canonical_root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "refusing temp path outside image cache directory",
            ));
        }

        let write_tmp = || -> io::Result<()> {
            let mut opts = OpenOptions::new();
            opts.write(true).create_new(true);
            #[cfg(unix)]
            {
                opts.mode(0o600);
                opts.custom_flags(libc::O_NOFOLLOW);
            }
            let mut file = opts.open(&tmp_path)?;
            file.write_all(data)?;
            file.sync_all()?;
            Ok(())
        };

        if let Err(e) = write_tmp() {
            let _ = fs::remove_file(&tmp_path);
            return Err(e);
        }

        fs::rename(&tmp_path, &path).inspect_err(|_| {
            let _ = fs::remove_file(&tmp_path);
        })
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

        let leaf = Self::cache_leaf_for_key(key)?;
        let cache_root = self.cache_dir.clone();
        let read_result = task::spawn_blocking(move || {
            let path = Self::resolve_disk_path(&cache_root, &leaf)?;
            fs::read(path)
        })
        .await
        .ok()?
        .ok()?;

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
        let Some(leaf) = Self::cache_leaf_for_key(&key) else {
            tracing::error!("Refusing to write image cache path outside cache directory");
            return;
        };

        let cache_root = self.cache_dir.clone();
        let data_for_blocking = serialized.clone();
        let write_result = task::spawn_blocking(move || {
            Self::write_cache_file_atomic(&cache_root, &leaf, &data_for_blocking)
        })
        .await;
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

    fn fresh_cache(test_name: &str) -> ImageCache {
        let handler = Arc::new(MemoryCacheHandler::with_config(&MemoryConfig {
            max_entries: 32,
            default_ttl: 0,
            ..Default::default()
        }));
        ImageCache::with_handler(handler, &test_project_path(test_name))
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
        let cache = fresh_cache("round-trip");
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
        let cache_a = ImageCache::with_handler(handler_a, &project_path);
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
            &project_path,
        );

        let got = cache_b.get("persistent").await.expect("expected disk-fallback hit in cache_b");
        assert_eq!(got.data, image.data);

        let in_handler_b = handler_b.get("image:persistent").await.unwrap();
        assert!(in_handler_b.is_some(), "write-through to handler_b missing");

        let _ = fs::remove_dir_all(&project_path);
    }

    #[test]
    fn test_is_safe_cache_leaf() {
        assert!(ImageCache::is_safe_cache_leaf("abc123.cache"));
        assert!(!ImageCache::is_safe_cache_leaf("../etc/passwd"));
        assert!(!ImageCache::is_safe_cache_leaf("abc/def.cache"));
        assert!(!ImageCache::is_safe_cache_leaf("abc\\def.cache"));
        assert!(!ImageCache::is_safe_cache_leaf("not-hex.cache"));
        assert!(!ImageCache::is_safe_cache_leaf("abc123.webp"));
    }

    #[test]
    fn test_resolve_cache_dir_rejects_parent_components() {
        let unsafe_path = PathBuf::from("/tmp/rari-cache-test/../../etc");
        let resolved = ImageCache::resolve_cache_dir(&unsafe_path);
        assert!(
            resolved.starts_with(temp_dir()),
            "expected safe fallback under temp_dir, got {}",
            resolved.display()
        );
        assert!(
            !resolved.components().any(|c| matches!(c, Component::ParentDir)),
            "fallback must not retain parent-dir components"
        );
    }

    #[test]
    fn test_resolve_cache_dir_keeps_safe_project_suffix() {
        let project = test_project_path("safe-resolve");
        let _ = fs::create_dir_all(&project);
        let resolved = ImageCache::resolve_cache_dir(&project);
        assert!(resolved.ends_with(Path::new(".cache/images")) || resolved.ends_with("images"));
        assert!(resolved.starts_with(fs::canonicalize(&project).unwrap()));
        let _ = fs::remove_dir_all(&project);
    }

    #[tokio::test]
    async fn test_handler_invalidate_drops_both_tiers() {
        let cache = fresh_cache("invalidate");
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
