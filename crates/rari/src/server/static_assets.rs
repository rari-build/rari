//! Axum handler wrappers that delegate to `rari_core::static_assets`.
//!
//! The core logic lives in `rari_core::static_assets` and takes `&CoreState` or
//! no state at all.  These wrappers adapt the `ServerState` type expected by the
//! `rari` router.

#![expect(clippy::missing_errors_doc)]

use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
};
use rari_core::{
    error_response::HttpError,
    static_assets as core_static_assets,
};

use crate::server::ServerState;

/// CORS preflight response for the React/RSC backend.
///
/// Contains the RSC-specific `X-RSC-Streaming` header.
pub fn cors_preflight_response() -> Response {
    use axum::http::HeaderValue;
    use axum::http::StatusCode;

    let mut builder = Response::builder().status(StatusCode::NO_CONTENT);
    #[expect(clippy::expect_used, reason = "Response::builder() always initializes headers")]
    let headers = builder.headers_mut().expect("Response builder should have headers");
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static(
            "Content-Type, Authorization, Accept, Origin, X-Requested-With, Cache-Control, X-RSC-Streaming",
        ),
    );
    headers.insert("Access-Control-Max-Age", HeaderValue::from_static("86400"));

    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    builder.body(Body::empty()).expect("Valid CORS preflight response")
}

/// Axum handler for CORS preflight (no state dependency).
#[expect(clippy::unused_async, reason = "Required by axum Handler trait")]
pub async fn cors_preflight_ok() -> Response {
    cors_preflight_response()
}

/// Serve the root `index.html`.
pub async fn root_handler(State(_state): State<ServerState>) -> Result<Response, HttpError> {
    core_static_assets::root_handler().await
}

/// SPA-style fallback: serve a static file or `index.html`.
pub async fn static_or_spa_handler(
    State(_state): State<ServerState>,
    Path(path): Path<String>,
) -> Result<Response, HttpError> {
    core_static_assets::static_or_spa_handler(&path).await
}

/// Serve a versioned build asset from `<public_dir>/assets/`.
pub async fn serve_static_asset(
    State(state): State<ServerState>,
    Path(asset_path): Path<String>,
) -> Result<Response, HttpError> {
    core_static_assets::serve_static_asset(&state.core, &asset_path).await
}
