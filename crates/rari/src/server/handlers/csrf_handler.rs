use crate::server::ServerState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use tracing::debug;

#[derive(Debug, Serialize)]
pub struct CsrfTokenResponse {
    pub token: String,
}

pub async fn get_csrf_token(State(state): State<ServerState>) -> Result<Response, StatusCode> {
    debug!("Generating CSRF token");

    let token = state.csrf_manager.generate_token();
    let response = CsrfTokenResponse { token };

    Ok(Json(response).into_response())
}
