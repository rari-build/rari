pub mod types;
pub mod utils;

use std::{
    env,
    future::{self, Future},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};

use axum::{
    Router,
    body::HttpBody,
    extract::DefaultBodyLimit,
    middleware::{self},
    routing,
    serve::ListenerExt,
};
use colored::Colorize;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use tokio::{
    fs,
    net::TcpListener,
    sync::{Mutex, RwLock},
};
use tower_http::{
    compression::{CompressionLayer, Predicate},
    services::ServeDir,
};
use types::{FallbackHtmlCache, ServerState};

use crate::{
    RscHtmlRenderer, RscRenderer,
    rendering::{base::ResourceLimits, layout::LayoutRenderer},
    runtime::JsExecutionRuntime,
    server::{
        actions::{handle_page_server_action, handle_server_action},
        cache::{
            handler::CacheHandlerRegistry, loader::CacheLoader, response,
            revalidate::revalidate_by_path, warmup,
        },
        config::{
            CACHE_LAYER_IMAGE, CACHE_LAYER_LAYOUT, CACHE_LAYER_OG, CACHE_LAYER_RESPONSE, Config,
        },
        image::{ImageCache, ImageConfig, ImageOptimizer, ImageState, handle_image_request},
        loader::ComponentLoader,
        middleware::{
            proxy::{self, ProxyLayer},
            request::{cors_middleware, security_headers_middleware},
        },
        og::{OgImageCache, OgImageGenerator, og_image_handler, og_image_handler_root},
        routing::{
            RoutesManifest,
            api::{api_cors_preflight, handle_api_route},
            api_routes,
            app::handle_app_route,
            app_router,
            route_info::get_route_info,
        },
        static_assets::{
            cors_preflight_ok, root_handler, serve_static_asset, static_or_spa_handler,
        },
        vite::{
            check_vite_server_health,
            hmr::handle_hmr_action,
            rsc::{health_check, register_client_component, register_component},
            vite_reverse_proxy, vite_src_proxy, vite_websocket_proxy,
        },
    },
};

#[derive(Clone, Copy, Debug)]
struct NotStreamingResponse;

