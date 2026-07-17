#![expect(clippy::missing_errors_doc)]

use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rari_error::RariError;
use tokio::fs;

use crate::server::{
    ServerState,
    config::Config,
    core::utils::{http::get_content_type, path_validation::validate_safe_path},
    error_response::HttpError,
};

fn static_dev() -> bool {
    Config::get().is_some_and(Config::is_development)
}

pub async fn root_handler(State(_state): State<ServerState>) -> Result<Response, HttpError> {
    let Some(config) = Config::get() else {
        tracing::error!("Failed to get global configuration for root_handler");
        return Err(HttpError::new(
            RariError::configuration("Configuration not available"),
            static_dev(),
        ));
    };

    let index_path = config.public_dir().join("index.html");
    if fs::try_exists(&index_path).await.unwrap_or(false) {
        match fs::read_to_string(&index_path).await {
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
                tracing::error!("Failed to read index.html: {}", e);
                return Err(HttpError::new(
                    RariError::io(format!("Failed to read index.html: {e}")),
                    static_dev(),
                ));
            }
        }
    }
    Ok(StatusCode::NOT_FOUND.into_response())
}

pub async fn static_or_spa_handler(
    State(_state): State<ServerState>,
    Path(path): Path<String>,
) -> Result<Response, HttpError> {
    const BLOCKED_FILES: &[&str] = &["server/manifest.json", "server/routes.json", "server/"];

    for blocked in BLOCKED_FILES {
        if path.starts_with(blocked) || path == *blocked {
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
    }

    let Some(config) = Config::get() else {
        tracing::error!("Failed to get global configuration for static_or_spa_handler");
        return Err(HttpError::new(
            RariError::configuration("Configuration not available"),
            static_dev(),
        ));
    };

    let Ok(file_path) = validate_safe_path(config.public_dir(), &path).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    if let Ok(metadata) = fs::metadata(&file_path).await
        && metadata.is_file()
    {
        match fs::read(&file_path).await {
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
                tracing::error!("Failed to read static file {}: {}", file_path.display(), e);
                return Err(HttpError::new(
                    RariError::io(format!("Failed to read static file: {e}")),
                    static_dev(),
                ));
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
                return Ok(StatusCode::NOT_FOUND.into_response());
            }
        }
    }

    let route_path = if path.is_empty() { "/" } else { &format!("/{path}") };

    let index_path = config.public_dir().join("index.html");
    if fs::try_exists(&index_path).await.unwrap_or(false) {
        match fs::read_to_string(&index_path).await {
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
                tracing::error!("Failed to read index.html: {}", e);
                return Err(HttpError::new(
                    RariError::io(format!("Failed to read index.html: {e}")),
                    static_dev(),
                ));
            }
        }
    }
    Ok(StatusCode::NOT_FOUND.into_response())
}

pub async fn serve_static_asset(
    State(state): State<ServerState>,
    Path(asset_path): Path<String>,
) -> Result<Response, HttpError> {
    if asset_path.contains("server/manifest.json")
        || asset_path.contains("server/routes.json")
        || asset_path.starts_with("../")
    {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let assets_dir = state.core.config.public_dir().join("assets");

    let Ok(file_path) = validate_safe_path(&assets_dir, &asset_path).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let Ok(metadata) = fs::metadata(&file_path).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    if !metadata.is_file() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    match fs::read(&file_path).await {
        Ok(content) => {
            let content_type = get_content_type(&asset_path);
            let cache_control = &state.core.config.caching.static_files;

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
            tracing::error!("Failed to read static asset {}: {}", file_path.display(), e);
            Err(HttpError::new(
                RariError::io(format!("Failed to read static asset: {e}")),
                static_dev(),
            ))
        }
    }
}

pub fn cors_preflight_response() -> Response {
    use axum::http::HeaderValue;

    let mut builder = Response::builder().status(StatusCode::NO_CONTENT);
    #[expect(clippy::expect_used, reason = "Response::builder() always initializes headers")]
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

    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    builder.body(Body::empty()).expect("Valid CORS preflight response")
}

#[axum::debug_handler]
pub async fn cors_preflight_ok() -> Response {
    cors_preflight_response()
}
