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
                    let _lease = match pool.acquire_slot_lease_for_execute(idx, &runtime).await {
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
            self.execute_on_admitted_healthy_slot("execute_script", |runtime| {
                runtime.execute_script(script_name.clone(), script_code.clone())
            })
            .await
        }
    }

    pub async fn execute_script_batch(
        &self,
        scripts: Vec<(String, String)>,
    ) -> UnboundedReceiver<(usize, Result<Value, RariError>)> {
        self.probe_and_heal().await;
        let (tx, rx) = unbounded_channel();
        let Some(first) = self.pick() else {
            for (script_idx, _) in scripts.iter().enumerate() {
                let _ = tx.send((script_idx, Err(pool_unavailable_error())));
            }
            return rx;
        };
        let mut candidates: Vec<usize> = self.all_healthy_indices();
        candidates.retain(|&i| i != first);
        candidates.insert(0, first);
        for idx in candidates {
            let Some(runtime) = self.runtime_at(idx) else {
                continue;
            };
            let Ok(lease) = self.acquire_owned_slot_lease_for_execute(idx, &runtime).await else {
                continue;
            };
            let tx = tx.clone();
            tokio::spawn(async move {
                let _lease = lease;
                let mut inner_rx = runtime.execute_script_batch(scripts).await;
                while let Some(item) = inner_rx.recv().await {
                    if tx.send(item).is_err() {
                        break;
                    }
                }
            });
            return rx;
        }
        for (script_idx, _) in scripts.iter().enumerate() {
            let _ = tx.send((script_idx, Err(pool_unavailable_error())));
        }
        rx
    }

    pub async fn execute_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RariError> {
        self.probe_and_heal().await;
        let function_name = function_name.to_string();
        self.execute_on_admitted_healthy_slot("execute_function", move |runtime| {
            runtime.execute_function(&function_name, args.clone())
        })
        .await
    }
}
