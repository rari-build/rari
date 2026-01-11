use crate::server::ServerState;
use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

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
) -> Result<Response, StatusCode> {
    og_image_handler(State(state), Path("/".to_string())).await
}
