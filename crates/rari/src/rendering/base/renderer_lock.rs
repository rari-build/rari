//! Cancel-correct exclusive access to the shared [`RscRenderer`].
//!
//! Holding a `tokio::sync::Mutex` across `.await` is cancel-unsafe: if the
//! caller is cancelled mid-await, the guard drops and shared state can be left
//! inconsistent. Drive critical sections on a spawned task so they run to
//! completion even when the request future is cancelled.

use std::{future::Future, sync::Arc};

use rari_error::RariError;
use tokio::sync::{Mutex, OwnedMutexGuard};

use crate::RscRenderer;

/// Run `f` with exclusive access to the renderer on a spawned task.
///
/// The spawned task is not cancelled when the caller is, so mutex critical
/// sections that `.await` still complete and leave shared state consistent.
///
/// # Errors
///
/// Returns [`RariError`] if the spawned task panics or fails to join.
pub async fn run_with_renderer<T, F, Fut>(
    renderer: Arc<Mutex<RscRenderer>>,
    f: F,
) -> Result<T, RariError>
where
    T: Send + 'static,
    F: FnOnce(OwnedMutexGuard<RscRenderer>) -> Fut + Send + 'static,
    Fut: Future<Output = T> + Send + 'static,
{
    tokio::spawn(async move {
        let guard = renderer.lock_owned().await;
        f(guard).await
    })
    .await
    .map_err(|e| RariError::js_runtime(format!("renderer task join failed: {e}")))
}

/// Like [`run_with_renderer`], but flattens an inner `Result`.
///
/// # Errors
///
/// Returns [`RariError`] if the spawned task fails to join, or if `f` returns
/// an error.
pub async fn run_with_renderer_result<T, F, Fut>(
    renderer: Arc<Mutex<RscRenderer>>,
    f: F,
) -> Result<T, RariError>
where
    T: Send + 'static,
    F: FnOnce(OwnedMutexGuard<RscRenderer>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, RariError>> + Send + 'static,
{
    run_with_renderer(renderer, f).await?
}