impl Predicate for NotStreamingResponse {
    fn should_compress<B>(&self, response: &http::Response<B>) -> bool
    where
        B: HttpBody,
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

const ROUTES_MANIFEST_PATH: &str = "dist/server/routes.json";

pub struct Server {
    router: Router,
    config: Config,
    listener: TcpListener,
    address: SocketAddr,
}

impl Server {
    #[expect(clippy::missing_errors_doc, clippy::too_many_lines)]
    pub async fn new(config: Config) -> Result<Self, RariError> {
        Config::set_global(config.clone())
            .map_err(|_| RariError::configuration("Failed to set global config".to_string()))?;

        let resource_limits = ResourceLimits {
            max_script_execution_time_ms: config.rsc.script_execution_timeout_ms,
            ..ResourceLimits::default()
        };

        let env_vars: rustc_hash::FxHashMap<String, String> = env::vars().collect();
        let js_runtime = Arc::new(JsExecutionRuntime::with_pool_size(
            Some(env_vars),
            config.server.js_pool_size,
        ));
        js_runtime.set_setup_mode(true);
        let mut renderer =
            RscRenderer::with_resource_limits(Arc::clone(&js_runtime), resource_limits);
        renderer.initialize().await?;

        let server_manifest = if config.is_production() {
            ComponentLoader::load_server_manifest_file().await?
        } else {
            None
        };

        if config.is_production() {
            if let Some(ref manifest) = server_manifest {
                ComponentLoader::load_production_components(&mut renderer, manifest).await?;
            }
        } else {
            ComponentLoader::load_app_router_components(&mut renderer).await?;
            ComponentLoader::load_server_actions_from_source(&mut renderer).await?;
        }

        ComponentLoader::load_ssr_client_components(&renderer.runtime).await?;
        ComponentLoader::load_client_reference_manifest(&renderer.runtime).await?;
        js_runtime.set_setup_mode(false);

        let routes_manifest = RoutesManifest::load_from_file(ROUTES_MANIFEST_PATH).await;

        let app_router = match &routes_manifest {
            Ok(manifest) => Some(Arc::new(app_router::AppRouter::new(manifest.app.clone()))),
            Err(e) => {
                tracing::error!(
                    "Failed to load app router from {}: {}. All routes will return 404.",
                    ROUTES_MANIFEST_PATH,
                    e
                );
                None
            }
        };

        let api_route_handler = match &routes_manifest {
            Ok(manifest) => Some(Arc::new(api_routes::ApiRouteHandler::new(
                Arc::clone(&renderer.runtime),
                manifest.api_manifest(),
            ))),
            Err(_) => None,
        };

        let ssr_renderer = {
            let runtime = Arc::clone(&renderer.runtime);
            let ssr = RscHtmlRenderer::new(runtime);
            Arc::new(ssr)
        };

        let renderer_arc = Arc::new(Mutex::new(renderer));

        {
            let renderer_for_hook = Arc::clone(&renderer_arc);
            js_runtime.set_post_rebuild_hook(Arc::new(move |_idx, slot_runtime| {
                let renderer_for_hook = Arc::clone(&renderer_for_hook);
                Box::pin(async move {
                    let renderer = renderer_for_hook.lock().await;
                    renderer.resync_slot(slot_runtime).await
                })
            }));
        }

        let project_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let cache_registry = Arc::new(CacheHandlerRegistry::from_env());

        let response_layer = config.cache.layer(CACHE_LAYER_RESPONSE);
        let response_handler = cache_registry.resolve(&response_layer.handler);
        let cache_config = response::CacheConfig::from_env(config.is_production());
        let response_cache =
            Arc::new(response::ResponseCache::new_with_handler(cache_config, response_handler));

        let image_layer = config.cache.layer(CACHE_LAYER_IMAGE);
        let image_handler = cache_registry.resolve(&image_layer.handler);

        let og_layer = config.cache.layer(CACHE_LAYER_OG);
        let og_handler = cache_registry.resolve(&og_layer.handler);

        let og_generator = {
            let runtime = Arc::clone(&js_runtime);
            let og_cache = OgImageCache::with_handler(og_handler, &project_root);
            let generator = Arc::new(OgImageGenerator::with_capacity_and_cache(
                runtime,
                project_root.clone(),
                og_cache,
            ));

            if let Ok(manifest) = &routes_manifest {
                if let Err(e) =
                    generator.load_og_entries(&manifest.og_images, server_manifest.as_ref()).await
                {
                    tracing::error!("Failed to load OG image manifest: {}", e);
                }
            }

            Some(generator)
        };

        let layout_layer = config.cache.layer(CACHE_LAYER_LAYOUT);

        let state = ServerState {
            renderer: renderer_arc,
            ssr_renderer,
            config: Arc::new(config.clone()),
            request_count: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            component_cache_configs: Arc::new(RwLock::new(FxHashMap::default())),
            page_cache_configs: Arc::new(RwLock::new(FxHashMap::default())),
            app_router,
            api_route_handler,
            html_cache: FallbackHtmlCache::default(),
            layout_html_cache: LayoutRenderer::create_shared_cache_from_config(
                &layout_layer,
                &cache_registry,
            ),
            response_cache,
            static_fast_cache: Arc::new(response::StaticFastCache::new()),
            og_generator,
            project_root,
            image_optimizer: None,
            cache_registry: Arc::clone(&cache_registry),
            image_handler,
        };

        if config.is_production() {
            CacheLoader::load_page_cache_configs(&state).await?;
            let warmup_state = state.clone();
            tokio::spawn(async move {
                warmup::warm_cache(&warmup_state).await;
            });
        }

        let mut config = config;
        let config_path = "dist/server/image.json";

        if let Ok(image_config_str) = fs::read_to_string(config_path).await
            && let Ok(image_config) = serde_json::from_str::<ImageConfig>(&image_config_str)
        {
            config.images = image_config;
        }

        if let Err(e) = proxy::initialize_proxy(&state).await {
            tracing::error!("Failed to initialize proxy: {}", e);
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

    async fn build_router(config: &Config, mut state: ServerState) -> Result<Router, RariError> {
        let small_body_limit = DefaultBodyLimit::max(100 * 1024);
        let medium_body_limit = DefaultBodyLimit::max(1024 * 1024);

        let image_cache = Arc::new(ImageCache::with_handler(
            Arc::clone(&state.image_handler),
            config.images.max_cache_size,
            &state.project_root,
        ));
        let image_optimizer = Arc::new(ImageOptimizer::with_cache(
            config.images.clone(),
            &state.project_root,
            image_cache,
        ));

        state.image_optimizer = Some(Arc::clone(&image_optimizer));

        let image_state = ImageState { optimizer: image_optimizer };

        let revalidation_router = Router::new()
            .route("/_rari/revalidate", routing::post(revalidate_by_path))
            .layer(small_body_limit);

        let mut router = Router::new()
            .route("/_rari/health", routing::get(health_check))
            .layer(medium_body_limit)
            .route("/_rari/route-info", routing::post(get_route_info))
            .layer(small_body_limit)
            .route("/_rari/action", routing::post(handle_server_action))
            .layer(medium_body_limit)
            .merge(revalidation_router);

        let image_router = Router::new()
            .route("/_rari/image", routing::get(handle_image_request))
            .with_state(image_state);

        router = router.merge(image_router);

        let og_router = Router::new()
            .route("/_rari/og/", routing::get(og_image_handler_root))
            .route("/_rari/og/{*path}", routing::get(og_image_handler))
            .with_state(state.clone());

        router = router.merge(og_router);

        if config.is_development() {
            let medium_body_limit = DefaultBodyLimit::max(1024 * 1024);
            let large_body_limit = DefaultBodyLimit::max(50 * 1024 * 1024);

            router = router
                .route("/_rari/register", routing::post(register_component))
                .route("/_rari/register-client", routing::post(register_client_component))
                .layer(large_body_limit)
                .route("/_rari/hmr", routing::post(handle_hmr_action))
                .route("/_rari/hmr", routing::options(cors_preflight_ok))
                .layer(medium_body_limit)
                .route("/vite-server", routing::get(vite_websocket_proxy))
                .route("/vite-server/", routing::get(vite_websocket_proxy))
                .route("/vite-server/{*path}", routing::any(vite_reverse_proxy))
                .route("/src/{*path}", routing::any(vite_src_proxy));

            if let Err(e) = check_vite_server_health().await {
                tracing::debug!("Vite server not yet available: {}", e);
            }
        }

        let has_app_router = state.app_router.is_some();

        if has_app_router {
            let medium_body_limit = DefaultBodyLimit::max(1024 * 1024);
            router = router
                .route("/api/{*path}", routing::options(api_cors_preflight))
                .route("/api/{*path}", routing::any(handle_api_route))
                .layer(medium_body_limit);
        }

        if has_app_router {
            if config.is_production() {
                router = router.route("/assets/{*path}", routing::get(serve_static_asset));
            }

            router = router
                .route("/", routing::get(handle_app_route))
                .route("/", routing::post(handle_page_server_action))
                .route("/", routing::options(cors_preflight_ok))
                .route("/{*path}", routing::get(handle_app_route))
                .route("/{*path}", routing::post(handle_page_server_action))
                .route("/{*path}", routing::options(cors_preflight_ok));
        } else if config.is_production() {
            router = router
                .route("/", routing::get(root_handler))
                .route("/{*path}", routing::get(static_or_spa_handler));
        } else {
            let static_service =
                ServeDir::new(config.public_dir()).append_index_html_on_directories(true);
            router = router.fallback_service(static_service);
        }

        let compression_layer = CompressionLayer::new().compress_when(NotStreamingResponse);
        router = router.layer(compression_layer);

        if config.is_development() {
            router = router.layer(middleware::from_fn(cors_middleware));
        } else {
            router = router.layer(middleware::from_fn(security_headers_middleware));
        }

        let mut router = router.with_state(state.clone());

        if has_app_router {
            router = router.layer(ProxyLayer::new(state));
        }

        Ok(router)
    }

    #[expect(clippy::missing_errors_doc)]
    pub async fn start(self) -> Result<(), RariError> {
        self.start_with_shutdown(future::pending()).await
    }

    #[expect(clippy::missing_errors_doc)]
    pub async fn start_with_shutdown(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), RariError> {
        self.display_startup_message();

        // Disable Nagle so the final streaming chunk is not delayed ~20–40ms waiting
        // for a delayed ACK (classic last-byte tax on short responses / localhost).
        let listener = self.listener.tap_io(|tcp_stream| {
            if let Err(err) = tcp_stream.set_nodelay(true) {
                tracing::warn!("failed to set TCP_NODELAY on incoming connection: {err}");
            }
        });

        axum::serve(listener, self.router.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|e| RariError::network(format!("Server error: {e}")))?;

        Ok(())
    }

    fn display_startup_message(&self) {
        let server_url = format!("http://{}", self.address);

        if self.config.is_production() {
            #[expect(clippy::print_stdout, reason = "Server startup information output")]
            {
                println!("  {} {}", "Mode:".bold(), "Production".green());
                println!("  {} {}", "Server:".bold(), server_url.cyan().underline());

                if let Some(origin) = &self.config.server.origin {
                    println!("  {} {}", "Origin:".bold(), origin.cyan());
                }
            }
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }
}
