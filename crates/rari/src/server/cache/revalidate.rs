#![expect(clippy::missing_errors_doc)]

use std::{env, sync::Arc};

use axum::{extract::State, http::StatusCode, response::Json};
use rari_error::RariError;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    rendering::base::RscRenderer,
    runtime::factory::JsRuntimeInterface,
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

pub(crate) async fn invalidate_use_cache_entries(
    renderer: &Arc<Mutex<RscRenderer>>,
    tag: Option<&str>,
    path: Option<&str>,
) -> Result<(), RariError> {
    let script = use_cache_invalidate_script(tag, path);
    let runtime = {
        let renderer = renderer.lock().await;
        Arc::clone(&renderer.runtime)
    };
    runtime.broadcast_script("use_cache_invalidate", &script).await
}

pub(crate) async fn invalidate_use_cache_entries_on(
    sticky_runtime: &Arc<dyn JsRuntimeInterface>,
    tag: Option<&str>,
    path: Option<&str>,
) -> Result<(), RariError> {
    let script = use_cache_invalidate_script(tag, path);
    sticky_runtime.execute_script("use_cache_invalidate".to_string(), script).await.map(|_| ())
}

fn use_cache_invalidate_script(tag: Option<&str>, path: Option<&str>) -> String {
    let tag_literal = tag
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "undefined".to_string());
    let path_literal = path
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "undefined".to_string());

    format!(
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
    )
}

pub(crate) async fn invalidate_route_caches(
    state: &ServerState,
    path: &str,
) -> Result<(), RariError> {
    invalidate_route_caches_inner(state, path, None).await
}

/// Like [`invalidate_route_caches`], but runs use-cache invalidation on `sticky_runtime`
/// instead of broadcasting (avoids re-entering a held pool slot lease).
pub(crate) async fn invalidate_route_caches_on(
    state: &ServerState,
    path: &str,
    sticky_runtime: &Arc<dyn JsRuntimeInterface>,
) -> Result<(), RariError> {
    invalidate_route_caches_inner(state, path, Some(sticky_runtime)).await
}

async fn invalidate_route_caches_inner(
    state: &ServerState,
    path: &str,
    sticky_runtime: Option<&Arc<dyn JsRuntimeInterface>>,
) -> Result<(), RariError> {
    state.response_cache.invalidate(path).await;
    state.response_cache.invalidate_by_tag(path).await;
    response::invalidate_static_fast_cache_for_path(&state.static_fast_cache, path);
    state.html_cache.remove(path);

    let use_cache_result = if let Some(runtime) = sticky_runtime {
        invalidate_use_cache_entries_on(runtime, None, Some(path)).await
    } else {
        invalidate_use_cache_entries(&state.renderer, None, Some(path)).await
    };
    if let Err(e) = use_cache_result {
        tracing::warn!(error = %e, path = %path, "use cache invalidate failed during route cache invalidation");
        return Err(e);
    }

    let rsc_cache_key =
        response::ResponseCache::generate_cache_key_with_mode(path, None, Some("rsc"), None);
    state.response_cache.invalidate(&rsc_cache_key).await;

    for key in state.response_cache.get_all_keys() {
        if response::ResponseCache::cache_key_matches_route(&key, path) {
            state.response_cache.invalidate(&key).await;
        }
    }

    state.layout_html_cache.clear().await.map_err(|e| {
        tracing::warn!(
            error = %e,
            path = %path,
            "layout_html_cache.clear failed during route cache invalidation"
        );
        RariError::from(format!("layout cache clear error: {e}"))
    })
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

            let res = match invalidate_route_caches(&state, path).await {
                Ok(()) => RevalidateResponse {
                    revalidated: true,
                    message: Some(format!("Revalidated path: {path}")),
                },
                Err(e) => {
                    tracing::error!(error = %e, path = %path, "route cache invalidation failed");
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
