use std::{path::PathBuf, sync::Arc};

use dashmap::DashMap;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

pub mod request;

use rari_core::state::CoreState;

use crate::{
    RscHtmlRenderer, RscRenderer,
    rendering::layout::LayoutHtmlCache,
    server::{
        cache::CacheHandlerRegistry,
        og::OgImageGenerator,
        routing::{ApiRouteHandler, AppRouter},
    },
};

#[derive(Clone)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary")]
pub struct ServerState {
    /// Shared core infrastructure (config, caches, image pipeline, etc.).
    pub core: Arc<CoreState>,
    /// RSC Flight-protocol renderer.
    pub renderer: Arc<Mutex<RscRenderer>>,
    /// React SSR renderer (Fizz).
    pub ssr_renderer: Arc<RscHtmlRenderer>,
    /// Component-level cache configuration overrides.
    pub component_cache_configs: Arc<RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    /// Page-level cache configuration overrides.
    pub page_cache_configs: Arc<RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    /// Application route table (from routes manifest).
    pub app_router: Option<Arc<AppRouter>>,
    /// API route handler.
    pub api_route_handler: Option<Arc<ApiRouteHandler>>,
    /// General-purpose string HTML cache.
    pub html_cache: Arc<DashMap<String, String>>,
    /// Layout HTML cache (Fizz-rendered layout output).
    pub layout_html_cache: Arc<LayoutHtmlCache>,
    /// Optional OG image generator.
    pub og_generator: Option<Arc<OgImageGenerator>>,
    /// Cache handler registry (resolves named cache layers to handlers).
    pub cache_registry: Arc<CacheHandlerRegistry>,
    /// Project root directory (duplicated from core for ergonomics).
    pub project_root: PathBuf,
}

#[derive(Debug, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary")]
pub struct RenderRequest {
    pub component_id: String,
    pub props: Option<Value>,
    pub ssr: Option<bool>,
}

#[derive(Debug, Serialize)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary")]
pub struct RenderResponse {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
    pub component_id: String,
    pub render_time_ms: u64,
}

#[derive(Debug, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary")]
pub struct RegisterRequest {
    pub component_id: String,
    pub component_code: String,
    pub cache_config: Option<FxHashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary")]
pub struct RegisterClientRequest {
    pub component_id: String,
    pub file_path: String,
    pub export_name: String,
}

#[derive(Debug, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary")]
pub struct HmrRegisterRequest {
    pub file_path: String,
}

#[derive(Debug, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary")]
pub struct ReloadComponentRequest {
    pub component_id: String,
    pub bundle_path: String,
}

#[derive(Debug, Serialize)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary")]
pub struct ReloadComponentResponse {
    pub success: bool,
    pub message: String,
}
