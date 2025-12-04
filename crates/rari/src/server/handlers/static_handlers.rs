use crate::server::ServerState;
use crate::server::config::Config;
use crate::server::loaders::cache_loader::CacheLoader;
use crate::server::utils::http_utils::get_content_type;
use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use tracing::{debug, error};

pub async fn root_handler(State(state): State<ServerState>) -> Result<Response, StatusCode> {
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
                    CacheLoader::find_matching_cache_config(&page_configs, "/")
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

pub async fn static_or_spa_handler(
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
                    CacheLoader::find_matching_cache_config(&page_configs, route_path)
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

pub async fn serve_static_asset(
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

pub fn cors_preflight_response() -> Response {
    use axum::http::HeaderValue;

    let mut builder = Response::builder().status(StatusCode::NO_CONTENT);
    let headers = builder.headers_mut().expect("Response builder should have headers");
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"),
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
pub async fn cors_preflight_ok() -> Response {
    cors_preflight_response()
}
