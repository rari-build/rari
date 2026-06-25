use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};

use crate::server::ServerState;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[non_exhaustive]
pub enum RevalidateRequest {
    Path {
        path: String,
        #[serde(default)]
        secret: Option<String>,
    },
    Tag {
        tag: String,
        #[serde(default)]
        secret: Option<String>,
    },
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct RevalidateResponse {
    pub revalidated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[axum::debug_handler]
pub async fn revalidate_by_path(
    State(state): State<ServerState>,
    Json(request): Json<RevalidateRequest>,
) -> Result<Json<RevalidateResponse>, StatusCode> {
    let expected_secret = std::env::var("RARI_REVALIDATE_SECRET").map_err(|_| {
        tracing::error!("RARI_REVALIDATE_SECRET not configured. Set this environment variable to enable revalidation.");
        StatusCode::FORBIDDEN
    })?;

    match &request {
        RevalidateRequest::Path { path, secret } => {
            match secret {
                Some(provided_secret) if provided_secret == &expected_secret => {}
                _ => {
                    return Ok(Json(RevalidateResponse {
                        revalidated: false,
                        message: Some("Invalid or missing secret".to_string()),
                    }));
                }
            }

            state.response_cache.invalidate(path).await;

            let path_pattern = format!("{path}?");
            let all_keys = state.response_cache.get_all_keys();

            for key in all_keys {
                if key.starts_with(&path_pattern) {
                    state.response_cache.invalidate(&key).await;
                }
            }

            let res = match state.layout_html_cache.clear().await {
                Ok(()) => RevalidateResponse {
                    revalidated: true,
                    message: Some(format!("Revalidated path: {path}")),
                },
                Err(e) => {
                    tracing::error!(error = %e, path = %path, "layout_html_cache.clear failed");
                    RevalidateResponse {
                        revalidated: false,
                        message: Some(format!(
                            "Revalidation failed: layout cache clear error: {e}"
                        )),
                    }
                }
            };

            Ok(Json(res))
        }
        RevalidateRequest::Tag { tag, secret } => {
            match secret {
                Some(provided_secret) if provided_secret == &expected_secret => {}
                _ => {
                    return Ok(Json(RevalidateResponse {
                        revalidated: false,
                        message: Some("Invalid or missing secret".to_string()),
                    }));
                }
            }

            state.response_cache.invalidate_by_tag(tag).await;

            let res = match state.layout_html_cache.invalidate_by_tag(tag).await {
                Ok(()) => RevalidateResponse {
                    revalidated: true,
                    message: Some(format!("Revalidated tag: {tag}")),
                },
                Err(e) => {
                    tracing::error!(error = %e, tag = %tag, "layout_html_cache.invalidate_by_tag failed");
                    RevalidateResponse {
                        revalidated: false,
                        message: Some(format!(
                            "Revalidation failed: layout cache invalidate_by_tag error: {e}"
                        )),
                    }
                }
            };

            Ok(Json(res))
        }
    }
}
