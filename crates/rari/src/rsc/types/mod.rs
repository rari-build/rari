use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;

pub mod elements;
pub mod tree;

#[derive(Debug, Clone)]
pub enum RscElement {
    Component {
        tag: String,
        key: Option<String>,
        props: FxHashMap<String, JsonValue>,
    },
    Suspense {
        fallback_ref: String,
        children_ref: String,
        boundary_id: String,
        props: FxHashMap<String, JsonValue>,
    },
    Promise {
        promise_id: String,
    },
    Reference(String),
    Text(String),
}

#[derive(Debug, Clone)]
pub struct SuspenseBoundary {
    pub boundary_id: String,
    pub fallback_ref: String,
    pub children_ref: String,
    pub has_promise: bool,
    pub promise_ids: Vec<String>,
    pub row_id: u32,
}
