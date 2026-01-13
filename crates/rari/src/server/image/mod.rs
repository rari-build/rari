mod cache;
mod config;
mod optimizer;
mod types;

pub use config::{ImageConfig, LocalPattern, RemotePattern};
pub use optimizer::ImageOptimizer;
pub use types::{ImageFormat, OptimizeParams, OptimizedImage};

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::server::security::ip_rate_limiter::EndpointRateLimiters;

#[derive(Clone)]
pub struct ImageState {
    pub optimizer: Arc<ImageOptimizer>,
    pub rate_limiters: EndpointRateLimiters,
}

pub async fn handle_image_request(
    State(state): State<ImageState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<OptimizeParams>,
) -> Result<Response, ImageError> {
    let ip = addr.ip().to_string();
    if let Err(retry_after) = state.rate_limiters.image_optimization.check(&ip) {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            [
                (header::RETRY_AFTER, retry_after.to_string()),
                (header::CONTENT_TYPE, "text/plain".to_string()),
            ],
            "Rate limit exceeded for image optimization. Please try again later.",
        )
            .into_response());
    }

    let (optimized, cache_hit) = state.optimizer.optimize(params).await?;

    let content_type = match optimized.format {
        ImageFormat::Avif => "image/avif",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::Gif => "image/gif",
    };

    let is_production = std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

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

    response
        .headers_mut()
        .insert("x-cache", x_cache.parse().expect("x-cache header value should be valid ASCII"));

    Ok(response)
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
