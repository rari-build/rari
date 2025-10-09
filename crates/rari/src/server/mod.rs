use crate::error::RariError;
use crate::rsc::jsx_transform::{extract_dependencies, transform_jsx};
use crate::rsc::layout_renderer::LayoutRenderer;
use crate::rsc::renderer::{ResourceLimits, RscRenderer};

use crate::runtime::JsExecutionRuntime;
use crate::runtime::dist_path_resolver::DistPathResolver;
use crate::server::actions::{handle_form_action, handle_server_action};
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

pub mod actions;
pub mod app_router;
pub mod config;
pub mod request_middleware;
pub mod vite_proxy;

const RSC_CONTENT_TYPE: &str = "text/x-component";
const CHUNKED_ENCODING: &str = "chunked";
const SERVER_MANIFEST_PATH: &str = "dist/server-manifest.json";
const DIST_DIR: &str = "dist";

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

#[derive(Clone)]
pub struct ServerState {
    pub renderer: Arc<tokio::sync::Mutex<RscRenderer>>,
    pub ssr_renderer: Arc<crate::rsc::ssr_renderer::SsrRenderer>,
    pub config: Arc<Config>,
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
    pub start_time: std::time::Instant,
    pub component_cache_configs:
        Arc<tokio::sync::RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    pub page_cache_configs: Arc<tokio::sync::RwLock<FxHashMap<String, FxHashMap<String, String>>>>,
    pub app_router: Option<Arc<app_router::AppRouter>>,
    pub module_reload_manager: Arc<crate::runtime::module_reload::ModuleReloadManager>,
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

        if config.is_development() {
            Self::load_server_actions_from_source(&mut renderer).await?;
        }

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

        let reload_config = crate::runtime::module_reload::ReloadConfig {
            enabled: config.hmr_reload_enabled(),
            max_retry_attempts: config.rsc.hmr_max_retry_attempts,
            reload_timeout_ms: config.rsc.hmr_reload_timeout_ms,
            parallel_reloads: config.rsc.hmr_parallel_reloads,
            debounce_delay_ms: config.rsc.hmr_debounce_delay_ms,
            max_history_size: config.rsc.hmr_max_history_size,
            enable_memory_monitoring: config.rsc.hmr_enable_memory_monitoring,
        };
        let mut module_reload_manager =
            crate::runtime::module_reload::ModuleReloadManager::new(reload_config);

        module_reload_manager.set_runtime(Arc::clone(&renderer.runtime));
        module_reload_manager.set_component_registry(Arc::clone(&renderer.component_registry));

        let project_root =
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let dist_path_resolver = Arc::new(DistPathResolver::new(project_root));
        module_reload_manager.set_dist_path_resolver(dist_path_resolver);

        let module_reload_manager = Arc::new(module_reload_manager);

        if config.hmr_reload_enabled() {
            info!(
                enabled = true,
                max_retry_attempts = config.rsc.hmr_max_retry_attempts,
                reload_timeout_ms = config.rsc.hmr_reload_timeout_ms,
                parallel_reloads = config.rsc.hmr_parallel_reloads,
                debounce_delay_ms = config.rsc.hmr_debounce_delay_ms,
                "HMR module reloading enabled"
            );
        } else {
            info!(
                enabled = false,
                mode = %config.mode,
                "HMR module reloading disabled"
            );
        }

        let renderer_arc = Arc::new(tokio::sync::Mutex::new(renderer));

        let ssr_renderer = {
            let runtime = renderer_arc.lock().await.runtime.clone();
            let ssr = crate::rsc::ssr_renderer::SsrRenderer::new(runtime);
            ssr.initialize().await?;
            Arc::new(ssr)
        };

