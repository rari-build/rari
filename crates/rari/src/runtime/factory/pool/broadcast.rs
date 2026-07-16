use std::{
    future::Future,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use component_ops::{build_invalidate_script, invalidate_script_name, load_component_code};
use futures::future::join_all;
use rari_error::RariError;
use tokio::time;

use super::{
    super::{component_ops, interface::JsRuntimeInterface},
    JsRuntimePool,
};

type DynRuntime = Arc<dyn JsRuntimeInterface>;

fn default_slot_error(idx: usize, err: &RariError) -> String {
    format!("runtime[{idx}]: {err}")
}

fn default_aggregate_error(label: &str, total: usize, errors: &[String]) -> RariError {
    RariError::js_runtime(format!(
        "{label} on {} of {total} runtimes: {}",
        errors.len(),
        errors.join("; ")
    ))
}

impl JsRuntimePool {
    async fn broadcast_to_healthy<F, Fut, T, FormatError, MakeAggregateError>(
        &self,
        format_error: FormatError,
        make_aggregate_error: MakeAggregateError,
        op: F,
    ) -> Result<(), RariError>
    where
        F: Fn(usize, DynRuntime) -> Fut,
        Fut: Future<Output = Result<T, RariError>>,
        FormatError: Fn(usize, &RariError) -> String,
        MakeAggregateError: FnOnce(usize, &[String]) -> RariError,
    {
        self.probe_and_heal().await;

        let healthy_snapshot: Vec<bool> =
            self.healthy.iter().map(|h| h.load(Ordering::Acquire)).collect();
        let timeout_ms = self.timeout_ms;

        let mut slot_futs = Vec::new();
        for (idx, healthy) in healthy_snapshot.iter().enumerate() {
            if !healthy {
                continue;
            }
            let Some(runtime) = self.runtime_at(idx) else {
                continue;
            };
            let pool = self;
            let fut = op(idx, Arc::clone(&runtime));
            slot_futs.push(async move {
                let _lease = match pool.acquire_slot_lease(idx).await {
                    Ok(guard) => guard,
                    Err(e) => return Err((idx, e)),
                };
                match time::timeout(Duration::from_millis(timeout_ms), fut).await {
                    Ok(Ok(_)) => Ok(()),
                    Ok(Err(e)) => Err((idx, e)),
                    Err(_) => {
                        Err((idx, RariError::timeout(format!("timed out after {timeout_ms} ms"))))
                    }
                }
            });
        }

        let executed = slot_futs.len();
        if executed == 0 {
            return Err(RariError::js_runtime(
                "No healthy JS runtime available in pool".to_string(),
            ));
        }

        let mut errors: Vec<String> = Vec::new();
        for result in join_all(slot_futs).await {
            match result {
                Ok(()) => {}
                Err((idx, e)) => {
                    self.mark_unhealthy(idx);
                    errors.push(format_error(idx, &e));
                }
            }
        }

        if errors.is_empty() { Ok(()) } else { Err(make_aggregate_error(executed, &errors)) }
    }

    pub async fn invalidate_component_all(&self, component_id: &str) -> Result<(), RariError> {
        let script = build_invalidate_script(component_id);
        let script_name = invalidate_script_name(component_id);
        let component_id_msg = component_id.to_string();
        self.broadcast_to_healthy(
            default_slot_error,
            |total, errors: &[String]| {
                default_aggregate_error(
                    &format!("Failed to invalidate component {component_id_msg}"),
                    total,
                    errors,
                )
            },
            |_idx, runtime: DynRuntime| {
                let script_name_local = script_name.clone();
                let script_local = script.clone();
                async move { runtime.execute_script(script_name_local, script_local).await }
            },
        )
        .await
    }

    pub async fn load_component_code_all(
        &self,
        component_id: &str,
        code: &str,
    ) -> Result<(), RariError> {
        let component_id_msg = component_id.to_string();
        let component_id_op = component_id.to_string();
        let code = code.to_string();
        self.broadcast_to_healthy(
            default_slot_error,
            |total, errors: &[String]| {
                default_aggregate_error(
                    &format!("Failed to load component code for {component_id_msg}"),
                    total,
                    errors,
                )
            },
            |_idx, runtime: DynRuntime| {
                let component_id_local = component_id_op.clone();
                let code_local = code.clone();
                async move {
                    load_component_code(runtime.as_ref(), &component_id_local, &code_local).await
                }
            },
        )
        .await
    }

    pub async fn broadcast_script(
        &self,
        script_name: &str,
        script_code: &str,
    ) -> Result<(), RariError> {
        let script_name_msg = script_name.to_string();
        let script_name_op = script_name.to_string();
        let script_code = script_code.to_string();
        self.broadcast_to_healthy(
            default_slot_error,
            |total, errors: &[String]| {
                default_aggregate_error(
                    &format!("Failed to broadcast script {script_name_msg}"),
                    total,
                    errors,
                )
            },
            |_idx, runtime: DynRuntime| {
                let name_local = script_name_op.clone();
                let code_local = script_code.clone();
                async move { runtime.execute_script(name_local, code_local).await }
            },
        )
        .await
    }

    pub async fn broadcast_add_module_to_loader(
        &self,
        specifier: &str,
        code: &str,
    ) -> Result<(), RariError> {
        let specifier = specifier.to_string();
        let code = code.to_string();
        self.broadcast_to_healthy(
            default_slot_error,
            |total, errors: &[String]| {
                default_aggregate_error("Failed to broadcast add_module_to_loader", total, errors)
            },
            |_idx, runtime: DynRuntime| {
                let specifier_local = specifier.clone();
                let code_local = code.clone();
                async move { runtime.add_module_to_loader(&specifier_local, code_local).await }
            },
        )
        .await
    }

    pub async fn broadcast_load_and_evaluate_module(
        &self,
        specifier: &str,
    ) -> Result<(), RariError> {
        let specifier = specifier.to_string();
        self.broadcast_to_healthy(
            |_idx, err: &RariError| err.to_string(),
            |total, errors: &[String]| {
                RariError::js_runtime(format!(
                    "Failed to broadcast load+evaluate module {specifier} on {} of {total} runtimes: {}",
                    errors.len(),
                    errors.join("; ")
                ))
            },
            |idx, runtime: DynRuntime| {
                let specifier_local = specifier.clone();
                async move {
                    let module_id = runtime.load_es_module(&specifier_local).await.map_err(|e| {
                        RariError::js_execution(format!("runtime[{idx}].load_es_module: {e}"))
                    })?;
                    runtime.evaluate_module(module_id).await.map_err(|e| {
                        RariError::js_execution(format!("runtime[{idx}].evaluate_module: {e}"))
                    })
                }
            },
        )
        .await
    }

    pub async fn broadcast_clear_module_loader_caches(
        &self,
        component_id: &str,
    ) -> Result<(), RariError> {
        let component_id_msg = component_id.to_string();
        let component_id_op = component_id.to_string();
        self.broadcast_to_healthy(
            default_slot_error,
            |total, errors: &[String]| {
                default_aggregate_error(
                    &format!(
                        "Failed to broadcast clear_module_loader_caches for {component_id_msg}"
                    ),
                    total,
                    errors,
                )
            },
            |_idx, runtime: DynRuntime| {
                let component_id_local = component_id_op.clone();
                async move { runtime.clear_module_loader_caches(&component_id_local).await }
            },
        )
        .await
    }
}
