use crate::error::RariError;
use crate::rsc::layout_renderer::LayoutRenderer;
use crate::rsc::renderer::{ResourceLimits, RscRenderer};

use crate::runtime::JsExecutionRuntime;
use crate::server::config::Config;
use crate::server::request_middleware::{
    cors_middleware, request_logger, security_headers_middleware,
};
use crate::server::vite_proxy::{
    check_vite_server_health, display_vite_proxy_info, vite_reverse_proxy, vite_src_proxy,
    vite_websocket_proxy,
};
use axum::http::HeaderValue;
use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    middleware,
    response::{Json, Response},
    routing::{any, get, post},
};
use colored::Colorize;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tracing::{debug, error, info, warn};

pub mod app_router;
pub mod config;
pub mod request_middleware;
pub mod vite_proxy;

const RSC_CONTENT_TYPE: &str = "text/x-component";
const CHUNKED_ENCODING: &str = "chunked";
const SERVER_MANIFEST_PATH: &str = "dist/server-manifest.json";
const DIST_DIR: &str = "dist";

#[derive(Clone)]
pub struct ServerState {
    pub renderer: Arc<tokio::sync::Mutex<RscRenderer>>,
    pub config: Arc<Config>,
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
    pub start_time: std::time::Instant,
    pub component_cache_configs:
        Arc<tokio::sync::RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    pub page_cache_configs: Arc<tokio::sync::RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    pub app_router: Option<Arc<app_router::AppRouter>>,
}

pub struct Server {
    router: Router,
    config: Config,
    listener: TcpListener,
    address: SocketAddr,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self, RariError> {
        info!("Initializing Rari server in {} mode", config.mode);

        Config::set_global(config.clone())
            .map_err(|_| RariError::configuration("Failed to set global config".to_string()))?;

        if let Err(e) = dotenvy::dotenv() {
            debug!("No .env file found or error loading .env: {}", e);
        }

        let env_vars: FxHashMap<String, String> = std::env::vars().collect();

        let js_runtime = Arc::new(JsExecutionRuntime::new(Some(env_vars)));

        let resource_limits = ResourceLimits {
            max_script_execution_time_ms: config.rsc.script_execution_timeout_ms,
            ..ResourceLimits::default()
        };

        let mut renderer = RscRenderer::with_resource_limits(js_runtime, resource_limits);
        renderer.initialize().await?;

        if config.is_production() {
            Self::load_production_components(&mut renderer).await?;
        }

        Self::load_app_router_components(&mut renderer, &config).await?;

        let app_router = {
            let manifest_path = "dist/app-routes.json";

            match app_router::AppRouter::from_file(manifest_path).await {
                Ok(router) => {
                    info!(
                        "Loaded app router from {} with {} routes",
                        manifest_path,
                        router.manifest().routes.len()
                    );
                    Some(Arc::new(router))
                }
                Err(e) => {
                    debug!("No app router manifest found at {}: {}", manifest_path, e);
                    None
                }
            }
        };

        let state = ServerState {
            renderer: Arc::new(tokio::sync::Mutex::new(renderer)),
            config: Arc::new(config.clone()),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            start_time: std::time::Instant::now(),
            component_cache_configs: Arc::new(tokio::sync::RwLock::new(FxHashMap::default())),
            page_cache_configs: Arc::new(tokio::sync::RwLock::new(FxHashMap::default())),
            app_router,
        };

        if config.is_production() {
            Self::load_page_cache_configs(&state).await?;
            Self::load_vite_cache_config(&state).await?;
        }

        let router = Self::build_router(&config, state.clone()).await?;

        let address = config.server_address();
        info!("Binding server to {}", address);

        let listener = TcpListener::bind(&address)
            .await
            .map_err(|e| RariError::network(format!("Failed to bind to {address}: {e}")))?;

        let socket_addr = listener
            .local_addr()
            .map_err(|e| RariError::network(format!("Failed to get local address: {e}")))?;

        Ok(Self { router, config, listener, address: socket_addr })
    }

