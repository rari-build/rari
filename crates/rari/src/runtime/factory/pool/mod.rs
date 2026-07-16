#![expect(clippy::missing_errors_doc)]

use std::{
    fmt::{self, Formatter, Result as FmtResult},
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deno_core::ModuleId;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde_json::Value;
use tokio::time;

use super::runtime::RariRuntime;

mod broadcast;
mod handle;
mod interface;
mod strategy;
#[cfg(test)]
mod tests;

pub use handle::PooledRuntime;
pub use strategy::{PickStrategy, RoundRobinStrategy};

use super::interface::JsRuntimeInterface;
use crate::server::middleware::request_context::RequestContext;

/// Matches [`crate::runtime::JsExecutionRuntime`] default script timeout.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// After this many ms, `pick` may re-admit slots marked unhealthy.
pub const DEFAULT_HEAL_AFTER_MS: u64 = 30_000;

/// Disable automatic healing (tests / explicit recovery only).
pub const HEAL_DISABLED: u64 = u64::MAX;

pub struct JsRuntimePool {
    runtimes: Vec<Arc<dyn JsRuntimeInterface>>,
    pick_strategy: Arc<dyn PickStrategy>,
    next_index: AtomicUsize,
    healthy: Vec<AtomicBool>,
    /// Unix millis when the slot was marked unhealthy; `0` if healthy / unknown.
    unhealthy_since_ms: Vec<AtomicU64>,
    setup_mode: AtomicBool,
    timeout_ms: u64,
    heal_after_ms: u64,
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
            .field("timeout_ms", &self.timeout_ms)
            .field("heal_after_ms", &self.heal_after_ms)
            .finish()
    }
}

fn unix_now_ms() -> u64 {
    u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis())
        .unwrap_or(u64::MAX)
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
        Self::with_strategy_and_limits(
            pool_size,
            env_vars,
            strategy,
            DEFAULT_TIMEOUT_MS,
            DEFAULT_HEAL_AFTER_MS,
        )
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Each isolate needs an owned env map clone matching RariRuntime::new"
    )]
    pub fn with_strategy_and_limits(
        pool_size: usize,
        env_vars: Option<FxHashMap<String, String>>,
        strategy: Arc<dyn PickStrategy>,
        timeout_ms: u64,
        heal_after_ms: u64,
    ) -> Result<Arc<Self>, RariError> {
        if pool_size == 0 {
            return Err(RariError::configuration(
                "JS runtime pool size must be at least 1".to_string(),
            ));
        }
        if timeout_ms == 0 {
            return Err(RariError::configuration(
                "JS runtime pool timeout_ms must be at least 1".to_string(),
            ));
        }

        let runtimes: Vec<Arc<dyn JsRuntimeInterface>> = (0..pool_size)
            .map(|_| Arc::new(RariRuntime::new(env_vars.clone())) as Arc<dyn JsRuntimeInterface>)
            .collect();

        let healthy = runtimes.iter().map(|_| AtomicBool::new(true)).collect();
        let unhealthy_since_ms = runtimes.iter().map(|_| AtomicU64::new(0)).collect();

        Ok(Arc::new(Self {
            runtimes,
            pick_strategy: strategy,
            next_index: AtomicUsize::new(0),
            healthy,
            unhealthy_since_ms,
            setup_mode: AtomicBool::new(false),
            timeout_ms,
            heal_after_ms,
        }))
    }

    pub fn size(&self) -> usize {
        self.runtimes.len()
    }

    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    pub fn heal_after_ms(&self) -> u64 {
        self.heal_after_ms
    }

    pub fn runtime_at(&self, idx: usize) -> Option<Arc<dyn JsRuntimeInterface>> {
        self.runtimes.get(idx).map(Arc::clone)
    }

    pub fn pick(&self) -> Option<usize> {
        self.heal_expired();

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
            if let Some(since) = self.unhealthy_since_ms.get(idx) {
                since.store(unix_now_ms(), Ordering::Release);
            }
            tracing::warn!("Marked JS runtime pool slot {} as unhealthy", idx);
        }
    }

    pub fn mark_healthy(&self, idx: usize) {
        if let Some(h) = self.healthy.get(idx) {
            h.store(true, Ordering::Release);
            if let Some(since) = self.unhealthy_since_ms.get(idx) {
                since.store(0, Ordering::Release);
            }
            tracing::info!("Marked JS runtime pool slot {} as healthy", idx);
        }
    }

    /// Re-admit slots that have been unhealthy for at least `heal_after_ms`.
    /// No-op when healing is disabled (`HEAL_DISABLED`).
    fn heal_expired(&self) {
        if self.heal_after_ms == HEAL_DISABLED {
            return;
        }
        let now = unix_now_ms();
        for (idx, flag) in self.healthy.iter().enumerate() {
            if flag.load(Ordering::Acquire) {
                continue;
            }
            let since =
                self.unhealthy_since_ms.get(idx).map(|s| s.load(Ordering::Acquire)).unwrap_or(0);
            if since == 0 {
                continue;
            }
            if now.saturating_sub(since) >= self.heal_after_ms {
                self.mark_healthy(idx);
            }
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

    pub fn pick_runtime(&self) -> Result<PooledRuntime, RariError> {
        let idx = self.pick().ok_or_else(|| {
            RariError::js_runtime("No healthy JS runtime available in pool".to_string())
        })?;
        let runtime = Arc::clone(&self.runtimes[idx]);
        Ok(PooledRuntime::new(idx, runtime, self.timeout_ms))
    }

    /// Load+evaluate on a single picked isolate.
    ///
    /// `ModuleId` values are isolate-local. For bootstrap / HMR that must reach every
    /// slot, use [`Self::broadcast_load_and_evaluate_module`] instead.
    pub async fn load_and_evaluate_on_picked(
        &self,
        specifier: &str,
    ) -> Result<(ModuleId, Value), RariError> {
        let handle = self.pick_runtime()?;
        let runtime = Arc::clone(handle.runtime());
        let specifier = specifier.to_string();
        self.run_with_timeout("load_and_evaluate_on_picked", async move {
            let module_id = runtime.load_es_module(&specifier).await?;
            let value = runtime.evaluate_module(module_id).await?;
            Ok((module_id, value))
        })
        .await
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
        let runtime = Arc::clone(handle.runtime());
        self.run_with_timeout("with_request_context", async {
            runtime.set_request_context(Arc::clone(&ctx)).await?;
            let result = op(Arc::clone(&runtime)).await;
            let cleanup = runtime.clear_request_context_if_matches(ctx).await;
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
        })
        .await
    }

    pub(super) async fn run_with_timeout<T, Fut>(
        &self,
        label: &str,
        fut: Fut,
    ) -> Result<T, RariError>
    where
        Fut: Future<Output = Result<T, RariError>>,
    {
        match time::timeout(Duration::from_millis(self.timeout_ms), fut).await {
            Ok(result) => result,
            Err(_) => {
                Err(RariError::timeout(format!("{label} timed out after {} ms", self.timeout_ms)))
            }
        }
    }
}
