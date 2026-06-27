use std::{
    fmt::{self, Formatter, Result as FmtResult},
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use deno_core::ModuleId;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;

use super::runtime::RariRuntime;

mod broadcast;
mod component_helpers;
mod handle;
mod interface;
mod strategy;
#[cfg(test)]
mod tests;

pub use handle::PooledRuntime;
pub use strategy::{PickStrategy, RoundRobinStrategy};

use super::interface::JsRuntimeInterface;
use crate::server::middleware::request_context::RequestContext;

pub struct JsRuntimePool {
    runtimes: Vec<Arc<dyn JsRuntimeInterface>>,
    pick_strategy: Arc<dyn PickStrategy>,
    next_index: AtomicUsize,
    healthy: Vec<AtomicBool>,
    setup_mode: AtomicBool,
}

#[expect(
    clippy::missing_fields_in_debug,
    reason = "Debug impl intentionally omits non-trivial fields"
)]
impl fmt::Debug for JsRuntimePool {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("JsRuntimePool")
            .field("size", &self.runtimes.len())
            .field(
                "healthy_count",
                &self.healthy.iter().filter(|h| h.load(Ordering::Acquire)).count(),
            )
            .finish()
    }
}

impl JsRuntimePool {
    pub fn new(
        pool_size: usize,
        env_vars: Option<FxHashMap<String, String>>,
    ) -> Result<Arc<Self>, RariError> {
        Self::with_strategy(pool_size, env_vars, Arc::new(RoundRobinStrategy))
    }

    pub fn with_strategy(
        pool_size: usize,
        env_vars: Option<FxHashMap<String, String>>,
        strategy: Arc<dyn PickStrategy>,
    ) -> Result<Arc<Self>, RariError> {
        if pool_size == 0 {
            return Err(RariError::configuration(
                "JS runtime pool size must be at least 1".to_string(),
            ));
        }

        let runtimes: Vec<Arc<dyn JsRuntimeInterface>> = (0..pool_size)
            .map(|_| Arc::new(RariRuntime::new(env_vars.clone())) as Arc<dyn JsRuntimeInterface>)
            .collect();

        let healthy = runtimes.iter().map(|_| AtomicBool::new(true)).collect();

        Ok(Arc::new(Self {
            runtimes,
            pick_strategy: strategy,
            next_index: AtomicUsize::new(0),
            healthy,
            setup_mode: AtomicBool::new(false),
        }))
    }

    pub fn size(&self) -> usize {
        self.runtimes.len()
    }

    pub fn runtime_at(&self, idx: usize) -> Option<Arc<dyn JsRuntimeInterface>> {
        self.runtimes.get(idx).map(Arc::clone)
    }

    pub fn pick(&self) -> Option<usize> {
        let healthy_indices: Vec<usize> = self
            .healthy
            .iter()
            .enumerate()
            .filter_map(|(i, h)| if h.load(Ordering::Acquire) { Some(i) } else { None })
            .collect();

        let idx = self.pick_strategy.pick(&healthy_indices, &self.next_index)?;
        if idx >= self.runtimes.len() {
            tracing::error!(
                idx,
                pool_size = self.runtimes.len(),
                "PickStrategy returned out-of-range index"
            );
            return None;
        }
        Some(idx)
    }

    pub fn mark_unhealthy(&self, idx: usize) {
        if let Some(h) = self.healthy.get(idx) {
            h.store(false, Ordering::Release);
            tracing::warn!("Marked JS runtime pool slot {} as unhealthy", idx);
        }
    }

    pub fn mark_healthy(&self, idx: usize) {
        if let Some(h) = self.healthy.get(idx) {
            h.store(true, Ordering::Release);
            tracing::info!("Marked JS runtime pool slot {} as healthy", idx);
        }
    }

    pub fn is_healthy(&self, idx: usize) -> bool {
        self.healthy.get(idx).map(|h| h.load(Ordering::Acquire)).unwrap_or(false)
    }

    pub fn healthy_count(&self) -> usize {
        self.healthy.iter().filter(|h| h.load(Ordering::Acquire)).count()
    }

    pub fn all_healthy_indices(&self) -> Vec<usize> {
        self.healthy
            .iter()
            .enumerate()
            .filter_map(|(i, h)| if h.load(Ordering::Acquire) { Some(i) } else { None })
            .collect()
    }

    pub fn set_setup_mode(&self, on: bool) {
        self.setup_mode.store(on, Ordering::Release);
        tracing::info!("JS runtime pool setup_mode = {}", on);
    }

    pub fn is_setup_mode(&self) -> bool {
        self.setup_mode.load(Ordering::Acquire)
    }

    pub fn pick_runtime(&self) -> Result<PooledRuntime, RariError> {
        let idx = self.pick().ok_or_else(|| {
            RariError::js_runtime("No healthy JS runtime available in pool".to_string())
        })?;
        let runtime = Arc::clone(&self.runtimes[idx]);
        Ok(PooledRuntime::new(idx, runtime))
    }

    pub async fn load_and_evaluate_module(
        &self,
        specifier: &str,
    ) -> Result<(ModuleId, JsonValue), RariError> {
        let handle = self.pick_runtime()?;
        let module_id = handle.load_es_module(specifier).await?;
        let value = handle.evaluate_module(module_id).await?;
        Ok((module_id, value))
    }

    pub async fn with_request_context<F, Fut, T>(
        &self,
        ctx: Arc<RequestContext>,
        op: F,
    ) -> Result<T, RariError>
    where
        F: FnOnce(Arc<dyn JsRuntimeInterface>) -> Fut,
        Fut: Future<Output = Result<T, RariError>>,
    {
        let handle = self.pick_runtime()?;
        handle.set_request_context(Arc::clone(&ctx)).await?;
        let result = op(Arc::clone(handle.runtime())).await;
        let cleanup = handle.clear_request_context_if_matches(ctx).await;
        match (result, cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(value), Err(cleanup_err)) => {
                tracing::error!(
                    "Failed to clear request context after successful operation: {}",
                    cleanup_err
                );
                Ok(value)
            }
            (Err(op_err), Ok(())) => Err(op_err),
            (Err(op_err), Err(cleanup_err)) => {
                tracing::error!(
                    "Failed to clear request context after operation error: {}",
                    cleanup_err
                );
                Err(op_err)
            }
        }
    }
}
