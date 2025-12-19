use crate::error::RariError;
use crate::rsc::rendering::core::ResourceLimits;
use crate::runtime::utils::DistPathResolver;
use crate::server::actions::{handle_form_action, handle_server_action};
use crate::server::cache::response_cache;
use crate::server::config::Config;
use crate::server::handlers::api_handler::{api_cors_preflight, handle_api_route};
use crate::server::handlers::app_handler::handle_app_route;
use crate::server::handlers::csrf_handler::get_csrf_token;
use crate::server::handlers::hmr_handlers::{
    hmr_invalidate_api_route, hmr_invalidate_component, hmr_register_component,
    hmr_reload_component, reload_component,
};
use crate::server::handlers::rsc_handlers::{
    health_check, list_components, register_client_component, register_component,
    rsc_render_handler, rsc_status_handler, server_status, stream_component,
};
use crate::server::handlers::static_handlers::{
    cors_preflight_ok, root_handler, serve_static_asset, static_or_spa_handler,
};
use crate::server::loaders::cache_loader::CacheLoader;
use crate::server::loaders::component_loader::ComponentLoader;
use crate::server::middleware::rate_limit::{create_rate_limit_layer, rate_limit_logger};
use crate::server::middleware::request_middleware::{
    cors_middleware, request_logger, security_headers_middleware,
};
use crate::server::routing::{api_routes, app_router};
use crate::server::types::ServerState;
use crate::server::vite::proxy::{
    check_vite_server_health, vite_reverse_proxy, vite_src_proxy, vite_websocket_proxy,
};
use axum::extract::DefaultBodyLimit;
use axum::{
    Router, middleware,
    routing::{any, get, post},
};
use colored::Colorize;
use rustc_hash::FxHashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tracing::{info, warn};

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

        let resource_limits = ResourceLimits {
            max_script_execution_time_ms: config.rsc.script_execution_timeout_ms,
            ..ResourceLimits::default()
        };

        info!("Initializing RSC renderer (mode: {})", config.mode);

        let env_vars: rustc_hash::FxHashMap<String, String> = std::env::vars().collect();
        let js_runtime = Arc::new(crate::runtime::JsExecutionRuntime::new(Some(env_vars)));
        let mut renderer =
            crate::rsc::RscRenderer::with_resource_limits(js_runtime, resource_limits);
        renderer.initialize().await?;

        if config.is_production() {
            ComponentLoader::load_production_components(&mut renderer).await?;
        } else {
            ComponentLoader::load_app_router_components(&mut renderer).await?;
            ComponentLoader::load_server_actions_from_source(&mut renderer).await?;
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
                Err(_) => None,
            }
        };

        let api_route_handler = {
            let manifest_path = "dist/app-routes.json";

            match api_routes::ApiRouteHandler::from_file(renderer.runtime.clone(), manifest_path)
                .await
            {
                Ok(handler) => {
                    info!(
                        "Loaded API route handler from {} with {} API routes",
                        manifest_path,
                        handler.manifest().api_routes.len()
                    );
                    Some(Arc::new(handler))
                }
                Err(_) => None,
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

        let ssr_renderer = {
            let runtime = renderer.runtime.clone();
            let ssr = crate::rsc::RscHtmlRenderer::new(runtime);
            ssr.initialize().await?;
            Arc::new(ssr)
        };

        let renderer_arc = Arc::new(tokio::sync::Mutex::new(renderer));

        let cache_config = response_cache::CacheConfig::from_env(config.is_production());
        let response_cache = Arc::new(response_cache::ResponseCache::new(cache_config));

        info!(
            "Response cache initialized: enabled={}, max_entries={}, default_ttl={}s",
            response_cache.config.enabled,
            response_cache.config.max_entries,
            response_cache.config.default_ttl
        );

        let csrf_manager = Arc::new(Self::initialize_csrf_manager());

        let state = ServerState {
            renderer: renderer_arc,
            ssr_renderer,
            config: Arc::new(config.clone()),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            start_time: std::time::Instant::now(),
            component_cache_configs: Arc::new(tokio::sync::RwLock::new(FxHashMap::default())),
            page_cache_configs: Arc::new(tokio::sync::RwLock::new(FxHashMap::default())),
            app_router,
            api_route_handler,
            module_reload_manager,
            html_cache: Arc::new(dashmap::DashMap::new()),
            response_cache,
            csrf_manager,
        };

        if config.is_production() {
            CacheLoader::load_page_cache_configs(&state).await?;
            CacheLoader::load_vite_cache_config(&state).await?;
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

    fn initialize_csrf_manager() -> crate::server::security::csrf::CsrfTokenManager {
        use crate::server::security::csrf::CsrfTokenManager;

        if let Ok(secret) = std::env::var("RARI_CSRF_SECRET") {
            if secret.len() < 32 {
                warn!(
                    "RARI_CSRF_SECRET is less than 32 bytes. Using it anyway, but consider using a stronger secret."
                );
            }
            info!("CSRF protection enabled with secret from RARI_CSRF_SECRET");
            CsrfTokenManager::new(secret.into_bytes())
        } else {
            CsrfTokenManager::new_with_random_secret()
        }
    }

    async fn build_router(config: &Config, state: ServerState) -> Result<Router<()>, RariError> {
        let small_body_limit = DefaultBodyLimit::max(100 * 1024);
        let medium_body_limit = DefaultBodyLimit::max(1024 * 1024);
        let large_body_limit = DefaultBodyLimit::max(50 * 1024 * 1024);

        let mut router = Router::new()
            .route("/api/rsc/stream", post(stream_component))
            .route("/api/rsc/stream", axum::routing::options(cors_preflight_ok))
            .layer(medium_body_limit)
            .route("/api/rsc/register", post(register_component))
            .route("/api/rsc/register-client", post(register_client_component))
            .route("/api/rsc/hmr-register", post(hmr_register_component))
            .route("/api/rsc/hmr-register", axum::routing::options(cors_preflight_ok))
            .layer(large_body_limit)
            .route("/api/rsc/components", get(list_components))
            .route("/api/rsc/health", get(health_check))
            .route("/api/rsc/status", get(server_status))
            .route("/_rsc_status", get(rsc_status_handler))
            .route("/rsc/render/{component_id}", get(rsc_render_handler))
            .route("/api/rsc/csrf-token", get(get_csrf_token))
            .layer(small_body_limit)
            .route("/api/rsc/action", post(handle_server_action))
            .route("/api/rsc/form-action", post(handle_form_action))
            .layer(medium_body_limit);

        if config.is_development() {
            info!("Adding development routes");

            let small_body_limit = DefaultBodyLimit::max(100 * 1024);
            let large_body_limit = DefaultBodyLimit::max(50 * 1024 * 1024);

            router = router
                .route("/api/rsc/hmr-invalidate", post(hmr_invalidate_component))
                .route("/api/rsc/hmr-invalidate", axum::routing::options(cors_preflight_ok))
                .route("/api/rsc/hmr-reload", post(hmr_reload_component))
                .route("/api/rsc/hmr-reload", axum::routing::options(cors_preflight_ok))
                .route("/api/rsc/hmr-invalidate-api-route", post(hmr_invalidate_api_route))
                .route(
                    "/api/rsc/hmr-invalidate-api-route",
                    axum::routing::options(cors_preflight_ok),
                )
                .layer(small_body_limit)
                .route("/api/rsc/reload-component", post(reload_component))
                .route("/api/rsc/reload-component", axum::routing::options(cors_preflight_ok))
                .layer(large_body_limit)
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
            info!("Registering API route handler");
            let medium_body_limit = DefaultBodyLimit::max(1024 * 1024);
            router = router
                .route("/api/{*path}", axum::routing::options(api_cors_preflight))
                .route("/api/{*path}", any(handle_api_route))
                .layer(medium_body_limit);
        }

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

        if let Some(rate_limit_layer) = create_rate_limit_layer(config) {
            info!(
                "Rate limiting enabled: {} req/sec per IP, burst size: {}",
                config.rate_limit.requests_per_second, config.rate_limit.burst_size
            );
            router = router.layer(rate_limit_layer).layer(middleware::from_fn(rate_limit_logger));
        } else {
            info!("Rate limiting disabled");
        }

        let middleware_stack =
            ServiceBuilder::new().layer(middleware::from_fn(request_logger)).into_inner();

        router = router.layer(middleware_stack);

        Ok(router.with_state(state))
    }

    pub async fn start(self) -> Result<(), RariError> {
        self.display_startup_message();

        info!("Starting Rari server on {}", self.address);

        axum::serve(self.listener, self.router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .map_err(|e| RariError::network(format!("Server error: {e}")))?;

        info!("Server shutdown complete");
        Ok(())
    }

    fn display_startup_message(&self) {
        let server_url = format!("http://{}", self.address);

        if self.config.is_production() {
            println!("  {} {}", "Mode:".bold(), "Production".green());
            println!("  {} {}", "Server:".bold(), server_url.cyan().underline());

            if let Some(origin) = &self.config.server.origin {
                println!("  {} {}", "Origin:".bold(), origin.cyan());
            }
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }
}
