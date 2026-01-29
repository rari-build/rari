use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

pub mod request;

use crate::server::cache::response_cache;
use crate::server::config;
use crate::server::og::OgImageGenerator;
use crate::server::routing;
use crate::server::security::csrf::CsrfTokenManager;
use crate::server::security::ip_rate_limiter::EndpointRateLimiters;

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
    pub csrf_manager: Option<Arc<CsrfTokenManager>>,
    pub og_generator: Option<Arc<OgImageGenerator>>,
    pub project_root: PathBuf,
    pub endpoint_rate_limiters: EndpointRateLimiters,
    pub image_optimizer: Option<Arc<crate::server::image::ImageOptimizer>>,
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
