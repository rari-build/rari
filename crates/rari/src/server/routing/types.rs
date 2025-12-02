use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteSegmentType {
    Static,
    Dynamic,
    CatchAll,
    OptionalCatchAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSegment {
    #[serde(rename = "type")]
    pub segment_type: RouteSegmentType,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}
