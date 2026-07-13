use std::env;

pub mod cache;
mod config;
mod optimizer;
mod scanner;
mod types;

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
pub use cache::ImageCache;
pub use config::{ImageConfig, ImageVariant, LocalPattern, RemotePattern};
pub use optimizer::{ImageOptimizer, PreloadImage};
use rari_error::RariError;
pub use scanner::{ImageUsageManifest, ScanError, scan_for_image_usage};
pub use types::{DEFAULT_IMAGE_QUALITY, ImageFormat, OptimizeParams, OptimizedImage};

use crate::server::{config::Config, error_response};

#[derive(Clone)]
#[non_exhaustive]
pub struct ImageState {
    pub optimizer: Arc<ImageOptimizer>,
}

#[expect(clippy::missing_errors_doc)]
pub async fn handle_image_request(
    State(state): State<ImageState>,
    Query(params): Query<OptimizeParams>,
) -> Result<Response, ImageError> {
    let (optimized, cache_hit) = state.optimizer.optimize(params).await?;

    let content_type = match optimized.format {
        ImageFormat::Avif => "image/avif",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::Gif => "image/gif",
    };

    let is_production = env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

    let cache_header = if is_production {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=0, must-revalidate"
    };

    let x_cache = if cache_hit { "HIT" } else { "MISS" };

    let mut response = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type), (header::CACHE_CONTROL, cache_header)],
        optimized.data,
    )
        .into_response();

    response.headers_mut().insert(
        "x-cache",
        #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
        x_cache.parse().expect("x-cache header value should be valid ASCII"),
    );

    Ok(response)
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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

impl From<&ImageError> for RariError {
    fn from(err: &ImageError) -> Self {
        match err {
            ImageError::InvalidUrl(_) | ImageError::InvalidParams(_) => {
                Self::bad_request(err.to_string())
            }
            ImageError::UnauthorizedDomain(_) => Self::forbidden(err.to_string()),
            ImageError::FetchError(_) => Self::network(err.to_string()),
            ImageError::ProcessingError(_) => Self::internal(err.to_string()),
        }
    }
}

impl IntoResponse for ImageError {
    fn into_response(self) -> Response {
        let is_dev = Config::get().is_some_and(Config::is_development);
        error_response::json_response(&RariError::from(&self), is_dev)
    }
}
