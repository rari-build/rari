use super::{
    ImageError,
    cache::{self, ImageCache},
    config::{ImageConfig, LocalPattern, RemotePattern},
    types::{ImageFormat, OptimizeParams, OptimizedImage},
};
use cow_utils::CowUtils;
use futures::stream::{self, StreamExt};
use image::{DynamicImage, imageops::FilterType};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;
use url::Url;

const MAX_SOURCE_IMAGE_SIZE: usize = 10 * 1024 * 1024;
const MAX_OUTPUT_WIDTH: u32 = 3840;
const MAX_OUTPUT_HEIGHT: u32 = 2160;
const AVIF_ENCODING_SPEED: u8 = 6;
const DEFAULT_CONCURRENCY: usize = 4;

#[derive(Debug, Clone)]
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
        let cache = Arc::new(ImageCache::new(config.max_cache_size, project_path));
        let http_client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects as usize))
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

    fn default_quality(&self) -> u8 {
        if self.config.quality_allowlist.is_empty() || self.config.quality_allowlist.contains(&75) {
            75
        } else {
            *self.config.quality_allowlist.first().unwrap_or(&75)
        }
    }

    pub fn get_preload_links(&self) -> Vec<String> {
        let preload_images =
            self.preload_images.read().unwrap_or_else(|poison| poison.into_inner());
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
            self.preload_images.write().unwrap_or_else(|poison| poison.into_inner());
        preload_images.clear();
    }

    pub async fn preoptimize_local_images(&self) -> Result<usize, ImageError> {
        self.preoptimize_local_images_internal(false).await
    }

    pub async fn preoptimize_local_images_preview(&self) -> Result<usize, ImageError> {
        self.preoptimize_local_images_internal(true).await
    }

    async fn preoptimize_local_images_internal(&self, dry_run: bool) -> Result<usize, ImageError> {
        if dry_run {
            tracing::info!("Starting local image pre-optimization preview (dry-run)...");
        } else {
            tracing::info!("Starting local image pre-optimization...");
        }

        if !self.config.preoptimize_manifest.is_empty() {
            tracing::info!(
                "Using preoptimize manifest with {} image variants",
                self.config.preoptimize_manifest.len()
            );
            return self.preoptimize_from_manifest(dry_run).await;
        }

        tracing::info!("No manifest found, scanning public directory...");

        let public_dir = self.project_path.join("public");
        match tokio::fs::try_exists(&public_dir).await {
            Ok(false) => {
                tracing::warn!(
                    "Public directory does not exist at {:?}, skipping local image pre-optimization",
                    public_dir
                );
                return Ok(0);
            }
            Err(e) => {
                tracing::error!(
                    "Failed to check if public directory exists at {:?}: {}",
                    public_dir,
                    e
                );
                return Err(ImageError::ProcessingError(format!(
                    "Failed to check public directory: {}",
                    e
                )));
            }
            Ok(true) => {}
        }

        tracing::debug!("Scanning public directory: {:?}", public_dir);

        let mut image_paths = Vec::new();
        let mut dirs_to_scan = vec![public_dir.clone()];

        while let Some(current_dir) = dirs_to_scan.pop() {
            let mut entries = tokio::fs::read_dir(&current_dir).await.map_err(|e| {
                ImageError::ProcessingError(format!(
                    "Failed to read directory {:?}: {}",
                    current_dir, e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                ImageError::ProcessingError(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();

                let file_type = entry.file_type().await.map_err(|e| {
                    ImageError::ProcessingError(format!(
                        "Failed to read file type for {:?}: {}",
                        path, e
                    ))
                })?;

                if file_type.is_symlink() {
                    continue;
                }

                if file_type.is_dir() {
                    dirs_to_scan.push(path);
                } else if file_type.is_file() {
                    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                    if !matches!(
                        extension.cow_to_lowercase().as_ref(),
                        "jpg" | "jpeg" | "png" | "webp" | "avif"
                    ) {
                        continue;
                    }

                    if let Ok(relative) = path.strip_prefix(&public_dir) {
                        let url_path =
                            format!("/{}", relative.to_string_lossy().cow_replace('\\', "/"));

                        if self.matches_local_patterns(&url_path) {
                            image_paths.push(url_path);
                        }
                    }
                }
            }
        }

        if image_paths.is_empty() {
            tracing::warn!("No local images found for pre-optimization");
            return Ok(0);
        }

        tracing::info!("Found {} local images to pre-optimize", image_paths.len());
        for path in &image_paths {
            tracing::debug!("  - {}", path);
        }

        if dry_run {
            tracing::info!("[DRY RUN] Would pre-optimize {} local images...", image_paths.len());
        } else {
            tracing::info!("Pre-optimizing {} local images...", image_paths.len());
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

        if !preload_list.is_empty() {
            let mut preload_images =
                self.preload_images.write().unwrap_or_else(|poison| poison.into_inner());
            preload_images.extend(preload_list);
            tracing::info!("Registered {} images for preloading", preload_images.len());
        }

        if tasks.is_empty() {
            tracing::warn!("No images to pre-optimize from manifest");
            return Ok(0);
        }

        tracing::info!("Pre-optimizing {} image variants from manifest", tasks.len());

        if dry_run {
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

                    let cache_key = self.generate_cache_key(&params);

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

        tracing::info!("Pre-optimized {} image variants from manifest", final_count);
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

        tracing::info!("Pre-optimizing for {} sizes: {:?}", sizes.len(), sizes);
        tracing::info!("Pre-optimizing with quality: {}", quality);

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

                    let cache_key = self.generate_cache_key(&params);

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

        tracing::info!("Pre-optimized {} image variants", final_count);
        Ok(final_count)
    }

    fn matches_local_patterns(&self, path: &str) -> bool {
        if self.config.local_patterns.is_empty() {
            return false;
        }

        for pattern in &self.config.local_patterns {
            if self.matches_local_pattern(path, pattern) {
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
                "Width {} exceeds maximum allowed ({})",
                w, MAX_OUTPUT_WIDTH
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

        let cache_key = self.generate_cache_key(&params);

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
            ImageError::ProcessingError(format!("Failed to acquire processing permit: {}", e))
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
        let optimized = tokio::task::spawn_blocking(move || {
            Self::process_image_blocking(source, &params_clone, &config_clone)
        })
        .await
        .map_err(|e| {
            ImageError::ProcessingError(format!("Image processing task failed: {}", e))
        })??;

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

    fn generate_cache_key(&self, params: &OptimizeParams) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(params.url.as_bytes());
        hasher.update(params.w.unwrap_or(0).to_le_bytes());
        hasher.update([params.q]);

        let format_str = params.f.as_deref().unwrap_or("avif");
        hasher.update(format_str.as_bytes());

        format!("{:x}", hasher.finalize())
    }

    fn validate_url(&self, url_str: &str) -> Result<(), ImageError> {
        if url_str.starts_with('/') {
            if self.config.local_patterns.is_empty() {
                return Err(ImageError::UnauthorizedDomain(format!(
                    "Local path not allowed: {}. Configure localPatterns in your image config to allow local paths.",
                    url_str
                )));
            }

            let mut allowed = false;
            for pattern in &self.config.local_patterns {
                if self.matches_local_pattern(url_str, pattern) {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return Err(ImageError::UnauthorizedDomain(format!(
                    "Local path not allowed: {}. Configure localPatterns in your image config to allow local paths.",
                    url_str
                )));
            }
            return Ok(());
        }

        let url = Url::parse(url_str)
            .map_err(|e| ImageError::InvalidUrl(format!("Invalid URL: {}", e)))?;

        if url.scheme() == "file" {
            return Err(ImageError::InvalidUrl("file:// URLs are not allowed".to_string()));
        }

        if self.config.remote_patterns.is_empty() {
            return Err(ImageError::UnauthorizedDomain(format!(
                "Remote URL not allowed: {}. Configure remotePatterns in your image config to allow external domains.",
                url_str
            )));
        }

        for pattern in &self.config.remote_patterns {
            if self.matches_pattern(&url, pattern) {
                return Ok(());
            }
        }

        Err(ImageError::UnauthorizedDomain(url_str.to_string()))
    }

    fn matches_local_pattern(&self, path: &str, pattern: &LocalPattern) -> bool {
        if !self.pathname_matches(path, &pattern.pathname) {
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

    fn pathname_matches(&self, path: &str, pattern: &str) -> bool {
        let path_without_query = if let Some(idx) = path.find('?') { &path[..idx] } else { path };

        if let Some(prefix) = pattern.strip_suffix("/**") {
            path_without_query.starts_with(prefix)
        } else if pattern.contains('*') {
            self.glob_match(path_without_query, pattern)
        } else {
            path_without_query == pattern
        }
    }

    fn glob_match(&self, text: &str, pattern: &str) -> bool {
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

    fn matches_pattern(&self, url: &Url, pattern: &RemotePattern) -> bool {
        if let Some(ref protocol) = pattern.protocol
            && url.scheme() != protocol
        {
            return false;
        }

        if let Some(host) = url.host_str() {
            if !self.hostname_matches(host, &pattern.hostname) {
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
            && !self.pathname_matches(url.path(), pathname)
        {
            return false;
        }

        if let Some(ref search) = pattern.search {
            if let Some(query) = url.query() {
                let full_query = format!("?{}", query);
                if &full_query != search {
                    return false;
                }
            } else if !search.is_empty() {
                return false;
            }
        }

        true
    }

    fn hostname_matches(&self, host: &str, pattern: &str) -> bool {
        if let Some(domain) = pattern.strip_prefix("*.") {
            host.ends_with(domain) || host == &domain[1..]
        } else {
            host == pattern
        }
    }

    async fn fetch_image(&self, url: &str) -> Result<Vec<u8>, ImageError> {
        if url.starts_with('/') {
            let public_path = self.project_path.join("public");
            let file_path = public_path.join(url.trim_start_matches('/'));

            let canonical_public = public_path.canonicalize().map_err(|e| {
                ImageError::FetchError(format!("Failed to canonicalize public directory: {}", e))
            })?;
            let canonical_file = file_path.canonicalize().map_err(|e| {
                ImageError::FetchError(format!(
                    "Failed to canonicalize file path {}: {}",
                    file_path.display(),
                    e
                ))
            })?;

            if !canonical_file.starts_with(&canonical_public) {
                return Err(ImageError::InvalidUrl(format!(
                    "Path traversal detected: {} escapes public directory",
                    url
                )));
            }

            let bytes = tokio::fs::read(&canonical_file).await.map_err(|e| {
                ImageError::FetchError(format!(
                    "Failed to read local file {}: {}",
                    canonical_file.display(),
                    e
                ))
            })?;

            if bytes.len() > MAX_SOURCE_IMAGE_SIZE {
                return Err(ImageError::InvalidParams(format!(
                    "Image too large: {} bytes (max {} bytes)",
                    bytes.len(),
                    MAX_SOURCE_IMAGE_SIZE
                )));
            }

            return Ok(bytes);
        }

        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| ImageError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ImageError::FetchError(format!("HTTP {}: {}", response.status(), url)));
        }

        if let Some(content_length) = response.content_length()
            && content_length as usize > MAX_SOURCE_IMAGE_SIZE
        {
            return Err(ImageError::InvalidParams(format!(
                "Image too large: {} bytes (max {} bytes)",
                content_length, MAX_SOURCE_IMAGE_SIZE
            )));
        }

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| ImageError::FetchError(e.to_string()))?;
            if bytes.len() + chunk.len() > MAX_SOURCE_IMAGE_SIZE {
                return Err(ImageError::InvalidParams(format!(
                    "Image too large (max {} bytes)",
                    MAX_SOURCE_IMAGE_SIZE
                )));
            }
            bytes.extend_from_slice(&chunk);
        }

        Ok(bytes)
    }

    fn determine_format_from_param(format_str: Option<&str>) -> ImageFormat {
        match format_str {
            Some("avif") => ImageFormat::Avif,
            Some("webp") => ImageFormat::WebP,
            Some("jpeg") | Some("jpg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            Some("gif") => ImageFormat::Gif,
            _ => ImageFormat::Avif,
        }
    }

    fn process_image_blocking(
        source: Vec<u8>,
        params: &OptimizeParams,
        _config: &ImageConfig,
    ) -> Result<OptimizedImage, ImageError> {
        let img = image::load_from_memory(&source)
            .map_err(|e| ImageError::ProcessingError(format!("Failed to decode image: {}", e)))?;

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
            let scale = (MAX_OUTPUT_WIDTH as f32 / img.width() as f32)
                .min(MAX_OUTPUT_HEIGHT as f32 / img.height() as f32);
            let new_width = (img.width() as f32 * scale) as u32;
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
        use image::codecs::avif::AvifEncoder;
        use std::io::Cursor;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let encoder =
            AvifEncoder::new_with_speed_quality(&mut cursor, AVIF_ENCODING_SPEED, quality);
        img.write_with_encoder(encoder)
            .map_err(|e| ImageError::ProcessingError(format!("AVIF encoding failed: {}", e)))?;

        Ok(buffer)
    }

    fn encode_webp(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
        let mut buffer = Vec::new();
        let encoder = webp::Encoder::from_image(img)
            .map_err(|e| ImageError::ProcessingError(format!("WebP encoding failed: {}", e)))?;

        let encoded = encoder.encode(quality as f32);
        buffer.extend_from_slice(&encoded);

        Ok(buffer)
    }

    fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
        use image::codecs::jpeg::JpegEncoder;
        use std::io::Cursor;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let encoder = JpegEncoder::new_with_quality(&mut cursor, quality);
        img.write_with_encoder(encoder)
            .map_err(|e| ImageError::ProcessingError(format!("JPEG encoding failed: {}", e)))?;

        Ok(buffer)
    }

    fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, ImageError> {
        use image::codecs::png::PngEncoder;
        use std::io::Cursor;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let encoder = PngEncoder::new(&mut cursor);
        img.write_with_encoder(encoder)
            .map_err(|e| ImageError::ProcessingError(format!("PNG encoding failed: {}", e)))?;

        Ok(buffer)
    }
}
