use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::server::types::ServerState;

#[derive(Debug, Deserialize)]
pub struct RouteInfoRequest {
    path: String,
}

#[derive(Debug, Serialize)]
pub struct RouteInfoResponse {
    exists: bool,
    layouts: Vec<String>,
    loading: Option<String>,
    #[serde(rename = "isDynamic")]
    is_dynamic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct RouteInfoError {
    error: String,
    code: String,
}

pub async fn get_route_info(
    State(state): State<ServerState>,
    Json(request): Json<RouteInfoRequest>,
) -> Result<Json<RouteInfoResponse>, (StatusCode, Json<RouteInfoError>)> {
    let path = request.path;

    if path.is_empty() || !path.starts_with('/') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(RouteInfoError {
                error: "Invalid path: must start with /".to_string(),
                code: "INVALID_PATH".to_string(),
            }),
        ));
    }

    let app_router = match &state.app_router {
        Some(router) => router,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(RouteInfoError {
                    error: "App router not available".to_string(),
                    code: "SERVER_ERROR".to_string(),
                }),
            ));
        }
    };

    match app_router.match_route(&path) {
        Ok(route_match) => {
            let layouts = route_match.layouts.iter().map(|layout| layout.path.clone()).collect();

            let loading = route_match.loading.map(|l| l.path);

            let params = if route_match.route.params.is_empty() {
                None
            } else {
                Some(route_match.route.params.clone())
            };

            Ok(Json(RouteInfoResponse {
                exists: true,
                layouts,
                loading,
                is_dynamic: route_match.route.is_dynamic,
                params,
            }))
        }
        Err(_) => Ok(Json(RouteInfoResponse {
            exists: false,
            layouts: vec![],
            loading: None,
            is_dynamic: false,
            params: None,
        })),
    }
}
