use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Avif,
    WebP,
    Jpeg,
    Png,
    Gif,
}

impl ImageFormat {
    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/avif" => Some(ImageFormat::Avif),
            "image/webp" => Some(ImageFormat::WebP),
            "image/jpeg" | "image/jpg" => Some(ImageFormat::Jpeg),
            "image/png" => Some(ImageFormat::Png),
            "image/gif" => Some(ImageFormat::Gif),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Avif => "avif",
            ImageFormat::WebP => "webp",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Gif => "gif",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OptimizeParams {
    pub url: String,
    #[serde(default)]
    pub w: Option<u32>,
    #[serde(default = "default_quality")]
    pub q: u8,
    #[serde(default)]
    pub f: Option<String>,
}

fn default_quality() -> u8 {
    75
}

#[derive(Debug, Clone)]
pub struct OptimizedImage {
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
}
