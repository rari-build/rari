#![expect(clippy::missing_errors_doc)]

use std::env;

use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use rari_error::RariError;

mod generator;
mod layout;
mod rendering;
mod resources;

pub(super) const MAX_OG_IMAGE_BYTES: usize = 10 * 1024 * 1024;

pub use generator::OgImageGenerator;
pub use rari_core::og::{OgImageCache, OgImageEntry, OgImageParams, OgImageResult, cache, types};

use crate::server::{ServerState, config::Config, error_response};

pub async fn og_image_handler(
    State(state): State<ServerState>,
    Path(route_path): Path<String>,
) -> Result<Response, StatusCode> {
    if let Some(og_generator) = &state.og_generator {
        let normalized_path = if route_path.is_empty() || route_path == "/" {
            "/".to_string()
        } else {
            format!("/{}", route_path.trim_start_matches('/'))
        };

        match og_generator.generate(&normalized_path).await {
            Ok((image_data, cache_hit)) => {
                let is_production =
                    env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

                let cache_header = if is_production {
                    "public, max-age=31536000, immutable"
                } else {
                    "public, max-age=0, must-revalidate"
                };

                let x_cache = if cache_hit { "HIT" } else { "MISS" };

                let mut response = (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "image/webp"), (header::CACHE_CONTROL, cache_header)],
                    image_data,
                )
                    .into_response();

                response.headers_mut().insert(
                    "x-cache",
                    #[expect(
                        clippy::expect_used,
                        reason = "Infallible operation with valid inputs"
                    )]
                    x_cache.parse().expect("x-cache header value should be valid ASCII"),
                );

                Ok(response)
            }
            Err(err) => {
                tracing::error!("OG image generation error: {}", err);
                Ok(err.into_response())
            }
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn og_image_handler_root(
    State(state): State<ServerState>,
) -> Result<Response, StatusCode> {
    og_image_handler(State(state), Path("/".to_string())).await
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OgImageError {
    #[error("OG image component not found for route: {0}")]
    ComponentNotFound(String),
    #[error("Failed to execute OG component: {0}")]
    ExecutionError(String),
    #[error("Failed to generate image: {0}")]
    GenerationError(String),
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<RariError> for OgImageError {
    fn from(err: RariError) -> Self {
        Self::InternalError(err.to_string())
    }
}

impl From<&OgImageError> for RariError {
    fn from(err: &OgImageError) -> Self {
        match err {
            OgImageError::ComponentNotFound(_) => Self::not_found(err.to_string()),
            OgImageError::InvalidParams(_) => Self::validation(err.to_string()),
            OgImageError::ExecutionError(_) => Self::js_execution(err.to_string()),
            OgImageError::GenerationError(_) | OgImageError::InternalError(_) => {
                Self::internal(err.to_string())
            }
        }
    }
}

impl IntoResponse for OgImageError {
    fn into_response(self) -> Response {
        let is_dev = Config::get().is_some_and(Config::is_development);
        error_response::json_response(&RariError::from(&self), is_dev)
    }
}
