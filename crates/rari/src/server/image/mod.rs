mod cache;
mod config;
mod optimizer;
mod types;

pub use config::{ImageConfig, RemotePattern};
pub use optimizer::ImageOptimizer;
pub use types::{ImageFormat, OptimizeParams, OptimizedImage};

use axum::{
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub async fn handle_image_request(
    State(optimizer): State<Arc<ImageOptimizer>>,
    Query(params): Query<OptimizeParams>,
) -> Result<Response, ImageError> {
    let optimized = optimizer.optimize(params).await?;

    let content_type = match optimized.format {
        ImageFormat::Avif => "image/avif",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::Gif => "image/gif",
    };

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        optimized.data,
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Unauthorized domain: {0}")]
    UnauthorizedDomain(String),
    #[error("Failed to fetch image: {0}")]
    FetchError(String),
    #[error("Failed to process image: {0}")]
    ProcessingError(String),
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
}

impl IntoResponse for ImageError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ImageError::InvalidUrl(_) | ImageError::InvalidParams(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            ImageError::UnauthorizedDomain(_) => (StatusCode::FORBIDDEN, self.to_string()),
            ImageError::FetchError(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ImageError::ProcessingError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, message).into_response()
    }
}
