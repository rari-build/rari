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
pub use scanner::{ImageUsageManifest, ScanError, scan_for_image_usage};
pub use types::{DEFAULT_IMAGE_QUALITY, ImageFormat, OptimizeParams, OptimizedImage};

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

impl IntoResponse for ImageError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::InvalidUrl(_) | Self::InvalidParams(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            Self::UnauthorizedDomain(_) => (StatusCode::FORBIDDEN, self.to_string()),
            Self::FetchError(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            Self::ProcessingError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, message).into_response()
    }
}
