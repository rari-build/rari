use std::{
    future::Future,
    sync::{Arc, atomic::Ordering},
};

use rari_error::RariError;

use super::{
    super::interface::JsRuntimeInterface,
    JsRuntimePool,
    component_helpers::{build_invalidate_script, dispatch_load_component, sanitize},
};

type DynRuntime = Arc<dyn JsRuntimeInterface>;

impl JsRuntimePool {
    async fn broadcast_to_healthy<F, Fut, T, FormatError, MakeAggregateError>(
        &self,
        mut format_error: FormatError,
        make_aggregate_error: MakeAggregateError,
        mut op: F,
    ) -> Result<(), RariError>
    where
        F: FnMut(usize, DynRuntime) -> Fut,
        Fut: Future<Output = Result<T, RariError>>,
        FormatError: FnMut(usize, &RariError) -> String,
        MakeAggregateError: FnOnce(usize, &[String]) -> RariError,
    {
        let healthy_snapshot: Vec<bool> =
            self.healthy.iter().map(|h| h.load(Ordering::Acquire)).collect();
        let mut errors: Vec<String> = Vec::new();
        let mut executed: usize = 0;
        for (idx, runtime) in self.runtimes.iter().cloned().enumerate() {
            if !healthy_snapshot[idx] {
                continue;
            }
            executed += 1;
            match op(idx, runtime).await {
                Ok(_) => {}
                Err(e) => {
                    self.mark_unhealthy(idx);
                    errors.push(format_error(idx, &e));
                }
            }
        }
        if executed == 0 {
            Err(RariError::js_runtime("No healthy JS runtime available in pool".to_string()))
        } else if errors.is_empty() {
            Ok(())
        } else {
            Err(make_aggregate_error(executed, &errors))
        }
    }

    pub async fn invalidate_component_all(&self, component_id: &str) -> Result<(), RariError> {
        let script = build_invalidate_script(component_id);
        let script_name = format!("invalidate_{}", sanitize(component_id));
        let component_id_msg = component_id.to_string();
        self.broadcast_to_healthy(
            |idx, err: &RariError| format!("runtime[{idx}]: {err}"),
            |total, errors: &[String]| RariError::js_runtime(format!(
                "Failed to invalidate component {component_id_msg} on {} of {total} runtimes: {}",
                errors.len(),
                errors.join("; ")
            )),
            |_idx, runtime: DynRuntime| {
                let script_name_local = script_name.clone();
                let script_local = script.clone();
                async move { runtime.execute_script(script_name_local, script_local).await }
            }
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
            |idx, err: &RariError| format!("runtime[{idx}]: {err}"),
            |total, errors: &[String]| RariError::js_runtime(format!(
                "Failed to load component code for {component_id_msg} on {} of {total} runtimes: {}",
                errors.len(),
                errors.join("; ")
            )),
            |_idx, runtime: DynRuntime| {
                let component_id_local = component_id_op.clone();
                let code_local = code.clone();
                async move {
                    dispatch_load_component(runtime.as_ref(), &component_id_local, &code_local)
                        .await
                }
            }
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
            |idx, err: &RariError| format!("runtime[{idx}]: {err}"),
            |total, errors: &[String]| {
                RariError::js_runtime(format!(
                    "Failed to broadcast script {script_name_msg} on {} of {total} runtimes: {}",
                    errors.len(),
                    errors.join("; ")
                ))
            },
            |_idx, runtime: DynRuntime| {
                let name_local = script_name_op.clone();
                let code_local = script_code.clone();
                async move { runtime.execute_script(name_local, code_local).await }
            },
        )
        .await
    }

    pub async fn broadcast_add_module_to_loader_only(
        &self,
        specifier: &str,
        code: &str,
    ) -> Result<(), RariError> {
        let specifier = specifier.to_string();
        let code = code.to_string();
        self.broadcast_to_healthy(
            |idx, err: &RariError| format!("runtime[{idx}]: {err}"),
            |total, errors: &[String]| {
                RariError::js_runtime(format!(
                    "Failed to broadcast add_module_to_loader_only on {} of {total} runtimes: {}",
                    errors.len(),
                    errors.join("; ")
                ))
            },
            |_idx, runtime: DynRuntime| {
                let specifier_local = specifier.clone();
                let code_local = code.clone();
                async move { runtime.add_module_to_loader_only(&specifier_local, code_local).await }
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
            |total, errors: &[String]| RariError::js_runtime(format!(
                "Failed to broadcast load+evaluate module {specifier} on {} of {total} runtimes: {}",
                errors.len(),
                errors.join("; ")
            )),
            |idx, runtime: DynRuntime| {
                let specifier_local = specifier.clone();
                async move {
                    let module_id =
                        runtime.load_es_module(&specifier_local).await.map_err(|e| {
                            RariError::js_execution(format!("runtime[{idx}].load_es_module: {e}"))
                        })?;
                    runtime.evaluate_module(module_id).await.map_err(|e| {
                        RariError::js_execution(format!("runtime[{idx}].evaluate_module: {e}"))
                    })
                }
            }
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
            |idx, err: &RariError| format!("runtime[{idx}]: {err}"),
            |total, errors: &[String]| RariError::js_runtime(format!(
                "Failed to broadcast clear_module_loader_caches for {component_id_msg} on {} of {total} runtimes: {}",
                errors.len(),
                errors.join("; ")
            )),
            |_idx, runtime: DynRuntime| {
                let component_id_local = component_id_op.clone();
                async move { runtime.clear_module_loader_caches(&component_id_local).await }
            }
        )
        .await
    }
}
