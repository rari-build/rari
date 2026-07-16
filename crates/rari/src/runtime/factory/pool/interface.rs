use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use rari_error::RariError;
use serde_json::Value;
use tokio::{sync::mpsc::unbounded_channel, time};

use super::{super::interface::AsyncBatchResult, JsRuntimePool};

fn pool_unavailable_error() -> RariError {
    RariError::js_runtime("No healthy JS runtime available in pool".to_string())
}

fn unavailable_future<T>() -> Pin<Box<dyn Future<Output = Result<T, RariError>> + Send>>
where
    T: Send + 'static,
{
    Box::pin(async move { Err(pool_unavailable_error()) })
}

fn timeout_error(label: &str, timeout_ms: u64) -> RariError {
    RariError::timeout(format!("{label} timed out after {timeout_ms} ms"))
}

impl JsRuntimePool {
    /// Execute a script on one healthy runtime, or on every healthy runtime when
    /// `setup_mode` is enabled. Setup broadcast keeps the first successful value and
    /// does not mark failed slots unhealthy (bootstrap should stay retryable).
    pub fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        let timeout_ms = self.timeout_ms;
        if self.setup_mode.load(Ordering::Acquire) {
            let runtimes = self.runtimes.clone();
            let healthy_snapshot: Vec<bool> =
                self.healthy.iter().map(|h| h.load(Ordering::Acquire)).collect();
            Box::pin(async move {
                match time::timeout(Duration::from_millis(timeout_ms), async {
                    let mut errors: Vec<String> = Vec::new();
                    let mut first_value: Option<Value> = None;
                    let mut executed: usize = 0;
                    for (idx, runtime) in runtimes.iter().enumerate() {
                        if !healthy_snapshot[idx] {
                            continue;
                        }
                        executed += 1;
                        match runtime.execute_script(script_name.clone(), script_code.clone()).await
                        {
                            Ok(v) => {
                                if first_value.is_none() {
                                    first_value = Some(v);
                                }
                            }
                            Err(e) => {
                                errors.push(format!("runtime[{idx}]: {e}"));
                            }
                        }
                    }

                    if executed == 0 {
                        Err(pool_unavailable_error())
                    } else if errors.is_empty() {
                        Ok(first_value.unwrap_or(Value::Null))
                    } else {
                        Err(RariError::js_runtime(format!(
                            "Failed to broadcast execute_script {script_name} on {} of {} runtimes: {}",
                            errors.len(),
                            executed,
                            errors.join("; ")
                        )))
                    }
                })
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(timeout_error("execute_script", timeout_ms)),
                }
            })
        } else {
            let runtime = match self.pick() {
                Some(idx) => Arc::clone(&self.runtimes[idx]),
                None => return unavailable_future(),
            };
            Box::pin(async move {
                match time::timeout(
                    Duration::from_millis(timeout_ms),
                    runtime.execute_script(script_name, script_code),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(timeout_error("execute_script", timeout_ms)),
                }
            })
        }
    }

    pub fn execute_script_batch(&self, scripts: Vec<(String, String)>) -> AsyncBatchResult {
        let runtime = match self.pick() {
            Some(idx) => Arc::clone(&self.runtimes[idx]),
            None => {
                let (tx, rx) = unbounded_channel();
                for (idx, _) in scripts.iter().enumerate() {
                    let _ = tx.send((idx, Err(pool_unavailable_error())));
                }
                return Box::pin(async move { rx });
            }
        };
        Box::pin(async move { runtime.execute_script_batch(scripts).await })
    }

    pub fn execute_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send + 'static>> {
        let timeout_ms = self.timeout_ms;
        let runtime = match self.pick() {
            Some(idx) => Arc::clone(&self.runtimes[idx]),
            None => return unavailable_future(),
        };
        let function_name = function_name.to_string();
        Box::pin(async move {
            match time::timeout(
                Duration::from_millis(timeout_ms),
                runtime.execute_function(&function_name, args),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => Err(timeout_error("execute_function", timeout_ms)),
            }
        })
    }
}
