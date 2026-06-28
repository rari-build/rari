use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, atomic::Ordering},
};

use rari_error::RariError;
use serde_json::Value as JsonValue;

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

impl JsRuntimePool {
    pub fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
        if self.setup_mode.load(Ordering::Acquire) {
            let runtimes = self.runtimes.clone();
            let healthy_snapshot: Vec<bool> =
                self.healthy.iter().map(|h| h.load(Ordering::Acquire)).collect();
            Box::pin(async move {
                let mut errors: Vec<String> = Vec::new();
                let mut first_value: Option<JsonValue> = None;
                let mut executed: usize = 0;
                for (idx, runtime) in runtimes.iter().enumerate() {
                    if !healthy_snapshot[idx] {
                        continue;
                    }
                    executed += 1;
                    match runtime.execute_script(script_name.clone(), script_code.clone()).await {
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
                    Ok(first_value.unwrap_or(JsonValue::Null))
                } else {
                    Err(RariError::js_runtime(format!(
                        "Failed to broadcast execute_script {script_name} on {} of {} runtimes: {}",
                        errors.len(),
                        executed,
                        errors.join("; ")
                    )))
                }
            })
        } else {
            let runtime = match self.pick() {
                Some(idx) => Arc::clone(&self.runtimes[idx]),
                None => return unavailable_future(),
            };
            Box::pin(async move { runtime.execute_script(script_name, script_code).await })
        }
    }

    pub fn execute_script_batch(&self, scripts: Vec<(String, String)>) -> AsyncBatchResult {
        let runtime = match self.pick() {
            Some(idx) => Arc::clone(&self.runtimes[idx]),
            None => {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
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
        args: Vec<JsonValue>,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send + 'static>> {
        let runtime = match self.pick() {
            Some(idx) => Arc::clone(&self.runtimes[idx]),
            None => return unavailable_future(),
        };
        let function_name = function_name.to_string();
        Box::pin(async move { runtime.execute_function(&function_name, args).await })
    }

    pub fn add_module_to_loader(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let runtimes = self.runtimes.clone();
        let specifier = specifier.to_string();
        let healthy_snapshot: Vec<bool> =
            self.healthy.iter().map(|h| h.load(Ordering::Acquire)).collect();
        Box::pin(async move {
            let mut errors: Vec<String> = Vec::new();
            let mut executed: usize = 0;
            for (idx, runtime) in runtimes.iter().enumerate() {
                if !healthy_snapshot[idx] {
                    continue;
                }
                executed += 1;
                if let Err(e) = runtime.add_module_to_loader(&specifier).await {
                    errors.push(format!("runtime[{idx}]: {e}"));
                }
            }

            if executed == 0 {
                Err(pool_unavailable_error())
            } else if errors.is_empty() {
                Ok(())
            } else {
                Err(RariError::js_runtime(format!(
                    "Failed to broadcast add_module_to_loader on {} of {} runtimes: {}",
                    errors.len(),
                    executed,
                    errors.join("; ")
                )))
            }
        })
    }

    pub fn add_module_to_loader_only(
        &self,
        specifier: &str,
        code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let runtimes = self.runtimes.clone();
        let specifier = specifier.to_string();
        let healthy_snapshot: Vec<bool> =
            self.healthy.iter().map(|h| h.load(Ordering::Acquire)).collect();
        Box::pin(async move {
            let mut errors: Vec<String> = Vec::new();
            let mut executed: usize = 0;
            for (idx, runtime) in runtimes.iter().enumerate() {
                if !healthy_snapshot[idx] {
                    continue;
                }
                executed += 1;
                if let Err(e) = runtime.add_module_to_loader_only(&specifier, code.clone()).await {
                    errors.push(format!("runtime[{idx}]: {e}"));
                }
            }
            if executed == 0 {
                Err(pool_unavailable_error())
            } else if errors.is_empty() {
                Ok(())
            } else {
                Err(RariError::js_runtime(format!(
                    "Failed to broadcast add_module_to_loader_only on {} of {} runtimes: {}",
                    errors.len(),
                    executed,
                    errors.join("; ")
                )))
            }
        })
    }

    pub fn clear_module_loader_caches(
        &self,
        component_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let runtimes = self.runtimes.clone();
        let component_id = component_id.to_string();
        let healthy_snapshot: Vec<bool> =
            self.healthy.iter().map(|h| h.load(Ordering::Acquire)).collect();
        Box::pin(async move {
            let mut errors: Vec<String> = Vec::new();
            let mut executed: usize = 0;
            for (idx, runtime) in runtimes.iter().enumerate() {
                if !healthy_snapshot[idx] {
                    continue;
                }
                executed += 1;
                if let Err(e) = runtime.clear_module_loader_caches(&component_id).await {
                    errors.push(format!("runtime[{idx}]: {e}"));
                }
            }
            if executed == 0 {
                Err(pool_unavailable_error())
            } else if errors.is_empty() {
                Ok(())
            } else {
                Err(RariError::js_runtime(format!(
                    "Failed to broadcast clear_module_loader_caches for {component_id} on {} of {} runtimes: {}",
                    errors.len(),
                    executed,
                    errors.join("; ")
                )))
            }
        })
    }
}