    async fn build_router(config: &Config, state: ServerState) -> Result<Router<()>, RariError> {
        let mut router = Router::new()
            .route("/api/test", get(test_handler))
            .route("/api/rsc/stream", post(stream_component))
            .route("/api/rsc/stream", axum::routing::options(cors_preflight_ok))
            .route("/api/rsc/register", post(register_component))
            .route("/api/rsc/register-direct", post(register_component_direct))
            .route("/api/rsc/register-client", post(register_client_component))
            .route("/api/rsc/hmr-register", post(hmr_register_component))
            .route("/api/rsc/components", get(list_components))
            .route("/api/rsc/health", get(health_check))
            .route("/api/rsc/status", get(server_status))
            .route("/_rsc_status", get(rsc_status_handler))
            .route("/rsc/render/{component_id}", get(rsc_render_handler))
            .route("/api/{*path}", axum::routing::options(cors_preflight_ok));

        if config.is_development() {
            info!("Adding development routes");

            router = router
                .route("/vite-server/", get(vite_websocket_proxy))
                .route("/vite-server/{*path}", any(vite_reverse_proxy))
                .route("/src/{*path}", any(vite_src_proxy));

            router = router.layer(middleware::from_fn(cors_middleware));

            if let Err(e) = check_vite_server_health().await {
                warn!("Vite development server check failed: {}", e);
                warn!("Make sure to start your Vite dev server for HMR to work");
            }
        } else {
            router = router.layer(middleware::from_fn(security_headers_middleware));
        }

        let has_app_router = std::path::Path::new("dist/app-routes.json").exists();

        if has_app_router {
            info!("App router enabled - using app route handler");
            router =
                router.route("/", get(handle_app_route)).route("/{*path}", get(handle_app_route));
        } else if config.is_production() {
            router =
                router.route("/", get(root_handler)).route("/{*path}", get(static_or_spa_handler));
        } else {
            let static_service =
                ServeDir::new(config.public_dir()).append_index_html_on_directories(true);
            router = router.fallback_service(static_service);
        }

        let middleware_stack =
            ServiceBuilder::new().layer(middleware::from_fn(request_logger)).into_inner();

        router = router.layer(middleware_stack);
        router = router.layer(DefaultBodyLimit::max(1024 * 1024 * 100));

        Ok(router.with_state(state))
    }

    pub async fn start(self) -> Result<(), RariError> {
        self.display_startup_message();

        info!("Starting Rari server on {}", self.address);

        axum::serve(self.listener, self.router)
            .await
            .map_err(|e| RariError::network(format!("Server error: {e}")))?;

        info!("Server shutdown complete");
        Ok(())
    }

    fn display_startup_message(&self) {
        let server_url = format!("http://{}", self.address);

        println!();
        println!("{}", "🚀 Rari Server".blue().bold());
        println!();

        if self.config.is_production() {
            println!("  {} {}", "Mode:".bold(), "Production".green());
        } else {
            println!("  {} {}", "Mode:".bold(), "Development".yellow());
        }

        println!("  {} {}", "Server:".bold(), server_url.cyan().underline());

        if let Some(origin) = &self.config.server.origin {
            println!("  {} {}", "Origin:".bold(), origin.cyan());
        }

        println!();

        if self.config.is_development() {
            display_vite_proxy_info();
        }

        println!("{}", "Server is ready!".green().bold());
        println!();
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    async fn load_production_components(renderer: &mut RscRenderer) -> Result<(), RariError> {
        info!("Loading production components from manifest");

        let manifest_path = std::path::Path::new(SERVER_MANIFEST_PATH);
        if !manifest_path.exists() {
            warn!(
                "No server manifest found at {}, production components will not be available",
                manifest_path.display()
            );
            return Ok(());
        }

        let manifest = Self::read_manifest(manifest_path)?;
        let components = Self::parse_manifest_components(&manifest)?;

        let mut loaded_count = 0;
        for (component_id, component_info) in components {
            match Self::load_component_from_manifest(component_id, component_info, renderer).await {
                Ok(()) => {
                    debug!("Loaded production component: {}", component_id);
                    loaded_count += 1;
                }
                Err(e) => {
                    error!("Failed to load component {}: {}", component_id, e);
                }
            }
        }

        info!("Loaded {} production components", loaded_count);
        Ok(())
    }

    async fn load_page_cache_configs(state: &ServerState) -> Result<(), RariError> {
        info!("Loading page cache configurations");

        let pages_dir = std::path::Path::new("src/pages");
        if !pages_dir.exists() {
            debug!("No pages directory found, skipping page cache config loading");
            return Ok(());
        }

        let mut loaded_count = 0;
        Self::scan_pages_directory(pages_dir, state, &mut loaded_count).await?;

        info!("Loaded {} page cache configurations", loaded_count);
        Ok(())
    }

    async fn scan_pages_directory(
        dir: &std::path::Path,
        state: &ServerState,
        loaded_count: &mut usize,
    ) -> Result<(), RariError> {
        let mut dirs_to_scan = vec![dir.to_path_buf()];

        while let Some(current_dir) = dirs_to_scan.pop() {
            let entries = std::fs::read_dir(&current_dir)
                .map_err(|e| RariError::io(format!("Failed to read pages directory: {e}")))?;

            for entry in entries {
                let entry = entry
                    .map_err(|e| RariError::io(format!("Failed to read directory entry: {e}")))?;
                let path = entry.path();

                if path.is_dir() {
                    dirs_to_scan.push(path);
                } else if path.is_file()
                    && let Some(extension) = path.extension()
                    && (extension == "tsx"
                        || extension == "jsx"
                        || extension == "ts"
                        || extension == "js")
                {
                    if let Err(e) = Self::load_page_cache_config(&path, state).await {
                        warn!("Failed to load cache config for {}: {}", path.display(), e);
                    } else {
                        *loaded_count += 1;
                    }
                }
            }
        }

        Ok(())
    }

    async fn load_page_cache_config(
        page_path: &std::path::Path,
        state: &ServerState,
    ) -> Result<(), RariError> {
        let content = std::fs::read_to_string(page_path)
            .map_err(|e| RariError::io(format!("Failed to read page file: {e}")))?;

        if let Some(cache_config) = Self::extract_cache_config_from_content(&content) {
            let route_path = Self::page_path_to_route(page_path)?;

            let mut page_configs = state.page_cache_configs.write().await;
            page_configs.insert(route_path.clone(), cache_config);

            debug!("Loaded cache config for route: {}", route_path);
        }

        Ok(())
    }

    fn page_path_to_route(page_path: &std::path::Path) -> Result<String, RariError> {
        let pages_dir = std::path::Path::new("src/pages");
        let relative_path = page_path.strip_prefix(pages_dir).map_err(|_| {
            RariError::configuration("Page path is not within pages directory".to_string())
        })?;

        let route = relative_path.with_extension("").to_string_lossy().replace('\\', "/");

        let route = if route == "index" { "/".to_string() } else { format!("/{}", route) };

        Ok(route)
    }

    fn extract_cache_config_from_content(content: &str) -> Option<FxHashMap<String, String>> {
        let cache_config_regex =
            Regex::new(r"export\s+const\s+cacheConfig\s*:\s*\w+\s*=\s*\{([^}]+)\}").ok()?;

        if let Some(captures) = cache_config_regex.captures(content) {
            let config_content = captures.get(1)?.as_str();
            let mut config = FxHashMap::default();

            let cache_control_regex = Regex::new(r"'cache-control'\s*:\s*'([^']+)'").ok()?;
            if let Some(cache_control_match) = cache_control_regex.captures(config_content) {
                config.insert(
                    "cache-control".to_string(),
                    cache_control_match.get(1)?.as_str().to_string(),
                );
            }

            let vary_regex = Regex::new(r"'vary'\s*:\s*'([^']+)'").ok()?;
            if let Some(vary_match) = vary_regex.captures(config_content) {
                config.insert("vary".to_string(), vary_match.get(1)?.as_str().to_string());
            }

            if !config.is_empty() {
                debug!("Extracted cache config: {:?}", config);
                return Some(config);
            }
        }

        None
    }

    fn find_matching_cache_config<'a>(
        page_configs: &'a FxHashMap<String, FxHashMap<String, String>>,
        route_path: &str,
    ) -> Option<&'a FxHashMap<String, String>> {
        if let Some(config) = page_configs.get(route_path) {
            return Some(config);
        }

