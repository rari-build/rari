use crate::rsc::streaming::RscStream;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct LayoutRenderContext {
    pub params: FxHashMap<String, String>,
    pub search_params: FxHashMap<String, Vec<String>>,
    pub headers: FxHashMap<String, String>,
    pub pathname: String,
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
    StaticWithPayload { html: String, rsc_payload: String },
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
