#![expect(clippy::missing_errors_doc)]

use std::{env, sync::Arc};

use axum::{extract::State, http::StatusCode, response::Json};
use rari_error::RariError;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    rendering::base::RscRenderer,
    server::{ServerState, cache::response},
};

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

async fn invalidate_use_cache_entries(
    renderer: &Arc<Mutex<RscRenderer>>,
    tag: Option<&str>,
    path: Option<&str>,
) -> Result<(), RariError> {
    let tag_literal = tag
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "undefined".to_string());
    let path_literal = path
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "undefined".to_string());

    let script = format!(
        "(async () => {{
            const invalidateDirect = globalThis.__rariInvalidateUseCache;
            const invalidateBridge = globalThis['~rari']?.invalidateUseCache;
            if (typeof invalidateDirect === 'function') {{
                if ({tag_literal} !== undefined)
                    await invalidateDirect({tag_literal});
                if ({path_literal} !== undefined)
                    await invalidateDirect({path_literal});
                return;
            }}
            if (typeof invalidateBridge === 'function') {{
                await invalidateBridge({{ tag: {tag_literal}, path: {path_literal} }});
            }}
        }})()"
    );

    let runtime = {
        let renderer = renderer.lock().await;
        Arc::clone(&renderer.runtime)
    };

    runtime.execute_script("use_cache_invalidate".to_string(), script).await.map(|_| ())
}

#[axum::debug_handler]
pub async fn revalidate_by_path(
    State(state): State<ServerState>,
    Json(request): Json<RevalidateRequest>,
) -> Result<Json<RevalidateResponse>, StatusCode> {
    let expected_secret = env::var("RARI_REVALIDATE_SECRET").map_err(|_| {
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
            response::invalidate_static_fast_cache_for_path(&state.static_fast_cache, path);
            let use_cache_result =
                invalidate_use_cache_entries(&state.renderer, None, Some(path)).await;

            let path_pattern = format!("{path}?");
            let all_keys = state.response_cache.get_all_keys();

            for key in all_keys {
                if key.starts_with(&path_pattern) {
                    state.response_cache.invalidate(&key).await;
                }
            }

            let layout_result = state.layout_html_cache.clear().await;

            let res = match (layout_result, use_cache_result) {
                (Ok(()), Ok(())) => RevalidateResponse {
                    revalidated: true,
                    message: Some(format!("Revalidated path: {path}")),
                },
                (Err(e), _) => {
                    tracing::error!(error = %e, path = %path, "layout_html_cache.clear failed");
                    RevalidateResponse {
                        revalidated: false,
                        message: Some(format!(
                            "Revalidation failed: layout cache clear error: {e}"
                        )),
                    }
                }
                (Ok(()), Err(e)) => {
                    tracing::error!(error = %e, path = %path, "use cache invalidate script failed");
                    RevalidateResponse {
                        revalidated: false,
                        message: Some(format!(
                            "Revalidation failed: use cache invalidate error: {e}"
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
            response::invalidate_static_fast_cache_for_path(&state.static_fast_cache, tag);
            let use_cache_result =
                invalidate_use_cache_entries(&state.renderer, Some(tag), None).await;

            let layout_result = state.layout_html_cache.invalidate_by_tag(tag).await;

            let res = match (layout_result, use_cache_result) {
                (Ok(()), Ok(())) => RevalidateResponse {
                    revalidated: true,
                    message: Some(format!("Revalidated tag: {tag}")),
                },
                (Err(e), _) => {
                    tracing::error!(error = %e, tag = %tag, "layout_html_cache.invalidate_by_tag failed");
                    RevalidateResponse {
                        revalidated: false,
                        message: Some(format!(
                            "Revalidation failed: layout cache invalidate_by_tag error: {e}"
                        )),
                    }
                }
                (Ok(()), Err(e)) => {
                    tracing::error!(error = %e, tag = %tag, "use cache invalidate script failed");
                    RevalidateResponse {
                        revalidated: false,
                        message: Some(format!(
                            "Revalidation failed: use cache invalidate error: {e}"
                        )),
                    }
                }
            };

            Ok(Json(res))
        }
    }
}
