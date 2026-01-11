use super::{
    ImageError,
    cache::ImageCache,
    config::{ImageConfig, LocalPattern, RemotePattern},
    types::{ImageFormat, OptimizeParams, OptimizedImage},
};
use image::{DynamicImage, imageops::FilterType};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

const MAX_SOURCE_IMAGE_SIZE: usize = 10 * 1024 * 1024;
const MAX_OUTPUT_WIDTH: u32 = 3840;
const MAX_OUTPUT_HEIGHT: u32 = 2160;

pub struct ImageOptimizer {
    cache: Arc<ImageCache>,
    config: ImageConfig,
    http_client: Client,
    project_path: PathBuf,
}

impl ImageOptimizer {
    pub fn new(config: ImageConfig, project_path: &Path) -> Self {
        let cache = Arc::new(ImageCache::new(config.max_cache_size, project_path));
        let http_client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects as usize))
            .build()
            .expect("Failed to create HTTP client");

        Self { cache, config, http_client, project_path: project_path.to_path_buf() }
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

        let is_production = std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

        if is_production && let Some(cached) = self.cache.get(&cache_key) {
            let img = image::load_from_memory(&cached)
                .map_err(|e| ImageError::ProcessingError(e.to_string()))?;

            let format = self.determine_format(&params);

            return Ok((
                OptimizedImage {
                    data: (*cached).clone(),
                    format,
                    width: img.width(),
                    height: img.height(),
                },
                true,
            ));
        }

        self.validate_url(&params.url)?;

        let source = self.fetch_image(&params.url).await?;
        let optimized = self.process_image(source, &params)?;

        let is_production = std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

        if is_production {
            self.cache.put(cache_key, optimized.data.clone());
        }

        Ok((optimized, false))
    }

    fn generate_cache_key(&self, params: &OptimizeParams) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(params.url.as_bytes());
        hasher.update(params.w.unwrap_or(0).to_le_bytes());
        hasher.update([params.q]);
        if let Some(ref format) = params.f {
            hasher.update(format.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    fn validate_url(&self, url_str: &str) -> Result<(), ImageError> {
        if url_str.starts_with('/') {
            if !self.config.local_patterns.is_empty() {
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

            let bytes = std::fs::read(&file_path).map_err(|e| {
                ImageError::FetchError(format!(
                    "Failed to read local file {}: {}",
                    file_path.display(),
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
        use futures::StreamExt;

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

    fn determine_format(&self, params: &OptimizeParams) -> ImageFormat {
        match params.f.as_deref() {
            Some("avif") => ImageFormat::Avif,
            Some("webp") => ImageFormat::WebP,
            Some("jpeg") | Some("jpg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            Some("gif") => ImageFormat::Gif,
            _ => ImageFormat::WebP,
        }
    }

    fn process_image(
        &self,
        source: Vec<u8>,
        params: &OptimizeParams,
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

        let format = self.determine_format(params);
        let data = match format {
            ImageFormat::Avif => self.encode_avif(&processed, params.q)?,
            ImageFormat::WebP => self.encode_webp(&processed, params.q)?,
            ImageFormat::Jpeg => self.encode_jpeg(&processed, params.q)?,
            ImageFormat::Png => self.encode_png(&processed)?,
            ImageFormat::Gif => {
                return Err(ImageError::ProcessingError("GIF encoding not supported".to_string()));
            }
        };

        Ok(OptimizedImage { data, format, width: processed.width(), height: processed.height() })
    }

    fn encode_avif(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
        use image::codecs::avif::AvifEncoder;
        use std::io::Cursor;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let encoder = AvifEncoder::new_with_speed_quality(&mut cursor, 8, quality);
        img.write_with_encoder(encoder)
            .map_err(|e| ImageError::ProcessingError(format!("AVIF encoding failed: {}", e)))?;

        Ok(buffer)
    }

    fn encode_webp(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
        let mut buffer = Vec::new();
        let encoder = webp::Encoder::from_image(img)
            .map_err(|e| ImageError::ProcessingError(format!("WebP encoding failed: {}", e)))?;

        let encoded = encoder.encode(quality as f32);
        buffer.extend_from_slice(&encoded);

        Ok(buffer)
    }

    fn encode_jpeg(&self, img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageError> {
        use image::codecs::jpeg::JpegEncoder;
        use std::io::Cursor;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let encoder = JpegEncoder::new_with_quality(&mut cursor, quality);
        img.write_with_encoder(encoder)
            .map_err(|e| ImageError::ProcessingError(format!("JPEG encoding failed: {}", e)))?;

        Ok(buffer)
    }

    fn encode_png(&self, img: &DynamicImage) -> Result<Vec<u8>, ImageError> {
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
