#![expect(clippy::missing_errors_doc)]

use std::{
    fmt::{self, Formatter, Result as FmtResult},
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deno_core::ModuleId;
use futures::future::join_all;
use parking_lot::RwLock;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde_json::Value;
use tokio::{
    sync::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard, OwnedMutexGuard},
    time,
};

use super::runtime::RariRuntime;

mod broadcast;
mod handle;
mod interface;
mod lease;
mod strategy;
#[cfg(test)]
mod tests;

pub use handle::PooledRuntime;
pub use lease::LeasedRequestRuntime;
pub use strategy::{PickStrategy, RoundRobinStrategy};

use super::interface::JsRuntimeInterface;
use crate::server::middleware::request_context::RequestContext;

/// Matches [`crate::runtime::JsExecutionRuntime`] default script timeout.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// After this many ms, `pick` may re-admit slots marked unhealthy.
pub const DEFAULT_HEAL_AFTER_MS: u64 = 30_000;

/// Disable automatic healing (tests / explicit recovery only).
pub const HEAL_DISABLED: u64 = u64::MAX;

/// Creates a fresh isolate when heal rebuilds a quarantined slot.
pub type RuntimeFactory = Arc<dyn Fn() -> Arc<dyn JsRuntimeInterface> + Send + Sync>;

/// Decrements a slot's soft stream load when dropped.
pub struct StreamingSlotGuard {
    load: Arc<AtomicUsize>,
}

impl Drop for StreamingSlotGuard {
    fn drop(&mut self) {
        self.load.fetch_sub(1, Ordering::Release);
    }
}