        let state = ServerState {
            renderer: renderer_arc,
            ssr_renderer,
            config: Arc::new(config.clone()),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            start_time: std::time::Instant::now(),
            component_cache_configs: Arc::new(tokio::sync::RwLock::new(FxHashMap::default())),
            page_cache_configs: Arc::new(tokio::sync::RwLock::new(FxHashMap::default())),
            app_router,
            module_reload_manager,
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
            .route("/api/rsc/stream", post(stream_component))
            .route("/api/rsc/stream", axum::routing::options(cors_preflight_ok))
            .route("/api/rsc/register", post(register_component))
            .route("/api/rsc/register-client", post(register_client_component))
            .route("/api/rsc/hmr-register", post(hmr_register_component))
            .route("/api/rsc/hmr-register", axum::routing::options(cors_preflight_ok))
            .route("/api/rsc/components", get(list_components))
            .route("/api/rsc/health", get(health_check))
            .route("/api/rsc/status", get(server_status))
            .route("/_rsc_status", get(rsc_status_handler))
            .route("/rsc/render/{component_id}", get(rsc_render_handler))
            .route("/api/rsc/action", post(handle_server_action))
            .route("/api/rsc/form-action", post(handle_form_action))
            .route("/api/{*path}", axum::routing::options(cors_preflight_ok));

        if config.is_development() {
            info!("Adding development routes");

            router = router
                .route("/api/rsc/hmr-invalidate", post(hmr_invalidate_component))
                .route("/api/rsc/hmr-invalidate", axum::routing::options(cors_preflight_ok))
                .route("/api/rsc/hmr-reload", post(hmr_reload_component))
                .route("/api/rsc/hmr-reload", axum::routing::options(cors_preflight_ok))
                .route("/api/rsc/reload-component", post(reload_component))
                .route("/api/rsc/reload-component", axum::routing::options(cors_preflight_ok))
                .route("/vite-server/", get(vite_websocket_proxy))
                .route("/vite-server/{*path}", any(vite_reverse_proxy))
                .route("/src/{*path}", any(vite_src_proxy));

            if let Err(e) = check_vite_server_health().await {
                warn!("Vite development server check failed: {}", e);
                warn!("Make sure to start your Vite dev server for HMR to work");
            }
        }

        let has_app_router = std::path::Path::new("dist/app-routes.json").exists();

        if has_app_router {
            info!("App router enabled - using app route handler");

            if config.is_production() {
                router = router.route("/assets/{*path}", get(serve_static_asset));
            }

            router = router
                .route("/", get(handle_app_route))
                .route("/", axum::routing::options(cors_preflight_ok))
                .route("/{*path}", get(handle_app_route))
                .route("/{*path}", axum::routing::options(cors_preflight_ok));
        } else if config.is_production() {
            router =
                router.route("/", get(root_handler)).route("/{*path}", get(static_or_spa_handler));
        } else {
            let static_service =
                ServeDir::new(config.public_dir()).append_index_html_on_directories(true);
            router = router.fallback_service(static_service);
        }

        if config.is_development() {
            router = router.layer(middleware::from_fn(cors_middleware));
        } else {
            router = router.layer(middleware::from_fn(security_headers_middleware));
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

    async fn load_server_actions_from_source(renderer: &mut RscRenderer) -> Result<(), RariError> {
        info!("Loading server actions from source");

        let src_dir = std::path::Path::new("src");
        if !src_dir.exists() {
            debug!("No src directory found, skipping server action loading");
            return Ok(());
        }

        let mut loaded_count = 0;
        Self::scan_for_server_actions(src_dir, renderer, &mut loaded_count).await?;

        info!("Loaded {} server action files", loaded_count);
        Ok(())
    }

    fn scan_for_server_actions<'a>(
        dir: &'a std::path::Path,
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
                    Self::scan_for_server_actions(&path, renderer, loaded_count).await?;
                } else if path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "ts" || s == "tsx" || s == "js" || s == "jsx")
                    .unwrap_or(false)
                {
                    let code = match std::fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    if has_use_server_directive(&code) {
                        let src_dir = std::path::Path::new("src");
                        let relative_path = path.strip_prefix(src_dir).unwrap_or(&path);
                        let action_id = relative_path
                            .to_str()
                            .unwrap_or("unknown")
                            .replace(".ts", "")
                            .replace(".tsx", "")
                            .replace(".js", "")
                            .replace(".jsx", "")
                            .replace('\\', "/");

                        debug!("Found server action file: {:?} with ID: {}", path, action_id);

                        let dist_path = std::path::Path::new("dist")
                            .join("server")
                            .join(format!("{}.js", action_id));

                        if dist_path.exists() {
                            match std::fs::read_to_string(&dist_path) {
                                Ok(dist_code) => {
                                    let cleaned_code = strip_module_syntax(&dist_code);
                                    match renderer
                                        .runtime
                                        .execute_script(
                                            format!(
                                                "load_action_{}.js",
                                                action_id.replace('/', "_")
                                            ),
                                            cleaned_code,
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            debug!(
                                                "Successfully loaded server action: {}",
                                                action_id
                                            );
                                            *loaded_count += 1;
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to load server action {}: {}",
                                                action_id, e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to read built server action {:?}: {}",
                                        dist_path, e
                                    );
                                }
                            }
                        } else {
                            debug!("Server action not yet built: {:?}", dist_path);
                        }
                    }
                }
            }

            Ok(())
        })
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

                    if has_use_server_directive(&component_code) {
                        let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);
                        let relative_str = relative_path
                            .to_str()
                            .unwrap_or("unknown")
                            .replace(".js", "")
                            .replace('\\', "/");

                        debug!("Loading server action file: {} from {:?}", relative_str, path);

                        let cleaned_code = strip_module_syntax(&component_code);
                        match renderer
                            .runtime
                            .execute_script(
                                format!("load_{}.js", relative_str.replace('/', "_")),
                                cleaned_code,
                            )
                            .await
                        {
                            Ok(_) => {
                                debug!("Successfully loaded server actions from: {}", relative_str);
                                *loaded_count += 1;
                            }
                            Err(e) => {
                                error!(
                                    "Failed to load server actions from {}: {}",
                                    relative_str, e
                                );
                            }
                        }
                        continue;
                    }

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

                    let is_client_component = has_use_client_directive(&component_code);

                    let transformed_module_code =
                        match transform_jsx(&component_code, &component_id) {
                            Ok(code) => code,
                            Err(e) => {
                                error!(
                                    "Failed to transform JSX for component {}: {}",
                                    component_id, e
                                );
                                continue;
                            }
                        };

                    let dependencies = extract_dependencies(&component_code);

                    {
                        let mut registry = renderer.component_registry.lock();
                        let _ = registry.register_component(
                            &component_id,
                            &component_code,
                            transformed_module_code.clone(),
                            dependencies.clone().into_iter().collect(),
                        );
                    }

                    let cleaned_code = strip_module_syntax(&transformed_module_code);

                    match renderer
                        .runtime
                        .execute_script(
                            format!("load_{}.js", component_id.replace('/', "_")),
                            cleaned_code,
                        )
                        .await
                    {
                        Ok(_) => {
                            debug!("Successfully loaded component: {}", component_id);

                            if is_client_component {
                                let mark_client_script = format!(
                                    r#"(function() {{
                                        const comp = globalThis["{}"];
                                        if (comp && typeof comp === 'function') {{
                                            comp.__isClientComponent = true;
                                            comp.__clientComponentId = "{}";
                                        }}
                                        return {{ componentId: "{}", isClient: true }};
                                    }})()"#,
                                    component_id, component_id, component_id
                                );

                                if let Err(e) = renderer
                                    .runtime
                                    .execute_script(
                                        format!(
                                            "mark_client_{}.js",
                                            component_id.replace('/', "_")
                                        ),
                                        mark_client_script,
                                    )
                                    .await
                                {
                                    warn!(
                                        "Failed to mark component {} as client: {}",
                                        component_id, e
                                    );
                                }
                            }

                            *loaded_count += 1;
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
    let is_app_router = request.component_id.starts_with("app/");

    if is_app_router {
        debug!("Registering app router component: {}", request.component_id);
    } else {
        debug!("Registering component: {}", request.component_id);
    }

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
            if is_app_router {
                info!("Successfully registered app router component: {}", request.component_id);
            } else {
                info!("Successfully registered component: {}", request.component_id);
            }

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

