pub mod actions;
pub mod api_error;
pub mod cache_loader;
pub mod component_loader;
pub mod config;
pub mod core;
pub mod handlers;
pub mod rendering;
pub mod request_context;
pub mod request_middleware;
pub mod request_type;
pub mod response_cache;
pub mod routing;
pub mod streaming_response;
pub mod utils;
pub mod vite;

pub use core::Server;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct ServerState {
    pub renderer: Arc<tokio::sync::Mutex<crate::rsc::RscRenderer>>,
    pub ssr_renderer: Arc<crate::rsc::RscHtmlRenderer>,
    pub config: Arc<config::Config>,
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
    pub start_time: std::time::Instant,
    pub component_cache_configs:
        Arc<tokio::sync::RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    pub page_cache_configs: Arc<tokio::sync::RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    pub app_router: Option<Arc<routing::AppRouter>>,
    pub api_route_handler: Option<Arc<routing::ApiRouteHandler>>,
    pub module_reload_manager: Arc<crate::runtime::module_reload::ModuleReloadManager>,
    pub html_cache: Arc<dashmap::DashMap<String, String>>,
    pub response_cache: Arc<response_cache::ResponseCache>,
}

#[derive(Debug, Deserialize)]
pub struct RenderRequest {
    pub component_id: String,
    pub props: Option<Value>,
    pub ssr: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RenderResponse {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
    pub component_id: String,
    pub render_time_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub component_id: String,
    pub component_code: String,
    pub cache_config: Option<FxHashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterClientRequest {
    pub component_id: String,
    pub file_path: String,
    pub export_name: String,
}

#[derive(Debug, Deserialize)]
pub struct HmrRegisterRequest {
    pub file_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ReloadComponentRequest {
    pub component_id: String,
    pub bundle_path: String,
}

#[derive(Debug, Serialize)]
pub struct ReloadComponentResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub mode: String,
    pub uptime_seconds: u64,
    pub request_count: u64,
    pub components_registered: usize,
    pub memory_usage: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::{Config, Mode};
    use crate::server::rendering::html_utils::is_complete_html_document;

    #[tokio::test]
    async fn test_server_creation() {
        let config = Config::new(Mode::Development);
        let server = Server::new(config).await;
        assert!(server.is_ok());
    }

    #[test]
    fn test_render_request_deserialization() {
        let json = r#"{"component_id": "TestComponent", "props": {"test": "value"}}"#;
        let request: RenderRequest = serde_json::from_str(json).expect("Valid test JSON");
        assert_eq!(request.component_id, "TestComponent");
        assert!(request.props.is_some());
    }

    #[test]
    fn test_render_response_serialization() {
        let response = RenderResponse {
            success: true,
            data: Some("test data".to_string()),
            error: None,
            component_id: "TestComponent".to_string(),
            render_time_ms: 42,
        };

        let json = serde_json::to_string(&response).expect("Valid response JSON");
        assert!(json.contains("TestComponent"));
        assert!(json.contains("test data"));
    }

    #[test]
    fn test_request_type_detection_integration() {
        use crate::server::request_type::{RenderMode, RequestTypeDetector};
        use axum::http::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/html"));
        let mode = RequestTypeDetector::detect_render_mode(&headers);
        assert_eq!(mode, RenderMode::Ssr);

        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/x-component"));
        let mode = RequestTypeDetector::detect_render_mode(&headers);
        assert_eq!(mode, RenderMode::RscNavigation);

        let headers = HeaderMap::new();
        let mode = RequestTypeDetector::detect_render_mode(&headers);
        assert_eq!(mode, RenderMode::Ssr);
    }

    #[test]
    fn test_is_complete_html_document() {
        let complete_with_doctype = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body><div id="root">Content</div></body>
</html>"#;
        assert!(is_complete_html_document(complete_with_doctype));

        let complete_without_doctype = r#"<html>
<head><title>Test</title></head>
<body><div id="root">Content</div></body>
</html>"#;
        assert!(is_complete_html_document(complete_without_doctype));

        let fragment = r#"<div id="root">Content</div>"#;
        assert!(!is_complete_html_document(fragment));

        let partial = r#"<html><head><title>Test</title></head></html>"#;
        assert!(!is_complete_html_document(partial));

        let with_whitespace = r#"
        <!DOCTYPE html>
<html>
<body>Content</body>
</html>"#;
        assert!(is_complete_html_document(with_whitespace));
    }
}
