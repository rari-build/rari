//! Guest framework renderers.
//!
//! rari is the host: it owns the HTTP server, routing, caching, static assets,
//! and the V8 runtime pool. A *guest* is the framework whose build produced a
//! self-contained server bundle that rari loads into that pool and invokes per
//! request. React (RSC) is the built-in renderer and does not go through this
//! module; every other [`Framework`] does.
//!
//! The host/guest contract is deliberately small so a new framework only has to
//! implement [`GuestRenderer`] and ship a bundle that speaks the streaming
//! protocol in [`stream`]:
//!
//! - the host matches the request against `dist/server/routes.json` (emitted by
//!   the framework's build via `@rari/core`) for caching and 404 fast paths,
//! - the host serves the guest's static output directories,
//! - the guest renders the page inside V8 and streams `status + headers` then
//!   body bytes back through the pool's streaming ops.

pub mod qwik;
pub mod stream;

use std::{
    future::Future,
    net::SocketAddr,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use axum::{
    body::Body,
    extract::{ConnectInfo, Path as AxumPath, State},
    http::{
        Request, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use rari_error::RariError;
use tokio::fs;

use crate::{
    async_trait::async_trait,
    runtime::{JsExecutionRuntime, factory::JsRuntimeInterface},
    server::{
        ServerState,
        config::{Config, Framework},
        host::utils::{http::get_content_type, path_validation::validate_safe_path},
    },
};

const IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// A directory of build output the host serves verbatim for a guest.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StaticMount {
    /// URL prefix, without trailing slash, e.g. `/build`.
    pub route_prefix: String,
    /// Directory on disk the prefix maps to.
    pub dir: PathBuf,
    /// Content-hashed output that may be cached forever.
    pub immutable: bool,
}

impl StaticMount {
    #[must_use]
    pub fn new(route_prefix: impl Into<String>, dir: PathBuf, immutable: bool) -> Self {
        Self { route_prefix: route_prefix.into(), dir, immutable }
    }
}

/// A framework renderer hosted by rari.
#[async_trait]
pub trait GuestRenderer: Send + Sync {
    /// Which framework this renderer serves.
    fn framework(&self) -> Framework;

    /// Static output directories the host should serve before consulting the
    /// renderer (client bundles, assets, copied `public/`).
    fn static_mounts(&self) -> Vec<StaticMount>;

    /// Re-install the guest bundle on a pool slot the runtime rebuilt after a
    /// failure, so the slot can serve requests again.
    async fn resync_slot(&self, runtime: &Arc<dyn JsRuntimeInterface>) -> Result<(), RariError>;

    /// Handle a page request end to end. Called for every method on every path
    /// that no host route or static mount claimed, so the guest can run its own
    /// form actions and RPC endpoints.
    async fn handle(
        &self,
        state: &ServerState,
        req: Request<Body>,
        client_addr: Option<SocketAddr>,
    ) -> Response;
}

/// Load the guest renderer selected by `config.framework`, if any. React
/// returns `None`: its renderer is the host's built-in RSC pipeline.
///
/// # Errors
///
/// Fails when the selected guest's bundle is missing or cannot be loaded into
/// the runtime pool.
pub async fn load(
    config: &Config,
    runtime: Arc<JsExecutionRuntime>,
    project_root: &Path,
) -> Result<Option<Arc<dyn GuestRenderer>>, RariError> {
    match config.framework {
        Framework::React => Ok(None),
        Framework::Qwik => {
            let guest = qwik::QwikGuest::load(runtime, project_root).await?;
            Ok(Some(Arc::new(guest)))
        }
    }
}

/// Serve `relative` from `dir` if it is a regular file there. `None` when it is
/// not, so the caller can fall through to the renderer.
pub async fn serve_mounted_file(
    dir: &Path,
    relative: &str,
    immutable: bool,
    default_cache_control: &str,
) -> Option<Response> {
    let file_path = validate_safe_path(dir, relative).await.ok()?;
    let metadata = fs::metadata(&file_path).await.ok()?;
    if !metadata.is_file() {
        return None;
    }
    let content = match fs::read(&file_path).await {
        Ok(content) => content,
        Err(err) => {
            tracing::error!("Failed to read static file {}: {err}", file_path.display());
            return None;
        }
    };
    let cache_control = if immutable { IMMUTABLE_CACHE_CONTROL } else { default_cache_control };
    Response::builder()
        .header(CONTENT_TYPE, get_content_type(relative))
        .header(CACHE_CONTROL, cache_control)
        .body(Body::from(content))
        .ok()
}

/// Axum handler body for a [`StaticMount`]: serves the file or 404s.
pub async fn serve_static_mount(
    dir: &Path,
    relative: &str,
    immutable: bool,
    state: &ServerState,
) -> Response {
    match serve_mounted_file(dir, relative, immutable, &state.config.caching.static_files).await {
        Some(response) => response,
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Build the axum route handler for one static mount.
pub fn static_mount_handler(
    mount: &StaticMount,
) -> impl Fn(State<ServerState>, AxumPath<String>) -> Pin<Box<dyn Future<Output = Response> + Send>>
+ Clone
+ Send
+ 'static {
    let dir = mount.dir.clone();
    let immutable = mount.immutable;
    move |State(state): State<ServerState>, AxumPath(path): AxumPath<String>| {
        let dir = dir.clone();
        Box::pin(async move { serve_static_mount(&dir, &path, immutable, &state).await })
    }
}

/// Axum handler for guest page routes: dispatches to the renderer in
/// [`ServerState::guest`].
pub async fn handle_guest_route(State(state): State<ServerState>, req: Request<Body>) -> Response {
    let Some(guest) = state.guest.clone() else {
        tracing::error!("Guest route hit but no guest renderer is loaded");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    // Present when served through `into_make_service_with_connect_info`; absent
    // in unit tests that call the router directly.
    let client_addr = req.extensions().get::<ConnectInfo<SocketAddr>>().map(|info| info.0);
    guest.handle(&state, req, client_addr).await
}