fn has_use_client_directive(code: &str) -> bool {
    for line in code.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("//") {
            continue;
        }

        if trimmed.starts_with("/*") {
            continue;
        }

        if trimmed == "'use client';"
            || trimmed == "\"use client\";"
            || trimmed == "'use client'"
            || trimmed == "\"use client\""
        {
            return true;
        }

        if !trimmed.starts_with("'use") && !trimmed.starts_with("\"use") {
            break;
        }
    }

    false
}

fn has_use_server_directive(code: &str) -> bool {
    for line in code.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("//") {
            continue;
        }

        if trimmed.starts_with("/*") {
            continue;
        }

        if trimmed == "'use server';"
            || trimmed == "\"use server\";"
            || trimmed == "'use server'"
            || trimmed == "\"use server\""
        {
            return true;
        }

        if !trimmed.starts_with("'use") && !trimmed.starts_with("\"use") {
            break;
        }
    }

    false
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

async fn reload_component_from_dist(
    state: &ServerState,
    file_path: &str,
    component_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dist_path = match get_dist_path_for_component(file_path) {
        Ok(path) => path,
        Err(e) => {
            error!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to resolve dist path for component"
            );
            return Err(format!("Path resolution error: {}", e).into());
        }
    };

    debug!("Reloading component {} from dist path: {}", component_id, dist_path.display());

    if !dist_path.exists() {
        warn!(
            component_id = component_id,
            dist_path = %dist_path.display(),
            source_path = file_path,
            "Dist file does not exist, Vite may not have finished building"
        );
        return Err(format!(
            "Dist file not found: {}. Vite may not have finished building yet. Last known good version will be preserved.",
            dist_path.display()
        )
        .into());
    }

    debug!("Found dist file at: {}", dist_path.display());

    let dist_code = match tokio::fs::read_to_string(&dist_path).await {
        Ok(code) => code,
        Err(e) => {
            error!(
                component_id = component_id,
                dist_path = %dist_path.display(),
                error = %e,
                error_kind = ?e.kind(),
                "Failed to read dist file. Last known good version will be preserved."
            );
            return Err(format!(
                "Failed to read dist file {}: {}. Last known good version will be preserved.",
                dist_path.display(),
                e
            )
            .into());
        }
    };

    debug!("Read {} bytes from dist file", dist_code.len());

    let cleaned_code = strip_module_syntax(&dist_code);

    let renderer = state.renderer.lock().await;

    let execution_result = renderer
        .runtime
        .execute_script(
            format!("hmr_reload_{}.js", component_id.replace('/', "_")),
            cleaned_code.clone(),
        )
        .await;

    if let Err(e) = execution_result {
        error!(
            component_id = component_id,
            dist_path = %dist_path.display(),
            error = %e,
            code_length = cleaned_code.len(),
            "Failed to execute component code during reload. Last known good version will be preserved."
        );
        return Err(format!(
            "Failed to execute component code: {}. Last known good version will be preserved.",
            e
        )
        .into());
    }

    let verification_script = format!(
        r#"(function() {{
            const expectedKey = '{}';
            const exists = typeof globalThis[expectedKey] !== 'undefined';
            const type = typeof globalThis[expectedKey];

            const allKeys = Object.keys(globalThis).filter(key =>
                typeof globalThis[key] === 'function' ||
                typeof globalThis[key] === 'object'
            );

            return {{
                success: exists,
                componentId: expectedKey,
                type: type,
                hasDefault: exists,
                expectedKey: expectedKey,
                actualKeys: allKeys
            }};
        }})()"#,
        component_id
    );

    let result_json = match renderer
        .runtime
        .execute_script(
            format!("verify_{}.js", component_id.replace('/', "_")),
            verification_script,
        )
        .await
    {
        Ok(json) => json,
        Err(e) => {
            error!(
                component_id = component_id,
                error = %e,
                "Failed to execute verification script. Last known good version will be preserved."
            );
            return Err(format!(
                "Failed to verify component reload: {}. Last known good version will be preserved.",
                e
            )
            .into());
        }
    };

    if let Some(success) = result_json.get("success").and_then(|v| v.as_bool()) {
        if success {
            info!(
                component_id = component_id,
                dist_path = %dist_path.display(),
                "Component successfully reloaded from dist"
            );

            if let Some(comp_type) = result_json.get("type").and_then(|v| v.as_str()) {
                debug!("Component type: {}", comp_type);
            }

            Ok(())
        } else {
            let actual_keys = result_json
                .get("actualKeys")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_else(|| "unknown".to_string());

            let expected_key =
                result_json.get("expectedKey").and_then(|v| v.as_str()).unwrap_or(component_id);

            error!(
                component_id = component_id,
                expected_key = expected_key,
                actual_keys = actual_keys,
                verification_result = ?result_json,
                "Component not found in globalThis after reload. Expected key '{}' not found. Available keys: [{}]. Last known good version will be preserved.",
                expected_key,
                actual_keys
            );
            Err(format!(
                "Component '{}' not found in globalThis after reload. Expected key '{}' but found keys: [{}]. Last known good version will be preserved.",
                component_id,
                expected_key,
                actual_keys
            )
            .into())
        }
    } else {
        error!(
            component_id = component_id,
            verification_result = ?result_json,
            "Invalid verification result format. Last known good version will be preserved."
        );
        Err("Invalid verification result format. Last known good version will be preserved.".into())
    }
}

