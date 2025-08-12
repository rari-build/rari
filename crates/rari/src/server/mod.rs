use crate::error::RariError;
use crate::rsc::renderer::{ResourceLimits, RscRenderer};
use crate::rsc::streaming::RscStreamingExt;

use crate::runtime::JsExecutionRuntime;
use crate::server::config::Config;
use crate::server::request_middleware::{
    cors_middleware, request_logger, security_headers_middleware,
};
use crate::server::vite_proxy::{
    check_vite_server_health, display_vite_proxy_info, vite_reverse_proxy, vite_websocket_proxy,
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
use futures::StreamExt;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tracing::{debug, error, info, warn};

pub mod config;
pub mod request_middleware;
pub mod vite_proxy;

const RSC_CONTENT_TYPE: &str = "text/x-component";
const NO_CACHE_CONTROL: &str = "no-cache";
const CHUNKED_ENCODING: &str = "chunked";
const SERVER_MANIFEST_PATH: &str = "dist/server-manifest.json";
const DIST_DIR: &str = "dist";

#[derive(Clone)]
pub struct ServerState {
    pub renderer: Arc<tokio::sync::Mutex<RscRenderer>>,
    pub config: Arc<Config>,
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
    pub start_time: std::time::Instant,
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

        if let Err(e) = dotenv::dotenv() {
            debug!("No .env file found or error loading .env: {}", e);
        }

        let env_vars: rustc_hash::FxHashMap<String, String> = std::env::vars().collect();

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
        let state = ServerState {
            renderer: Arc::new(tokio::sync::Mutex::new(renderer)),
            config: Arc::new(config.clone()),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            start_time: std::time::Instant::now(),
        };

        let router = Self::build_router(&config, state).await?;

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

    async fn build_router(config: &Config, state: ServerState) -> Result<Router, RariError> {
        let mut router = Router::new()
            .route("/api/test", get(test_handler))
            .route("/api/rsc/render", post(render_component))
            .route("/api/rsc/stream", post(stream_component))
            .route("/api/rsc/stream-v2", post(stream_component_v2))
            .route("/api/rsc/stream-v2", axum::routing::options(cors_preflight_ok))
            .route("/api/rsc/register", post(register_component))
            .route("/api/rsc/register-client", post(register_client_component))
            .route("/api/rsc/hmr-register", post(hmr_register_component))
            .route("/api/rsc/components", get(list_components))
            .route("/api/rsc/health", get(health_check))
            .route("/api/rsc/status", get(server_status))
            .route("/_rsc_status", get(rsc_status_handler))
            .route("/rsc/render/:component_id", get(rsc_render_handler))
            .route("/api/*path", axum::routing::options(cors_preflight_ok))
            .with_state(state);

        if config.is_development() {
            info!("Adding development routes");

            router = router
                .route("/vite-server/", get(vite_websocket_proxy))
                .route("/vite-server/*path", any(vite_reverse_proxy));

            router = router.layer(middleware::from_fn(cors_middleware));

            if let Err(e) = check_vite_server_health().await {
                warn!("Vite development server check failed: {}", e);
                warn!("Make sure to start your Vite dev server for HMR to work");
            }
        } else {
            router = router.layer(middleware::from_fn(security_headers_middleware));
        }

        if config.is_production() {
            router =
                router.route("/", get(root_handler)).route("/*path", get(static_or_spa_handler));
        } else {
            let static_service =
                ServeDir::new(config.public_dir()).append_index_html_on_directories(true);
            router = router.fallback_service(static_service);
        }

        let middleware_stack =
            ServiceBuilder::new().layer(middleware::from_fn(request_logger)).into_inner();

        router = router.layer(middleware_stack);
        router = router.layer(DefaultBodyLimit::max(1024 * 1024 * 100));

        Ok(router)
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
async fn render_component(
    State(state): State<ServerState>,
    Json(request): Json<RenderRequest>,
) -> Result<Json<RenderResponse>, StatusCode> {
    let start_time = std::time::Instant::now();

    state.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    debug!("Rendering component: {}", request.component_id);

    let props_str = request.props.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let result = {
        let mut renderer = state.renderer.lock().await;
        renderer.render_to_rsc_format(&request.component_id, props_str.as_deref()).await
    };

    match result {
        Ok(rsc_data) => {
            let render_time = start_time.elapsed().as_millis() as u64;

            Ok(Json(RenderResponse {
                success: true,
                data: Some(rsc_data),
                error: None,
                component_id: request.component_id,
                render_time_ms: render_time,
            }))
        }
        Err(e) => {
            let _render_time = start_time.elapsed().as_millis() as u64;

            error!("Failed to render component {}: {}", request.component_id, e);

            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[axum::debug_handler]
async fn stream_component(
    State(state): State<ServerState>,
    Json(request): Json<RenderRequest>,
) -> Result<Response, StatusCode> {
    debug!("Streaming component: {}", request.component_id);

    let props_str = request.props.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let stream_result = {
        let mut renderer = state.renderer.lock().await;
        renderer.render_with_readable_stream(&request.component_id, props_str.as_deref())
    };

    match stream_result {
        Ok(mut rsc_stream) => {
            debug!("Successfully created stream for component: {}", request.component_id);

            let byte_stream = async_stream::stream! {
                while let Some(chunk_result) = rsc_stream.next().await {
                    match chunk_result {
                        Ok(chunk_bytes) => yield Ok(chunk_bytes),
                        Err(e) => {
                            error!("Stream error for component {}: {}", request.component_id, e);
                            yield Err(std::io::Error::other(e.to_string()));
                            break;
                        }
                    }
                }
            };

            let body = Body::from_stream(byte_stream);

            Ok(Response::builder()
                .header("content-type", RSC_CONTENT_TYPE)
                .header("cache-control", NO_CACHE_CONTROL)
                .header("transfer-encoding", CHUNKED_ENCODING)
                .body(body)
                .expect("Valid streaming response"))
        }
        Err(e) => {
            error!("Failed to create stream for component {}: {}", request.component_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[axum::debug_handler]
async fn stream_component_v2(
    State(state): State<ServerState>,
    Json(request): Json<RenderRequest>,
) -> Result<Response, StatusCode> {
    debug!("Streaming component V2 (true streaming): {}", request.component_id);

    let props_str = request.props.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let stream_result = {
        let renderer = state.renderer.lock().await;
        renderer.render_with_true_streaming(&request.component_id, props_str.as_deref()).await
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

            Ok(Response::builder()
                .header("content-type", RSC_CONTENT_TYPE)
                .header("cache-control", NO_CACHE_CONTROL)
                .header("transfer-encoding", CHUNKED_ENCODING)
                .header("x-rsc-streaming-version", "2")
                .header("access-control-allow-origin", "*")
                .header("access-control-expose-headers", "x-rsc-streaming-version")
                .body(body)
                .expect("Valid streaming response"))
        }
        Err(e) => {
            error!("Failed to create true streaming for component {}: {}", request.component_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// removed debug_promises endpoint and promise_debug module

#[axum::debug_handler]
async fn register_component(
    State(state): State<ServerState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Registering component: {}", request.component_id);

    let result = {
        let mut renderer = state.renderer.lock().await;
        renderer.register_component(&request.component_id, &request.component_code).await
    };

    match result {
        Ok(_) => {
            info!("Successfully registered component: {}", request.component_id);
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
    info!("HMR component re-registration requested for file: {}", request.file_path);

    let file_path = request.file_path.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if let Err(e) = immediate_component_reregistration(&state_clone, &file_path).await {
            error!("Failed to immediately re-register component for {}: {}", file_path, e);
        }
    });

    info!("HMR component re-registration triggered for: {}", request.file_path);
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
    info!("Immediately re-registering component for file: {}", file_path);

    let path = std::path::Path::new(file_path);
    let component_name =
        path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("UnknownComponent");

    info!("Re-registering component '{}' from file: {}", component_name, file_path);

    {
        let renderer = state.renderer.lock().await;
        renderer.clear_script_cache();
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

    let client = reqwest::Client::new();
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
    info!("Vite transformation completed: {:?}", response_data);

    info!("Component '{}' transformation triggered successfully", component_name);
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

            Ok(Response::builder()
                .header("content-type", RSC_CONTENT_TYPE)
                .header("cache-control", NO_CACHE_CONTROL)
                .body(Body::from(rsc_data))
                .expect("Valid RSC response"))
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

async fn root_handler() -> Result<Response, StatusCode> {
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
                return Ok(Response::builder()
                    .header("content-type", "text/html")
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

async fn static_or_spa_handler(Path(path): Path<String>) -> Result<Response, StatusCode> {
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
                return Ok(Response::builder()
                    .header("content-type", content_type)
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
    let index_path = config.public_dir().join("index.html");
    if index_path.exists() {
        match std::fs::read_to_string(&index_path) {
            Ok(content) => {
                return Ok(Response::builder()
                    .header("content-type", "text/html")
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
        HeaderValue::from_static("Content-Type, Authorization, Accept, Origin, X-Requested-With"),
    );
    headers.insert("Access-Control-Max-Age", HeaderValue::from_static("86400"));
    builder.body(Body::empty()).expect("Valid preflight response")
}

#[axum::debug_handler]
async fn cors_preflight_ok() -> Response {
    cors_preflight_response()
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
