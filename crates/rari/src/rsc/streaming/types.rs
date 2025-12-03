use serde::Serialize;

#[derive(Debug, Clone)]
pub struct PartialRenderResult {
    pub initial_content: serde_json::Value,
    pub pending_promises: Vec<PendingSuspensePromise>,
    pub boundaries: Vec<SuspenseBoundaryInfo>,
    pub has_suspense: bool,
}

#[derive(Debug, Clone)]
pub struct PendingSuspensePromise {
    pub id: String,
    pub boundary_id: String,
    pub component_path: String,
    pub promise_handle: String,
}

#[derive(Debug, Clone)]
pub struct SuspenseBoundaryInfo {
    pub id: String,
    pub fallback_content: serde_json::Value,
    pub parent_boundary_id: Option<String>,
    pub pending_promise_count: usize,
    pub parent_path: Vec<String>,
    pub is_in_content_area: bool,
    pub skeleton_rendered: bool,
    pub is_resolved: bool,
    pub position_hints: Option<PositionHints>,
}

#[derive(Debug, Clone)]
pub struct PositionHints {
    pub in_content_area: bool,
    pub dom_path: Vec<String>,
    pub is_stable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryUpdate {
    pub boundary_id: String,
    pub content: serde_json::Value,
    pub row_id: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dom_path: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryError {
    pub boundary_id: String,
    pub error_message: String,
    pub row_id: u32,
}

#[derive(Debug, Clone)]
pub struct RscStreamChunk {
    pub data: Vec<u8>,
    pub chunk_type: RscChunkType,
    pub row_id: u32,
    pub is_final: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RscChunkType {
    ModuleImport,
    InitialShell,
    BoundaryUpdate,
    BoundaryError,
    StreamComplete,
}
