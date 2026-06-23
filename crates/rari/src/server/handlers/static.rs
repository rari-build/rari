use crate::server::ServerState;
use crate::server::config::Config;
use crate::server::core::utils::http::get_content_type;
use crate::server::core::utils::path_validation::validate_safe_path;
use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use tracing::error;

pub async fn root_handler(State(_state): State<ServerState>) -> Result<Response, StatusCode> {
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
                let cache_control = config.get_cache_control_for_route("/");
                let response_builder = Response::builder()
                    .header("content-type", "text/html")
                    .header("cache-control", cache_control);

                #[expect(
                    clippy::expect_used,
                    reason = "Response::builder() with valid components never fails"
                )]
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
    State(_state): State<ServerState>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    const BLOCKED_FILES: &[&str] = &["server/manifest.json", "server/routes.json", "server/"];

    for blocked in BLOCKED_FILES {
        if path.starts_with(blocked) || path == *blocked {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for static_or_spa_handler");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let file_path = match validate_safe_path(config.public_dir(), &path) {
        Ok(path) => path,
        Err(_) => {
            return Err(StatusCode::NOT_FOUND);
        }
    };

    if file_path.is_file() {
        match std::fs::read(&file_path) {
            Ok(content) => {
                let content_type = get_content_type(&path);
                let cache_control = &config.caching.static_files;
                #[expect(
                    clippy::expect_used,
                    reason = "Response::builder() with valid components never fails"
                )]
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
        let last_segment = path.rsplit('/').next().unwrap_or(&path);
        if last_segment.contains('.') {
            let parts: Vec<&str> = last_segment.split('.').collect();
            if parts.len() >= 2
                && let Some(extension) = parts.last()
                && extension.len() >= 2
                && extension.len() <= 5
                && extension.chars().all(char::is_alphanumeric)
            {
                return Err(StatusCode::NOT_FOUND);
            }
        }
    }

    let route_path = if path.is_empty() {
        "/"
    } else {
        &format!("/{path}")
    };

    let index_path = config.public_dir().join("index.html");
    if index_path.exists() {
        match std::fs::read_to_string(&index_path) {
            Ok(content) => {
                let cache_control = config.get_cache_control_for_route(route_path);
                let response_builder = Response::builder()
                    .header("content-type", "text/html")
                    .header("cache-control", cache_control);

                #[expect(
                    clippy::expect_used,
                    reason = "Response::builder() with valid components never fails"
                )]
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
    if asset_path.contains("server/manifest.json")
        || asset_path.contains("server/routes.json")
        || asset_path.starts_with("../")
    {
        return Err(StatusCode::NOT_FOUND);
    }

    let assets_dir = state.config.public_dir().join("assets");

    let file_path = match validate_safe_path(&assets_dir, &asset_path) {
        Ok(path) => path,
        Err(_) => {
            return Err(StatusCode::NOT_FOUND);
        }
    };

    if !file_path.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }

    match std::fs::read(&file_path) {
        Ok(content) => {
            let content_type = get_content_type(&asset_path);
            let cache_control = &state.config.caching.static_files;

            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
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
    #[expect(
        clippy::expect_used,
        reason = "Response::builder() always initializes headers"
    )]
    let headers = builder
        .headers_mut()
        .expect("Response builder should have headers");
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
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    builder
        .body(Body::empty())
        .expect("Valid preflight response")
}

#[axum::debug_handler]
pub async fn cors_preflight_ok() -> Response {
    cors_preflight_response()
}
