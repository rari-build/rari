//! Shared HTTP mapping for [`RariError`].
//!
//! Prefer these helpers at Axum boundaries so `status_code()`, `safe_message()`,
//! and `to_json_response()` are not discarded for bare [`StatusCode`] values.

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rari_error::RariError;

/// Map a [`RariError`] to an Axum [`StatusCode`].
#[must_use]
pub fn status(err: &RariError) -> StatusCode {
    StatusCode::from_u16(err.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

/// JSON error body using [`RariError::to_json_response`].
#[must_use]
pub fn json_response(err: &RariError, is_development: bool) -> Response {
    let body = err.to_json_response(is_development).to_string();

    #[expect(
        clippy::expect_used,
        reason = "Response::builder() with valid status/headers never fails"
    )]
    Response::builder()
        .status(status(err))
        .header("content-type", "application/json")
        .header("cache-control", "no-store")
        .body(Body::from(body))
        .expect("Valid RariError JSON response")
}

/// Axum error type that renders as a structured JSON [`RariError`] response.
#[derive(Debug)]
#[non_exhaustive]
pub struct HttpError {
    pub error: RariError,
    pub is_development: bool,
}

impl HttpError {
    #[must_use]
    pub fn new(error: RariError, is_development: bool) -> Self {
        Self { error, is_development }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        json_response(&self.error, self.is_development)
    }
}
