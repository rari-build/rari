use crate::server::ServerState;
use axum::{
    extract::{ConnectInfo, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Debug, Serialize)]
pub struct CsrfTokenResponse {
    pub token: String,
}

pub async fn get_csrf_token(
    State(state): State<ServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Response, StatusCode> {
    let ip = addr.ip().to_string();

    if let Err(retry_after) = state.endpoint_rate_limiters.csrf_token.check(&ip) {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            [
                (header::RETRY_AFTER, retry_after.to_string()),
                (header::CONTENT_TYPE, "text/plain".to_string()),
            ],
            "Rate limit exceeded for CSRF token generation. Please try again later.",
        )
            .into_response());
    }

    let token = state.csrf_manager.generate_token();
    let response = CsrfTokenResponse { token };

    Ok(Json(response).into_response())
}