/// Called after a slot isolate is rebuilt and passes the basic probe.
pub type PostRebuildHook = Arc<
    dyn Fn(
            usize,
            Arc<dyn JsRuntimeInterface>,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>
        + Send
        + Sync,
>;

pub struct JsRuntimePool {
    runtimes: Vec<RwLock<Arc<dyn JsRuntimeInterface>>>,
    runtime_factory: RuntimeFactory,
    pick_strategy: Arc<dyn PickStrategy>,
    next_index: AtomicUsize,
    healthy: Vec<AtomicBool>,
    /// Unix millis when the slot was marked unhealthy; `0` if healthy / unknown.
    unhealthy_since_ms: Vec<AtomicU64>,
    /// True after an immediate zero-healthy heal attempt until the slot recovers
    /// or is freshly marked unhealthy. Prevents heal storms while quarantine runs.
    heal_attempted: Vec<AtomicBool>,
    /// Per-slot lease so `with_request_context` cannot interleave on one runtime.
    slot_leases: Vec<Arc<AsyncMutex<()>>>,
    /// Soft in-flight stream counts for least-busy streaming admission.
    stream_load: Vec<Arc<AtomicUsize>>,
    /// Set when request-context cleanup fails or times out; blocks probe-only heal.
    needs_rebuild: Vec<AtomicBool>,
    setup_mode: AtomicBool,
    timeout_ms: u64,
    heal_after_ms: u64,
    /// Optional app-level resync after isolate rebuild (pipelines + components).
    post_rebuild_hook: parking_lot::RwLock<Option<PostRebuildHook>>,
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

        let runtime_factory: RuntimeFactory = Arc::new(move || {
            Arc::new(RariRuntime::new(env_vars.clone())) as Arc<dyn JsRuntimeInterface>
        });

        let runtimes: Vec<RwLock<Arc<dyn JsRuntimeInterface>>> =
            (0..pool_size).map(|_| RwLock::new(runtime_factory())).collect();

        let healthy = runtimes.iter().map(|_| AtomicBool::new(true)).collect();
        let unhealthy_since_ms = runtimes.iter().map(|_| AtomicU64::new(0)).collect();
        let heal_attempted = runtimes.iter().map(|_| AtomicBool::new(false)).collect();
        let slot_leases = (0..pool_size).map(|_| Arc::new(AsyncMutex::new(()))).collect();
        let stream_load = (0..pool_size).map(|_| Arc::new(AtomicUsize::new(0))).collect();
        let needs_rebuild = (0..pool_size).map(|_| AtomicBool::new(false)).collect();

        Ok(Arc::new(Self {
            runtimes,
            runtime_factory,
            pick_strategy: strategy,
            next_index: AtomicUsize::new(0),
            healthy,
            unhealthy_since_ms,
            heal_attempted,
            slot_leases,
            stream_load,
            needs_rebuild,
            setup_mode: AtomicBool::new(false),
            timeout_ms,
            heal_after_ms,
            post_rebuild_hook: parking_lot::RwLock::new(None),
        }))
    }

    pub fn set_post_rebuild_hook(&self, hook: PostRebuildHook) {
        *self.post_rebuild_hook.write() = Some(hook);
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
        self.runtimes.get(idx).map(|slot| Arc::clone(&slot.read()))
    }

    fn replace_runtime_at(&self, idx: usize, runtime: Arc<dyn JsRuntimeInterface>) -> bool {
        let Some(slot) = self.runtimes.get(idx) else {
            return false;
        };
        *slot.write() = runtime;
        true
    }

    fn refresh_unhealthy_timestamp(&self, idx: usize) {
        if let Some(since) = self.unhealthy_since_ms.get(idx) {
            since.store(unix_now_ms(), Ordering::Release);
        }
    }

    fn mark_needs_rebuild(&self, idx: usize) {
        if let Some(flag) = self.needs_rebuild.get(idx) {
            flag.store(true, Ordering::Release);
        }
    }

    fn clear_needs_rebuild(&self, idx: usize) {
        if let Some(flag) = self.needs_rebuild.get(idx) {
            flag.store(false, Ordering::Release);
        }
    }

    fn needs_rebuild(&self, idx: usize) -> bool {
        self.needs_rebuild.get(idx).is_some_and(|f| f.load(Ordering::Acquire))
    }

    fn runtime_still_installed(&self, idx: usize, expected: &Arc<dyn JsRuntimeInterface>) -> bool {
        self.runtime_at(idx).is_some_and(|current| Arc::ptr_eq(&current, expected))
    }

    fn heal_eligible(&self, idx: usize) -> bool {
        if self.heal_after_ms == HEAL_DISABLED {
            return false;
        }
        if self.is_healthy(idx) {
            return false;
        }
        let since =
            self.unhealthy_since_ms.get(idx).map(|s| s.load(Ordering::Acquire)).unwrap_or(0);
        if since == 0 {
            return false;
        }
        let now = unix_now_ms();
        let quarantine_elapsed = now.saturating_sub(since) >= self.heal_after_ms;
        if quarantine_elapsed {
            return true;
        }
        // Never leave the pool with zero healthy slots waiting on the heal delay —
        // allow one immediate attempt per outage, then enforce quarantine.
        if self.healthy_count() == 0 {
            let attempted = self.heal_attempted.get(idx).is_some_and(|f| f.load(Ordering::Acquire));
            return !attempted;
        }
        false
    }

    fn slot_admissible_for_execute(
        &self,
        idx: usize,
        expected: &Arc<dyn JsRuntimeInterface>,
    ) -> bool {
        self.is_healthy(idx)
            && !self.needs_rebuild(idx)
            && self.runtime_still_installed(idx, expected)
    }

    pub(super) async fn acquire_slot_lease(
        &self,
        idx: usize,
    ) -> Result<AsyncMutexGuard<'_, ()>, RariError> {
        let Some(lease) = self.slot_leases.get(idx) else {
            return Err(RariError::js_runtime(format!(
                "No lease available for JS runtime pool slot {idx}"
            )));
        };
        Ok(lease.lock().await)
    }

    /// Acquire a slot lease for heal work. Returns `None` when the slot is no longer
    /// heal-eligible after waiting (quarantine refreshed or already healed elsewhere).
    pub(super) async fn acquire_slot_lease_for_heal(
        &self,
        idx: usize,
    ) -> Result<Option<AsyncMutexGuard<'_, ()>>, RariError> {
        let guard = self.acquire_slot_lease(idx).await?;
        if !self.heal_eligible(idx) {
            return Ok(None);
        }
        Ok(Some(guard))
    }

    /// Lock the slot, then verify it is still admissible for the expected runtime.
    pub(super) async fn acquire_slot_lease_for_execute(
        &self,
        idx: usize,
        expected: &Arc<dyn JsRuntimeInterface>,
    ) -> Result<AsyncMutexGuard<'_, ()>, RariError> {
        let guard = self.acquire_slot_lease(idx).await?;
        if !self.slot_admissible_for_execute(idx, expected) {
            return Err(RariError::js_runtime(format!(
                "JS runtime pool slot {idx} is no longer admissible for execution"
            )));
        }
        Ok(guard)
    }

    /// Owned lease for work that must outlive `&self` (e.g. spawned batch forwarding).
    pub(super) async fn acquire_owned_slot_lease_for_execute(
        &self,
        idx: usize,
        expected: &Arc<dyn JsRuntimeInterface>,
    ) -> Result<OwnedMutexGuard<()>, RariError> {
        let Some(lease) = self.slot_leases.get(idx).cloned() else {
            return Err(RariError::js_runtime(format!(
                "No lease available for JS runtime pool slot {idx}"
            )));
        };
        let guard = lease.lock_owned().await;
        if !self.slot_admissible_for_execute(idx, expected) {
            return Err(RariError::js_runtime(format!(
                "JS runtime pool slot {idx} is no longer admissible for execution"
            )));
        }
        Ok(guard)
    }

    pub(super) fn mark_unhealthy_if_runtime_matches(
        &self,
        idx: usize,
        expected: &Arc<dyn JsRuntimeInterface>,
    ) {
        if self.runtime_still_installed(idx, expected) {
            self.mark_unhealthy(idx);
        }
    }

    /// Try each healthy slot until one admits the lease, then run `op` under timeout.
    /// Propagates execution errors immediately; returns pool-unavailable only when every
    /// candidate is missing or fails admission.
    pub(super) async fn execute_on_admitted_healthy_slot<T, F, Fut>(
        &self,
        op_label: &str,
        mut op: F,
    ) -> Result<T, RariError>
    where
        T: Send,
        F: FnMut(Arc<dyn JsRuntimeInterface>) -> Fut,
        Fut: Future<Output = Result<T, RariError>> + Send,
    {
        let first = self.pick().ok_or_else(|| {
            RariError::js_runtime("No healthy JS runtime available in pool".to_string())
        })?;
        let mut candidates: Vec<usize> = self.all_healthy_indices();
        candidates.retain(|&i| i != first);
        candidates.insert(0, first);

        let timeout_ms = self.timeout_ms;
        for idx in candidates {
            let Some(runtime) = self.runtime_at(idx) else {
                continue;
            };
            let Ok(_lease) = self.acquire_slot_lease_for_execute(idx, &runtime).await else {
                continue;
            };
            match time::timeout(Duration::from_millis(timeout_ms), op(Arc::clone(&runtime))).await {
                Ok(result) => return result,
                Err(_) => {
                    self.mark_unhealthy_if_runtime_matches(idx, &runtime);
                    return Err(RariError::timeout(format!(
                        "{op_label} timed out after {timeout_ms} ms"
                    )));
                }
            }
        }
        Err(RariError::js_runtime("No healthy JS runtime available in pool".to_string()))
    }

    /// Replace the isolate in `idx`. Caller must hold the slot lease.
    ///
    /// In-flight sticky handles keep the old `Arc` until dropped; new picks see the replacement.
    fn rebuild_slot_held(&self, idx: usize) -> Result<Arc<dyn JsRuntimeInterface>, RariError> {
        if idx >= self.runtimes.len() {
            return Err(RariError::js_runtime(format!(
                "Cannot rebuild JS runtime pool slot {idx}: out of range"
            )));
        }
        let replacement = (self.runtime_factory)();
        let installed = Arc::clone(&replacement);
        if !self.replace_runtime_at(idx, replacement) {
            return Err(RariError::js_runtime(format!(
                "Cannot rebuild JS runtime pool slot {idx}: slot missing"
            )));
        }
        tracing::info!(idx, "Rebuilt JS runtime pool slot");
        Ok(installed)
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
        if !healthy_indices.contains(&idx) {
            tracing::error!(
                idx,
                ?healthy_indices,
                "PickStrategy returned index not in healthy snapshot"
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
            if let Some(attempted) = self.heal_attempted.get(idx) {
                attempted.store(false, Ordering::Release);
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
            if let Some(attempted) = self.heal_attempted.get(idx) {
                attempted.store(false, Ordering::Release);
            }
            tracing::info!("Marked JS runtime pool slot {} as healthy", idx);
        }
    }

    fn expired_unhealthy_indices(&self) -> Vec<usize> {
        (0..self.runtimes.len()).filter(|&idx| self.heal_eligible(idx)).collect()
    }

    /// Probe expired-unhealthy slots with a lightweight script before re-admission.
    /// Successful probes call [`Self::mark_healthy`]. Failures rebuild the isolate and
    /// probe again; if that also fails the quarantine timestamp is refreshed.
    ///
    /// Each slot heals under its lease so concurrent callers cannot probe/rebuild the
    /// same index at once, and request-context work cannot interleave mid-heal.
    pub async fn probe_and_heal(&self) {
        let indices = self.expired_unhealthy_indices();
        if indices.is_empty() {
            return;
        }
        let probe_timeout_ms = self.timeout_ms.clamp(1, 1_000);

        let heal_futs = indices.into_iter().map(|idx| async move {
            let Ok(guard) = self.acquire_slot_lease_for_heal(idx).await else {
                return;
            };
            let Some(_lease) = guard else {
                return;
            };
            if self.is_healthy(idx) {
                return;
            }

            if let Some(attempted) = self.heal_attempted.get(idx) {
                attempted.store(true, Ordering::Release);
            }

            let force_rebuild = self.needs_rebuild(idx);
            if !force_rebuild {
                let Some(runtime) = self.runtime_at(idx) else {
                    return;
                };
                if self.probe_runtime(idx, &runtime, probe_timeout_ms).await
                    && self.runtime_still_installed(idx, &runtime)
                    && !self.needs_rebuild(idx)
                {
                    self.mark_healthy(idx);
                    return;
                }
            }

            tracing::warn!(idx, "Heal probe failed; rebuilding JS runtime pool slot");
            let replacement = match self.rebuild_slot_held(idx) {
                Ok(runtime) => runtime,
                Err(e) => {
                    tracing::error!(idx, error = %e, "Failed to rebuild JS runtime pool slot");
                    self.refresh_unhealthy_timestamp(idx);
                    return;
                }
            };
            if self.probe_runtime(idx, &replacement, probe_timeout_ms).await
                && self.runtime_still_installed(idx, &replacement)
            {
                let hook = self.post_rebuild_hook.read().clone();
                let resync_ok = if let Some(hook) = hook {
                    match time::timeout(
                        Duration::from_millis(self.timeout_ms),
                        hook(idx, Arc::clone(&replacement)),
                    )
                    .await
                    {
                        Ok(Ok(())) => true,
                        Ok(Err(e)) => {
                            tracing::error!(
                                idx,
                                error = %e,
                                "Post-rebuild resync failed; keeping slot unhealthy"
                            );
                            false
                        }
                        Err(_) => {
                            tracing::error!(
                                idx,
                                timeout_ms = self.timeout_ms,
                                "Post-rebuild resync timed out; keeping slot unhealthy"
                            );
                            false
                        }
                    }
                } else if self.runtimes.len() > 1 {
                    tracing::error!(
                        idx,
                        "Rebuilt JS runtime pool slot left unhealthy: post-rebuild resync required for pool_size > 1"
                    );
                    false
                } else {
                    true
                };

                if resync_ok {
                    self.clear_needs_rebuild(idx);
                    self.mark_healthy(idx);
                } else {
                    self.refresh_unhealthy_timestamp(idx);
                }
            } else {
                tracing::warn!(
                    idx,
                    "Heal probe still failing after rebuild; keeping slot unhealthy"
                );
                self.refresh_unhealthy_timestamp(idx);
            }
        });
        join_all(heal_futs).await;
    }

    async fn probe_runtime(
        &self,
        idx: usize,
        runtime: &Arc<dyn JsRuntimeInterface>,
        probe_timeout_ms: u64,
    ) -> bool {
        let probe = time::timeout(
            Duration::from_millis(probe_timeout_ms),
            runtime.execute_script("__pool_heal_probe__".into(), "1".into()),
        )
        .await;
        match probe {
            Ok(Ok(_)) => true,
            Ok(Err(e)) => {
                tracing::warn!(idx, error = %e, "Heal probe script failed");
                false
            }
            Err(_) => {
                tracing::warn!(idx, "Heal probe timed out");
                false
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

    pub async fn pick_runtime(&self) -> Result<PooledRuntime, RariError> {
        self.probe_and_heal().await;
        let idx = self.pick().ok_or_else(|| {
            RariError::js_runtime("No healthy JS runtime available in pool".to_string())
        })?;
        let runtime = self.runtime_at(idx).ok_or_else(|| {
            RariError::js_runtime("No healthy JS runtime available in pool".to_string())
        })?;
        Ok(PooledRuntime::new(idx, runtime, self.timeout_ms))
    }

    /// Prefer healthy slots with the fewest in-flight streams (soft lease).
    pub fn pick_least_busy(&self) -> Option<usize> {
        let healthy_indices: Vec<usize> = self
            .healthy
            .iter()
            .enumerate()
            .filter_map(|(i, h)| if h.load(Ordering::Acquire) { Some(i) } else { None })
            .collect();
        if healthy_indices.is_empty() {
            return None;
        }

        let min_load = healthy_indices
            .iter()
            .filter_map(|&i| self.stream_load.get(i).map(|c| c.load(Ordering::Acquire)))
            .min()?;
        let candidates: Vec<usize> = healthy_indices
            .into_iter()
            .filter(|&i| {
                self.stream_load.get(i).is_some_and(|c| c.load(Ordering::Acquire) == min_load)
            })
            .collect();
        self.pick_strategy
            .pick(&candidates, &self.next_index)
            .filter(|&idx| idx < self.runtimes.len() && self.is_healthy(idx))
    }

    /// Sticky streaming pick that increments soft load until the guard drops.
    pub async fn pick_runtime_for_streaming(
        &self,
    ) -> Result<(PooledRuntime, StreamingSlotGuard), RariError> {
        self.probe_and_heal().await;
        let idx = self.pick_least_busy().ok_or_else(|| {
            RariError::js_runtime("No healthy JS runtime available in pool".to_string())
        })?;
        let runtime = self.runtime_at(idx).ok_or_else(|| {
            RariError::js_runtime("No healthy JS runtime available in pool".to_string())
        })?;
        let load = self.stream_load.get(idx).cloned().ok_or_else(|| {
            RariError::js_runtime("No healthy JS runtime available in pool".to_string())
        })?;
        load.fetch_add(1, Ordering::AcqRel);
        Ok((PooledRuntime::new(idx, runtime, self.timeout_ms), StreamingSlotGuard { load }))
    }

    pub fn stream_load_at(&self, idx: usize) -> usize {
        self.stream_load.get(idx).map(|c| c.load(Ordering::Acquire)).unwrap_or(0)
    }

    /// Load+evaluate on a single picked isolate.
    ///
    /// `ModuleId` values are isolate-local. For bootstrap / HMR that must reach every
    /// slot, use [`Self::broadcast_load_and_evaluate_module`] instead.
    pub async fn load_and_evaluate_on_picked(
        &self,
        specifier: &str,
    ) -> Result<(ModuleId, Value), RariError> {
        self.probe_and_heal().await;
        let specifier = specifier.to_string();
        self.execute_on_admitted_healthy_slot("load_and_evaluate_on_picked", move |runtime| {
            let specifier = specifier.clone();
            async move {
                let module_id = runtime.load_es_module(&specifier).await?;
                let value = runtime.evaluate_module(module_id).await?;
                Ok((module_id, value))
            }
        })
        .await
    }

    pub async fn with_request_context<F, Fut, T>(
        self: &Arc<Self>,
        ctx: Arc<RequestContext>,
        op: F,
    ) -> Result<T, RariError>
    where
        T: Send + 'static,
        F: FnOnce(Arc<dyn JsRuntimeInterface>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, RariError>> + Send + 'static,
    {
        let handle = self.pick_runtime().await?;
        let idx = handle.idx();
        let runtime = Arc::clone(handle.runtime());
        let pool = Arc::clone(self);
        let timeout_ms = self.timeout_ms;

        tokio::spawn(async move {
            let _lease_guard = pool.acquire_slot_lease_for_execute(idx, &runtime).await?;

            match time::timeout(
                Duration::from_millis(timeout_ms),
                runtime.set_request_context(Arc::clone(&ctx)),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    pool.mark_unhealthy_if_runtime_matches(idx, &runtime);
                    pool.mark_needs_rebuild(idx);
                    return Err(RariError::timeout(format!(
                        "set_request_context timed out after {timeout_ms} ms"
                    )));
                }
            }

            let Ok(op_result) =
                time::timeout(Duration::from_millis(timeout_ms), op(Arc::clone(&runtime))).await
            else {
                // A hung operation is not proof the isolate is dead. Only quarantine when
                // cleanup fails or times out (isolate unresponsive).
                let cleanup = time::timeout(
                    Duration::from_millis(timeout_ms),
                    runtime.clear_request_context_if_matches(Arc::clone(&ctx)),
                )
                .await;
                match cleanup {
                    Ok(Ok(())) => {}
                    Ok(Err(cleanup_err)) => {
                        tracing::error!(
                            "Failed to clear request context after timeout: {}",
                            cleanup_err
                        );
                        pool.mark_unhealthy_if_runtime_matches(idx, &runtime);
                        pool.mark_needs_rebuild(idx);
                    }
                    Err(_) => {
                        tracing::error!(
                            "Clearing request context timed out after {timeout_ms} ms; slot needs rebuild"
                        );
                        pool.mark_unhealthy_if_runtime_matches(idx, &runtime);
                        pool.mark_needs_rebuild(idx);
                    }
                }
                return Err(RariError::timeout(format!(
                    "with_request_context timed out after {timeout_ms} ms"
                )));
            };

            let cleanup = time::timeout(
                Duration::from_millis(timeout_ms),
                runtime.clear_request_context_if_matches(ctx),
            )
            .await;
            match (op_result, cleanup) {
                (Ok(value), Ok(Ok(()))) => Ok(value),
                (Ok(_value), Ok(Err(cleanup_err))) => {
                    pool.mark_unhealthy_if_runtime_matches(idx, &runtime);
                    pool.mark_needs_rebuild(idx);
                    tracing::error!(
                        "Failed to clear request context after successful operation: {}",
                        cleanup_err
                    );
                    Err(cleanup_err)
                }
                (Ok(_value), Err(_)) => {
                    pool.mark_unhealthy_if_runtime_matches(idx, &runtime);
                    pool.mark_needs_rebuild(idx);
                    Err(RariError::timeout(format!(
                        "clear_request_context timed out after {timeout_ms} ms"
                    )))
                }
                (Err(op_err), Ok(Ok(()))) => Err(op_err),
                (Err(op_err), Ok(Err(cleanup_err))) => {
                    pool.mark_unhealthy_if_runtime_matches(idx, &runtime);
                    pool.mark_needs_rebuild(idx);
                    tracing::error!(
                        "Failed to clear request context after operation error: {}",
                        cleanup_err
                    );
                    Err(op_err)
                }
                (Err(op_err), Err(_)) => {
                    pool.mark_unhealthy_if_runtime_matches(idx, &runtime);
                    pool.mark_needs_rebuild(idx);
                    tracing::error!(
                        "Clearing request context timed out after {timeout_ms} ms"
                    );
                    Err(op_err)
                }
            }
        })
        .await
        .map_err(|e| RariError::js_runtime(format!("with_request_context task join failed: {e}")))?
    }
}
