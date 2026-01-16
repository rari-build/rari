use crate::rsc::rendering::streaming::RscStream;
use crate::server::routing::types::ParamValue;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct LayoutRenderContext {
    pub params: FxHashMap<String, ParamValue>,
    pub search_params: FxHashMap<String, Vec<String>>,
    pub headers: FxHashMap<String, String>,
    pub pathname: String,
    pub metadata: Option<PageMetadata>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
pub enum OpenGraphImage {
    Simple(String),
    Detailed(OpenGraphImageDescriptor),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
pub struct RobotsMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nocache: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
pub enum IconValue {
    Single(String),
    Multiple(Vec<String>),
    Detailed(Vec<IconDescriptor>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
pub enum ThemeColorMetadata {
    Simple(String),
    Detailed(Vec<ThemeColorDescriptor>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThemeColorDescriptor {
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppleWebAppMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "statusBarStyle")]
    pub status_bar_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "capable")]
    pub capable: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct BoundaryInfo {
    pub id: String,
    pub has_fallback: bool,
}

#[derive(Debug, Clone)]
pub struct SuspenseDetectionResult {
    pub has_suspense: bool,
    pub boundary_count: usize,
    pub boundaries: Vec<BoundaryInfo>,
}

pub enum RenderResult {
    Static(String),
    Streaming(RscStream),
}

#[derive(Debug, Clone)]
pub struct BoundaryPosition {
    pub boundary_id: String,
    pub parent_path: Vec<usize>,
    pub is_in_content_area: bool,
    pub dom_path: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct LayoutStructure {
    pub has_navigation: bool,
    pub navigation_position: Option<usize>,
    pub content_position: Option<usize>,
    pub suspense_boundaries: Vec<BoundaryPosition>,
}

impl LayoutStructure {
    pub fn new() -> Self {
        Self {
            has_navigation: false,
            navigation_position: None,
            content_position: None,
            suspense_boundaries: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.has_navigation
            && let (Some(nav_pos), Some(content_pos)) =
                (self.navigation_position, self.content_position)
            && nav_pos >= content_pos
        {
            return false;
        }

        for boundary in &self.suspense_boundaries {
            if !boundary.is_in_content_area {
                return false;
            }
        }

        true
    }
}

impl Default for LayoutStructure {
    fn default() -> Self {
        Self::new()
    }
}
