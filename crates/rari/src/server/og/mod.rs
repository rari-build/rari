#![expect(clippy::missing_errors_doc)]

mod cache;
mod generator;
mod layout;
mod rendering;
mod resources;
mod types;

pub(super) const MAX_OG_IMAGE_BYTES: usize = 10 * 1024 * 1024;

use std::env;

use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
pub use cache::OgImageCache;
pub use generator::OgImageGenerator;
use rari_error::RariError;
pub use types::{OgImageParams, OgImageResult};

use crate::server::ServerState;

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
                let status = if err.to_string().contains("not found") {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                };
                Ok((status, err.to_string()).into_response())
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

impl IntoResponse for OgImageError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::ComponentNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            Self::InvalidParams(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::ExecutionError(_) | Self::GenerationError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            Self::InternalError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        (status, message).into_response()
    }
}
