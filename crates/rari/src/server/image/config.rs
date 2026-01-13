use super::ImageFormat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfig {
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size: usize,
    #[serde(default)]
    pub remote_patterns: Vec<RemotePattern>,
    #[serde(default)]
    pub local_patterns: Vec<LocalPattern>,
    #[serde(default = "default_device_sizes")]
    pub device_sizes: Vec<u32>,
    #[serde(default = "default_image_sizes")]
    pub image_sizes: Vec<u32>,
    #[serde(default = "default_formats")]
    pub formats: Vec<ImageFormat>,
    #[serde(default = "default_quality_allowlist")]
    pub quality_allowlist: Vec<u8>,
    #[serde(default = "default_max_redirects")]
    pub max_redirects: u8,
    #[serde(default = "default_minimum_cache_ttl")]
    pub minimum_cache_ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePattern {
    pub protocol: Option<String>,
    pub hostname: String,
    pub port: Option<String>,
    pub pathname: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPattern {
    pub pathname: String,
    pub search: Option<String>,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            max_cache_size: default_max_cache_size(),
            remote_patterns: Vec::new(),
            local_patterns: Vec::new(),
            device_sizes: default_device_sizes(),
            image_sizes: default_image_sizes(),
            formats: default_formats(),
            quality_allowlist: default_quality_allowlist(),
            max_redirects: default_max_redirects(),
            minimum_cache_ttl: default_minimum_cache_ttl(),
        }
    }
}

fn default_max_cache_size() -> usize {
    100 * 1024 * 1024
}

fn default_device_sizes() -> Vec<u32> {
    vec![640, 750, 828, 1080, 1200, 1920, 2048, 3840]
}

fn default_image_sizes() -> Vec<u32> {
    vec![16, 32, 48, 64, 96, 128, 256, 384]
}

fn default_formats() -> Vec<ImageFormat> {
    vec![ImageFormat::WebP]
}

fn default_quality_allowlist() -> Vec<u8> {
    vec![25, 50, 75, 100]
}

fn default_max_redirects() -> u8 {
    3
}

fn default_minimum_cache_ttl() -> u64 {
    60
}
