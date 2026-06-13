use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsReloadResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsModuleCacheInfo {
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    pub specifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsComponentVerification {
    pub success: bool,
    pub component_id: String,
    pub exists: bool,
    pub is_function: bool,
    pub is_valid_component: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    pub timestamp: f64,
}

impl JsReloadResult {
    pub fn success(specifier: String) -> Self {
        Self {
            success: true,
            specifier: Some(specifier),
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
            ),
            error: None,
            stack: None,
            name: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            specifier: None,
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
            ),
            error: Some(error),
            stack: None,
            name: Some("ReloadError".to_string()),
        }
    }
}

impl JsModuleCacheInfo {
    pub fn found(component_id: String, specifier: String) -> Self {
        Self { found: true, component_id: Some(component_id), specifier, error: None, stack: None }
    }

    pub fn not_found(specifier: String) -> Self {
        Self { found: false, component_id: None, specifier, error: None, stack: None }
    }
}

impl JsComponentVerification {
    pub fn success(component_id: String) -> Self {
        Self {
            success: true,
            component_id,
            exists: true,
            is_function: true,
            is_valid_component: true,
            component_name: None,
            component_type: None,
            error: None,
            stack: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        }
    }

    pub fn failure(component_id: String, error: String) -> Self {
        Self {
            success: false,
            component_id,
            exists: false,
            is_function: false,
            is_valid_component: false,
            component_name: None,
            component_type: None,
            error: Some(error),
            stack: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        }
    }
}
