use std::sync::Arc;

use rari_error::RariError;
use serde_json::Value;
use tokio::{
    sync::OwnedMutexGuard,
    time::{self, Duration},
};

use super::{super::interface::JsRuntimeInterface, JsRuntimePool};
use crate::server::middleware::request_context::RequestContext;

/// Holds a pool slot lease with request context installed for multi-step work.
///
/// Call [`Self::release`] when finished. If dropped without release, context cleanup
/// is best-effort via a spawned task.
pub struct LeasedRequestRuntime {
    pool: Arc<JsRuntimePool>,
    idx: usize,
    runtime: Arc<dyn JsRuntimeInterface>,
    ctx: Arc<RequestContext>,
    _lease: OwnedMutexGuard<()>,
    timeout_ms: u64,
    released: bool,
}

impl LeasedRequestRuntime {
    pub fn runtime(&self) -> &Arc<dyn JsRuntimeInterface> {
        &self.runtime
    }

    pub fn request_context(&self) -> &Arc<RequestContext> {
        &self.ctx
    }

    pub async fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Result<Value, RariError> {
        match time::timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.execute_script(script_name, script_code),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                self.pool.mark_unhealthy_if_runtime_matches(self.idx, &self.runtime);
                Err(RariError::timeout(format!(
                    "execute_script timed out after {} ms",
                    self.timeout_ms
                )))
            }
        }
    }

    pub async fn release(mut self) -> Result<(), RariError> {
        self.released = true;
        let cleanup = time::timeout(
            Duration::from_millis(self.timeout_ms),
            self.runtime.clear_request_context_if_matches(Arc::clone(&self.ctx)),
        )
        .await;
        match cleanup {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                self.pool.mark_unhealthy_if_runtime_matches(self.idx, &self.runtime);
                self.pool.mark_needs_rebuild(self.idx);
                Err(e)
            }
            Err(_) => {
                self.pool.mark_unhealthy_if_runtime_matches(self.idx, &self.runtime);
                self.pool.mark_needs_rebuild(self.idx);
                Err(RariError::timeout(format!(
                    "clear_request_context timed out after {} ms",
                    self.timeout_ms
                )))
            }
        }
    }
}

impl Drop for LeasedRequestRuntime {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        let runtime = Arc::clone(&self.runtime);
        let ctx = Arc::clone(&self.ctx);
        tokio::spawn(async move {
            let _ = runtime.clear_request_context_if_matches(ctx).await;
        });
    }
}

impl JsRuntimePool {
    pub async fn acquire_request_runtime(
        self: &Arc<Self>,
        ctx: Arc<RequestContext>,
    ) -> Result<LeasedRequestRuntime, RariError> {
        self.probe_and_heal().await;
        let handle = self.pick_runtime().await?;
        let idx = handle.idx();
        let runtime = Arc::clone(handle.runtime());
        let lease = self.acquire_owned_slot_lease_for_execute(idx, &runtime).await?;

        match time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.set_request_context(Arc::clone(&ctx)),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                self.mark_unhealthy_if_runtime_matches(idx, &runtime);
                self.mark_needs_rebuild(idx);
                return Err(RariError::timeout(format!(
                    "set_request_context timed out after {} ms",
                    self.timeout_ms
                )));
            }
        }

        Ok(LeasedRequestRuntime {
            pool: Arc::clone(self),
            idx,
            runtime,
            ctx,
            _lease: lease,
            timeout_ms: self.timeout_ms,
            released: false,
        })
    }
}
