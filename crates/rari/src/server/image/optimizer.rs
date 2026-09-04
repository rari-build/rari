#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]
use std::{
    io,
    path::{Path, PathBuf},
    sync::{
        Arc, PoisonError, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use cow_utils::CowUtils;
use futures::stream::{self, StreamExt};
use image::{DynamicImage, imageops::FilterType};
use rari_error::RariError;
use reqwest::{Client, header::LOCATION, redirect::Policy};
use tokio::{fs, io::AsyncReadExt, sync::Semaphore, task};
use url::Url;

use super::{
    ImageError,
    cache::{self, ImageCache},
    config::{ImageConfig, LocalPattern, RemotePattern},
    types::{DEFAULT_IMAGE_QUALITY, ImageFormat, OptimizeParams, OptimizedImage},
};
use crate::{
    server::core::utils::path_validation::validate_safe_path,
    utils::{cast, float},
};

const MAX_SOURCE_IMAGE_SIZE: usize = 10 * 1024 * 1024;
const MAX_STATIC_IMAGE_SOURCE_MAP_SIZE: usize = 10 * 1024 * 1024;
const MAX_OUTPUT_WIDTH: u32 = 3840;
const MAX_OUTPUT_HEIGHT: u32 = 2160;
const AVIF_ENCODING_SPEED: u8 = 6;
const DEFAULT_CONCURRENCY: usize = 4;

fn encode_url_path_segments(path: &str) -> String {
    let had_leading_slash = path.starts_with('/');
    let encoded = path
        .trim_start_matches('/')
        .split('/')
        .map(|segment| urlencoding::encode(segment).into_owned())
        .collect::<Vec<_>>()
        .join("/");
    if had_leading_slash { format!("/{encoded}") } else { encoded }
}

fn decode_url_path_segments(path: &str) -> String {
    let had_leading_slash = path.starts_with('/');
    let decoded = path
        .trim_start_matches('/')
        .split('/')
        .map(|segment| {
            urlencoding::decode(segment)
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_else(|_| segment.to_string())
        })
        .collect::<Vec<_>>()
        .join("/");
    if had_leading_slash { format!("/{decoded}") } else { decoded }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PreloadImage {
    pub url: String,
    pub width: u32,
    pub quality: u8,
    pub format: ImageFormat,
}

pub struct ImageOptimizer {
    cache: Arc<ImageCache>,
    config: ImageConfig,
    http_client: Client,
    project_path: PathBuf,
    processing_semaphore: Arc<Semaphore>,
    concurrency: usize,
    preload_images: Arc<RwLock<Vec<PreloadImage>>>,
}

impl ImageOptimizer {
    pub fn new(config: ImageConfig, project_path: &Path) -> Self {
        let cache = Arc::new(ImageCache::new(project_path));
        Self::with_cache(config, project_path, cache)
    }

    pub fn with_cache(config: ImageConfig, project_path: &Path, cache: Arc<ImageCache>) -> Self {
        #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
        let http_client = Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        let mut concurrency = config.optimization_concurrency.unwrap_or(DEFAULT_CONCURRENCY);
        if concurrency == 0 {
            tracing::warn!("optimization_concurrency is 0, clamping to 1");
            concurrency = 1;
        }
        let processing_semaphore = Arc::new(Semaphore::new(concurrency));

        Self {
            cache,
            config,
            http_client,
            project_path: project_path.to_path_buf(),
            processing_semaphore,
            concurrency,
            preload_images: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn out_dir_path(&self) -> PathBuf {
        let path = Path::new(&self.config.out_dir);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            let relative = self.config.out_dir.trim_matches(|c| c == '/' || c == '\\');
            self.project_path.join(relative)
        }
    }

    fn default_quality(&self) -> u8 {
        if self.config.quality_allowlist.is_empty()
            || self.config.quality_allowlist.contains(&DEFAULT_IMAGE_QUALITY)
        {
            DEFAULT_IMAGE_QUALITY
        } else {
            *self.config.quality_allowlist.first().unwrap_or(&DEFAULT_IMAGE_QUALITY)
        }
    }

    pub fn get_preload_links(&self) -> Vec<String> {
        let preload_images = self.preload_images.read().unwrap_or_else(PoisonError::into_inner);
        preload_images
            .iter()
            .map(|img| {
                format!(
                    r#"<link rel="preload" as="image" href="/_image?url={}&w={}&q={}&f={}" />"#,
                    urlencoding::encode(&img.url),
                    img.width,
                    img.quality,
                    img.format.extension()
                )
            })
            .collect()
    }

    pub fn clear_preload_images(&self) {
        let mut preload_images =
            self.preload_images.write().unwrap_or_else(PoisonError::into_inner);
        preload_images.clear();
    }

    pub async fn preoptimize_local_images(&self) -> Result<usize, ImageError> {
        self.preoptimize_local_images_internal(false).await
    }

    pub async fn preoptimize_local_images_preview(&self) -> Result<usize, ImageError> {
        self.preoptimize_local_images_internal(true).await
    }

    async fn preoptimize_local_images_internal(&self, dry_run: bool) -> Result<usize, ImageError> {
        if !self.config.preoptimize_manifest.is_empty() {
            return self.preoptimize_from_manifest(dry_run).await;
        }

        if self.config.local_patterns.is_empty() {
            tracing::debug!("No local_patterns configured, skipping local scan");
            return Ok(0);
        }

        tracing::debug!(
            "No manifest found, scanning public/ and dist/{}...",
            self.config.assets_dir
        );

        let mut image_paths = Vec::new();
        let assets_dir = self.config.assets_dir.trim_matches('/').to_string();
        let assets_url_prefix = format!("/{assets_dir}");
        let scan_roots: [(PathBuf, Option<&str>); 2] = [
            (self.project_path.join("public"), None),
            (self.out_dir_path().join(&assets_dir), Some(assets_url_prefix.as_str())),
        ];

        for (root_dir, forced_prefix) in &scan_roots {
            match fs::try_exists(root_dir).await {
                Ok(false) => continue,
                Err(e) => {
                    tracing::warn!("Failed to check image scan directory {:?}: {}", root_dir, e);
                    continue;
                }
                Ok(true) => {}
            }

            tracing::debug!("Scanning image directory: {:?}", root_dir);

            let mut dirs_to_scan = vec![root_dir.clone()];

            while let Some(current_dir) = dirs_to_scan.pop() {
                let mut entries = match fs::read_dir(&current_dir).await {
                    Ok(entries) => entries,
                    Err(e) => {
                        tracing::warn!("Failed to read directory {:?}: {}", current_dir, e);
                        continue;
                    }
                };

                loop {
                    let entry = match entries.next_entry().await {
                        Ok(Some(entry)) => entry,
                        Ok(None) => break,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to read directory entry in {:?}: {}",
                                current_dir,
                                e
                            );
                            break;
                        }
                    };
                    let path = entry.path();

                    let file_type = match entry.file_type().await {
                        Ok(file_type) => file_type,
                        Err(e) => {
                            tracing::warn!("Failed to read file type for {:?}: {}", path, e);
                            continue;
                        }
                    };

                    if file_type.is_symlink() {
                        continue;
                    }

                    #[expect(
                        clippy::filetype_is_file,
                        reason = "We specifically want only regular files, not FIFOs, sockets, or devices"
                    )]
                    if file_type.is_dir() {
                        dirs_to_scan.push(path);
                    } else if file_type.is_file() {
                        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                        if !matches!(
                            extension.cow_to_lowercase().as_ref(),
                            "jpg" | "jpeg" | "png" | "webp" | "avif" | "gif"
                        ) {
                            continue;
                        }

                        let Ok(relative) = path.strip_prefix(root_dir) else {
                            continue;
                        };
                        let relative_url =
                            relative.to_string_lossy().cow_replace('\\', "/").into_owned();
                        let encoded_relative = encode_url_path_segments(&relative_url);
                        let url_path = match forced_prefix {
                            Some(prefix) => format!("{prefix}/{encoded_relative}"),
                            None => format!("/{encoded_relative}"),
                        };

                        if self.matches_local_patterns(&url_path) {
                            image_paths.push(url_path);
                        }
                    }
                }
            }
        }

        if image_paths.is_empty() {
            image_paths = self.collect_source_map_image_urls().await?;
        }

        if image_paths.is_empty() {
            tracing::debug!("No local images found for pre-optimization");
            return Ok(0);
        }

        image_paths.sort_unstable();
        image_paths.dedup();

        tracing::debug!("Found {} local images to scan", image_paths.len());
        for path in &image_paths {
            tracing::debug!("  - {}", path);
        }

        self.optimize_image_urls_internal(image_paths, dry_run).await
    }

    async fn preoptimize_from_manifest(&self, dry_run: bool) -> Result<usize, ImageError> {
        let formats = if self.config.formats.is_empty() {
            vec![ImageFormat::Avif]
        } else {
            self.config.formats.clone()
        };

        let default_quality = self.default_quality();

        let mut tasks = Vec::new();
        let mut preload_list = Vec::new();

        for variant in &self.config.preoptimize_manifest {
            if let Err(e) = self.validate_url(&variant.src) {
                tracing::debug!("Skipping {} - validation failed: {}", variant.src, e);
                continue;
            }

            let quality = variant.quality.unwrap_or(default_quality);

            if !self.config.quality_allowlist.is_empty()
                && !self.config.quality_allowlist.contains(&quality)
            {
                tracing::debug!(
                    "Skipping {} - quality {} not in allowlist {:?}",
                    variant.src,
                    quality,
                    self.config.quality_allowlist
                );
                continue;
            }

            let should_preload = variant.preload.unwrap_or(false);

            let widths: Vec<u32> = if let Some(width) = variant.width {
                vec![width]
            } else {
                let mut sizes = self.config.device_sizes.clone();
                sizes.extend(self.config.image_sizes.clone());
                if sizes.is_empty() {
                    vec![384, 640, 750, 828, 1080, 1200, 1920]
                } else {
                    sizes.sort_unstable();
                    sizes.dedup();
                    sizes
                }
            };

            for &width in &widths {
                for &format in &formats {
                    tasks.push((variant.src.clone(), width, format, quality));

                    if should_preload && format == formats[0] {
                        preload_list.push(PreloadImage {
                            url: variant.src.clone(),
                            width,
                            quality,
                            format,
                        });
                    }
                }
            }
        }

        if tasks.is_empty() {
            tracing::debug!("No images to pre-optimize from manifest");
            return Ok(0);
        }

        if dry_run {
            tracing::info!("Starting local image pre-optimization preview (dry-run)...");
            tracing::info!("Using preoptimize manifest with {} image variants", tasks.len());
            tracing::info!("[DRY RUN] Would process {} image variants:", tasks.len());
            for (url, width, format, q) in &tasks {
                tracing::info!(
                    "  - {} (width={}, quality={}, ext={}, format={:?})",
                    url,
                    width,
                    q,
                    format.extension(),
                    format
                );
            }
            if !preload_list.is_empty() {
                tracing::info!(
                    "[DRY RUN] Would register {} images for preloading",
                    preload_list.len()
                );
            }
            return Ok(tasks.len());
        }

        let mut needs_optimization = 0;
        for (url, width, format, q) in &tasks {
            let params = OptimizeParams {
                url: url.clone(),
                w: Some(*width),
                q: *q,
                f: Some(format.extension().to_string()),
            };
            let cache_key = Self::generate_cache_key(&params);
            if self.cache.get(&cache_key).await.is_none() {
                needs_optimization += 1;
            }
        }

        if needs_optimization == 0 {
            tracing::debug!("All {} image variants are already cached", tasks.len());
            if !preload_list.is_empty() {
                let mut preload_images =
                    self.preload_images.write().unwrap_or_else(PoisonError::into_inner);
                preload_images.extend(preload_list);
                tracing::debug!("Registered {} images for preloading", preload_images.len());
            }
            return Ok(0);
        }

        tracing::info!("Starting local image pre-optimization...");
        tracing::info!("Using preoptimize manifest with {} image variants", tasks.len());
        tracing::info!(
            "Pre-optimizing {} image variants from manifest ({} already cached)",
            needs_optimization,
            tasks.len() - needs_optimization
        );

        if !preload_list.is_empty() {
            let mut preload_images =
                self.preload_images.write().unwrap_or_else(PoisonError::into_inner);
            preload_images.extend(preload_list);
            tracing::debug!("Registered {} images for preloading", preload_images.len());
        }

        let optimized_count = Arc::new(AtomicUsize::new(0));

        let results: Vec<_> = stream::iter(tasks)
            .map(|(url, width, format, q)| {
                let optimized_count = Arc::clone(&optimized_count);

                async move {
                    let params = OptimizeParams {
                        url: url.clone(),
                        w: Some(width),
                        q,
                        f: Some(format.extension().to_string()),
                    };

                    let cache_key = Self::generate_cache_key(&params);

                    if self.cache.get(&cache_key).await.is_some() {
                        return Ok::<_, ImageError>(false);
                    }

                    match self.optimize(params).await {
                        Ok(_) => {
                            optimized_count.fetch_add(1, Ordering::Relaxed);
                            Ok(true)
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to pre-optimize {} (width={}, quality={}, ext={}, format={:?}): {}",
                                url,
                                width,
                                q,
                                format.extension(),
                                format,
                                e
                            );
                            Err(e)
                        }
                    }
                }
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;

        let final_count = optimized_count.load(Ordering::Relaxed);
        let errors = results.iter().filter(|r| r.is_err()).count();

        if errors > 0 {
            tracing::warn!("Pre-optimization completed with {} errors", errors);
        }

        if final_count > 0 {
            tracing::info!("Pre-optimized {} image variants from manifest", final_count);
        }
        Ok(final_count)
    }

    async fn optimize_image_urls_internal(
        &self,
        urls: Vec<String>,
        dry_run: bool,
    ) -> Result<usize, ImageError> {
        let mut sizes = self.config.device_sizes.clone();
        sizes.extend(self.config.image_sizes.clone());

        if sizes.is_empty() {
            sizes = vec![384, 640, 750, 828, 1080, 1200, 1920];
        }

        sizes.sort_unstable();
        sizes.dedup();

        let formats = if self.config.formats.is_empty() {
            vec![ImageFormat::Avif]
        } else {
            self.config.formats.clone()
        };

        let quality = self.default_quality();

        let mut tasks = Vec::new();
        for url in &urls {
            for &width in &sizes {
                for &format in &formats {
                    tasks.push((url.clone(), width, format, quality));
                }
            }
        }

        tracing::debug!("Generated {} optimization tasks", tasks.len());

        if dry_run {
            tracing::info!("Starting local image pre-optimization preview (dry-run)...");
            tracing::info!("Found {} local images to pre-optimize", urls.len());
            tracing::info!("Pre-optimizing for {} sizes: {:?}", sizes.len(), sizes);
            tracing::info!("Pre-optimizing with quality: {}", quality);
            tracing::info!("[DRY RUN] Would process {} image variants:", tasks.len());
            for (url, width, format, q) in &tasks {
                tracing::info!(
                    "  - {} (width={}, quality={}, ext={}, format={:?})",
                    url,
                    width,
                    q,
                    format.extension(),
                    format
                );
            }
            return Ok(tasks.len());
        }

        let mut needs_optimization = 0;
        for (url, width, format, q) in &tasks {
            let params = OptimizeParams {
                url: url.clone(),
                w: Some(*width),
                q: *q,
                f: Some(format.extension().to_string()),
            };
            let cache_key = Self::generate_cache_key(&params);
            if self.cache.get(&cache_key).await.is_none() {
                needs_optimization += 1;
            }
        }

        if needs_optimization == 0 {
            tracing::debug!("All {} image variants are already cached", tasks.len());
            return Ok(0);
        }

        tracing::info!("Starting local image pre-optimization...");
        tracing::info!("Found {} local images to pre-optimize", urls.len());
        tracing::info!("Pre-optimizing for {} sizes: {:?}", sizes.len(), sizes);
        tracing::info!("Pre-optimizing with quality: {}", quality);
        tracing::info!(
            "Pre-optimizing {} image variants ({} already cached)",
            needs_optimization,
            tasks.len() - needs_optimization
        );

        let optimized_count = Arc::new(AtomicUsize::new(0));

        let results: Vec<_> = stream::iter(tasks)
            .map(|(url, width, format, q)| {
                let optimized_count = Arc::clone(&optimized_count);

                async move {
                    let params = OptimizeParams {
                        url: url.clone(),
                        w: Some(width),
                        q,
                        f: Some(format.extension().to_string()),
                    };

                    let cache_key = Self::generate_cache_key(&params);

                    if self.cache.get(&cache_key).await.is_some() {
                        return Ok::<_, ImageError>(false);
                    }

                    match self.optimize(params).await {
                        Ok(_) => {
                            optimized_count.fetch_add(1, Ordering::Relaxed);
                            Ok(true)
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to pre-optimize {} (width={}, quality={}, ext={}, format={:?}): {}",
                                url,
                                width,
                                q,
                                format.extension(),
                                format,
                                e
                            );
                            Err(e)
                        }
                    }
                }
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;

        let final_count = optimized_count.load(Ordering::Relaxed);
        let errors = results.iter().filter(|r| r.is_err()).count();

        if errors > 0 {
            tracing::warn!("Pre-optimization completed with {} errors", errors);
        }

        if final_count > 0 {
            tracing::info!("Pre-optimized {} image variants", final_count);
        }
        Ok(final_count)
    }

    fn matches_local_patterns(&self, path: &str) -> bool {
        if self.config.local_patterns.is_empty() {
            return false;
        }

        for pattern in &self.config.local_patterns {
            if Self::matches_local_pattern(path, pattern) {
                return true;
            }
        }

        false
    }

    pub async fn optimize(
        &self,
        params: OptimizeParams,
    ) -> Result<(OptimizedImage, bool), ImageError> {
        if let Some(w) = params.w
            && w > MAX_OUTPUT_WIDTH
        {
            return Err(ImageError::InvalidParams(format!(
                "Width {w} exceeds maximum allowed ({MAX_OUTPUT_WIDTH})"
            )));
        }

        if !self.config.quality_allowlist.is_empty()
            && !self.config.quality_allowlist.contains(&params.q)
        {
            return Err(ImageError::InvalidParams(format!(
                "Quality {} not in allowlist",
                params.q
            )));
        }

        let cache_key = Self::generate_cache_key(&params);

        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok((
                OptimizedImage {
                    data: cached.data.clone(),
                    format: cached.format,
                    width: cached.width,
                    height: cached.height,
                },
                true,
            ));
        }

        let _permit = self.processing_semaphore.acquire().await.map_err(|e| {
            ImageError::ProcessingError(format!("Failed to acquire processing permit: {e}"))
        })?;

        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok((
                OptimizedImage {
                    data: cached.data.clone(),
                    format: cached.format,
                    width: cached.width,
                    height: cached.height,
                },
                true,
            ));
        }

        self.validate_url(&params.url)?;

        let source = self.fetch_image(&params.url).await?;

        let params_clone = params.clone();
        let config_clone = self.config.clone();
        let optimized = task::spawn_blocking(move || {
            Self::process_image_blocking(&source, &params_clone, &config_clone)
        })
        .await
        .map_err(|e| ImageError::ProcessingError(format!("Image processing task failed: {e}")))??;

        self.cache
            .put(
                cache_key,
                cache::CachedImage {
                    data: optimized.data.clone(),
                    width: optimized.width,
                    height: optimized.height,
                    format: optimized.format,
                },
            )
            .await;

        Ok((optimized, false))
    }

    fn generate_cache_key(params: &OptimizeParams) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(params.url.as_bytes());
        hasher.update(params.w.unwrap_or(0).to_le_bytes());
        hasher.update([params.q]);

        let format_str = params.f.as_deref().unwrap_or("avif");
        hasher.update(format_str.as_bytes());

        hex::encode(hasher.finalize())
    }

    fn validate_url(&self, url_str: &str) -> Result<(), ImageError> {
        if url_str.starts_with('/') {
            if self.config.local_patterns.is_empty() {
                return Err(ImageError::UnauthorizedDomain(format!(
                    "Local path not allowed: {url_str}. Configure localPatterns in your image config to allow local paths."
                )));
            }

            let mut allowed = false;
            for pattern in &self.config.local_patterns {
                if Self::matches_local_pattern(url_str, pattern) {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return Err(ImageError::UnauthorizedDomain(format!(
                    "Local path not allowed: {url_str}. Configure localPatterns in your image config to allow local paths."
                )));
            }
            return Ok(());
        }

        self.validate_remote_url(url_str).map(|_| ())
    }

    fn matches_local_pattern(path: &str, pattern: &LocalPattern) -> bool {
        if !Self::pathname_matches(path, &pattern.pathname) {
            return false;
        }

        if let Some(ref search) = pattern.search {
            if let Some(query_start) = path.find('?') {
                let query = &path[query_start..];
                if query != search {
                    return false;
                }
            } else if !search.is_empty() {
                return false;
            }
        }

        true
    }

    fn pathname_matches(path: &str, pattern: &str) -> bool {
        let path_without_query = if let Some(idx) = path.find('?') { &path[..idx] } else { path };

        if let Some(prefix) = pattern.strip_suffix("/**") {
            path_without_query == prefix
                || (path_without_query.starts_with(prefix)
                    && path_without_query.as_bytes().get(prefix.len()) == Some(&b'/'))
        } else if pattern.contains('*') {
            Self::glob_match(path_without_query, pattern)
        } else {
            path_without_query == pattern
        }
    }

    fn glob_match(text: &str, pattern: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('*').collect();
        if pattern_parts.len() == 1 {
            return text == pattern;
        }

        let mut pos = 0;
        for (i, part) in pattern_parts.iter().enumerate() {
            if i == 0 {
                if !text.starts_with(part) {
                    return false;
                }
                pos = part.len();
            } else if i == pattern_parts.len() - 1 {
                if !text[pos..].ends_with(part) {
                    return false;
                }
            } else if let Some(idx) = text[pos..].find(part) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }
        true
    }

    fn matches_pattern(url: &Url, pattern: &RemotePattern) -> bool {
        if let Some(ref protocol) = pattern.protocol
            && url.scheme() != protocol
        {
            return false;
        }

        if let Some(host) = url.host_str() {
            if !Self::hostname_matches(host, &pattern.hostname) {
                return false;
            }
        } else {
            return false;
        }

        if let Some(ref port) = pattern.port
            && url.port().map(|p| p.to_string()) != Some(port.clone())
        {
            return false;
        }

        if let Some(ref pathname) = pattern.pathname
            && !Self::pathname_matches(url.path(), pathname)
        {
            return false;
        }

        if let Some(ref search) = pattern.search {
            if let Some(query) = url.query() {
                let full_query = format!("?{query}");
                if &full_query != search {
                    return false;
                }
            } else if !search.is_empty() {
                return false;
            }
        }

        true
    }

    fn hostname_matches(host: &str, pattern: &str) -> bool {
        if let Some(domain) = pattern.strip_prefix("*.") {
            host.ends_with(domain) || host == &domain[1..]
        } else {
            host == pattern
        }
    }

    fn validate_remote_url(&self, url: &str) -> Result<Url, ImageError> {
        let parsed = Url::parse(url)
            .map_err(|e| ImageError::InvalidUrl(format!("Invalid URL '{url}': {e}")))?;

        match parsed.scheme() {
            "http" | "https" => {}
            other => {
                return Err(ImageError::InvalidUrl(format!("Unsupported URL scheme '{other}'")));
            }
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| ImageError::InvalidUrl(format!("URL '{url}' is missing a host")))?;

        let host_lower = host.cow_to_ascii_lowercase();

        if host_lower == "localhost"
            || host_lower == "127.0.0.1"
            || host_lower == "::1"
            || host_lower == "0.0.0.0"
            || host_lower.starts_with("127.")
        {
            return Err(ImageError::UnauthorizedDomain(format!(
                "Loopback host '{host}' is not allowed"
            )));
        }

        if let Some(host_enum) = parsed.host() {
            match host_enum {
                url::Host::Ipv4(ip) => {
                    let octets = ip.octets();
                    if octets[0] == 10
                        || octets[0] == 127
                        || (octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31))
                        || (octets[0] == 192 && octets[1] == 168)
                        || (octets[0] == 169 && octets[1] == 254)
                        || (octets[0] == 100 && (octets[1] >= 64 && octets[1] <= 127))
                        || octets[0] == 0
                    {
                        return Err(ImageError::UnauthorizedDomain(format!(
                            "Private or reserved IP address '{host}' is not allowed"
                        )));
                    }
                }
                url::Host::Ipv6(ip) => {
                    let segments = ip.segments();

                    let is_private_ipv4 = |octets: [u8; 4]| -> bool {
                        octets[0] == 10
                            || octets[0] == 127
                            || (octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31))
                            || (octets[0] == 192 && octets[1] == 168)
                            || (octets[0] == 169 && octets[1] == 254)
                            || (octets[0] == 100 && (octets[1] >= 64 && octets[1] <= 127))
                            || octets[0] == 0
                    };

                    if let Some(ipv4) = ip.to_ipv4_mapped() {
                        let octets = ipv4.octets();
                        if is_private_ipv4(octets) {
                            return Err(ImageError::UnauthorizedDomain(format!(
                                "Private or reserved IPv6 address '{host}' is not allowed"
                            )));
                        }
                    } else if segments[0] == 0x2002 {
                        let octets = [
                            (segments[1] >> 8) as u8,
                            (segments[1] & 0xff) as u8,
                            (segments[2] >> 8) as u8,
                            (segments[2] & 0xff) as u8,
                        ];
                        if is_private_ipv4(octets) {
                            return Err(ImageError::UnauthorizedDomain(format!(
                                "Private or reserved IPv6 address '{host}' is not allowed"
                            )));
                        }
                    } else if segments[0] == 0x2001 && segments[1] == 0x0000 {
                        let server_octets = [
                            (segments[2] >> 8) as u8,
                            (segments[2] & 0xff) as u8,
                            (segments[3] >> 8) as u8,
                            (segments[3] & 0xff) as u8,
                        ];
                        if is_private_ipv4(server_octets) {
                            return Err(ImageError::UnauthorizedDomain(format!(
                                "Private or reserved IPv6 address '{host}' is not allowed"
                            )));
                        }

                        let client_octets = [
                            cast::u16_to_u8((segments[6] >> 8) ^ 0xff),
                            cast::u16_to_u8((segments[6] & 0xff) ^ 0xff),
                            cast::u16_to_u8((segments[7] >> 8) ^ 0xff),
                            cast::u16_to_u8((segments[7] & 0xff) ^ 0xff),
                        ];
                        if is_private_ipv4(client_octets) {
                            return Err(ImageError::UnauthorizedDomain(format!(
                                "Private or reserved IPv6 address '{host}' is not allowed"
                            )));
                        }
                    } else if ip.is_loopback()
                        || (segments[0] & 0xfe00) == 0xfc00
                        || (segments[0] & 0xffc0) == 0xfe80
                    {
                        return Err(ImageError::UnauthorizedDomain(format!(
                            "Private or reserved IPv6 address '{host}' is not allowed"
                        )));
                    }
                }
                url::Host::Domain(domain) => {
                    let domain_lower = domain.cow_to_ascii_lowercase();
                    if domain_lower.ends_with(".local")
                        || domain_lower.ends_with(".internal")
                        || domain_lower.ends_with(".localhost")
                        || domain_lower == "metadata.google.internal"
                    {
                        return Err(ImageError::UnauthorizedDomain(format!(
                            "Internal domain '{host}' is not allowed"
                        )));
                    }
                }
            }
        }

        if self.config.remote_patterns.is_empty() {
            return Err(ImageError::UnauthorizedDomain(format!(
                "No remote image domains are configured; rejecting host '{host}'"
            )));
        }

        let mut allowed = false;
        for pattern in &self.config.remote_patterns {
            if Self::matches_pattern(&parsed, pattern) {
                allowed = true;
                break;
            }
        }

        if !allowed {
            return Err(ImageError::UnauthorizedDomain(format!(
                "Host '{host}' is not allowed for remote images"
            )));
        }

        Ok(parsed)
    }

    async fn make_validated_request(&self, url: &str) -> Result<reqwest::Response, ImageError> {
        let validated_url = self.validate_remote_url(url)?;
        self.http_client
            .get(validated_url)
            .send()
            .await
            .map_err(|e| ImageError::FetchError(e.to_string()))
    }

    async fn read_file_bytes_capped(
        path: &Path,
        max_size: usize,
        context: &str,
    ) -> Result<Vec<u8>, ImageError> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            ImageError::FetchError(format!("Failed to stat {context} {}: {e}", path.display()))
        })?;
        if !metadata.is_file() {
            return Err(ImageError::FetchError(format!(
                "Refusing to read non-regular {context} {}",
                path.display()
            )));
        }

        let file = fs::File::open(path).await.map_err(|e| {
            ImageError::FetchError(format!("Failed to read {context} {}: {e}", path.display()))
        })?;
        let mut bytes = Vec::new();
        file.take(max_size as u64 + 1).read_to_end(&mut bytes).await.map_err(|e| {
            ImageError::FetchError(format!("Failed to read {context} {}: {e}", path.display()))
        })?;

        if bytes.len() > max_size {
            return Err(ImageError::InvalidParams(format!(
                "Image too large: at least {} bytes (max {max_size} bytes)",
                bytes.len(),
            )));
        }

        Ok(bytes)
    }

    async fn read_image_bytes_capped(path: &Path, context: &str) -> Result<Vec<u8>, ImageError> {
        Self::read_file_bytes_capped(path, MAX_SOURCE_IMAGE_SIZE, context).await
    }

    async fn load_static_image_source_map(
        &self,
    ) -> Result<Option<serde_json::Map<String, serde_json::Value>>, ImageError> {
        let map_path = match validate_safe_path(
            &self.out_dir_path(),
            "server/static-image-sources.json",
        )
        .await
        {
            Ok(path) => path,
            Err(RariError::NotFound(_, _)) => return Ok(None),
            Err(e) => {
                return Err(ImageError::FetchError(format!(
                    "Failed to resolve static image source map: {}",
                    e.message()
                )));
            }
        };

        let map_bytes = match Self::read_file_bytes_capped(
            &map_path,
            MAX_STATIC_IMAGE_SOURCE_MAP_SIZE,
            "static image source map",
        )
        .await
        {
            Ok(bytes) => bytes,
            Err(ImageError::InvalidParams(_)) => {
                return Err(ImageError::FetchError(format!(
                    "static image source map too large (max {MAX_STATIC_IMAGE_SOURCE_MAP_SIZE} bytes)"
                )));
            }
            Err(error) => return Err(error),
        };
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&map_bytes)
            .map_err(|e| {
                ImageError::FetchError(format!("Failed to parse static image source map: {e}"))
            })?;

        Ok(Some(map))
    }

    async fn collect_source_map_image_urls(&self) -> Result<Vec<String>, ImageError> {
        let Some(map) = self.load_static_image_source_map().await? else {
            return Ok(Vec::new());
        };

        let mut urls = Vec::new();
        for url in map.keys() {
            if self.matches_local_patterns(url) {
                urls.push(url.clone());
            }
        }

        if !urls.is_empty() {
            tracing::debug!(
                "Falling back to {} URLs from static image source map for pre-optimization",
                urls.len()
            );
        }

        Ok(urls)
    }

    async fn read_local_image_under_root(
        &self,
        url: &str,
        root: &Path,
        root_label: &str,
    ) -> Result<Option<Vec<u8>>, ImageError> {
        let canonical_file = match validate_safe_path(root, url).await {
            Ok(path) => path,
            Err(RariError::NotFound(_, _)) => return Ok(None),
            Err(RariError::BadRequest(message, _)) => {
                return Err(ImageError::InvalidUrl(format!(
                    "Path traversal detected for {root_label}: {message}"
                )));
            }
            Err(e) => {
                return Err(ImageError::FetchError(format!(
                    "Failed to resolve local image under {root_label}: {}",
                    e.message()
                )));
            }
        };

        Ok(Some(Self::read_image_bytes_capped(&canonical_file, "local file").await?))
    }

    async fn read_static_image_source_map(&self, url: &str) -> Result<Option<Vec<u8>>, ImageError> {
        let Some(map) = self.load_static_image_source_map().await? else {
            return Ok(None);
        };

        let Some(source_value) = map.get(url) else {
            return Ok(None);
        };
        let Some(source_path) = source_value.as_str() else {
            return Ok(None);
        };

        let path = PathBuf::from(source_path);
        if !path.is_absolute() {
            return Ok(None);
        }

        let canonical_project = fs::canonicalize(&self.project_path).await.map_err(|e| {
            ImageError::FetchError(format!("Failed to canonicalize project directory: {e}"))
        })?;
        let canonical_candidate = match fs::canonicalize(&path).await {
            Ok(path) => path,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(ImageError::FetchError(format!(
                    "Failed to canonicalize mapped static image {}: {}",
                    path.display(),
                    e
                )));
            }
        };
        let Ok(relative) = canonical_candidate.strip_prefix(&canonical_project) else {
            return Err(ImageError::InvalidUrl(
                "Path traversal detected: mapped static image escapes project directory"
                    .to_string(),
            ));
        };
        let relative = relative.to_string_lossy();

        let canonical_file = match validate_safe_path(&self.project_path, relative.as_ref()).await {
            Ok(path) => path,
            Err(RariError::NotFound(_, _)) => return Ok(None),
            Err(RariError::BadRequest(message, _)) => {
                return Err(ImageError::InvalidUrl(format!(
                    "Path traversal detected: mapped static image escapes project directory ({message})"
                )));
            }
            Err(e) => {
                return Err(ImageError::FetchError(format!(
                    "Failed to resolve mapped static image: {}",
                    e.message()
                )));
            }
        };

        Ok(Some(Self::read_image_bytes_capped(&canonical_file, "mapped static image").await?))
    }

    async fn fetch_image(&self, url: &str) -> Result<Vec<u8>, ImageError> {
        if url.starts_with('/') {
            let pathname = url.split_once('?').map_or(url, |(path, _)| path);
            let fs_pathname = decode_url_path_segments(pathname);

            if let Some(bytes) = self
                .read_local_image_under_root(
                    &fs_pathname,
                    &self.project_path.join("public"),
                    "public",
                )
                .await?
            {
                return Ok(bytes);
            }

            let assets_dir = self.config.assets_dir.trim_matches('/');
            let assets_prefix = format!("/{assets_dir}");
            if (fs_pathname == assets_prefix
                || fs_pathname.starts_with(&format!("{assets_prefix}/")))
                && let Some(bytes) = self
                    .read_local_image_under_root(&fs_pathname, &self.out_dir_path(), "dist")
                    .await?
            {
                return Ok(bytes);
            }

            if let Some(relative) = fs_pathname.strip_prefix("/src/")
                && let Some(bytes) = self
                    .read_local_image_under_root(relative, &self.project_path.join("src"), "src")
                    .await?
            {
                return Ok(bytes);
            }

            if let Some(bytes) = self.read_static_image_source_map(pathname).await? {
                return Ok(bytes);
            }

            return Err(ImageError::FetchError(format!(
                "Local image not found for {url}. Checked public/, dist/, src/, and static import source map."
            )));
        }

        let mut current_url = url.to_string();
        let mut redirect_count = 0;

        let redact_url = |value: &str| {
            Url::parse(value)
                .map(|u| {
                    let host = u.host_str().unwrap_or("");
                    match u.port() {
                        Some(port) => {
                            format!("{}://{}:{}{}", u.scheme(), host, port, u.path())
                        }
                        None => format!("{}://{}{}", u.scheme(), host, u.path()),
                    }
                })
                .unwrap_or_else(|_| "<invalid url>".to_string())
        };

        loop {
            let response = self.make_validated_request(&current_url).await?;

            if response.status().is_redirection() {
                if redirect_count >= self.config.max_redirects {
                    return Err(ImageError::FetchError(format!(
                        "Too many redirects (max {})",
                        self.config.max_redirects
                    )));
                }

                let location =
                    response.headers().get(LOCATION).and_then(|v| v.to_str().ok()).ok_or_else(
                        || ImageError::FetchError("Redirect without Location header".to_string()),
                    )?;

                let redirect_url = if location.starts_with("http://")
                    || location.starts_with("https://")
                {
                    location.to_string()
                } else {
                    let base = Url::parse(&current_url)
                        .map_err(|e| ImageError::InvalidUrl(format!("Invalid base URL: {e}")))?;
                    base.join(location)
                        .map_err(|e| ImageError::InvalidUrl(format!("Invalid redirect URL: {e}")))?
                        .to_string()
                };

                tracing::debug!(
                    "Following validated redirect: {} -> {}",
                    redact_url(&current_url),
                    redact_url(&redirect_url)
                );
                current_url = redirect_url;
                redirect_count += 1;
                continue;
            }

            if !response.status().is_success() {
                return Err(ImageError::FetchError(format!(
                    "HTTP {}: {}",
                    response.status(),
                    redact_url(&current_url)
                )));
            }

            if let Some(content_length) = response.content_length()
                && cast::u64_to_usize(content_length) > MAX_SOURCE_IMAGE_SIZE
            {
                return Err(ImageError::InvalidParams(format!(
                    "Image too large: {content_length} bytes (max {MAX_SOURCE_IMAGE_SIZE} bytes)"
                )));
            }

            let mut bytes = if let Some(content_length) = response.content_length() {
                let capacity = cast::u64_to_usize(content_length).min(MAX_SOURCE_IMAGE_SIZE);
                Vec::with_capacity(capacity)
            } else {
                Vec::new()
            };
            let mut stream = response.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| ImageError::FetchError(e.to_string()))?;
                if bytes.len() + chunk.len() > MAX_SOURCE_IMAGE_SIZE {
                    return Err(ImageError::InvalidParams(format!(
                        "Image too large (max {MAX_SOURCE_IMAGE_SIZE} bytes)"
                    )));
                }
                bytes.extend_from_slice(&chunk);
            }

            return Ok(bytes);
        }
    }

    fn determine_format_from_param(format_str: Option<&str>) -> ImageFormat {
        match format_str {
            Some("webp") => ImageFormat::WebP,
            Some("jpeg" | "jpg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            _ => ImageFormat::Avif,
        }
    }

    fn process_image_blocking(
        source: &[u8],
        params: &OptimizeParams,
        _config: &ImageConfig,
    ) -> Result<OptimizedImage, ImageError> {
        let img = image::load_from_memory(source)
            .map_err(|e| ImageError::ProcessingError(format!("Failed to decode image: {e}")))?;

        if img.width() > MAX_OUTPUT_WIDTH * 2 || img.height() > MAX_OUTPUT_HEIGHT * 2 {
            return Err(ImageError::InvalidParams(format!(
                "Source image too large: {}x{} (max {}x{})",
                img.width(),
                img.height(),
                MAX_OUTPUT_WIDTH * 2,
                MAX_OUTPUT_HEIGHT * 2
            )));
        }

        let processed = if let Some(width) = params.w {
            let target_width = width.min(MAX_OUTPUT_WIDTH);
            if target_width < img.width() {
                img.resize(target_width, u32::MAX, FilterType::Lanczos3)
            } else {
                img
            }
        } else if img.width() > MAX_OUTPUT_WIDTH || img.height() > MAX_OUTPUT_HEIGHT {
            let scale = (float::u32_to_f32(MAX_OUTPUT_WIDTH) / float::u32_to_f32(img.width()))
                .min(float::u32_to_f32(MAX_OUTPUT_HEIGHT) / float::u32_to_f32(img.height()));
            let new_width = cast::f32_to_u32(float::u32_to_f32(img.width()) * scale);
            img.resize(new_width, u32::MAX, FilterType::Lanczos3)
        } else {
            img
        };

        let format = Self::determine_format_from_param(params.f.as_deref());

        let data = match format {
            ImageFormat::Avif => Self::encode_avif(&processed, params.q)?,
            ImageFormat::WebP => Self::encode_webp(&processed, params.q)?,
            ImageFormat::Jpeg => Self::encode_jpeg(&processed, params.q)?,
            ImageFormat::Png => Self::encode_png(&processed)?,
            ImageFormat::Gif => {
                return Err(ImageError::ProcessingError("GIF encoding not supported".to_string()));
            }
        };

        Ok(OptimizedImage { data, format, width: processed.width(), height: processed.height() })
    }

    fn encode_avif(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
        use std::io::Cursor;

        use image::codecs::avif::AvifEncoder;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let encoder =
            AvifEncoder::new_with_speed_quality(&mut cursor, AVIF_ENCODING_SPEED, quality);
        img.write_with_encoder(encoder)
            .map_err(|e| ImageError::ProcessingError(format!("AVIF encoding failed: {e}")))?;

        Ok(buffer)
    }

    fn encode_webp(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
        let mut buffer = Vec::new();
        let encoder = webp::Encoder::from_image(img)
            .map_err(|e| ImageError::ProcessingError(format!("WebP encoding failed: {e}")))?;

        let encoded = encoder.encode(f32::from(quality));
        buffer.extend_from_slice(&encoded);

        Ok(buffer)
    }

    fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
        use std::io::Cursor;

        use image::codecs::jpeg::JpegEncoder;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let encoder = JpegEncoder::new_with_quality(&mut cursor, quality);
        img.write_with_encoder(encoder)
            .map_err(|e| ImageError::ProcessingError(format!("JPEG encoding failed: {e}")))?;

        Ok(buffer)
    }

    fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, ImageError> {
        use std::io::Cursor;

        use image::codecs::png::PngEncoder;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let encoder = PngEncoder::new(&mut cursor);
        img.write_with_encoder(encoder)
            .map_err(|e| ImageError::ProcessingError(format!("PNG encoding failed: {e}")))?;

        Ok(buffer)
    }
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use crate::server::image::config::LocalPattern;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn test_project(name: &str) -> PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("rari-image-optimizer-{name}-{id}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test project");
        dir
    }

    fn tiny_png() -> Vec<u8> {
        vec![
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0xc9, 0xfe, 0x92,
            0xef, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ]
    }

    fn assets_config() -> ImageConfig {
        ImageConfig {
            local_patterns: vec![LocalPattern { pathname: "/assets/**".to_string(), search: None }],
            formats: vec![ImageFormat::Png],
            quality_allowlist: vec![75],
            ..ImageConfig::default()
        }
    }

    #[test]
    fn pathname_matches_requires_path_segment_boundary() {
        assert!(ImageOptimizer::pathname_matches("/assets", "/assets/**"));
        assert!(ImageOptimizer::pathname_matches("/assets/hero.png", "/assets/**"));
        assert!(!ImageOptimizer::pathname_matches("/assets-private/hero.png", "/assets/**"));
        assert!(ImageOptimizer::pathname_matches("/src/app/hero.png", "/src/**"));
        assert!(!ImageOptimizer::pathname_matches("/src-private/hero.png", "/src/**"));
    }

    #[test]
    fn local_pattern_rejects_assets_private_prefix() {
        let pattern = LocalPattern { pathname: "/assets/**".to_string(), search: None };
        assert!(ImageOptimizer::matches_local_pattern("/assets/hero.png", &pattern));
        assert!(!ImageOptimizer::matches_local_pattern("/assets-private/hero.png", &pattern));
    }

    #[test]
    fn encodes_and_decodes_path_segments_like_static_imports() {
        assert_eq!(encode_url_path_segments("/assets/100%-hash.png"), "/assets/100%25-hash.png");
        assert_eq!(encode_url_path_segments("/assets/my photo.png"), "/assets/my%20photo.png");
        assert_eq!(decode_url_path_segments("/assets/100%25-hash.png"), "/assets/100%-hash.png");
        assert_eq!(decode_url_path_segments("/assets/my%20photo.png"), "/assets/my photo.png");
        assert_eq!(
            decode_url_path_segments(&encode_url_path_segments("/assets/写真.png")),
            "/assets/写真.png"
        );
    }

    #[test]
    fn out_dir_path_preserves_absolute_configured_paths() {
        let project = test_project("out-dir-abs");
        let absolute_out = env::temp_dir()
            .join(format!("rari-image-abs-out-{}", TEST_ID.fetch_add(1, Ordering::Relaxed)));
        let config =
            ImageConfig { out_dir: absolute_out.to_string_lossy().into_owned(), ..assets_config() };
        let optimizer = ImageOptimizer::new(config, &project);
        assert_eq!(optimizer.out_dir_path(), absolute_out);

        let relative = ImageOptimizer::new(
            ImageConfig { out_dir: "build-output".to_string(), ..assets_config() },
            &project,
        );
        assert_eq!(relative.out_dir_path(), project.join("build-output"));
    }

    #[tokio::test]
    async fn fetch_resolves_encoded_dist_asset_without_source_map() {
        let project = test_project("dist-encoded");
        let assets = project.join("dist").join("assets");
        fs::create_dir_all(&assets).expect("assets dir");

        let unicode_url = format!("/assets/{}-abc12345.png", urlencoding::encode("写真"));
        let cases = [
            ("100%-abc12345.png", "/assets/100%25-abc12345.png"),
            ("my photo-abc12345.png", "/assets/my%20photo-abc12345.png"),
            ("写真-abc12345.png", unicode_url.as_str()),
        ];

        for (file_name, encoded_url) in cases {
            fs::write(assets.join(file_name), tiny_png()).expect("write png");
            let optimizer = ImageOptimizer::new(assets_config(), &project);
            let bytes = optimizer
                .fetch_image(encoded_url)
                .await
                .expect("encoded dist asset should resolve");
            assert_eq!(bytes, tiny_png());
        }

        assert!(
            !project.join("dist/server/static-image-sources.json").exists(),
            "source map must be absent for dist-only coverage"
        );
    }

    #[tokio::test]
    async fn fetch_rejects_non_regular_local_path() {
        let project = test_project("non-regular");
        let public = project.join("public");
        fs::create_dir_all(public.join("trap.png")).expect("dir disguised as image");

        let optimizer = ImageOptimizer::new(
            ImageConfig {
                local_patterns: vec![LocalPattern { pathname: "/**".to_string(), search: None }],
                ..ImageConfig::default()
            },
            &project,
        );

        let err = optimizer.fetch_image("/trap.png").await.expect_err("directory must fail");
        assert!(err.to_string().contains("non-regular"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn scan_emits_encoded_urls_for_special_filenames() {
        let project = test_project("scan-encode");
        let assets = project.join("dist").join("assets");
        fs::create_dir_all(&assets).expect("assets dir");
        fs::write(assets.join("100%-deadbeef.png"), tiny_png()).expect("write");
        fs::write(assets.join("cool pic-deadbeef.png"), tiny_png()).expect("write");

        let optimizer = ImageOptimizer::new(assets_config(), &project);
        let count =
            optimizer.preoptimize_local_images_preview().await.expect("preoptimize preview");
        assert!(count > 0);

        let encoded_percent = "/assets/100%25-deadbeef.png";
        let encoded_space = "/assets/cool%20pic-deadbeef.png";
        assert!(optimizer.fetch_image(encoded_percent).await.is_ok());
        assert!(optimizer.fetch_image(encoded_space).await.is_ok());
    }

    #[tokio::test]
    async fn scan_includes_gif_files() {
        let project = test_project("scan-gif");
        let assets = project.join("dist").join("assets");
        fs::create_dir_all(&assets).expect("assets dir");
        fs::write(assets.join("frame.gif"), tiny_png()).expect("write gif");
        fs::write(assets.join("notes.txt"), b"skip").expect("write txt");

        let optimizer = ImageOptimizer::new(assets_config(), &project);
        let count =
            optimizer.preoptimize_local_images_preview().await.expect("preoptimize preview");
        assert!(count > 0, "gif assets should be included in the local scan");

        let txt_only = test_project("scan-txt-only");
        let txt_assets = txt_only.join("dist").join("assets");
        fs::create_dir_all(&txt_assets).expect("assets dir");
        fs::write(txt_assets.join("notes.txt"), b"skip").expect("write txt");
        let txt_optimizer = ImageOptimizer::new(assets_config(), &txt_only);
        let txt_count =
            txt_optimizer.preoptimize_local_images_preview().await.expect("preoptimize preview");
        assert_eq!(txt_count, 0, "non-image files must not be scanned");
    }

    #[tokio::test]
    async fn scan_falls_back_to_source_map_urls_when_asset_roots_missing() {
        let project = test_project("scan-source-map-only");
        let server_dir = project.join("dist").join("server");
        fs::create_dir_all(&server_dir).expect("server dir");

        let source = project.join("photo.png");
        fs::write(&source, tiny_png()).expect("write source");

        let map = serde_json::json!({
            "/assets/photo-deadbeef.png": source.to_string_lossy(),
        });
        fs::write(server_dir.join("static-image-sources.json"), map.to_string())
            .expect("write source map");

        let optimizer = ImageOptimizer::new(assets_config(), &project);
        let count =
            optimizer.preoptimize_local_images_preview().await.expect("preoptimize preview");
        assert!(count > 0, "source-map URLs should be preoptimized when scan roots are absent");
    }

    #[tokio::test]
    async fn fetch_continues_when_out_dir_is_missing() {
        let project = test_project("missing-out-dir");
        let public = project.join("public");
        fs::create_dir_all(&public).expect("public");
        fs::write(public.join("hero.png"), tiny_png()).expect("write public image");

        let optimizer = ImageOptimizer::new(
            ImageConfig {
                out_dir: "does-not-exist".to_string(),
                local_patterns: vec![LocalPattern { pathname: "/**".to_string(), search: None }],
                formats: vec![ImageFormat::Png],
                quality_allowlist: vec![75],
                ..ImageConfig::default()
            },
            &project,
        );

        let bytes = optimizer
            .fetch_image("/hero.png")
            .await
            .expect("missing out_dir must not abort local fetch before public/ fallback completes");
        assert_eq!(bytes, tiny_png());
    }
}
