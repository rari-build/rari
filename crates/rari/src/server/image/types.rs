use cow_utils::CowUtils;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ImageFormat {
    Avif,
    WebP,
    Jpeg,
    Png,
    Gif,
}

impl<'de> Deserialize<'de> for ImageFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.cow_to_lowercase().as_ref() {
            "avif" => Ok(ImageFormat::Avif),
            "webp" => Ok(ImageFormat::WebP),
            "jpeg" | "jpg" => Ok(ImageFormat::Jpeg),
            "png" => Ok(ImageFormat::Png),
            "gif" => Ok(ImageFormat::Gif),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["avif", "webp", "jpeg", "jpg", "png", "gif"],
            )),
        }
    }
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