        for (pattern, config) in page_configs {
            if Server::matches_route_pattern(pattern, route_path) {
                return Some(config);
            }
        }

        None
    }

    fn matches_route_pattern(pattern: &str, path: &str) -> bool {
        if pattern == path {
            return true;
        }

        if let Some(prefix) = pattern.strip_suffix("/*") {
            return path.starts_with(prefix)
                && (path.len() == prefix.len() || path.chars().nth(prefix.len()) == Some('/'));
        }

        if pattern.contains('*') {
            let regex_pattern = pattern.replace('*', ".*").replace('/', "\\/");
            if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                return regex.is_match(path);
            }
        }

        false
    }

    async fn load_vite_cache_config(state: &ServerState) -> Result<(), RariError> {
        let cache_config_path = std::path::Path::new("dist/cache-config.json");

        if !cache_config_path.exists() {
            debug!("No vite cache config file found, using defaults");
            return Ok(());
        }

        match std::fs::read_to_string(cache_config_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(config_json) => {
                    if let Some(routes) = config_json.get("routes").and_then(|r| r.as_object()) {
                        let mut page_configs = state.page_cache_configs.write().await;

                        for (route, cache_control) in routes {
                            if let Some(cache_str) = cache_control.as_str()
                                && !page_configs.contains_key(route)
                            {
                                let mut cache_config = FxHashMap::default();
                                cache_config
                                    .insert("cache-control".to_string(), cache_str.to_string());
                                page_configs.insert(route.clone(), cache_config);
                            }
                        }

                        info!("Loaded vite cache configuration with {} routes", routes.len());
                    }
                }
                Err(e) => {
                    warn!("Failed to parse vite cache config: {}", e);
                }
            },
            Err(e) => {
                warn!("Failed to read vite cache config file: {}", e);
            }
        }

        Ok(())
    }

    fn read_manifest(manifest_path: &std::path::Path) -> Result<serde_json::Value, RariError> {
        let manifest_content = std::fs::read_to_string(manifest_path)
            .map_err(|e| RariError::io(format!("Failed to read server manifest: {e}")))?;

        serde_json::from_str(&manifest_content)
            .map_err(|e| RariError::configuration(format!("Failed to parse server manifest: {e}")))
    }

    fn parse_manifest_components(
        manifest: &serde_json::Value,
    ) -> Result<&serde_json::Map<String, serde_json::Value>, RariError> {
        manifest.get("components").and_then(|c| c.as_object()).ok_or_else(|| {
            RariError::configuration("Invalid manifest: missing components".to_string())
        })
    }

    async fn load_component_from_manifest(
        component_id: &str,
        component_info: &serde_json::Value,
        renderer: &mut RscRenderer,
    ) -> Result<(), RariError> {
        let bundle_path =
            component_info.get("bundlePath").and_then(|p| p.as_str()).ok_or_else(|| {
                RariError::configuration(format!("Component {component_id} missing bundlePath"))
            })?;

        let component_file = std::path::Path::new(DIST_DIR).join(bundle_path);

        if !component_file.exists() {
            return Err(RariError::not_found(format!(
                "Component file not found: {}",
                component_file.display()
            )));
        }

        let component_code = std::fs::read_to_string(&component_file)
            .map_err(|e| RariError::io(format!("Failed to read component file: {e}")))?;

        renderer
            .register_component(component_id, &component_code)
            .await
            .map_err(|e| RariError::internal(format!("Failed to register component: {e}")))
    }

    async fn load_app_router_components(
        renderer: &mut RscRenderer,
        _config: &Config,
    ) -> Result<(), RariError> {
        info!("Loading app router components");

        let server_dir = std::path::Path::new(DIST_DIR).join("server");
        if !server_dir.exists() {
            debug!(
                "No server directory found at {}, skipping app router component loading",
                server_dir.display()
            );
            return Ok(());
        }

        let mut loaded_count = 0;
        Self::load_server_components_recursive(
            &server_dir,
            &server_dir,
            renderer,
            &mut loaded_count,
        )
        .await?;

        info!("Loaded {} app router components", loaded_count);
        Ok(())
    }

    fn load_server_components_recursive<'a>(
        dir: &'a std::path::Path,
        base_dir: &'a std::path::Path,
        renderer: &'a mut RscRenderer,
        loaded_count: &'a mut usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), RariError>> + 'a>> {
        Box::pin(async move {
            let entries = std::fs::read_dir(dir).map_err(|e| {
                RariError::io(format!("Failed to read directory {}: {}", dir.display(), e))
            })?;

            for entry in entries {
                let entry = entry
                    .map_err(|e| RariError::io(format!("Failed to read directory entry: {e}")))?;
                let path = entry.path();

                if path.is_dir() {
                    Self::load_server_components_recursive(&path, base_dir, renderer, loaded_count)
                        .await?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("js") {
                    let component_code = std::fs::read_to_string(&path).map_err(|e| {
                        RariError::io(format!("Failed to read component file: {e}"))
                    })?;

                    let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);
                    let relative_str = relative_path
                        .to_str()
                        .unwrap_or("unknown")
                        .replace(".js", "")
                        .replace('\\', "/");

                    let component_id = if relative_str.starts_with("app/") {
                        relative_str.clone()
                    } else {
                        relative_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string()
                    };

                    debug!("Loading component: {} from {:?}", component_id, path);

                    let is_client_component = component_code.contains("'use client'")
                        || component_code.contains("\"use client\"");

                    let cleaned_code = strip_module_syntax(&component_code);

                    match renderer
                        .runtime
                        .execute_script(
                            format!("load_{}.js", component_id.replace('/', "_")),
                            cleaned_code,
                        )
                        .await
                    {
                        Ok(_) => {
                            debug!("Successfully executed component code: {}", component_id);

                            let mark_client_script = if is_client_component {
                                format!(
                                    r#"(function() {{
                                        const comp = globalThis["{}"];
                                        if (comp && typeof comp === 'function') {{
                                            comp.__isClientComponent = true;
                                            comp.__clientComponentId = "{}";
                                        }}
                                        const exists = typeof globalThis["{}"] !== 'undefined';
                                        const type = typeof globalThis["{}"];
                                        return {{ exists, type, componentId: "{}", isClient: true }};
                                    }})()"#,
                                    component_id,
                                    component_id,
                                    component_id,
                                    component_id,
                                    component_id
                                )
                            } else {
                                format!(
                                    r#"(function() {{
                                        const exists = typeof globalThis["{}"] !== 'undefined';
                                        const type = typeof globalThis["{}"];
                                        return {{ exists, type, componentId: "{}", isClient: false }};
                                    }})()"#,
                                    component_id, component_id, component_id
                                )
                            };

                            match renderer
                                .runtime
                                .execute_script(
                                    format!("verify_{}.js", component_id.replace('/', "_")),
                                    mark_client_script,
                                )
                                .await
                            {
                                Ok(result) => {
                                    info!("Component {} in globalThis: {:?}", component_id, result);
                                    *loaded_count += 1;
                                }
                                Err(e) => {
                                    error!("Failed to verify component {}: {}", component_id, e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to execute component {}: {}", component_id, e);
                        }
                    }
                }
            }

            Ok(())
        })
    }
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

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub mode: String,
    pub uptime_seconds: u64,
    pub request_count: u64,
    pub components_registered: usize,
    pub memory_usage: Option<u64>,
}

