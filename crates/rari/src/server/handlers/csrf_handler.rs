use crate::server::ServerState;
use crate::server::utils::ip_extractor::extract_client_ip;
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode, header},
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
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let Some(csrf_manager) = &state.csrf_manager else {
        return Ok((
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain".to_string())],
            "CSRF protection is not enabled. Set RARI_CSRF_SECRET environment variable to enable it.",
        )
            .into_response());
    };

    let ip = extract_client_ip(&headers, &addr);

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

    let token = csrf_manager.generate_token();
    let response = CsrfTokenResponse { token };

    Ok(Json(response).into_response())
}