fn extract_component_id(
    file_path: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path = std::path::Path::new(file_path);

    let relative_path = if path.is_absolute() {
        let components: Vec<_> = path.components().collect();
        if let Some(src_idx) = components.iter().position(|c| c.as_os_str() == "src") {
            let after_src: std::path::PathBuf = components[src_idx + 1..].iter().collect();
            after_src
        } else {
            return Err(format!("Path does not contain 'src' directory: {}", file_path).into());
        }
    } else {
        let src_dir = std::path::Path::new("src");
        if let Ok(rel) = path.strip_prefix(src_dir) {
            rel.to_path_buf()
        } else {
            path.to_path_buf()
        }
    };

    let component_id = relative_path
        .to_str()
        .ok_or("Invalid path encoding")?
        .trim_end_matches(".tsx")
        .trim_end_matches(".ts")
        .trim_end_matches(".jsx")
        .trim_end_matches(".js")
        .replace('\\', "/");

    Ok(component_id)
}

fn get_dist_path_for_component(
    file_path: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let component_id = extract_component_id(file_path)?;

    let dist_path =
        std::path::Path::new("dist").join("server").join(format!("{}.js", component_id));

    Ok(dist_path)
}

#[axum::debug_handler]
async fn hmr_register_component(
    State(state): State<ServerState>,
    Json(request): Json<HmrRegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    let file_path = request.file_path.clone();

    let component_id = match extract_component_id(&file_path) {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to extract component ID from {}: {}", file_path, e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    info!("HMR register request for component: {} from file: {}", component_id, file_path);
    debug!("Extracted component ID: {} from path: {}", component_id, file_path);

    let path = std::path::Path::new(&file_path);

    {
        let renderer = state.renderer.lock().await;
        let mut registry = renderer.component_registry.lock();
        registry.mark_module_stale(&component_id);
        debug!("Marked component {} as stale", component_id);
    }

    let reload_result = reload_component_from_dist(&state, &file_path, &component_id).await;

    let mut reload_error_details: Option<serde_json::Value> = None;

    match &reload_result {
        Ok(_) => {
            info!(
                component_id = component_id,
                file_path = file_path,
                "Successfully reloaded component from dist"
            );
        }
        Err(e) => {
            error!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to reload component from dist, preserving last known good version"
            );

            #[allow(clippy::disallowed_methods)]
            {
                reload_error_details = Some(serde_json::json!({
                    "stage": "dist_reload",
                    "message": e.to_string(),
                    "component_id": component_id,
                    "file_path": file_path,
                    "preserved_last_good": true
                }));
            }
        }
    }

    if reload_result.is_err() {
        debug!(
            component_id = component_id,
            "Attempting fallback re-registration after dist reload failure"
        );

        if let Err(e) = immediate_component_reregistration(&state, &file_path).await {
            error!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to immediately re-register component, preserving last known good version"
            );

            #[allow(clippy::disallowed_methods)]
            return Ok(Json(serde_json::json!({
                "success": false,
                "file_path": request.file_path,
                "component_id": component_id,
                "reloaded": false,
                "preserved_last_good": true,
                "error": {
                    "stage": "fallback_registration",
                    "message": e.to_string(),
                    "previous_error": reload_error_details,
                    "suggestion": "Component reload failed. Last known good version is still available. Consider checking for syntax errors or manual page refresh."
                }
            })));
        } else {
            info!(component_id = component_id, "Fallback re-registration succeeded");
        }
    }

    let mut reloaded = reload_result.is_ok();
    let mut module_reload_error: Option<String> = None;

    if state.config.hmr_reload_enabled() {
        debug!("HMR reload is enabled, triggering debounced module reload for {}", component_id);

        match state.module_reload_manager.reload_module_debounced(&component_id, path).await {
            Ok(()) => {
                debug!("Scheduled debounced reload for component: {}", component_id);
                reloaded = true;
            }
            Err(e) => {
                error!(
                    component_id = component_id,
                    error = %e,
                    "Failed to schedule module reload, preserving last known good version"
                );
                module_reload_error = Some(e.to_string());
            }
        }
    } else {
        debug!("HMR reload is disabled, skipping module reload");
    }

    #[allow(clippy::disallowed_methods)]
    let response = if reloaded {
        serde_json::json!({
            "success": true,
            "file_path": request.file_path,
            "component_id": component_id,
            "reloaded": true,
            "error": null
        })
    } else if reload_error_details.is_some() || module_reload_error.is_some() {
        serde_json::json!({
            "success": true,
            "file_path": request.file_path,
            "component_id": component_id,
            "reloaded": false,
            "preserved_last_good": true,
            "error": {
                "dist_reload": reload_error_details,
                "module_reload": module_reload_error,
                "suggestion": "Component reload encountered errors. Last known good version is still available. Check console for details or try a manual page refresh."
            }
        })
    } else {
        serde_json::json!({
            "success": true,
            "file_path": request.file_path,
            "component_id": component_id,
            "reloaded": false,
            "error": null
        })
    };

    Ok(Json(response))
}

