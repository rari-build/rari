use crate::server::ServerState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CsrfTokenResponse {
    pub token: String,
}

pub async fn get_csrf_token(State(state): State<ServerState>) -> Result<Response, StatusCode> {
    let token = state.csrf_manager.generate_token();
    let response = CsrfTokenResponse { token };

    Ok(Json(response).into_response())
}
