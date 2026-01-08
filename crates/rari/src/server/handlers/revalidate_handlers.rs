use crate::server::ServerState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RevalidatePathRequest {
    pub path: String,
    #[serde(default)]
    pub secret: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevalidateTagRequest {
    pub tag: String,
    #[serde(default)]
    pub secret: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RevalidateResponse {
    pub revalidated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[axum::debug_handler]
pub async fn revalidate_by_path(
    State(state): State<ServerState>,
    Json(request): Json<RevalidatePathRequest>,
) -> Result<Json<RevalidateResponse>, StatusCode> {
    let expected_secret = std::env::var("RARI_REVALIDATE_SECRET").map_err(|_| {
        tracing::error!("RARI_REVALIDATE_SECRET not configured. Set this environment variable to enable revalidation.");
        StatusCode::FORBIDDEN
    })?;

    match request.secret {
        Some(provided_secret) if provided_secret == expected_secret => {}
        _ => {
            return Ok(Json(RevalidateResponse {
                revalidated: false,
                message: Some("Invalid or missing secret".to_string()),
            }));
        }
    }

    state.response_cache.invalidate(&request.path).await;

    let path_pattern = format!("{}?", request.path);
    let all_keys = state.response_cache.get_all_keys();

    for key in all_keys {
        if key.starts_with(&path_pattern) {
            state.response_cache.invalidate(&key).await;
        }
    }

    #[allow(clippy::disallowed_methods)]
    Ok(Json(RevalidateResponse {
        revalidated: true,
        message: Some(format!("Revalidated path: {}", request.path)),
    }))
}

#[axum::debug_handler]
pub async fn revalidate_by_tag(
    State(state): State<ServerState>,
    Json(request): Json<RevalidateTagRequest>,
) -> Result<Json<RevalidateResponse>, StatusCode> {
    let expected_secret = std::env::var("RARI_REVALIDATE_SECRET").map_err(|_| {
        tracing::error!("RARI_REVALIDATE_SECRET not configured. Set this environment variable to enable revalidation.");
        StatusCode::FORBIDDEN
    })?;

    match request.secret {
        Some(provided_secret) if provided_secret == expected_secret => {}
        _ => {
            return Ok(Json(RevalidateResponse {
                revalidated: false,
                message: Some("Invalid or missing secret".to_string()),
            }));
        }
    }

    state.response_cache.invalidate_by_tag(&request.tag).await;

    #[allow(clippy::disallowed_methods)]
    Ok(Json(RevalidateResponse {
        revalidated: true,
        message: Some(format!("Revalidated tag: {}", request.tag)),
    }))
}
