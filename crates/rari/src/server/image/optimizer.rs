use super::{
    ImageError,
    cache::ImageCache,
    config::{ImageConfig, RemotePattern},
    types::{ImageFormat, OptimizeParams, OptimizedImage},
};
use image::{DynamicImage, imageops::FilterType};
use reqwest::Client;
use std::sync::Arc;
use url::Url;

pub struct ImageOptimizer {
    cache: Arc<ImageCache>,
    config: ImageConfig,
    http_client: Client,
}

impl ImageOptimizer {
    pub fn new(config: ImageConfig) -> Self {
        let cache = Arc::new(ImageCache::new(config.max_cache_size));
        let http_client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects as usize))
            .build()
            .expect("Failed to create HTTP client");

        Self { cache, config, http_client }
    }

    pub async fn optimize(&self, params: OptimizeParams) -> Result<OptimizedImage, ImageError> {
        if !self.config.quality_allowlist.is_empty()
            && !self.config.quality_allowlist.contains(&params.q)
        {
            return Err(ImageError::InvalidParams(format!(
                "Quality {} not in allowlist",
                params.q
            )));
        }

        let cache_key = self.generate_cache_key(&params);

        if let Some(cached) = self.cache.get(&cache_key) {
            let img = image::load_from_memory(&cached)
                .map_err(|e| ImageError::ProcessingError(e.to_string()))?;

            let format = self.determine_format(&params);

            return Ok(OptimizedImage {
                data: (*cached).clone(),
                format,
                width: img.width(),
                height: img.height(),
            });
        }

        self.validate_url(&params.url)?;

        let source = self.fetch_image(&params.url).await?;
        let optimized = self.process_image(source, &params)?;

        self.cache.put(cache_key, optimized.data.clone());

        Ok(optimized)
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
        let url = Url::parse(url_str)
            .map_err(|e| ImageError::InvalidUrl(format!("Invalid URL: {}", e)))?;

        if url.scheme() == "file" || url_str.starts_with('/') {
            return Ok(());
        }

        if self.config.remote_patterns.is_empty() {
            return Ok(());
        }

        if !self.config.remote_patterns.is_empty() {
            for pattern in &self.config.remote_patterns {
                if self.matches_pattern(&url, pattern) {
                    return Ok(());
                }
            }
        }

        Err(ImageError::UnauthorizedDomain(url_str.to_string()))
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
            && !url.path().starts_with(pathname)
        {
            return false;
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
            return Err(ImageError::InvalidUrl("Local file paths not yet supported".to_string()));
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

        let bytes = response.bytes().await.map_err(|e| ImageError::FetchError(e.to_string()))?;

        Ok(bytes.to_vec())
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
            .map_err(|e| ImageError::ProcessingError(e.to_string()))?;

        let processed = if let Some(width) = params.w {
            if width < img.width() {
                img.resize(width, u32::MAX, FilterType::Lanczos3)
            } else {
                img
            }
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
