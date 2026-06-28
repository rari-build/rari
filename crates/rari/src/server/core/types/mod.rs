use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};

use dashmap::DashMap;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

pub mod request;

use crate::{
    RscHtmlRenderer, RscRenderer,
    rendering::layout::LayoutHtmlCache,
    server::{
        cache::{CacheHandlerRegistry, handler::CacheHandler, response::ResponseCache},
        config::Config,
        image::ImageOptimizer,
        og::OgImageGenerator,
        routing::{ApiRouteHandler, AppRouter},
    },
};

#[derive(Clone)]
#[non_exhaustive]
pub struct ServerState {
    pub renderer: Arc<Mutex<RscRenderer>>,
    pub ssr_renderer: Arc<RscHtmlRenderer>,
    pub config: Arc<Config>,
    pub request_count: Arc<AtomicU64>,
    pub start_time: Instant,
    pub component_cache_configs: Arc<RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    pub page_cache_configs: Arc<RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    pub app_router: Option<Arc<AppRouter>>,
    pub api_route_handler: Option<Arc<ApiRouteHandler>>,
    pub html_cache: Arc<DashMap<String, String>>,
    pub layout_html_cache: Arc<LayoutHtmlCache>,
    pub response_cache: Arc<ResponseCache>,
    pub og_generator: Option<Arc<OgImageGenerator>>,
    pub project_root: PathBuf,
    pub image_optimizer: Option<Arc<ImageOptimizer>>,
    pub cache_registry: Arc<CacheHandlerRegistry>,
    pub image_handler: Arc<dyn CacheHandler>,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct RenderRequest {
    pub component_id: String,
    pub props: Option<Value>,
    pub ssr: Option<bool>,
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct RenderResponse {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
    pub component_id: String,
    pub render_time_ms: u64,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct RegisterRequest {
    pub component_id: String,
    pub component_code: String,
    pub cache_config: Option<FxHashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct RegisterClientRequest {
    pub component_id: String,
    pub file_path: String,
    pub export_name: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct HmrRegisterRequest {
    pub file_path: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReloadComponentRequest {
    pub component_id: String,
    pub bundle_path: String,
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct ReloadComponentResponse {
    pub success: bool,
    pub message: String,
}
