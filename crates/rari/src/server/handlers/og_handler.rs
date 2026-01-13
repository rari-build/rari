use crate::server::ServerState;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

pub async fn og_image_handler(
    State(state): State<ServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(route_path): Path<String>,
) -> Result<Response, StatusCode> {
    if let Some(og_generator) = &state.og_generator {
        let normalized_path = if route_path.is_empty() || route_path == "/" {
            "/".to_string()
        } else {
            format!("/{}", route_path.trim_start_matches('/'))
        };

        let ip = addr.ip().to_string();
        if let Err(retry_after) = state.endpoint_rate_limiters.og_generation.check(&ip) {
            return Ok((
                StatusCode::TOO_MANY_REQUESTS,
                [
                    (header::RETRY_AFTER, retry_after.to_string()),
                    (header::CONTENT_TYPE, "text/plain".to_string()),
                ],
                "Rate limit exceeded for OG image generation. Please try again later.",
            )
                .into_response());
        }

        match og_generator.generate(&normalized_path).await {
            Ok((image_data, cache_hit)) => {
                let is_production =
                    std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

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
    connect_info: ConnectInfo<SocketAddr>,
) -> Result<Response, StatusCode> {
    og_image_handler(State(state), connect_info, Path("/".to_string())).await
}
