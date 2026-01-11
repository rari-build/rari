mod cache;
mod generator;
mod layout;
mod rendering;
mod resources;
mod types;

pub use cache::OgImageCache;
pub use generator::OgImageGenerator;
pub use types::{OgImageParams, OgImageResult};

use crate::error::RariError;
use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub async fn handle_og_image_request(
    State(generator): State<Arc<OgImageGenerator>>,
    Path(route_path): Path<String>,
) -> Result<Response, OgImageError> {
    let normalized_path = if route_path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", route_path.trim_start_matches('/'))
    };

    let (image_data, cache_hit) = generator.generate(&normalized_path).await?;

    let x_cache = if cache_hit { "HIT" } else { "MISS" };

    let mut response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/webp"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        image_data,
    )
        .into_response();

    response
        .headers_mut()
        .insert("x-cache", x_cache.parse().expect("x-cache header value should be valid ASCII"));

    Ok(response)
}

#[derive(Debug, thiserror::Error)]
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
        OgImageError::InternalError(err.to_string())
    }
}

impl IntoResponse for OgImageError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            OgImageError::ComponentNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            OgImageError::InvalidParams(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            OgImageError::ExecutionError(_) | OgImageError::GenerationError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            OgImageError::InternalError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        (status, message).into_response()
    }
}
