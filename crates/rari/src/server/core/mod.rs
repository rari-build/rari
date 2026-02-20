use crate::error::RariError;
use crate::rsc::rendering::core::ResourceLimits;
use crate::runtime::utils::DistPathResolver;
use crate::server::actions::{handle_form_action, handle_server_action};
use crate::server::cache::response_cache;
use crate::server::config::Config;
use crate::server::handlers::api_handler::{api_cors_preflight, handle_api_route};
use crate::server::handlers::app_handler::handle_app_route;
use crate::server::handlers::csrf_handler::get_csrf_token;
use crate::server::handlers::hmr_handlers::handle_hmr_action;
use crate::server::handlers::revalidate_handlers::revalidate_by_path;
use crate::server::handlers::route_info_handler::get_route_info;
use crate::server::handlers::rsc_handlers::{
    health_check, register_client_component, register_component, stream_component,
};
use crate::server::handlers::static_handlers::{
    cors_preflight_ok, root_handler, serve_static_asset, static_or_spa_handler,
};
use crate::server::loaders::cache_loader::CacheLoader;
use crate::server::loaders::component_loader::ComponentLoader;
use crate::server::middleware::proxy_middleware::ProxyLayer;
use crate::server::middleware::rate_limit::{
    create_rate_limit_layer, create_strict_rate_limit_layer, rate_limit_logger,
};
use crate::server::middleware::request_middleware::{cors_middleware, security_headers_middleware};
use crate::server::middleware::spam_blocker::{SpamBlocker, spam_blocker_middleware};
use crate::server::routing::{api_routes, app_router};
use crate::server::types::ServerState;
use crate::server::vite::proxy::{
    check_vite_server_health, vite_reverse_proxy, vite_src_proxy, vite_websocket_proxy,
};
use axum::extract::DefaultBodyLimit;
use axum::{
    Router,
    middleware::{self},
    routing::{any, get, post},
};
use colored::Colorize;
use rustc_hash::FxHashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tracing::{debug, error};

#[derive(Clone, Copy, Debug)]
struct NotStreamingResponse;

impl tower_http::compression::Predicate for NotStreamingResponse {
    fn should_compress<B>(&self, response: &http::Response<B>) -> bool
    where
        B: axum::body::HttpBody,
    {
        if response.headers().get("content-encoding").is_some() {
            return false;
        }

        if let Some(transfer_encoding) = response.headers().get("transfer-encoding")
            && transfer_encoding == "chunked"
        {
            return false;
        }

        true
    }
}

