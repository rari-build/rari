use cow_utils::CowUtils;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Deserializer, Serialize, de::Error};

pub const DEFAULT_IMAGE_QUALITY: u8 = 75;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Archive, RkyvDeserialize, RkyvSerialize)]
#[rkyv(compare(PartialEq), derive(Debug))]
#[non_exhaustive]
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
            "avif" => Ok(Self::Avif),
            "webp" => Ok(Self::WebP),
            "jpeg" | "jpg" => Ok(Self::Jpeg),
            "png" => Ok(Self::Png),
            "gif" => Ok(Self::Gif),
            _ => Err(Error::unknown_variant(&s, &["avif", "webp", "jpeg", "jpg", "png", "gif"])),
        }
    }
}

impl ImageFormat {
    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/avif" => Some(Self::Avif),
            "image/webp" => Some(Self::WebP),
            "image/jpeg" | "image/jpg" => Some(Self::Jpeg),
            "image/png" => Some(Self::Png),
            "image/gif" => Some(Self::Gif),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Avif => "avif",
            Self::WebP => "webp",
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Gif => "gif",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
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
    DEFAULT_IMAGE_QUALITY
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OptimizedImage {
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
}
