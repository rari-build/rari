use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use rari_error::RariError;
use serde_json::{Value, json};
use tokio::{
    sync::{Mutex as AsyncMutex, Notify, mpsc},
    task::yield_now,
    time::sleep,
};

use super::*;
use crate::{
    runtime::factory::interface::QueueStreamingScriptFuture,
    server::middleware::request_context::RequestContext,
};

struct CountingRuntime {
    calls: Arc<AtomicUsize>,
    hang_script: Arc<AtomicBool>,
    hang_load: Arc<AtomicBool>,
    fail_script: Arc<AtomicBool>,
}

impl CountingRuntime {
    fn new(calls: Arc<AtomicUsize>) -> Self {
        Self {
            calls,
            hang_script: Arc::new(AtomicBool::new(false)),
            hang_load: Arc::new(AtomicBool::new(false)),
            fail_script: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl JsRuntimeInterface for CountingRuntime {
    fn execute_script(
        &self,
        _script_name: String,
        _script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        let calls = Arc::clone(&self.calls);
        let hang = self.hang_script.load(Ordering::SeqCst);
        let fail = self.fail_script.load(Ordering::SeqCst);
        Box::pin(async move {
            if hang {
                sleep(Duration::from_secs(60)).await;
            }
            calls.fetch_add(1, Ordering::SeqCst);
            if fail {
                return Err(RariError::js_runtime("script failed".to_string()));
            }
            Ok(json!({"ok": true}))
        })
    }

    fn execute_script_batch(
        &self,
        _scripts: Vec<(String, String)>,
    ) -> Pin<
        Box<dyn Future<Output = mpsc::UnboundedReceiver<(usize, Result<Value, RariError>)>> + Send>,
    > {
        let (_tx, rx) = mpsc::unbounded_channel();
        Box::pin(async move { rx })
    }

    fn execute_function(
        &self,
        _function_name: &str,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send + 'static>> {
        Box::pin(async move { Ok(json!(null)) })
    }

    fn add_module_to_loader(
        &self,
        _specifier: &str,
        _code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn load_es_module(
        &self,
        _specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>> {
        let hang = self.hang_load.load(Ordering::SeqCst);
        Box::pin(async move {
            if hang {
                sleep(Duration::from_secs(60)).await;
            }
            Ok(0)
        })
    }

    fn evaluate_module(
        &self,
        _module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        Box::pin(async move { Ok(json!(null)) })
    }

    fn get_module_namespace(
        &self,
        _module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        Box::pin(async move { Ok(json!(null)) })
    }

    fn clear_module_loader_caches(
        &self,
        _component_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn set_request_context(
        &self,
        _request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn clear_request_context_if_matches(
        &self,
        _expected_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn execute_script_for_streaming(
        &self,
        _stream_id: String,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn queue_script_for_streaming(
        &self,
        _stream_id: String,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
        _request_context: Option<Arc<RequestContext>>,
    ) -> QueueStreamingScriptFuture {
        Box::pin(async move {
            let completion: Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> =
                Box::pin(async move { Ok(()) });
            Ok(completion)
        })
    }

    fn register_request_context(
        &self,
        _request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn unregister_request_context(
        &self,
        _request_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }
}

fn wrap_runtimes(
    runtimes: Vec<Arc<dyn JsRuntimeInterface>>,
) -> Vec<RwLock<Arc<dyn JsRuntimeInterface>>> {
    runtimes.into_iter().map(RwLock::new).collect()
}

fn counting_runtime_factory() -> RuntimeFactory {
    Arc::new(|| {
        Arc::new(CountingRuntime::new(Arc::new(AtomicUsize::new(0)))) as Arc<dyn JsRuntimeInterface>
    })
}

fn pool_from_runtimes(
    runtimes: Vec<Arc<dyn JsRuntimeInterface>>,
    timeout_ms: u64,
    heal_after_ms: u64,
) -> Arc<JsRuntimePool> {
    let size = runtimes.len();
    Arc::new(JsRuntimePool {
        runtimes: wrap_runtimes(runtimes),
        runtime_factory: counting_runtime_factory(),
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy: (0..size).map(|_| AtomicBool::new(true)).collect(),
        unhealthy_since_ms: (0..size).map(|_| AtomicU64::new(0)).collect(),
        slot_leases: (0..size).map(|_| Arc::new(AsyncMutex::new(()))).collect(),
        stream_load: (0..size).map(|_| Arc::new(AtomicUsize::new(0))).collect(),
        needs_rebuild: (0..size).map(|_| AtomicBool::new(false)).collect(),
        setup_mode: AtomicBool::new(false),
        timeout_ms,
        heal_after_ms,
        post_rebuild_hook: parking_lot::RwLock::new(None),
    })
}

fn build_pool_with_counters(size: usize) -> (Arc<JsRuntimePool>, Vec<Arc<AtomicUsize>>) {
    let mut runtimes: Vec<Arc<dyn JsRuntimeInterface>> = Vec::with_capacity(size);
    let mut counters: Vec<Arc<AtomicUsize>> = Vec::with_capacity(size);

    for _ in 0..size {
        let counter = Arc::new(AtomicUsize::new(0));
        counters.push(Arc::clone(&counter));
        let runtime = CountingRuntime::new(Arc::clone(&counter));
        runtimes.push(Arc::new(runtime));
    }

    let pool = pool_from_runtimes(runtimes, DEFAULT_TIMEOUT_MS, HEAL_DISABLED);
    (pool, counters)
}

#[tokio::test]
async fn round_robin_distributes_across_runtimes() -> Result<(), RariError> {
    let (pool, counters) = build_pool_with_counters(3);

    for _ in 0..9 {
        pool.execute_script("test".into(), "1+1".into()).await?;
    }

    assert_eq!(counters[0].load(Ordering::SeqCst), 3);
    assert_eq!(counters[1].load(Ordering::SeqCst), 3);
    assert_eq!(counters[2].load(Ordering::SeqCst), 3);
    Ok(())
}

#[tokio::test]
async fn pool_size_zero_is_rejected() {
    let result = JsRuntimePool::new(0, None);
    assert!(result.is_err());
}

#[tokio::test]
async fn pool_size_one_picks_always_index_zero() -> Result<(), RariError> {
    let (pool, counters) = build_pool_with_counters(1);

    for _ in 0..5 {
        pool.execute_script("test".into(), "1+1".into()).await?;
    }

    assert_eq!(counters[0].load(Ordering::SeqCst), 5);
    Ok(())
}

#[tokio::test]
async fn mark_unhealthy_changes_round_robin_distribution() -> Result<(), RariError> {
    let (pool, counters) = build_pool_with_counters(3);

    pool.mark_unhealthy(0);
    pool.mark_unhealthy(2);

    for _ in 0..6 {
        pool.execute_script("test".into(), "1+1".into()).await?;
    }

    assert_eq!(counters[0].load(Ordering::SeqCst), 0);
    assert_eq!(counters[2].load(Ordering::SeqCst), 0);
    assert!(counters[1].load(Ordering::SeqCst) >= 1);
    Ok(())
}

#[tokio::test]
async fn mark_healthy_restores_round_robin() -> Result<(), RariError> {
    let (pool, counters) = build_pool_with_counters(3);

    pool.mark_unhealthy(1);

    for _ in 0..3 {
        pool.execute_script("test".into(), "1+1".into()).await?;
    }
    let before = counters[1].load(Ordering::SeqCst);
    assert_eq!(before, 0);

    pool.mark_healthy(1);

    for _ in 0..3 {
        pool.execute_script("test".into(), "1+1".into()).await?;
    }

    assert!(counters[1].load(Ordering::SeqCst) > before);
    Ok(())
}

#[tokio::test]
async fn probe_and_heal_re_admits_after_successful_probe() -> Result<(), RariError> {
    let (_unused, counters) = build_pool_with_counters(2);
    let runtimes = (0..2)
        .map(|i| {
            Arc::new(CountingRuntime::new(Arc::clone(&counters[i]))) as Arc<dyn JsRuntimeInterface>
        })
        .collect::<Vec<_>>();
    let pool = pool_from_runtimes(runtimes, DEFAULT_TIMEOUT_MS, 1);

    pool.mark_unhealthy(0);
    assert!(!pool.is_healthy(0));

    sleep(Duration::from_millis(5)).await;
    let _ = pool.pick();
    assert!(!pool.is_healthy(0), "sync pick must not re-admit without a probe");

    pool.probe_and_heal().await;
    assert!(pool.is_healthy(0), "successful heal probe should re-admit the slot");
    Ok(())
}

#[tokio::test]
#[expect(clippy::expect_used)]
async fn probe_and_heal_rebuilds_and_re_admits_when_probe_fails() -> Result<(), RariError> {
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime::new(Arc::clone(&calls));
    runtime.fail_script.store(true, Ordering::SeqCst);
    let original: Arc<dyn JsRuntimeInterface> = Arc::new(runtime);

    let rebuilds = Arc::new(AtomicUsize::new(0));
    let rebuilds_for_factory = Arc::clone(&rebuilds);
    let pool = Arc::new(JsRuntimePool {
        runtimes: wrap_runtimes(vec![Arc::clone(&original)]),
        runtime_factory: Arc::new(move || {
            rebuilds_for_factory.fetch_add(1, Ordering::SeqCst);
            Arc::new(CountingRuntime::new(Arc::new(AtomicUsize::new(0))))
                as Arc<dyn JsRuntimeInterface>
        }),
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy: vec![AtomicBool::new(true)],
        unhealthy_since_ms: vec![AtomicU64::new(0)],
        slot_leases: vec![Arc::new(AsyncMutex::new(()))],
        stream_load: vec![Arc::new(AtomicUsize::new(0))],
        needs_rebuild: vec![AtomicBool::new(false)],
        setup_mode: AtomicBool::new(false),
        timeout_ms: DEFAULT_TIMEOUT_MS,
        heal_after_ms: 1,
        post_rebuild_hook: parking_lot::RwLock::new(None),
    });

    pool.mark_unhealthy(0);
    sleep(Duration::from_millis(5)).await;
    pool.probe_and_heal().await;

    assert_eq!(rebuilds.load(Ordering::SeqCst), 1, "failed probe should rebuild once");
    assert!(pool.is_healthy(0), "successful post-rebuild probe should re-admit");
    let replacement = pool.runtime_at(0).expect("slot exists");
    assert!(!Arc::ptr_eq(&original, &replacement), "slot must hold the rebuilt isolate");
    Ok(())
}

#[tokio::test]
async fn probe_and_heal_keeps_unhealthy_when_rebuild_still_fails() -> Result<(), RariError> {
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime::new(Arc::clone(&calls));
    runtime.fail_script.store(true, Ordering::SeqCst);

    let pool = Arc::new(JsRuntimePool {
        runtimes: wrap_runtimes(vec![Arc::new(runtime)]),
        runtime_factory: Arc::new(|| {
            let rebuilt = CountingRuntime::new(Arc::new(AtomicUsize::new(0)));
            rebuilt.fail_script.store(true, Ordering::SeqCst);
            Arc::new(rebuilt) as Arc<dyn JsRuntimeInterface>
        }),
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy: vec![AtomicBool::new(true)],
        unhealthy_since_ms: vec![AtomicU64::new(0)],
        slot_leases: vec![Arc::new(AsyncMutex::new(()))],
        stream_load: vec![Arc::new(AtomicUsize::new(0))],
        needs_rebuild: vec![AtomicBool::new(false)],
        setup_mode: AtomicBool::new(false),
        timeout_ms: DEFAULT_TIMEOUT_MS,
        heal_after_ms: 1,
        post_rebuild_hook: parking_lot::RwLock::new(None),
    });

    pool.mark_unhealthy(0);
    sleep(Duration::from_millis(5)).await;
    pool.probe_and_heal().await;
    assert!(!pool.is_healthy(0), "slot must stay unhealthy when rebuild probe also fails");
    assert!(calls.load(Ordering::SeqCst) >= 1, "original probe should have executed");
    Ok(())
}

#[tokio::test]
async fn execute_script_times_out_when_runtime_hangs() {
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime::new(Arc::clone(&calls));
    runtime.hang_script.store(true, Ordering::SeqCst);

    let pool = pool_from_runtimes(vec![Arc::new(runtime)], 20, HEAL_DISABLED);

    let result = pool.execute_script("hang.js".into(), "1".into()).await;
    assert!(result.is_err(), "expected timeout error");
    let msg = match result {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("timed out"), "got: {msg}");
}

#[test]
fn healthy_count_tracks_marks() {
    let (pool, _) = build_pool_with_counters(4);
    assert_eq!(pool.healthy_count(), 4);

    pool.mark_unhealthy(1);
    pool.mark_unhealthy(3);
    assert_eq!(pool.healthy_count(), 2);

    pool.mark_healthy(1);
    assert_eq!(pool.healthy_count(), 3);

    assert_eq!(pool.all_healthy_indices(), vec![0, 1, 2]);
}

struct BroadcastingRuntime {
    fail_next: Arc<AtomicBool>,
}

impl JsRuntimeInterface for BroadcastingRuntime {
    fn execute_script(
        &self,
        _script_name: String,
        _script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        let fail = Arc::clone(&self.fail_next);
        Box::pin(async move {
            if fail.load(Ordering::SeqCst) {
                Err(RariError::js_runtime("simulated failure".to_string()))
            } else {
                Ok(json!({"ok": true}))
            }
        })
    }

    fn execute_script_batch(
        &self,
        _scripts: Vec<(String, String)>,
    ) -> Pin<
        Box<dyn Future<Output = mpsc::UnboundedReceiver<(usize, Result<Value, RariError>)>> + Send>,
    > {
        let (_tx, rx) = mpsc::unbounded_channel();
        Box::pin(async move { rx })
    }

    fn execute_function(
        &self,
        _function_name: &str,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send + 'static>> {
        Box::pin(async move { Ok(json!(null)) })
    }

    fn add_module_to_loader(
        &self,
        _specifier: &str,
        _code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let fail = Arc::clone(&self.fail_next);
        Box::pin(async move {
            if fail.load(Ordering::SeqCst) {
                Err(RariError::js_runtime("simulated load failure".to_string()))
            } else {
                Ok(())
            }
        })
    }

    fn load_es_module(
        &self,
        _specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>> {
        Box::pin(async move { Ok(0) })
    }

    fn evaluate_module(
        &self,
        _module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        let fail = Arc::clone(&self.fail_next);
        Box::pin(async move {
            if fail.load(Ordering::SeqCst) {
                Err(RariError::js_runtime("simulated evaluate failure".to_string()))
            } else {
                Ok(json!(null))
            }
        })
    }

    fn get_module_namespace(
        &self,
        _module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        Box::pin(async move { Ok(json!(null)) })
    }

    fn clear_module_loader_caches(
        &self,
        _component_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn set_request_context(
        &self,
        _request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn clear_request_context_if_matches(
        &self,
        _expected_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn execute_script_for_streaming(
        &self,
        _stream_id: String,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn queue_script_for_streaming(
        &self,
        _stream_id: String,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
        _request_context: Option<Arc<RequestContext>>,
    ) -> QueueStreamingScriptFuture {
        Box::pin(async move {
            let completion: Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> =
                Box::pin(async move { Ok(()) });
            Ok(completion)
        })
    }

    fn register_request_context(
        &self,
        _request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn unregister_request_context(
        &self,
        _request_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }
}

fn build_pool_with_fail_flags(size: usize) -> (Arc<JsRuntimePool>, Vec<Arc<AtomicBool>>) {
    let mut runtimes: Vec<Arc<dyn JsRuntimeInterface>> = Vec::with_capacity(size);
    let mut flags: Vec<Arc<AtomicBool>> = Vec::with_capacity(size);

    for _ in 0..size {
        let flag = Arc::new(AtomicBool::new(false));
        flags.push(Arc::clone(&flag));
        runtimes.push(Arc::new(BroadcastingRuntime { fail_next: Arc::clone(&flag) }));
    }

    let pool = pool_from_runtimes(runtimes, DEFAULT_TIMEOUT_MS, HEAL_DISABLED);

    (pool, flags)
}

#[tokio::test]
async fn setup_mode_broadcasts_to_all_runtimes() -> Result<(), RariError> {
    let (pool, counters) = build_pool_with_counters(3);

    pool.set_setup_mode(true);
    pool.execute_script("setup.js".into(), "init".into()).await?;

    assert_eq!(counters[0].load(Ordering::SeqCst), 1);
    assert_eq!(counters[1].load(Ordering::SeqCst), 1);
    assert_eq!(counters[2].load(Ordering::SeqCst), 1);

    pool.set_setup_mode(false);
    for _ in 0..6 {
        pool.execute_script("req.js".into(), "go".into()).await?;
    }

    assert_eq!(counters[0].load(Ordering::SeqCst), 3);
    assert_eq!(counters[1].load(Ordering::SeqCst), 3);
    assert_eq!(counters[2].load(Ordering::SeqCst), 3);
    Ok(())
}

#[tokio::test]
async fn setup_mode_aggregates_errors() {
    let (pool, flags) = build_pool_with_fail_flags(3);

    pool.set_setup_mode(true);
    flags[1].store(true, Ordering::SeqCst);

    let result = pool.execute_script("setup.js".into(), "init".into()).await;
    assert!(result.is_err());
    let err_msg = match result {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(err_msg.contains("1 of 3 runtimes"), "got: {err_msg}");
}

#[tokio::test]
async fn setup_mode_returns_pool_unavailable_when_all_unhealthy() {
    let (pool, _counters) = build_pool_with_counters(2);
    pool.mark_unhealthy(0);
    pool.mark_unhealthy(1);
    pool.set_setup_mode(true);

    let result = pool.execute_script("setup.js".into(), "init".into()).await;
    assert!(result.is_err(), "setup-mode broadcast with zero healthy runtimes must not return Ok");
    let err_msg = match result {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(err_msg.contains("No healthy JS runtime available in pool"), "got: {err_msg}");
}

#[tokio::test]
async fn invalidate_component_all_returns_error_when_some_runtime_fails() {
    let (pool, flags) = build_pool_with_fail_flags(2);

    flags[1].store(true, Ordering::SeqCst);

    let result = pool.invalidate_component_all("MyComponent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn invalidate_component_all_succeeds_when_all_runtimes_ok() {
    let (pool, _flags) = build_pool_with_fail_flags(2);

    let result = pool.invalidate_component_all("MyComponent").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn broadcast_continues_when_one_runtime_hangs() {
    let hang_calls = Arc::new(AtomicUsize::new(0));
    let ok_calls = Arc::new(AtomicUsize::new(0));
    let hang_runtime = CountingRuntime::new(Arc::clone(&hang_calls));
    hang_runtime.hang_script.store(true, Ordering::SeqCst);
    let ok_runtime = CountingRuntime::new(Arc::clone(&ok_calls));

    let pool =
        pool_from_runtimes(vec![Arc::new(hang_runtime), Arc::new(ok_runtime)], 30, HEAL_DISABLED);

    let started = Instant::now();
    let result = pool.broadcast_script("s.js", "1").await;
    let elapsed = started.elapsed();

    assert!(result.is_err(), "hanging slot should make broadcast report aggregate error");
    assert!(ok_calls.load(Ordering::SeqCst) >= 1, "healthy slot must still run despite peer hang");
    assert!(!pool.is_healthy(0), "timed-out slot must be marked unhealthy");
    assert!(pool.is_healthy(1), "successful slot must stay healthy");
    assert!(
        elapsed < Duration::from_secs(2),
        "peer hang must not serialize the full shared budget; elapsed={elapsed:?}"
    );
}

#[tokio::test]
async fn broadcast_load_and_evaluate_module_reports_evaluate_failures() {
    let (pool, flags) = build_pool_with_fail_flags(2);
    flags[1].store(true, Ordering::SeqCst);

    let result = pool.broadcast_load_and_evaluate_module("MyComponent").await;
    assert!(result.is_err());
    let msg = match result {
        Ok(()) => String::new(),
        Err(e) => e.to_string(),
    };

    assert!(msg.contains("runtime[1].evaluate_module"), "got: {msg}");
    assert!(!msg.contains("runtime[1].load_es_module"), "got: {msg}");
}

#[tokio::test]
async fn pool_size_one_behaves_like_single_runtime() -> Result<(), RariError> {
    let (pool, counters) = build_pool_with_counters(1);

    for _ in 0..5 {
        pool.execute_script("test".into(), "1+1".into()).await?;
    }

    assert_eq!(counters[0].load(Ordering::SeqCst), 5);
    Ok(())
}

#[test]
fn pick_returns_none_when_all_unhealthy() {
    let (pool, _) = build_pool_with_counters(2);
    pool.mark_unhealthy(0);
    pool.mark_unhealthy(1);
    assert!(pool.pick().is_none());
}

#[tokio::test]
async fn execute_script_returns_pool_unavailable_when_all_unhealthy() {
    let (pool, counters) = build_pool_with_counters(2);
    pool.mark_unhealthy(0);
    pool.mark_unhealthy(1);

    let result = pool.execute_script("x.js".into(), "1".into()).await;
    assert!(result.is_err());
    let msg = match result {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("No healthy JS runtime available in pool"), "got: {msg}");

    assert_eq!(counters[0].load(Ordering::SeqCst), 0);
    assert_eq!(counters[1].load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn execute_script_batch_emits_pool_unavailable_for_each_script_when_all_unhealthy()
-> Result<(), RariError> {
    let (pool, _counters) = build_pool_with_counters(2);
    pool.mark_unhealthy(0);
    pool.mark_unhealthy(1);

    let scripts = vec![
        ("a".to_string(), "1".to_string()),
        ("b".to_string(), "2".to_string()),
        ("c".to_string(), "3".to_string()),
    ];
    let expected_len = scripts.len();
    let mut rx = pool.execute_script_batch(scripts).await;

    let mut errors: Vec<(usize, String)> = Vec::new();
    for _ in 0..expected_len {
        match rx.recv().await {
            Some((idx, Err(e))) => errors.push((idx, e.to_string())),
            Some((idx, Ok(_))) => {
                return Err(RariError::js_runtime(format!("expected error at idx {idx}, got Ok")));
            }
            None => {
                return Err(RariError::js_runtime(format!(
                    "channel closed before receiving all {expected_len} errors"
                )));
            }
        }
    }
    assert_eq!(rx.recv().await, None, "receiver should be closed after N errors");

    assert_eq!(errors.len(), expected_len);
    let mut indices: Vec<usize> = errors.iter().map(|(i, _)| *i).collect();
    indices.sort_unstable();
    assert_eq!(indices, vec![0, 1, 2]);
    for (_, msg) in &errors {
        assert!(msg.contains("No healthy JS runtime available in pool"), "got: {msg}");
    }
    Ok(())
}

#[tokio::test]
async fn pick_runtime_returns_error_when_all_unhealthy() {
    let (pool, _) = build_pool_with_counters(2);
    pool.mark_unhealthy(0);
    pool.mark_unhealthy(1);
    assert!(pool.pick_runtime().await.is_err());
}

#[tokio::test]
async fn pooled_runtime_is_sticky_across_calls() -> Result<(), RariError> {
    let (pool, _counters) = build_pool_with_counters(3);
    let handle = pool.pick_runtime().await?;
    let expected_idx = handle.idx();
    let expected_runtime = Arc::clone(handle.runtime());

    for i in 0..3 {
        if i != expected_idx {
            pool.mark_unhealthy(i);
        }
    }

    assert!(Arc::ptr_eq(handle.runtime(), &expected_runtime));
    assert_eq!(handle.idx(), expected_idx);

    pool.mark_unhealthy(expected_idx);

    assert!(Arc::ptr_eq(handle.runtime(), &expected_runtime));
    assert_eq!(handle.idx(), expected_idx);
    Ok(())
}

#[tokio::test]
async fn pooled_runtime_routes_through_self_runtime_after_pool_changes() -> Result<(), RariError> {
    let (pool, counters) = build_pool_with_counters(3);
    let handle = pool.pick_runtime().await?;
    let picked_idx = handle.idx();

    pool.mark_unhealthy(0);
    pool.mark_unhealthy(2);

    handle.execute_script("s.js".into(), "x".into()).await?;
    handle.execute_script("s.js".into(), "y".into()).await?;

    assert_eq!(counters[picked_idx].load(Ordering::SeqCst), 2);
    assert_eq!(counters[(picked_idx + 1) % 3].load(Ordering::SeqCst), 0);
    Ok(())
}

struct RequestContextRuntime {
    last_seen: Arc<Mutex<Option<Arc<RequestContext>>>>,
    fail_cleanup: Arc<AtomicBool>,
}

impl JsRuntimeInterface for RequestContextRuntime {
    fn execute_script(
        &self,
        _script_name: String,
        _script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        Box::pin(async move { Ok(json!({"ok": true})) })
    }

    fn execute_script_batch(
        &self,
        _scripts: Vec<(String, String)>,
    ) -> Pin<
        Box<dyn Future<Output = mpsc::UnboundedReceiver<(usize, Result<Value, RariError>)>> + Send>,
    > {
        let (_tx, rx) = mpsc::unbounded_channel();
        Box::pin(async move { rx })
    }

    fn execute_function(
        &self,
        _function_name: &str,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send + 'static>> {
        Box::pin(async move { Ok(json!(null)) })
    }

    fn add_module_to_loader(
        &self,
        _specifier: &str,
        _code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn load_es_module(
        &self,
        _specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>> {
        Box::pin(async move { Ok(0) })
    }

    fn evaluate_module(
        &self,
        _module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        Box::pin(async move { Ok(json!(null)) })
    }

    fn get_module_namespace(
        &self,
        _module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        Box::pin(async move { Ok(json!(null)) })
    }

    fn clear_module_loader_caches(
        &self,
        _component_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn set_request_context(
        &self,
        request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let last_seen = Arc::clone(&self.last_seen);
        Box::pin(async move {
            let mut guard = last_seen
                .lock()
                .map_err(|_| RariError::js_runtime("request context mutex poisoned".to_string()))?;
            *guard = Some(Arc::clone(&request_context));
            Ok(())
        })
    }

    fn clear_request_context_if_matches(
        &self,
        expected_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let last_seen = Arc::clone(&self.last_seen);
        let fail_cleanup = Arc::clone(&self.fail_cleanup);
        Box::pin(async move {
            if fail_cleanup.load(Ordering::Acquire) {
                return Err(RariError::js_runtime("forced cleanup failure".to_string()));
            }
            let mut guard = last_seen
                .lock()
                .map_err(|_| RariError::js_runtime("request context mutex poisoned".to_string()))?;
            if let Some(current) = guard.as_ref() {
                if Arc::ptr_eq(current, &expected_context) {
                    *guard = None;
                    return Ok(());
                }
                return Err(RariError::js_runtime("request context mismatch on clear".to_string()));
            }
            Err(RariError::js_runtime("no request context to clear".to_string()))
        })
    }

    fn execute_script_for_streaming(
        &self,
        _stream_id: String,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn queue_script_for_streaming(
        &self,
        _stream_id: String,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
        _request_context: Option<Arc<RequestContext>>,
    ) -> QueueStreamingScriptFuture {
        Box::pin(async move {
            let completion: Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> =
                Box::pin(async move { Ok(()) });
            Ok(completion)
        })
    }

    fn register_request_context(
        &self,
        _request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn unregister_request_context(
        &self,
        _request_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }
}

type LastSeenSlot = Arc<Mutex<Option<Arc<RequestContext>>>>;

fn build_pool_with_distinct_request_context_runtimes(
    size: usize,
) -> (Arc<JsRuntimePool>, Vec<LastSeenSlot>) {
    let last_seens: Vec<LastSeenSlot> = (0..size).map(|_| Arc::new(Mutex::new(None))).collect();
    let runtimes: Vec<Arc<dyn JsRuntimeInterface>> = last_seens
        .iter()
        .map(|ls| {
            Arc::new(RequestContextRuntime {
                last_seen: Arc::clone(ls),
                fail_cleanup: Arc::new(AtomicBool::new(false)),
            }) as Arc<dyn JsRuntimeInterface>
        })
        .collect();
    let pool = pool_from_runtimes(runtimes, DEFAULT_TIMEOUT_MS, HEAL_DISABLED);
    (pool, last_seens)
}

#[tokio::test]
async fn pooled_runtime_set_and_clear_request_context_round_trip() -> Result<(), RariError> {
    let (pool, last_seens) = build_pool_with_distinct_request_context_runtimes(2);

    let handle = pool.pick_runtime().await?;
    let picked_idx = handle.idx();
    let ctx = Arc::new(RequestContext::new("/test".to_string()));

    handle.set_request_context(Arc::clone(&ctx)).await?;
    assert!(
        last_seens[picked_idx].lock().map(|guard| guard.is_some()).unwrap_or(false),
        "set_request_context must touch the picked runtime, not another slot"
    );
    for (i, ls) in last_seens.iter().enumerate() {
        if i != picked_idx {
            assert!(
                ls.lock().map(|guard| guard.is_none()).unwrap_or(true),
                "non-picked runtime {i} must not be touched"
            );
        }
    }

    handle.clear_request_context_if_matches(Arc::clone(&ctx)).await?;
    assert!(
        last_seens[picked_idx].lock().map(|guard| guard.is_none()).unwrap_or(false),
        "clear must touch the same runtime that was set"
    );
    Ok(())
}

#[tokio::test]
async fn load_and_evaluate_on_picked_returns_ok_on_healthy_pool() -> Result<(), RariError> {
    let (pool, _counters) = build_pool_with_counters(2);
    let result = pool.load_and_evaluate_on_picked("m.js").await;
    assert!(result.is_ok(), "load_and_evaluate_on_picked should succeed on healthy pool");
    let (module_id, _) = result?;
    assert_eq!(module_id, 0, "fake CountingRuntime returns Ok(0) for load_es_module");
    Ok(())
}

#[tokio::test]
async fn load_and_evaluate_on_picked_returns_pool_unavailable_when_all_unhealthy() {
    let (pool, _counters) = build_pool_with_counters(2);
    pool.mark_unhealthy(0);
    pool.mark_unhealthy(1);

    let result = pool.load_and_evaluate_on_picked("m.js").await;
    assert!(result.is_err());
    let msg = match result {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("No healthy JS runtime available in pool"), "got: {msg}");
}

#[tokio::test]
async fn with_request_context_runs_op_and_cleans_up() {
    let (pool, last_seens) = build_pool_with_distinct_request_context_runtimes(2);

    let ctx = Arc::new(RequestContext::new("/with_req_ctx".to_string()));
    let ctx_for_op = Arc::clone(&ctx);

    let observed_during_op = Arc::new(Mutex::new(false));

    let observed_inside = Arc::clone(&observed_during_op);
    let result = pool
        .with_request_context(Arc::clone(&ctx), move |runtime| {
            let observed = observed_inside;
            let ctx_for_op = ctx_for_op;
            async move {
                let _ = runtime.execute_script("noop".into(), "1".into()).await?;
                let mut observed = observed
                    .lock()
                    .map_err(|_| RariError::js_runtime("observed mutex poisoned".to_string()))?;
                *observed = true;
                drop(observed);
                let _ = ctx_for_op;
                Ok::<_, RariError>(())
            }
        })
        .await;

    assert!(result.is_ok());
    assert!(
        observed_during_op.lock().map(|guard| *guard).unwrap_or(false),
        "op must have executed on a runtime"
    );
    let touched: Vec<usize> = last_seens
        .iter()
        .enumerate()
        .filter_map(|(i, ls)| {
            if ls.lock().map(|guard| guard.is_some()).unwrap_or(false) { Some(i) } else { None }
        })
        .collect();
    assert_eq!(
        touched.len(),
        0,
        "no runtime should retain ctx after with_request_context returns; touched: {touched:?}"
    );
}

#[tokio::test]
async fn with_request_context_cleans_up_even_when_op_errors() {
    let (pool, last_seens) = build_pool_with_distinct_request_context_runtimes(1);

    let ctx = Arc::new(RequestContext::new("/error_path".to_string()));

    let result = pool
        .with_request_context(Arc::clone(&ctx), |_runtime| async {
            Err::<(), _>(RariError::js_runtime("op failed".to_string()))
        })
        .await;

    assert!(result.is_err());
    assert!(
        last_seens[0].lock().map(|guard| guard.is_none()).unwrap_or(false),
        "ctx must be cleared even when op returns Err"
    );
}

#[tokio::test]
async fn with_request_context_serializes_same_slot_so_ops_see_own_context() {
    let (pool, last_seens) = build_pool_with_distinct_request_context_runtimes(1);
    let last_seen = Arc::clone(&last_seens[0]);

    let ctx_a = Arc::new(RequestContext::new("/lease_a".to_string()));
    let ctx_b = Arc::new(RequestContext::new("/lease_b".to_string()));

    let pool_a = Arc::clone(&pool);
    let last_a = Arc::clone(&last_seen);
    let ctx_a_op = Arc::clone(&ctx_a);
    let fut_a = async move {
        pool_a
            .with_request_context(Arc::clone(&ctx_a_op), move |_runtime| {
                let last_a = last_a;
                let ctx_a_op = ctx_a_op;
                async move {
                    sleep(Duration::from_millis(30)).await;
                    let guard = last_a
                        .lock()
                        .map_err(|_| RariError::js_runtime("last_seen poisoned".to_string()))?;
                    let current = guard.as_ref().ok_or_else(|| {
                        RariError::js_runtime("expected request context during op A".to_string())
                    })?;
                    if !Arc::ptr_eq(current, &ctx_a_op) {
                        return Err(RariError::js_runtime(
                            "op A observed a different request context".to_string(),
                        ));
                    }
                    Ok(())
                }
            })
            .await
    };

    let pool_b = Arc::clone(&pool);
    let last_b = Arc::clone(&last_seen);
    let ctx_b_op = Arc::clone(&ctx_b);
    let fut_b = async move {
        pool_b
            .with_request_context(Arc::clone(&ctx_b_op), move |_runtime| {
                let last_b = last_b;
                let ctx_b_op = ctx_b_op;
                async move {
                    sleep(Duration::from_millis(30)).await;
                    let guard = last_b
                        .lock()
                        .map_err(|_| RariError::js_runtime("last_seen poisoned".to_string()))?;
                    let current = guard.as_ref().ok_or_else(|| {
                        RariError::js_runtime("expected request context during op B".to_string())
                    })?;
                    if !Arc::ptr_eq(current, &ctx_b_op) {
                        return Err(RariError::js_runtime(
                            "op B observed a different request context".to_string(),
                        ));
                    }
                    Ok(())
                }
            })
            .await
    };

    let (res_a, res_b) = tokio::join!(fut_a, fut_b);
    assert!(res_a.is_ok(), "op A failed: {res_a:?}");
    assert!(res_b.is_ok(), "op B failed: {res_b:?}");
    assert!(
        last_seen.lock().map(|guard| guard.is_none()).unwrap_or(false),
        "request context must be cleared after both ops"
    );
}

#[tokio::test]
async fn load_and_evaluate_on_picked_quarantines_timed_out_slot() {
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime::new(Arc::clone(&calls));
    runtime.hang_load.store(true, Ordering::SeqCst);

    let pool = pool_from_runtimes(vec![Arc::new(runtime)], 20, HEAL_DISABLED);

    let result = pool.load_and_evaluate_on_picked("m.js").await;
    assert!(result.is_err(), "expected timeout");
    let msg = match result {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("timed out"), "got: {msg}");
    assert!(!pool.is_healthy(0), "timed-out slot must be quarantined");
    assert!(pool.pick().is_none(), "quarantined slot must not be selected again");
}

#[tokio::test]
async fn with_request_context_returns_cleanup_error_and_quarantines() {
    let fail_flag = Arc::new(AtomicBool::new(true));
    let last_seen = Arc::new(Mutex::new(None));
    let runtime: Arc<dyn JsRuntimeInterface> = Arc::new(RequestContextRuntime {
        last_seen: Arc::clone(&last_seen),
        fail_cleanup: Arc::clone(&fail_flag),
    });
    let rebuilds = Arc::new(AtomicUsize::new(0));
    let rebuilds_for_factory = Arc::clone(&rebuilds);
    let pool = Arc::new(JsRuntimePool {
        runtimes: wrap_runtimes(vec![runtime]),
        runtime_factory: Arc::new(move || {
            rebuilds_for_factory.fetch_add(1, Ordering::SeqCst);
            Arc::new(CountingRuntime::new(Arc::new(AtomicUsize::new(0))))
                as Arc<dyn JsRuntimeInterface>
        }),
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy: vec![AtomicBool::new(true)],
        unhealthy_since_ms: vec![AtomicU64::new(0)],
        slot_leases: vec![Arc::new(AsyncMutex::new(()))],
        stream_load: vec![Arc::new(AtomicUsize::new(0))],
        needs_rebuild: vec![AtomicBool::new(false)],
        setup_mode: AtomicBool::new(false),
        timeout_ms: DEFAULT_TIMEOUT_MS,
        heal_after_ms: 1,
        post_rebuild_hook: parking_lot::RwLock::new(None),
    });

    let ctx = Arc::new(RequestContext::new("/cleanup_fails".to_string()));
    let result = pool.with_request_context(Arc::clone(&ctx), |_runtime| async { Ok(42_i32) }).await;
    assert!(result.is_err(), "cleanup failure must be returned");
    assert!(!pool.is_healthy(0), "cleanup failure must quarantine the slot");

    sleep(Duration::from_millis(5)).await;
    pool.probe_and_heal().await;
    assert_eq!(
        rebuilds.load(Ordering::SeqCst),
        1,
        "cleanup failure must force rebuild on heal, not probe-only re-admit"
    );
    assert!(pool.is_healthy(0), "successful rebuild probe should re-admit");
}

#[tokio::test]
#[expect(clippy::unwrap_used, reason = "test task joins are hard failures")]
async fn queued_waiter_rejects_contaminated_slot_before_rebuild() {
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime: Arc<dyn JsRuntimeInterface> = Arc::new(CountingRuntime::new(Arc::clone(&calls)));
    let pool = pool_from_runtimes(vec![Arc::clone(&runtime)], DEFAULT_TIMEOUT_MS, HEAL_DISABLED);

    let context_installed = Arc::new(Notify::new());
    let release_blocker = Arc::new(Notify::new());
    let context_installed_blocker = Arc::clone(&context_installed);
    let release_blocker_blocker = Arc::clone(&release_blocker);

    let pool_hold = Arc::clone(&pool);
    let blocker = tokio::spawn(async move {
        pool_hold
            .with_request_context(
                Arc::new(RequestContext::new("/block".to_string())),
                move |_rt| async move {
                    context_installed_blocker.notify_waiters();
                    release_blocker_blocker.notified().await;
                    Ok(())
                },
            )
            .await
    });

    context_installed.notified().await;
    pool.mark_needs_rebuild(0);

    let pool_exec = Arc::clone(&pool);
    let exec = tokio::spawn(async move { pool_exec.execute_script("x".into(), "1".into()).await });

    for _ in 0..100 {
        yield_now().await;
    }
    release_blocker.notify_one();

    let result = exec.await.unwrap();
    assert!(result.is_err(), "must reject contaminated slot after acquiring lease");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "waiter must not execute on contaminated runtime before rebuild"
    );

    let blocker_result = blocker.await.unwrap();
    assert!(blocker_result.is_ok(), "blocker should complete cleanly");
}

#[tokio::test]
async fn setup_mode_error_message_uses_executed_not_total() {
    let (pool, flags) = build_pool_with_fail_flags(4);
    pool.mark_unhealthy(1);
    pool.mark_unhealthy(3);
    pool.set_setup_mode(true);
    flags[2].store(true, Ordering::SeqCst);

    let result = pool.execute_script("setup.js".into(), "init".into()).await;
    let msg = match result {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(
        msg.contains("1 of 2 runtimes"),
        "error must report failed/attempted (1 of 2), got: {msg}"
    );
    assert!(!msg.contains("1 of 4 runtimes"), "error must not use total pool size, got: {msg}");
}

#[tokio::test]
async fn with_request_context_op_timeout_keeps_slot_healthy_when_cleanup_ok() {
    let last_seen = Arc::new(Mutex::new(None));
    let runtime: Arc<dyn JsRuntimeInterface> = Arc::new(RequestContextRuntime {
        last_seen: Arc::clone(&last_seen),
        fail_cleanup: Arc::new(AtomicBool::new(false)),
    });
    let pool = pool_from_runtimes(vec![runtime], 30, HEAL_DISABLED);
    let ctx = Arc::new(RequestContext::new("/timeout_ok_cleanup".to_string()));

    let result = pool
        .with_request_context(Arc::clone(&ctx), |_runtime| async {
            sleep(Duration::from_millis(80)).await;
            Ok(1_i32)
        })
        .await;

    assert!(result.is_err(), "expected timeout error");
    let msg = match result {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("timed out"), "got: {msg}");
    assert!(
        pool.is_healthy(0),
        "successful cleanup after op timeout must not quarantine the only slot"
    );
    assert!(
        last_seen.lock().map(|guard| guard.is_none()).unwrap_or(false),
        "request context must be cleared after timed-out op"
    );
}

#[tokio::test]
async fn probe_and_heal_immediately_when_no_healthy_slots() -> Result<(), RariError> {
    let runtime = CountingRuntime::new(Arc::new(AtomicUsize::new(0)));
    let pool = pool_from_runtimes(vec![Arc::new(runtime)], DEFAULT_TIMEOUT_MS, 60_000);
    pool.mark_unhealthy(0);
    assert_eq!(pool.healthy_count(), 0);

    pool.probe_and_heal().await;
    assert!(pool.is_healthy(0), "empty pool must heal immediately without waiting heal_after_ms");
    Ok(())
}

#[tokio::test]
async fn pick_runtime_for_streaming_prefers_least_busy_slot() -> Result<(), RariError> {
    let (pool, _counters) = build_pool_with_counters(3);

    let (first, guard0) = pool.pick_runtime_for_streaming().await?;
    assert_eq!(pool.stream_load_at(first.idx()), 1);

    let (second, guard1) = pool.pick_runtime_for_streaming().await?;
    assert_ne!(second.idx(), first.idx());
    assert_eq!(pool.stream_load_at(second.idx()), 1);

    let (third, guard2) = pool.pick_runtime_for_streaming().await?;
    assert_ne!(third.idx(), first.idx());
    assert_ne!(third.idx(), second.idx());

    let busy_idx = first.idx();
    drop(guard0);
    assert_eq!(pool.stream_load_at(busy_idx), 0);

    let (again, _guard) = pool.pick_runtime_for_streaming().await?;
    assert_eq!(again.idx(), busy_idx, "freed slot should be preferred over busier ones");

    drop(guard1);
    drop(guard2);
    Ok(())
}

#[tokio::test]
async fn invalidate_component_all_error_uses_executed_not_total() {
    let (pool, flags) = build_pool_with_fail_flags(4);
    pool.mark_unhealthy(0);
    pool.mark_unhealthy(3);
    flags[2].store(true, Ordering::SeqCst);

    let result = pool.invalidate_component_all("MyComponent").await;
    let msg = match result {
        Ok(()) => String::new(),
        Err(e) => e.to_string(),
    };
    assert!(
        msg.contains("1 of 2 runtimes"),
        "error must report failed/attempted (1 of 2), got: {msg}"
    );
}