pub struct Server {
    router: Router,
    config: Config,
    listener: TcpListener,
    address: SocketAddr,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self, RariError> {
        Config::set_global(config.clone())
            .map_err(|_| RariError::configuration("Failed to set global config".to_string()))?;

        let resource_limits = ResourceLimits {
            max_script_execution_time_ms: config.rsc.script_execution_timeout_ms,
            ..ResourceLimits::default()
        };

        let env_vars: rustc_hash::FxHashMap<String, String> = std::env::vars().collect();
        let js_runtime = Arc::new(crate::runtime::JsExecutionRuntime::new(Some(env_vars)));
        let mut renderer =
            crate::rsc::RscRenderer::with_resource_limits(js_runtime.clone(), resource_limits);
        renderer.initialize().await?;

        if config.is_production() {
            ComponentLoader::load_production_components(&mut renderer).await?;
        } else {
            ComponentLoader::load_app_router_components(&mut renderer).await?;
            ComponentLoader::load_server_actions_from_source(&mut renderer).await?;
        }

        let app_router = {
            let manifest_path = "dist/server/routes.json";

            match app_router::AppRouter::from_file(manifest_path).await {
                Ok(router) => Some(Arc::new(router)),
                Err(e) => {
                    tracing::error!(
                        "Failed to load app router from {}: {}. All routes will return 404.",
                        manifest_path,
                        e
                    );
                    None
                }
            }
        };

        let api_route_handler = {
            let manifest_path = "dist/server/routes.json";

            match api_routes::ApiRouteHandler::from_file(renderer.runtime.clone(), manifest_path)
                .await
            {
                Ok(handler) => Some(Arc::new(handler)),
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
        let dist_path_resolver = Arc::new(DistPathResolver::new(project_root.clone()));
        module_reload_manager.set_dist_path_resolver(dist_path_resolver);

        let module_reload_manager = Arc::new(module_reload_manager);

        let ssr_renderer = {
            let runtime = renderer.runtime.clone();
            let ssr = crate::rsc::RscHtmlRenderer::new(runtime);
            Arc::new(ssr)
        };

        let renderer_arc = Arc::new(tokio::sync::Mutex::new(renderer));

        let cache_config = response_cache::CacheConfig::from_env(config.is_production());
        let response_cache = Arc::new(response_cache::ResponseCache::new(cache_config));

        let csrf_manager = Self::initialize_csrf_manager().map(Arc::new);

        let og_generator = {
            let runtime = js_runtime.clone();
            let generator = Arc::new(crate::server::og::OgImageGenerator::with_capacity(
                runtime,
                project_root.clone(),
                100,
            ));

            let manifest_path = "dist/server/routes.json";
            if let Err(e) = generator.load_manifest(manifest_path).await {
                tracing::error!("Failed to load OG image manifest: {}", e);
            }

            Some(generator)
        };

        let endpoint_rate_limiters =
            crate::server::security::ip_rate_limiter::EndpointRateLimiters::for_environment(
                config.is_production(),
            );
        endpoint_rate_limiters.start_cleanup_tasks();

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
            og_generator,
            project_root,
            endpoint_rate_limiters,
            image_optimizer: None,
        };

        if config.is_production() {
            CacheLoader::load_page_cache_configs(&state).await?;
        }

        let mut config = config;
        let config_path = "dist/server/image.json";

        if let Ok(image_config_str) = std::fs::read_to_string(config_path)
            && let Ok(image_config) =
                serde_json::from_str::<crate::server::image::ImageConfig>(&image_config_str)
        {
            config.images = image_config;
        }

        if let Err(e) = crate::server::middleware::proxy_middleware::initialize_proxy(&state).await
        {
            error!("Failed to initialize proxy: {}", e);
        }

        let router = Self::build_router(&config, state.clone()).await?;

        let address = config.server_address();

        let listener = TcpListener::bind(&address)
            .await
            .map_err(|e| RariError::network(format!("Failed to bind to {address}: {e}")))?;

        let socket_addr = listener
            .local_addr()
            .map_err(|e| RariError::network(format!("Failed to get local address: {e}")))?;

        Ok(Self { router, config, listener, address: socket_addr })
    }

    fn initialize_csrf_manager() -> Option<crate::server::security::csrf::CsrfTokenManager> {
        use crate::server::security::csrf::CsrfTokenManager;

        if let Ok(secret) = std::env::var("RARI_CSRF_SECRET") {
            Some(CsrfTokenManager::new(secret.into_bytes()))
        } else {
            None
        }
    }

    async fn build_router(config: &Config, mut state: ServerState) -> Result<Router, RariError> {
        let small_body_limit = DefaultBodyLimit::max(100 * 1024);
        let medium_body_limit = DefaultBodyLimit::max(1024 * 1024);

        let image_optimizer = Arc::new(crate::server::image::ImageOptimizer::new(
            config.images.clone(),
            &state.project_root,
        ));

        state.image_optimizer = Some(Arc::clone(&image_optimizer));

        let image_state = crate::server::image::ImageState {
            optimizer: image_optimizer,
            rate_limiters: state.endpoint_rate_limiters.clone(),
        };

        let revalidation_router = Router::new()
            .route("/_rari/revalidate", post(revalidate_by_path))
            .layer(small_body_limit)
            .layer(create_strict_rate_limit_layer(Some(
                config.rate_limit.revalidate_requests_per_minute,
            )));

        let mut router = Router::new()
            .route("/_rari/stream", post(stream_component))
            .route("/_rari/stream", axum::routing::options(cors_preflight_ok))
            .layer(medium_body_limit)
            .route("/_rari/csrf-token", get(get_csrf_token))
            .route("/_rari/route-info", post(get_route_info))
            .layer(small_body_limit)
            .route("/_rari/action", post(handle_server_action))
            .route("/_rari/form-action", post(handle_form_action))
            .layer(medium_body_limit)
            .merge(revalidation_router);

        let image_router = Router::new()
            .route("/_rari/image", get(crate::server::image::handle_image_request))
            .with_state(image_state);

        router = router.merge(image_router);

        let og_router = Router::new()
            .route("/_rari/og/", get(crate::server::handlers::og_handler::og_image_handler_root))
            .route("/_rari/og/{*path}", get(crate::server::handlers::og_handler::og_image_handler))
            .with_state(state.clone());

        router = router.merge(og_router);

        if config.is_development() {
            let medium_body_limit = DefaultBodyLimit::max(1024 * 1024);
            let large_body_limit = DefaultBodyLimit::max(50 * 1024 * 1024);

            router = router
                .route("/_rari/health", get(health_check))
                .route("/_rari/register", post(register_component))
                .route("/_rari/register-client", post(register_client_component))
                .layer(large_body_limit)
                .route("/_rari/hmr", post(handle_hmr_action))
                .route("/_rari/hmr", axum::routing::options(cors_preflight_ok))
                .layer(medium_body_limit)
                .route("/vite-server/", get(vite_websocket_proxy))
                .route("/vite-server/{*path}", any(vite_reverse_proxy))
                .route("/src/{*path}", any(vite_src_proxy));

            if let Err(e) = check_vite_server_health().await {
                debug!("Vite server not yet available: {}", e);
            }
        }

        let has_app_router = state.app_router.is_some();

        if has_app_router {
            let medium_body_limit = DefaultBodyLimit::max(1024 * 1024);
            router = router
                .route("/api/{*path}", axum::routing::options(api_cors_preflight))
                .route("/api/{*path}", any(handle_api_route))
                .layer(medium_body_limit);
        }

        if has_app_router {
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

        let compression_layer = CompressionLayer::new().compress_when(NotStreamingResponse);
        router = router.layer(compression_layer);

        if config.spam_blocker.enabled {
            let spam_blocker = SpamBlocker::new();
            spam_blocker.clone().start_cleanup_task();
            router = router.layer(middleware::from_fn(spam_blocker_middleware));
            router = router.layer(axum::Extension(spam_blocker));
        }

        if config.is_development() {
            router = router.layer(middleware::from_fn(cors_middleware));
        } else {
            router = router.layer(middleware::from_fn(security_headers_middleware));
        }

        if let Some(rate_limit_layer) = create_rate_limit_layer(config) {
            router = router.layer(rate_limit_layer).layer(middleware::from_fn(rate_limit_logger));
        }

        let mut router = router.with_state(state.clone());

        if has_app_router {
            router = router.layer(ProxyLayer::new(state));
        }

        Ok(router)
    }

    pub async fn start(self) -> Result<(), RariError> {
        self.display_startup_message();

        axum::serve(self.listener, self.router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .map_err(|e| RariError::network(format!("Server error: {e}")))?;

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