async fn immediate_component_reregistration(
    state: &ServerState,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = std::path::Path::new(file_path);
    let component_name =
        path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("UnknownComponent");

    debug!(
        component_name = component_name,
        file_path = file_path,
        "Starting immediate component re-registration"
    );

    {
        let mut renderer = state.renderer.lock().await;
        renderer.clear_script_cache();

        if let Err(e) = renderer.clear_component_module_cache(component_name).await {
            warn!(
                component_name = component_name,
                error = %e,
                "Failed to clear component module cache, continuing anyway"
            );
        }
    }

    let content = match tokio::fs::read_to_string(file_path).await {
        Ok(c) => c,
        Err(e) => {
            error!(
                component_name = component_name,
                file_path = file_path,
                error = %e,
                error_kind = ?e.kind(),
                "Failed to read source file for immediate re-registration"
            );
            return Err(format!("Failed to read source file: {}", e).into());
        }
    };

    {
        let mut renderer = state.renderer.lock().await;

        if let Err(e) = renderer.register_component(component_name, &content).await {
            error!(
                component_name = component_name,
                error = %e,
                "Failed to register component directly, preserving last known good version"
            );
            Err(format!("Failed to register component: {}", e).into())
        } else {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Err(e) = renderer.clear_component_module_cache(component_name).await {
                warn!(
                    component_name = component_name,
                    error = %e,
                    "Failed to clear component module cache after initial registration"
                );
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            if let Err(e) = renderer.register_component(component_name, &content).await {
                error!(
                    component_name = component_name,
                    error = %e,
                    "Failed to re-register component after cache clear, preserving last known good version"
                );
                return Err(
                    format!("Failed to re-register component after cache clear: {}", e).into()
                );
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

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
                                component_name = component_name,
                                attempts = verification_attempts,
                                "Component verification failed after all attempts, but component may still be available"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            component_name = component_name,
                            attempt = attempt,
                            error = %e,
                            "Component verification script execution failed"
                        );
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            }

            warn!(
                component_name = component_name,
                attempts = verification_attempts,
                "Component verification failed after all attempts, but component may still be available"
            );
            Ok(())
        }
    }
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

#[derive(Debug, Deserialize)]
struct HmrInvalidateRequest {
    #[serde(rename = "componentId")]
    component_id: String,
    #[serde(rename = "filePath")]
    #[allow(dead_code)]
    file_path: Option<String>,
}

#[axum::debug_handler]
async fn hmr_invalidate_component(
    State(state): State<ServerState>,
    Json(payload): Json<HmrInvalidateRequest>,
) -> Json<Value> {
    info!("HMR invalidate request for component: {}", payload.component_id);

    let result = {
        let renderer = state.renderer.lock().await;

        {
            let mut registry = renderer.component_registry.lock();
            registry.mark_module_stale(&payload.component_id);
            debug!("Marked component {} as stale during invalidation", payload.component_id);
        }

        renderer.clear_component_cache(&payload.component_id);
        debug!("Cleared component cache for {}", payload.component_id);

        {
            let mut registry = renderer.component_registry.lock();
            registry.remove_component(&payload.component_id);
            debug!("Removed component {} from registry", payload.component_id);
        }

        if let Err(e) = renderer.runtime.clear_module_loader_caches(&payload.component_id).await {
            warn!("Failed to clear module loader caches for {}: {}", payload.component_id, e);
        }

        let clear_script = format!(
            r#"
            (function() {{
                let clearedCount = 0;
                const componentId = "{}";

                if (typeof globalThis[componentId] !== 'undefined') {{
                    delete globalThis[componentId];
                    clearedCount++;
                }}

                if (globalThis.__rsc_modules && globalThis.__rsc_modules[componentId]) {{
                    delete globalThis.__rsc_modules[componentId];
                    clearedCount++;
                }}

                if (globalThis.__rsc_functions && globalThis.__rsc_functions[componentId]) {{
                    delete globalThis.__rsc_functions[componentId];
                    clearedCount++;
                }}

                if (globalThis.__rsc_component_functions && globalThis.__rsc_component_functions.has(componentId)) {{
                    globalThis.__rsc_component_functions.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis.__rsc_component_server_functions && globalThis.__rsc_component_server_functions.has(componentId)) {{
                    globalThis.__rsc_component_server_functions.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis.__rsc_component_data && globalThis.__rsc_component_data.has(componentId)) {{
                    globalThis.__rsc_component_data.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis.__rsc_component_namespaces && globalThis.__rsc_component_namespaces.has(componentId)) {{
                    globalThis.__rsc_component_namespaces.delete(componentId);
                    clearedCount++;
                }}

                return {{
                    success: true,
                    clearedCount: clearedCount,
                    componentId: componentId
                }};
            }})()
            "#,
            payload.component_id
        );

        renderer
            .runtime
            .execute_script(
                format!("hmr_clear_cache_{}.js", payload.component_id.replace('/', "_")),
                clear_script,
            )
            .await
    };

    match result {
        Ok(clear_result) => {
            info!("Successfully invalidated component cache for: {}", payload.component_id);
            #[allow(clippy::disallowed_methods)]
            Json(serde_json::json!({
                "success": true,
                "componentId": payload.component_id,
                "cleared": clear_result
            }))
        }
        Err(e) => {
            error!("Failed to invalidate component cache for {}: {}", payload.component_id, e);
            #[allow(clippy::disallowed_methods)]
            Json(serde_json::json!({
                "success": false,
                "componentId": payload.component_id,
                "error": e.to_string()
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
struct HmrReloadRequest {
    #[serde(rename = "componentId")]
    component_id: String,
    #[serde(rename = "filePath")]
    file_path: String,
}

#[axum::debug_handler]
async fn hmr_reload_component(
    State(state): State<ServerState>,
    Json(payload): Json<HmrReloadRequest>,
) -> Json<Value> {
    debug!(
        "HMR reload request for component: {} from file: {}",
        payload.component_id, payload.file_path
    );

    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for HMR reload");
            #[allow(clippy::disallowed_methods)]
            return Json(serde_json::json!({
                "success": false,
                "componentId": payload.component_id,
                "error": "Configuration not available"
            }));
        }
    };

    let client = reqwest::Client::new();
    let vite_base_url = format!("http://{}", config.vite_address());

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let file_path = if payload.file_path.starts_with('/') {
        payload.file_path.clone()
    } else {
        format!("/{}", payload.file_path)
    };

    let vite_url = format!("{}{}?t={}", vite_base_url, file_path, timestamp);

    debug!("Fetching transpiled code from Vite: {}", vite_url);

    let transpiled_code = match client.get(&vite_url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                error!("Vite returned error status: {}", response.status());
                #[allow(clippy::disallowed_methods)]
                return Json(serde_json::json!({
                    "success": false,
                    "componentId": payload.component_id,
                    "error": format!("Vite returned status: {}", response.status())
                }));
            }

            match response.text().await {
                Ok(code) => code,
                Err(e) => {
                    error!("Failed to read response from Vite: {}", e);
                    #[allow(clippy::disallowed_methods)]
                    return Json(serde_json::json!({
                        "success": false,
                        "componentId": payload.component_id,
                        "error": format!("Failed to read response: {}", e)
                    }));
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch from Vite dev server: {}", e);
            #[allow(clippy::disallowed_methods)]
            return Json(serde_json::json!({
                "success": false,
                "componentId": payload.component_id,
                "error": format!("Failed to fetch from Vite: {}", e)
            }));
        }
    };

    debug!("Fetched {} bytes of transpiled code", transpiled_code.len());

    let result = {
        let mut renderer = state.renderer.lock().await;
        renderer.register_component(&payload.component_id, &transpiled_code).await
    };

    match result {
        Ok(()) => {
            info!("Successfully reloaded component: {}", payload.component_id);
            #[allow(clippy::disallowed_methods)]
            Json(serde_json::json!({
                "success": true,
                "componentId": payload.component_id,
                "codeSize": transpiled_code.len()
            }))
        }
        Err(e) => {
            error!("Failed to reload component {}: {}", payload.component_id, e);
            #[allow(clippy::disallowed_methods)]
            Json(serde_json::json!({
                "success": false,
                "componentId": payload.component_id,
                "error": e.to_string()
            }))
        }
    }
}

#[axum::debug_handler]
async fn reload_component(
    State(state): State<ServerState>,
    Json(payload): Json<ReloadComponentRequest>,
) -> Result<Json<ReloadComponentResponse>, StatusCode> {
    info!(
        "Reload component request for: {} from bundle: {}",
        payload.component_id, payload.bundle_path
    );

    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let bundle_full_path = project_root.join(&payload.bundle_path);

    let invalidate_result = {
        let renderer = state.renderer.lock().await;
        renderer.runtime.invalidate_component(&payload.component_id).await
    };

    if let Err(e) = invalidate_result {
        warn!("Failed to invalidate component (non-fatal): {}", e);
    }

    let load_result = {
        let renderer = state.renderer.lock().await;
        renderer.runtime.load_component(&payload.component_id, &bundle_full_path).await
    };

    match load_result {
        Ok(()) => {
            info!("Component reloaded successfully: {}", payload.component_id);
            Ok(Json(ReloadComponentResponse {
                success: true,
                message: format!("Component {} reloaded successfully", payload.component_id),
            }))
        }
        Err(e) => {
            error!("Failed to reload component {}: {}", payload.component_id, e);
            Ok(Json(ReloadComponentResponse {
                success: false,
                message: format!("Failed to reload component: {}", e),
            }))
        }
    }
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

async fn serve_static_asset(
    State(state): State<ServerState>,
    Path(asset_path): Path<String>,
) -> Result<Response, StatusCode> {
    let file_path = state.config.public_dir().join("assets").join(&asset_path);

    if !file_path.exists() || !file_path.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }

    match std::fs::read(&file_path) {
        Ok(content) => {
            let content_type = get_content_type(&asset_path);
            let cache_control = &state.config.caching.static_files;

            Ok(Response::builder()
                .header("content-type", content_type)
                .header("cache-control", cache_control)
                .body(Body::from(content))
                .expect("Valid static asset response"))
        }
        Err(e) => {
            error!("Failed to read static asset {}: {}", file_path.display(), e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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

async fn render_fallback_html(state: &ServerState, path: &str) -> Result<Response, StatusCode> {
    debug!("Rendering fallback HTML shell for path: {}", path);

    let index_path = state.config.public_dir().join("index.html");
    if index_path.exists() && state.config.is_production() {
        match std::fs::read_to_string(&index_path) {
            Ok(html_content) => {
                debug!("Serving built index.html as fallback");
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/html; charset=utf-8")
                    .body(Body::from(html_content))
                    .expect("Valid HTML response"));
            }
            Err(e) => {
                error!("Failed to read built index.html: {}", e);
            }
        }
    }

    if state.config.is_development() {
        let vite_port = state.config.vite.port;
        let html_shell = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Rari App Router</title>
</head>
<body>
  <div id="root"></div>
  <script type="module">
    import 'http://localhost:{}/@id/virtual:rari-entry-client';
  </script>
</body>
</html>"#,
            vite_port
        );

        debug!("Serving development HTML shell as fallback");
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(html_shell))
            .expect("Valid HTML response"));
    }

    let error_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Build Required</title>
</head>
<body>
  <div style="padding: 40px; font-family: sans-serif;">
    <h1>Build Required</h1>
    <p>Please build your application first:</p>
    <pre>npm run build</pre>
    <p>Or run in development mode with Vite:</p>
    <pre>npm run dev</pre>
  </div>
</body>
</html>"#;

    warn!("No built files found, serving error page");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(error_html))
        .expect("Valid HTML response"))
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

    let accept_header = headers.get("accept").and_then(|v| v.to_str().ok()).unwrap_or("");
    let wants_rsc = accept_header.contains("text/x-component");

    debug!("Accept header: '{}', wants_rsc: {}", accept_header, wants_rsc);

    if wants_rsc {
        match layout_renderer.render_route(&route_match, &context).await {
            Ok(rsc_wire_format) => {
                debug!("Successfully rendered RSC wire format ({} bytes)", rsc_wire_format.len());
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/x-component")
                    .body(Body::from(rsc_wire_format))
                    .expect("Valid RSC response"))
            }
            Err(e) => {
                error!("Failed to render RSC: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        let total_start = std::time::Instant::now();

        let rsc_start = std::time::Instant::now();
        let rsc_wire_format = match layout_renderer.render_route(&route_match, &context).await {
            Ok(rsc) => rsc,
            Err(e) => {
                error!("RSC rendering failed: {}, falling back to shell", e);
                return render_fallback_html(&state, path).await;
            }
        };
        let rsc_duration = rsc_start.elapsed();
        debug!("⚡ RSC render took: {:?}", rsc_duration);

        let ssr_start = std::time::Instant::now();
        let (html, ssr_timing) = match state
            .ssr_renderer
            .render_to_html_with_timing(&rsc_wire_format, &state.config)
            .await
        {
            Ok((html, timing)) => (html, timing),
            Err(e) => {
                error!("SSR rendering failed: {}, falling back to shell", e);
                return render_fallback_html(&state, path).await;
            }
        };
        let ssr_duration = ssr_start.elapsed();
        debug!(
            "⚡ SSR render took: {:?} (parse: {:.2}ms, serialize: {:.2}ms, v8: {:.2}ms, template: {:.2}ms, inject: {:.2}ms)",
            ssr_duration,
            ssr_timing.parse_rsc_ms,
            ssr_timing.serialize_to_v8_ms,
            ssr_timing.v8_execution_ms,
            ssr_timing.load_template_ms,
            ssr_timing.inject_template_ms
        );

        let total_duration = total_start.elapsed();
        debug!(
            "⚡⚡⚡ Total render took: {:?} (RSC: {:?}, SSR: {:?})",
            total_duration, rsc_duration, ssr_duration
        );

        let html_with_assets = match inject_assets_into_html(&html, &state.config).await {
            Ok(html) => html,
            Err(_) => html,
        };

        let mut response_builder = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8");

        let page_configs = state.page_cache_configs.read().await;
        if let Some(page_cache_config) = Server::find_matching_cache_config(&page_configs, path) {
            for (key, value) in page_cache_config {
                response_builder = response_builder.header(key.to_lowercase(), value);
            }
        }

        Ok(response_builder.body(Body::from(html_with_assets)).expect("Valid HTML response"))
    }
}

async fn inject_assets_into_html(html: &str, config: &Config) -> Result<String, StatusCode> {
    let template_path = if config.is_development() { "index.html" } else { "dist/index.html" };

    let template = match tokio::fs::read_to_string(template_path).await {
        Ok(t) => t,
        Err(_) => {
            if html.trim_start().starts_with("<html") {
                return Ok(format!("<!DOCTYPE html>\n{}", html));
            }
            return Ok(html.to_string());
        }
    };

    let mut tags = Vec::new();

    for line in template.lines() {
        if line.contains("<link") && line.contains("stylesheet") {
            tags.push(line.trim().to_string());
        }
        if line.contains("<script") {
            tags.push(line.trim().to_string());
        }
    }

    let assets = tags.join("\n");

    let mut final_html = html.to_string();
    if let Some(body_end) = final_html.rfind("</body>") {
        final_html.insert_str(body_end, &format!("\n{}\n", assets));
    }

    if !final_html.trim_start().starts_with("<!DOCTYPE") {
        final_html = format!("<!DOCTYPE html>\n{}", final_html);
    }

    Ok(final_html)
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
