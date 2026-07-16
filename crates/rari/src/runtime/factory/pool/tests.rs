use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use rari_error::RariError;
use serde_json::{Value, json};
use tokio::{sync::mpsc, time::sleep};

use super::*;
use crate::server::middleware::request_context::RequestContext;

struct CountingRuntime {
    calls: Arc<AtomicUsize>,
    hang_script: Arc<AtomicBool>,
}

impl CountingRuntime {
    fn new(calls: Arc<AtomicUsize>) -> Self {
        Self { calls, hang_script: Arc::new(AtomicBool::new(false)) }
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
        Box::pin(async move {
            if hang {
                sleep(Duration::from_secs(60)).await;
            }
            calls.fetch_add(1, Ordering::SeqCst);
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
        _request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn clear_request_context(&self) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn clear_request_context_if_matches(
        &self,
        _expected_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn execute_script_with_request_context(
        &self,
        _request_context: Arc<RequestContext>,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        self.execute_script(script_name, script_code)
    }

    fn execute_script_for_streaming(
        &self,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }
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

    let healthy = (0..size).map(|_| AtomicBool::new(true)).collect();
    let unhealthy_since_ms = (0..size).map(|_| AtomicU64::new(0)).collect();

    let pool = Arc::new(JsRuntimePool {
        runtimes,
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy,
        unhealthy_since_ms,
        setup_mode: AtomicBool::new(false),
        timeout_ms: DEFAULT_TIMEOUT_MS,
        heal_after_ms: HEAL_DISABLED,
    });

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
async fn heal_expired_re_admits_unhealthy_slots() -> Result<(), RariError> {
    let (_unused, counters) = build_pool_with_counters(2);
    let runtimes = (0..2)
        .map(|i| {
            Arc::new(CountingRuntime::new(Arc::clone(&counters[i]))) as Arc<dyn JsRuntimeInterface>
        })
        .collect::<Vec<_>>();
    let pool = Arc::new(JsRuntimePool {
        runtimes,
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy: (0..2).map(|_| AtomicBool::new(true)).collect(),
        unhealthy_since_ms: (0..2).map(|_| AtomicU64::new(0)).collect(),
        setup_mode: AtomicBool::new(false),
        timeout_ms: DEFAULT_TIMEOUT_MS,
        heal_after_ms: 1,
    });

    pool.mark_unhealthy(0);
    assert!(!pool.is_healthy(0));

    sleep(Duration::from_millis(5)).await;
    let _ = pool.pick();
    assert!(pool.is_healthy(0), "expired unhealthy slot should be healed on pick");
    Ok(())
}

#[tokio::test]
async fn execute_script_times_out_when_runtime_hangs() {
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime::new(Arc::clone(&calls));
    runtime.hang_script.store(true, Ordering::SeqCst);

    let pool = Arc::new(JsRuntimePool {
        runtimes: vec![Arc::new(runtime)],
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy: vec![AtomicBool::new(true)],
        unhealthy_since_ms: vec![AtomicU64::new(0)],
        setup_mode: AtomicBool::new(false),
        timeout_ms: 20,
        heal_after_ms: HEAL_DISABLED,
    });

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

    fn clear_request_context(&self) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn clear_request_context_if_matches(
        &self,
        _expected_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }

    fn execute_script_with_request_context(
        &self,
        _request_context: Arc<RequestContext>,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        self.execute_script(script_name, script_code)
    }

    fn execute_script_for_streaming(
        &self,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
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

    let healthy = (0..size).map(|_| AtomicBool::new(true)).collect();
    let unhealthy_since_ms = (0..size).map(|_| AtomicU64::new(0)).collect();

    let pool = Arc::new(JsRuntimePool {
        runtimes,
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy,
        unhealthy_since_ms,
        setup_mode: AtomicBool::new(false),
        timeout_ms: DEFAULT_TIMEOUT_MS,
        heal_after_ms: HEAL_DISABLED,
    });

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

#[test]
fn pick_runtime_returns_error_when_all_unhealthy() {
    let (pool, _) = build_pool_with_counters(2);
    pool.mark_unhealthy(0);
    pool.mark_unhealthy(1);
    assert!(pool.pick_runtime().is_err());
}

#[tokio::test]
async fn pooled_runtime_is_sticky_across_calls() -> Result<(), RariError> {
    let (pool, _counters) = build_pool_with_counters(3);
    let handle = pool.pick_runtime()?;
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
    let handle = pool.pick_runtime()?;
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

    fn clear_request_context(&self) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        Box::pin(async move { Ok(()) })
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

    fn execute_script_with_request_context(
        &self,
        _request_context: Arc<RequestContext>,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        self.execute_script(script_name, script_code)
    }

    fn execute_script_for_streaming(
        &self,
        _script_name: String,
        _script_code: String,
        _chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
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
    let healthy = runtimes.iter().map(|_| AtomicBool::new(true)).collect();
    let unhealthy_since_ms = runtimes.iter().map(|_| AtomicU64::new(0)).collect();
    let pool = Arc::new(JsRuntimePool {
        runtimes,
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy,
        unhealthy_since_ms,
        setup_mode: AtomicBool::new(false),
        timeout_ms: DEFAULT_TIMEOUT_MS,
        heal_after_ms: HEAL_DISABLED,
    });
    (pool, last_seens)
}

#[tokio::test]
async fn pooled_runtime_set_and_clear_request_context_round_trip() -> Result<(), RariError> {
    let (pool, last_seens) = build_pool_with_distinct_request_context_runtimes(2);

    let handle = pool.pick_runtime()?;
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
async fn with_request_context_returns_op_value_when_cleanup_fails() -> Result<(), RariError> {
    let fail_flag = Arc::new(AtomicBool::new(true));
    let last_seen = Arc::new(Mutex::new(None));
    let runtime: Arc<dyn JsRuntimeInterface> = Arc::new(RequestContextRuntime {
        last_seen: Arc::clone(&last_seen),
        fail_cleanup: Arc::clone(&fail_flag),
    });
    let runtimes: Vec<Arc<dyn JsRuntimeInterface>> = vec![Arc::clone(&runtime)];
    let healthy = runtimes.iter().map(|_| AtomicBool::new(true)).collect();
    let unhealthy_since_ms = runtimes.iter().map(|_| AtomicU64::new(0)).collect();
    let pool = Arc::new(JsRuntimePool {
        runtimes,
        pick_strategy: Arc::new(RoundRobinStrategy),
        next_index: AtomicUsize::new(0),
        healthy,
        unhealthy_since_ms,
        setup_mode: AtomicBool::new(false),
        timeout_ms: DEFAULT_TIMEOUT_MS,
        heal_after_ms: HEAL_DISABLED,
    });

    let ctx = Arc::new(RequestContext::new("/cleanup_fails".to_string()));

    let value =
        pool.with_request_context(Arc::clone(&ctx), |_runtime| async { Ok(42_i32) }).await?;
    assert_eq!(value, 42);
    Ok(())
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
