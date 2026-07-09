use bytes::Bytes;
use rustc_hash::FxHashMap;
use tokio::sync::mpsc::Receiver;

use crate::server::routing::types::ParamValue;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LayoutRenderContext {
    pub params: FxHashMap<String, ParamValue>,
    pub search_params: FxHashMap<String, Vec<String>>,
    pub headers: FxHashMap<String, String>,
    pub pathname: String,
    pub template_navigation_id: Option<u32>,
    pub metadata: Option<PageMetadata>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct PageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "openGraph")]
    pub open_graph: Option<OpenGraphMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter: Option<TwitterMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robots: Option<RobotsMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<IconsMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "themeColor")]
    pub theme_color: Option<ThemeColorMetadata>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "appleWebApp")]
    pub apple_web_app: Option<AppleWebAppMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternates: Option<AlternatesMetadata>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct OpenGraphMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "siteName")]
    pub site_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<OpenGraphImage>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub og_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum OpenGraphImage {
    Simple(String),
    Detailed(OpenGraphImageDescriptor),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct OpenGraphImageDescriptor {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct TwitterMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct RobotsMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nocache: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct IconsMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apple: Option<IconValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<Vec<IconDescriptor>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum IconValue {
    Single(String),
    Multiple(Vec<String>),
    Detailed(Vec<IconDescriptor>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct IconDescriptor {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub icon_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum ThemeColorMetadata {
    Simple(String),
    Detailed(Vec<ThemeColorDescriptor>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct ThemeColorDescriptor {
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct AlternatesMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<FxHashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<FxHashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct AppleWebAppMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "statusBarStyle")]
    pub status_bar_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "capable")]
    pub capable: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChunkedContentType {
    Html,
    RscFlight,
}

#[non_exhaustive]
pub enum RenderResult {
    Static(String),
    StaticBinary(Vec<u8>),
    Chunked {
        content_type: ChunkedContentType,
        shell: Bytes,
        closing: Bytes,
        chunks: Receiver<Result<Vec<u8>, String>>,
    },
}
