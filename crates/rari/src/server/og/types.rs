use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct OgImageResult {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsxElement {
    #[serde(rename = "type")]
    pub element_type: String,
    pub props: serde_json::Value,
    pub children: Vec<JsxChild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsxChild {
    Element(Box<JsxElement>),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OgImageEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}
