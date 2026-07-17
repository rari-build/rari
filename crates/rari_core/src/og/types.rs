use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "Shared across crate boundary; backend constructs via literal syntax"
)]
pub struct OgImageParams {
    pub width: u32,
    pub height: u32,
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub props: serde_json::Value,
}

impl Default for OgImageParams {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 630,
            content_type: "image/png".to_string(),
            props: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "Shared across crate boundary; backend constructs via literal syntax"
)]
pub struct OgImageResult {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "Cross-crate boundary type: backends construct exhaustively; in-tree so no semver isolation"
)]
pub struct JsxElement {
    #[serde(rename = "type")]
    pub element_type: String,
    pub props: serde_json::Value,
    pub children: Vec<JsxChild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[expect(
    clippy::exhaustive_enums,
    reason = "Cross-crate boundary type: backends match exhaustively; in-tree so no semver isolation"
)]
pub enum JsxChild {
    Element(Box<JsxElement>),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "Shared across crate boundary; backend constructs via literal syntax"
)]
pub struct OgImageEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    #[serde(rename = "additionalPaths", default, skip_serializing_if = "Option::is_none")]
    pub additional_paths: Option<Vec<String>>,
}