#[axum::debug_handler]
async fn test_handler() -> &'static str {
    "Hello from Rari server!"
}

#[axum::debug_handler]
async fn stream_component(
    State(state): State<ServerState>,
    Json(request): Json<RenderRequest>,
) -> Result<Response, StatusCode> {
    debug!("Streaming component: {}", request.component_id);

    let props_str = request.props.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let stream_result = {
        let renderer = state.renderer.lock().await;
        renderer.render_with_streaming(&request.component_id, props_str.as_deref()).await
    };

    match stream_result {
        Ok(mut rsc_stream) => {
            debug!("Successfully created true streaming for component: {}", request.component_id);

            let byte_stream = async_stream::stream! {
                while let Some(chunk) = rsc_stream.next_chunk().await {
                    yield Ok::<Vec<u8>, std::io::Error>(chunk.data);
                }
            };

            let body = Body::from_stream(byte_stream);

            let cache_control = state.config.get_cache_control_for_route("/api/rsc/stream");

            Ok(Response::builder()
                .header("content-type", RSC_CONTENT_TYPE)
                .header("cache-control", cache_control)
                .header("transfer-encoding", CHUNKED_ENCODING)
                .body(body)
                .expect("Valid streaming response"))
        }
        Err(e) => {
            error!("Failed to create true streaming for component {}: {}", request.component_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[axum::debug_handler]
async fn register_component(
    State(state): State<ServerState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Registering component: {}", request.component_id);

    if let Some(cache_config) = &request.cache_config {
        let mut cache_configs = state.component_cache_configs.write().await;
        cache_configs.insert(request.component_id.clone(), cache_config.clone());
        debug!("Stored cache config for component: {}", request.component_id);
    }

    let result = {
        let mut renderer = state.renderer.lock().await;
        renderer.register_component(&request.component_id, &request.component_code).await
    };

    match result {
        Ok(_) => {
            info!("Successfully registered component: {}", request.component_id);

            let renderer = state.renderer.lock().await;
            let is_client =
                renderer.serializer.lock().is_client_component_registered(&request.component_id);

            if is_client {
                let mark_script = format!(
                    r#"(function() {{
                        const comp = globalThis["{}"];
                        if (comp && typeof comp === 'function') {{
                            comp.__isClientComponent = true;
                            comp.__clientComponentId = "{}";
                        }}
                    }})()"#,
                    request.component_id, request.component_id
                );

                if let Err(e) = renderer
                    .runtime
                    .execute_script(
                        format!("mark_client_{}.js", request.component_id.replace('/', "_")),
                        mark_script,
                    )
                    .await
                {
                    error!("Failed to mark {} as client component: {}", request.component_id, e);
                } else {
                    debug!("Marked {} as client component", request.component_id);
                }
            }

            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": true,
                "component_id": request.component_id
            })))
        }
        Err(e) => {
            error!("Failed to register component {}: {}", request.component_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn register_component_direct(
    State(state): State<ServerState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Registering component directly (no transform): {}", request.component_id);

    if let Some(cache_config) = &request.cache_config {
        let mut cache_configs = state.component_cache_configs.write().await;
        cache_configs.insert(request.component_id.clone(), cache_config.clone());
        debug!("Stored cache config for component: {}", request.component_id);
    }

    let result = {
        let renderer = state.renderer.lock().await;

        let cleaned_code = strip_module_syntax(&request.component_code);

        renderer
            .runtime
            .execute_script(
                format!("register_direct_{}.js", request.component_id.replace('/', "_")),
                cleaned_code,
            )
            .await
    };

    match result {
        Ok(_) => {
            info!("Successfully registered component directly: {}", request.component_id);
            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": true,
                "component_id": request.component_id
            })))
        }
        Err(e) => {
            error!("Failed to register component directly {}: {}", request.component_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn strip_module_syntax(code: &str) -> String {
    let mut result = String::new();
    let mut in_exports_comment = false;

    for line in code.lines() {
        let trimmed = line.trim();

        if trimmed == "\"use module\";" || trimmed == "'use module';" {
            continue;
        }

        if trimmed.starts_with("// Exports:") {
            in_exports_comment = true;
            continue;
        }

        if in_exports_comment {
            if trimmed.is_empty() || trimmed.starts_with("//") && !trimmed.contains("Exports:") {
                in_exports_comment = false;
                result.push_str(line);
                result.push('\n');
            }
            continue;
        }

        if trimmed.starts_with("import ") || trimmed.starts_with("import{") {
            continue;
        }

        if trimmed.starts_with("export ") {
            let without_export = line.replacen("export ", "", 1);
            result.push_str(&without_export);
            result.push('\n');
        } else if trimmed.starts_with("export{") || trimmed.starts_with("export {") {
            continue;
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

#[axum::debug_handler]
async fn register_client_component(
    State(state): State<ServerState>,
    Json(request): Json<RegisterClientRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!(
        "Registering client component: {} from path: {} with export: {}",
        request.component_id, request.file_path, request.export_name
    );

    {
        let renderer = state.renderer.lock().await;
        renderer.register_client_component(
            &request.component_id,
            &request.file_path,
            &request.export_name,
        );

        let serializer = renderer.serializer.lock();
        let is_registered = serializer.is_client_component_registered(&request.component_id);
        info!("Client component {} registration status: {}", request.component_id, is_registered);
    }

    info!("Successfully registered client component: {}", request.component_id);
    #[allow(clippy::disallowed_methods)]
    Ok(Json(serde_json::json!({
        "success": true,
        "component_id": request.component_id
    })))
}

#[axum::debug_handler]
async fn hmr_register_component(
    State(state): State<ServerState>,
    Json(request): Json<HmrRegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    let file_path = request.file_path.clone();

    if let Err(e) = immediate_component_reregistration(&state, &file_path).await {
        error!("Failed to immediately re-register component for {}: {}", file_path, e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[allow(clippy::disallowed_methods)]
    Ok(Json(serde_json::json!({
        "success": true,
        "file_path": request.file_path
    })))
}

async fn immediate_component_reregistration(
    state: &ServerState,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = std::path::Path::new(file_path);
    let component_name =
        path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("UnknownComponent");

    {
        let mut renderer = state.renderer.lock().await;
        renderer.clear_script_cache();

        if let Err(e) = renderer.clear_component_module_cache(component_name).await {
            warn!("Failed to clear component module cache: {}", e);
        }
    }

    if let Ok(content) = tokio::fs::read_to_string(file_path).await {
        let mut renderer = state.renderer.lock().await;

        if let Err(e) = renderer.register_component(component_name, &content).await {
            error!("Failed to register component directly: {}", e);
        } else {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Err(e) = renderer.clear_component_module_cache(component_name).await {
                warn!("Failed to clear component module cache: {}", e);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            if let Err(e) = renderer.register_component(component_name, &content).await {
                error!("Failed to re-register component after cache clear: {}", e);
                return Ok(());
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            if let Err(e) = renderer.ensure_component_loaded_with_force(component_name, true).await
            {
                error!("Failed to load component after re-registration: {}", e);
                return Ok(());
            }

            let verification_attempts = 3;
            for attempt in 1..=verification_attempts {
                let _unique_id = format!(
                    "{}_{}",
                    component_name,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
                let verification_script = format!(
                    r#"
                    (function() {{
                        const componentName = "{}";
                        const hashedComponentName = "Component_" + componentName;
                        const componentExists = (
                            typeof globalThis[componentName] === 'function' ||
                            typeof globalThis[hashedComponentName] === 'function' ||
                            (globalThis.__rsc_modules && globalThis.__rsc_modules[componentName] &&
                             (typeof globalThis.__rsc_modules[componentName].default === 'function' ||
                              typeof Object.values(globalThis.__rsc_modules[componentName])[0] === 'function'))
                        );

                        const debugInfo = {{
                            globalExists: typeof globalThis[componentName],
                            moduleRegistryExists: !!globalThis.__rsc_modules,
                            moduleExists: globalThis.__rsc_modules ? !!globalThis.__rsc_modules[componentName] : false,
                            moduleDefaultExists: globalThis.__rsc_modules && globalThis.__rsc_modules[componentName] ? typeof globalThis.__rsc_modules[componentName].default : 'module_not_found',
                            moduleFirstExportExists: globalThis.__rsc_modules && globalThis.__rsc_modules[componentName] ? typeof Object.values(globalThis.__rsc_modules[componentName])[0] : 'module_not_found',
                            hashedExists: typeof globalThis[hashedComponentName],
                            availableGlobals: Object.keys(globalThis).filter(k => typeof globalThis[k] === 'function' && k.match(/^[A-Z]/)).slice(0, 20),
                            moduleKeys: globalThis.__rsc_modules ? Object.keys(globalThis.__rsc_modules).slice(0, 20) : []
                        }};

                        return {{ success: componentExists, componentName: componentName, debugInfo: debugInfo }};
                    }})()
                    "#,
                    component_name
                );

                match renderer
                    .runtime
                    .execute_script(
                        format!("hmr_verify_{}.js", component_name),
                        verification_script,
                    )
                    .await
                {
                    Ok(result) => {
                        if let Some(success) = result.get("success").and_then(|v| v.as_bool())
                            && success
                        {
                            return Ok(());
                        }

                        if attempt == verification_attempts {
                            warn!(
                                "Component '{}' verification failed after {} attempts",
                                component_name, verification_attempts
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Component '{}' verification script execution failed on attempt {}: {}",
                            component_name, attempt, e
                        );
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            }

            warn!(
                "Component '{}' verification failed after {} attempts",
                component_name, verification_attempts
            );
            return Ok(());
        }
    } else {
        warn!("Failed to read component file directly, trying Vite transformation");
    }

    let vite_port = std::env::var("VITE_PORT")
        .unwrap_or_else(|_| "5173".to_string())
        .parse::<u16>()
        .unwrap_or(5173);

    let vite_url = format!("http://localhost:{vite_port}/api/vite/hmr-transform");

    #[allow(clippy::disallowed_methods)]
    let request_body = serde_json::json!({
        "filePath": file_path
    });

    info!("Triggering Vite transformation for: {}", file_path);

    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build()?;

    let response = client
        .post(&vite_url)
        .header("Content-Type", "application/json")
        .body(request_body.to_string())
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Vite transformation failed: {error_text}").into());
    }

    let response_data: serde_json::Value = response.json().await?;

    let component_code = if let Some(components) = response_data.get("components") {
        if let Some(component) = components.as_array().and_then(|arr| arr.first()) {
            component.get("code").and_then(|c| c.as_str()).map(|s| s.to_string())
        } else {
            None
        }
    } else {
        None
    };

    if let Some(code) = component_code {
        let mut renderer = state.renderer.lock().await;
        if let Err(e) = renderer.register_component(component_name, &code).await {
            return Err(format!("Failed to register component: {e}").into());
        }
    } else {
        warn!("No component code received from Vite transformation");
    }

    Ok(())
}

#[axum::debug_handler]
async fn list_components(State(state): State<ServerState>) -> Json<Value> {
    let components = {
        let renderer = state.renderer.lock().await;
        renderer.list_components()
    };

    #[allow(clippy::disallowed_methods)]
    Json(serde_json::json!({
        "success": true,
        "components": components
    }))
}

#[axum::debug_handler]
async fn health_check() -> Json<Value> {
    #[allow(clippy::disallowed_methods)]
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[axum::debug_handler]
async fn rsc_status_handler() -> Json<Value> {
    #[allow(clippy::disallowed_methods)]
    Json(serde_json::json!({
        "status": "ready"
    }))
}

#[axum::debug_handler]
async fn rsc_render_handler(
    State(state): State<ServerState>,
    Path(component_id): Path<String>,
    Query(params): Query<FxHashMap<String, String>>,
) -> Result<Response, StatusCode> {
    let start_time = std::time::Instant::now();

    state.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    debug!("RSC rendering component: {}", component_id);

    let props: Option<Value> = params.get("props").and_then(|p| {
        if p.trim().is_empty() || p == "{}" { None } else { serde_json::from_str(p).ok() }
    });

    let props_str = props.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let result = {
        let mut renderer = state.renderer.lock().await;
        renderer.render_to_rsc_format(&component_id, props_str.as_deref()).await
    };

    match result {
        Ok(rsc_data) => {
            let _render_time = start_time.elapsed().as_millis() as u64;

            let cache_configs = state.component_cache_configs.read().await;
            let mut response_builder = Response::builder().header("content-type", RSC_CONTENT_TYPE);

            if let Some(component_cache_config) = cache_configs.get(&component_id) {
                for (key, value) in component_cache_config {
                    response_builder = response_builder.header(key.to_lowercase(), value);
                }
                debug!("Applied component-specific cache headers for: {}", component_id);
            } else {
                let cache_control = state
                    .config
                    .get_cache_control_for_route(&format!("/rsc/render/{}", component_id));
                response_builder = response_builder.header("cache-control", cache_control);
            }

            Ok(response_builder.body(Body::from(rsc_data)).expect("Valid RSC response"))
        }
        Err(e) => {
            let _render_time = start_time.elapsed().as_millis() as u64;

            error!("Failed to render RSC component {}: {}", component_id, e);

            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[axum::debug_handler]
async fn server_status(State(state): State<ServerState>) -> Json<StatusResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let request_count = state.request_count.load(std::sync::atomic::Ordering::Relaxed);
    let components = {
        let renderer = state.renderer.lock().await;
        renderer.list_components()
    };

    Json(StatusResponse {
        status: "running".to_string(),
        mode: state.config.mode.to_string(),
        uptime_seconds: uptime,
        request_count,
        components_registered: components.len(),
        memory_usage: get_memory_usage(),
    })
}

fn get_memory_usage() -> Option<u64> {
    use sysinfo::{Pid, System};

    let mut sys = System::new_all();
    sys.refresh_memory();

    let pid = Pid::from(std::process::id() as usize);

    if let Some(process) = sys.process(pid) {
        return Some(process.memory());
    }

    Some(sys.used_memory() * 1024)
}

async fn root_handler(State(state): State<ServerState>) -> Result<Response, StatusCode> {
    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for root_handler");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let index_path = config.public_dir().join("index.html");
    if index_path.exists() {
        match std::fs::read_to_string(&index_path) {
            Ok(content) => {
                let mut response_builder = Response::builder().header("content-type", "text/html");

                let page_configs = state.page_cache_configs.read().await;
                if let Some(page_cache_config) =
                    Server::find_matching_cache_config(&page_configs, "/")
                {
                    for (key, value) in page_cache_config {
                        response_builder = response_builder.header(key.to_lowercase(), value);
                    }
                    debug!("Applied cache headers for root route");
                }

                return Ok(response_builder
                    .body(Body::from(content))
                    .expect("Valid HTML response"));
            }
            Err(e) => {
                error!("Failed to read index.html: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }
    Err(StatusCode::NOT_FOUND)
}

async fn static_or_spa_handler(
    State(state): State<ServerState>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for static_or_spa_handler");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let file_path = config.public_dir().join(&path);
    if file_path.exists() && file_path.is_file() {
        match std::fs::read(&file_path) {
            Ok(content) => {
                let content_type = get_content_type(&path);
                let cache_control = &config.caching.static_files;
                return Ok(Response::builder()
                    .header("content-type", content_type)
                    .header("cache-control", cache_control)
                    .body(Body::from(content))
                    .expect("Valid static file response"));
            }
            Err(e) => {
                error!("Failed to read static file {}: {}", file_path.display(), e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }
    if path.contains('.') {
        return Err(StatusCode::NOT_FOUND);
    }

    let route_path = if path.is_empty() { "/" } else { &format!("/{}", path) };

    let index_path = config.public_dir().join("index.html");
    if index_path.exists() {
        match std::fs::read_to_string(&index_path) {
            Ok(content) => {
                let mut response_builder = Response::builder().header("content-type", "text/html");

                let page_configs = state.page_cache_configs.read().await;
                if let Some(page_cache_config) =
                    Server::find_matching_cache_config(&page_configs, route_path)
                {
                    for (key, value) in page_cache_config {
                        response_builder = response_builder.header(key.to_lowercase(), value);
                    }
                    debug!("Applied cache headers for route: {}", route_path);
                }

                return Ok(response_builder
                    .body(Body::from(content))
                    .expect("Valid HTML response"));
            }
            Err(e) => {
                error!("Failed to read index.html: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }
    Err(StatusCode::NOT_FOUND)
}

fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

fn cors_preflight_response() -> Response {
    let mut builder = Response::builder().status(StatusCode::NO_CONTENT);
    let headers = builder.headers_mut().expect("Response builder should have headers");
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static(
            "Content-Type, Authorization, Accept, Origin, X-Requested-With, Cache-Control, X-RSC-Streaming",
        ),
    );
    headers.insert("Access-Control-Max-Age", HeaderValue::from_static("86400"));
    builder.body(Body::empty()).expect("Valid preflight response")
}

#[axum::debug_handler]
async fn cors_preflight_ok() -> Response {
    cors_preflight_response()
}

fn extract_search_params(
    query_params: FxHashMap<String, String>,
) -> FxHashMap<String, Vec<String>> {
    let mut search_params = FxHashMap::default();
    for (key, value) in query_params {
        search_params.insert(key, vec![value]);
    }
    search_params
}

fn extract_headers(headers: &axum::http::HeaderMap) -> FxHashMap<String, String> {
    let mut header_map = FxHashMap::default();
    for (name, value) in headers {
        if let Ok(value_str) = value.to_str() {
            header_map.insert(name.to_string(), value_str.to_string());
        }
    }
    header_map
}

#[axum::debug_handler]
async fn handle_app_route(
    State(state): State<ServerState>,
    uri: axum::http::Uri,
    axum::extract::Query(query_params): axum::extract::Query<FxHashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> Result<Response, StatusCode> {
    let path = uri.path();
    let app_router = match &state.app_router {
        Some(router) => router,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let route_match = match app_router.match_route(path) {
        Ok(m) => m,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    debug!("App route matched: {} -> {}", path, route_match.route.path);

    let search_params = extract_search_params(query_params);

    let request_headers = extract_headers(&headers);

    let context = crate::rsc::layout_renderer::create_layout_context(
        route_match.params.clone(),
        search_params,
        request_headers,
    );

    let layout_renderer = LayoutRenderer::new(state.renderer.clone());

    match layout_renderer.render_route(&route_match, &context).await {
        Ok(rsc_wire_format) => {
            debug!("Successfully rendered route with layouts");

            let accept_header = headers.get("accept").and_then(|v| v.to_str().ok()).unwrap_or("");

            let wants_rsc = accept_header.contains("text/x-component");

            if wants_rsc {
                debug!("Sending RSC wire format (Accept: text/x-component)");
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/x-component")
                    .body(Body::from(rsc_wire_format))
                    .expect("Valid RSC response"))
            } else {
                debug!("Sending HTML shell for initial page load");
                let html_shell = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Rari App Router Example</title>
</head>
<body>
  <div id="root"></div>
  <script type="module">
    import 'http://localhost:5173/@id/virtual:rari-entry-client';
  </script>
</body>
</html>"#;

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/html; charset=utf-8")
                    .body(Body::from(html_shell))
                    .expect("Valid HTML response"))
            }
        }
        Err(e) => {
            error!("Failed to render route: {}", e);

            if let Some(error_entry) = &route_match.error {
                match layout_renderer
                    .render_error(&error_entry.file_path, &e.to_string(), &context)
                    .await
                {
                    Ok(error_html) => Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header("content-type", "text/html; charset=utf-8")
                        .body(Body::from(error_html))
                        .expect("Valid HTML response")),
                    Err(_) => {
                        let fallback_html = format!(
                            r#"<!DOCTYPE html>
<html>
<head>
    <title>Error - Rari App Router</title>
    <meta charset="utf-8">
</head>
<body>
    <h1>Error</h1>
    <p>Failed to render route: {}</p>
    <p>Route: {}</p>
</body>
</html>"#,
                            e, route_match.route.path
                        );

                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header("content-type", "text/html; charset=utf-8")
                            .body(Body::from(fallback_html))
                            .expect("Valid HTML response"))
                    }
                }
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::Mode;

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
}
