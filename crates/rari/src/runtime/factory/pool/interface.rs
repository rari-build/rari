use std::{sync::atomic::Ordering, time::Duration};

use futures::future::join_all;
use rari_error::RariError;
use serde_json::Value;
use tokio::{
    sync::mpsc::{UnboundedReceiver, unbounded_channel},
    time,
};

use super::JsRuntimePool;

fn pool_unavailable_error() -> RariError {
    RariError::js_runtime("No healthy JS runtime available in pool".to_string())
}

fn timeout_error(label: &str, timeout_ms: u64) -> RariError {
    RariError::timeout(format!("{label} timed out after {timeout_ms} ms"))
}

impl JsRuntimePool {
    /// Execute a script on one healthy runtime, or on every healthy runtime when
    /// `setup_mode` is enabled. Setup broadcast keeps the first successful value and
    /// does not mark failed slots unhealthy (bootstrap should stay retryable).
    pub async fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Result<Value, RariError> {
        self.probe_and_heal().await;
        let timeout_ms = self.timeout_ms;
        if self.setup_mode.load(Ordering::Acquire) {
            let healthy_snapshot: Vec<bool> =
                self.healthy.iter().map(|h| h.load(Ordering::Acquire)).collect();

            let mut slot_futs = Vec::new();
            for (idx, healthy) in healthy_snapshot.iter().enumerate() {
                if !healthy {
                    continue;
                }
                let Some(runtime) = self.runtime_at(idx) else {
                    continue;
                };
                let script_name = script_name.clone();
                let script_code = script_code.clone();
                let pool = self;
                slot_futs.push(async move {
                    let _lease = match pool.acquire_slot_lease(idx).await {
                        Ok(guard) => guard,
                        Err(e) => return Err(format!("runtime[{idx}]: {e}")),
                    };
                    match time::timeout(
                        Duration::from_millis(timeout_ms),
                        runtime.execute_script(script_name, script_code),
                    )
                    .await
                    {
                        Ok(Ok(v)) => Ok((idx, v)),
                        Ok(Err(e)) => Err(format!("runtime[{idx}]: {e}")),
                        Err(_) => Err(format!("runtime[{idx}]: timed out after {timeout_ms} ms")),
                    }
                });
            }

            let executed = slot_futs.len();
            if executed == 0 {
                return Err(pool_unavailable_error());
            }

            let mut errors: Vec<String> = Vec::new();
            let mut successes: Vec<(usize, Value)> = Vec::new();
            for result in join_all(slot_futs).await {
                match result {
                    Ok((idx, v)) => successes.push((idx, v)),
                    Err(msg) => errors.push(msg),
                }
            }
            successes.sort_by_key(|(idx, _)| *idx);
            let first_value = successes.into_iter().next().map(|(_, v)| v);

            if errors.is_empty() {
                Ok(first_value.unwrap_or(Value::Null))
            } else {
                Err(RariError::js_runtime(format!(
                    "Failed to broadcast execute_script {script_name} on {} of {executed} runtimes: {}",
                    errors.len(),
                    errors.join("; ")
                )))
            }
        } else {
            let Some(idx) = self.pick() else {
                return Err(pool_unavailable_error());
            };
            let Some(runtime) = self.runtime_at(idx) else {
                return Err(pool_unavailable_error());
            };
            let _lease = self.acquire_slot_lease(idx).await?;
            match time::timeout(
                Duration::from_millis(timeout_ms),
                runtime.execute_script(script_name, script_code),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    self.mark_unhealthy(idx);
                    Err(timeout_error("execute_script", timeout_ms))
                }
            }
        }
    }

    pub async fn execute_script_batch(
        &self,
        scripts: Vec<(String, String)>,
    ) -> UnboundedReceiver<(usize, Result<Value, RariError>)> {
        self.probe_and_heal().await;
        let Some(runtime) = self.pick().and_then(|idx| self.runtime_at(idx)) else {
            let (tx, rx) = unbounded_channel();
            for (idx, _) in scripts.iter().enumerate() {
                let _ = tx.send((idx, Err(pool_unavailable_error())));
            }
            return rx;
        };
        runtime.execute_script_batch(scripts).await
    }

    pub async fn execute_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RariError> {
        self.probe_and_heal().await;
        let timeout_ms = self.timeout_ms;
        let Some(idx) = self.pick() else {
            return Err(pool_unavailable_error());
        };
        let Some(runtime) = self.runtime_at(idx) else {
            return Err(pool_unavailable_error());
        };
        let _lease = self.acquire_slot_lease(idx).await?;
        let function_name = function_name.to_string();
        match time::timeout(
            Duration::from_millis(timeout_ms),
            runtime.execute_function(&function_name, args),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                self.mark_unhealthy(idx);
                Err(timeout_error("execute_function", timeout_ms))
            }
        }
    }
}
