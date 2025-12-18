use crate::error::RariError;
use crate::runtime::module_loader::RariModuleLoader;
use crate::runtime::ops::StreamOpState;
use crate::runtime::runtime_factory::constants::*;
use crate::runtime::runtime_factory::v8_utils::{
    is_promise, run_event_loop_with_error_handling, run_event_loop_with_promise_timeout, v8_to_json,
};
use crate::with_scope;
use deno_core::{JsRuntime, PollEventLoopOptions};
use serde_json::Value as JsonValue;
use std::rc::Rc;
use tokio::sync::mpsc;
use tracing::error;

pub fn has_export_statement(code: &str) -> bool {
    for line in code.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
            continue;
        }

        if trimmed.starts_with("export ")
            || trimmed.starts_with("export{")
            || trimmed.starts_with("export {")
        {
            return true;
        }
    }

    false
}

pub async fn execute_script(
    runtime: &mut JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    script_name: &str,
    script_code: &str,
) -> Result<JsonValue, RariError> {
    if let Some(cached_result) = module_loader.module_caching.get(script_name) {
        return Ok(cached_result);
    }

    let script_code_string = script_code.to_string();

    let is_registrable_module = script_name.starts_with(COMPONENT_PREFIX)
        && !script_name.starts_with(VERIFY_REGISTRATION_PREFIX);

    if module_loader.is_already_evaluated(script_name) {
        return Ok(create_already_evaluated_response(script_name));
    }

    let has_actual_module_syntax = script_code.trim().starts_with("import ")
        || script_code.contains("\"use module\"")
        || has_export_statement(script_code);

    if is_registrable_module || has_actual_module_syntax {
        return execute_as_module(
            runtime,
            module_loader,
            script_name,
            &script_code_string,
            is_registrable_module,
        )
        .await;
    }

    execute_as_script(runtime, module_loader, script_name, &script_code_string).await
}

async fn execute_as_module(
    runtime: &mut JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    script_name: &str,
    script_code: &str,
    is_registrable_module: bool,
) -> Result<JsonValue, RariError> {
    let specifier_str = module_loader.create_specifier(script_name, "rari_internal");

    module_loader.add_module(&specifier_str, script_name, script_code.to_string());

    let specifier = deno_core::resolve_url(&specifier_str).map_err(|url_err| {
        RariError::js_execution(format!(
            "Failed to create module specifier for '{script_name}': {url_err}"
        ))
    })?;

    let module_id_future = runtime.load_side_es_module(&specifier);
    let module_id_result = module_id_future.await;

    let module_id: usize = match module_id_result {
        Ok(id) => id,
        Err(load_err) => {
            if load_err.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR) {
                println!(
                    "[RARI_HMR] Module '{script_name}' already loaded, continuing with evaluation"
                );
                return Ok(create_already_loaded_response(script_name));
            } else {
                return Err(RariError::js_execution(format!(
                    "Failed to load module '{script_name}': {load_err}"
                )));
            }
        }
    };

    let eval_completion_future = runtime.mod_evaluate(module_id);
    let eval_result = eval_completion_future.await;

    match eval_result {
        Ok(_) => {
            run_event_loop_with_error_handling(
                runtime,
                &format!("module execution for '{script_name}'"),
            )
            .await?;
        }
        Err(eval_err) => {
            if eval_err.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR) {
                println!("[RARI_HMR] Module '{script_name}' already evaluated, continuing");
            } else {
                return Err(RariError::js_execution(format!(
                    "Failed to evaluate module '{script_name}': {eval_err}"
                )));
            }
        }
    }

    if is_registrable_module {
        handle_component_registration(runtime, module_loader, script_name, &specifier_str).await?;
    }

    module_loader.mark_module_evaluated(script_name);

    Ok(JsonValue::Null)
}

async fn handle_component_registration(
    runtime: &mut JsRuntime,
    _module_loader: &Rc<RariModuleLoader>,
    script_name: &str,
    specifier_str: &str,
) -> Result<(), RariError> {
    let registration_script = create_registration_script(specifier_str, script_name);

    let reg_result = match runtime.execute_script("registration_script", registration_script) {
        Ok(result) => result,
        Err(err) => {
            let error_str = err.to_string();
            if error_str.contains(MODULE_ALREADY_EVALUATED_ERROR) || error_str.contains("assertion")
            {
                return Err(RariError::js_runtime(format!(
                    "Runtime error during registration: {err}"
                )));
            }

            return Err(RariError::js_execution(format!(
                "Failed to execute registration script for '{script_name}': {err}"
            )));
        }
    };

    run_event_loop_with_error_handling(
        runtime,
        &format!("component registration for '{script_name}'"),
    )
    .await?;

    let json_result = with_scope!(runtime, |scope| {
        let local_result = deno_core::v8::Local::new(scope, reg_result);
        v8_to_json(scope, local_result)
    })?;

    if let JsonValue::Object(result_obj) = &json_result
        && let Some(JsonValue::Bool(success)) = result_obj.get("success")
        && !success
        && let Some(JsonValue::String(error)) = result_obj.get("error")
    {
        println!("[RARI_ERROR] Registration failed: {error}");
        return Err(RariError::js_execution(format!(
            "Failed to register component '{script_name}': {error}"
        )));
    }

    Ok(())
}

async fn execute_as_script(
    runtime: &mut JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    script_name: &str,
    script_code: &str,
) -> Result<JsonValue, RariError> {
    match runtime.execute_script("script", script_code.to_string()) {
        Ok(_global_v8_val) => {
            run_event_loop_with_error_handling(
                runtime,
                &format!("initial simple exec for '{script_name}'"),
            )
            .await?;

            let is_promise_result = with_scope!(runtime, |scope| {
                let local_v8_val = deno_core::v8::Local::new(scope, &_global_v8_val);
                is_promise(scope, local_v8_val)
            });

            if is_promise_result {
                handle_promise_result(runtime, script_name, _global_v8_val).await
            } else {
                handle_non_promise_result(runtime, _global_v8_val)
            }
        }
        Err(e) => {
            handle_script_error(runtime, module_loader, script_name, script_code, e.into()).await
        }
    }
}

async fn handle_promise_result(
    runtime: &mut JsRuntime,
    script_name: &str,
    _global_v8_val: deno_core::v8::Global<deno_core::v8::Value>,
) -> Result<JsonValue, RariError> {
    let setup_promise_storage = r#"
        (function() {
            if (!globalThis['~promises']) globalThis['~promises'] = {};
            globalThis['~promises'].currentObject = __temp_promise_ref__;
        })()
        "#;

    with_scope!(runtime, |scope| {
        let local_v8_val = deno_core::v8::Local::new(scope, &_global_v8_val);
        let context = scope.get_current_context();
        let global = context.global(scope);
        let key = match deno_core::v8::String::new(scope, "__temp_promise_ref__") {
            Some(key) => key,
            None => {
                error!("Failed to create V8 string for __temp_promise_ref__");
                return Err(RariError::internal("Failed to create V8 string".to_string()));
            }
        };
        global.set(scope, key.into(), local_v8_val);
        Ok::<(), RariError>(())
    })?;

    runtime
        .execute_script("store_promise", setup_promise_storage.to_string())
        .map_err(|e| RariError::js_execution(format!("Failed to store promise: {}", e)))?;

    let setup_script = PROMISE_SETUP_SCRIPT;

    match runtime.execute_script(format!("{script_name}_promise_setup"), setup_script.to_string()) {
        Ok(_) => {
            let promise_timeout_ms = std::env::var("RARI_PROMISE_RESOLUTION_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5000);

            run_event_loop_with_promise_timeout(runtime, script_name, promise_timeout_ms).await?;

            let extract_script = PROMISE_EXTRACT_SCRIPT;

            match runtime
                .execute_script(format!("{script_name}_extract_value"), extract_script.to_string())
            {
                Ok(extracted_value) => {
                    let json_result = with_scope!(runtime, |scope| {
                        let local_v8_val = deno_core::v8::Local::new(scope, extracted_value);
                        v8_to_json(scope, local_v8_val)
                    })?;

                    if let JsonValue::Object(ref obj) = json_result
                        && let Some(JsonValue::Bool(true)) = obj.get("~error")
                    {
                        let message =
                            obj.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                        let stack = obj.get("stack").and_then(|v| v.as_str());

                        return Err(RariError::js_execution(if let Some(stack_trace) = stack {
                            format!("{}\n{}", message, stack_trace)
                        } else {
                            message.to_string()
                        }));
                    }

                    Ok(json_result)
                }
                Err(_) => with_scope!(runtime, |scope| {
                    let local_v8_val = deno_core::v8::Local::new(scope, _global_v8_val);
                    v8_to_json(scope, local_v8_val)
                }),
            }
        }
        Err(_) => with_scope!(runtime, |scope| {
            let local_v8_val = deno_core::v8::Local::new(scope, _global_v8_val);
            v8_to_json(scope, local_v8_val)
        }),
    }
}

fn handle_non_promise_result(
    runtime: &mut JsRuntime,
    _global_v8_val: deno_core::v8::Global<deno_core::v8::Value>,
) -> Result<JsonValue, RariError> {
    let json_result = with_scope!(runtime, |scope| {
        let local_v8_val = deno_core::v8::Local::new(scope, _global_v8_val);
        v8_to_json(scope, local_v8_val)
    })?;

    if let JsonValue::Object(ref obj) = json_result
        && let Some(JsonValue::Bool(true)) = obj.get("~error")
    {
        let message = obj.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        let stack = obj.get("stack").and_then(|v| v.as_str());

        return Err(RariError::js_execution(if let Some(stack_trace) = stack {
            format!("{}\n{}", message, stack_trace)
        } else {
            message.to_string()
        }));
    }

    Ok(json_result)
}

async fn handle_script_error(
    runtime: &mut JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    script_name: &str,
    script_code: &str,
    e: deno_core::error::AnyError,
) -> Result<JsonValue, RariError> {
    let error_string = e.to_string();

    if error_string.contains("SyntaxError") {
        return Err(RariError::js_execution(format!(
            "Syntax error in script '{script_name}': {error_string}"
        )));
    }

    if error_string.contains("assertion") || error_string.contains("panicked") {
        return Err(RariError::js_runtime(format!("Critical runtime error: {error_string}")));
    }

    if error_string.contains("Error")
        && script_code.contains("throw")
        && !(error_string.contains("module.exports is not supported")
            && script_code.contains("typeof module"))
    {
        return Err(RariError::js_execution(format!(
            "Runtime error in script '{script_name}': {error_string}"
        )));
    }

    if should_retry_as_module(&error_string, script_code) {
        retry_as_module(runtime, module_loader, script_name, script_code).await
    } else {
        Err(RariError::js_execution(format!(
            "Failed to execute script '{script_name}': {error_string}"
        )))
    }
}

fn should_retry_as_module(error_string: &str, script_code: &str) -> bool {
    error_string.contains("Cannot use import statement")
        || error_string.contains("Unexpected token 'export'")
        || (error_string.contains("ReferenceError")
            && !script_code.contains("throw")
            && (script_code.contains("URL")
                || script_code.contains("fetch")
                || script_code.contains("process")))
        || (error_string.contains("module.exports is not supported")
            && script_code.contains("typeof"))
}

async fn retry_as_module(
    runtime: &mut JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    script_name: &str,
    script_code: &str,
) -> Result<JsonValue, RariError> {
    let specifier_str = module_loader.create_specifier(script_name, "rari_internal");
    let module_code = module_loader.transform_to_esmodule(script_code, script_name);

    module_loader.add_module(&specifier_str, script_name, module_code);

    let specifier = deno_core::resolve_url(&specifier_str).map_err(|url_err| {
        RariError::js_execution(format!(
            "Failed to create module specifier for '{script_name}': {url_err}"
        ))
    })?;

    let module_id: usize = runtime.load_side_es_module(&specifier).await.map_err(|load_err| {
        if load_err.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR)
            || load_err.to_string().contains("assertion")
        {
            RariError::js_runtime(format!("Runtime error loading module: {load_err}"))
        } else {
            RariError::js_execution(format!("Failed to load module '{script_name}': {load_err}"))
        }
    })?;

    runtime.mod_evaluate(module_id).await.map_err(|eval_err| {
        if eval_err.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR)
            || eval_err.to_string().contains("assertion")
        {
            RariError::js_runtime(format!("Runtime error evaluating module: {eval_err}"))
        } else {
            RariError::js_execution(format!(
                "Failed to evaluate module '{script_name}': {eval_err}"
            ))
        }
    })?;

    run_event_loop_with_error_handling(runtime, &format!("module exec for '{script_name}'"))
        .await?;

    Ok(JsonValue::Null)
}

pub async fn execute_script_for_streaming(
    runtime: &mut JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    script_name: &str,
    script_code: &str,
    chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
) -> Result<(), RariError> {
    {
        let op_state_rc = runtime.op_state();
        let mut op_state = op_state_rc.borrow_mut();
        if let Some(stream_state) = op_state.try_borrow_mut::<StreamOpState>() {
            stream_state.chunk_sender = Some(chunk_sender);
        } else {
            return Err(RariError::js_runtime(
                "StreamOpState not available in runtime".to_string(),
            ));
        }
    }

    let specifier_str = module_loader.create_specifier(script_name, "rari_internal_streaming");
    let module_code = module_loader.transform_to_esmodule(script_code, script_name);

    module_loader.add_module(&specifier_str, script_name, module_code);

    let specifier = deno_core::resolve_url(&specifier_str).map_err(|e| {
        RariError::js_execution(format!(
            "Failed to create module specifier for streaming '{script_name}': {e}"
        ))
    })?;

    let module_id: usize = runtime.load_side_es_module(&specifier).await.map_err(|e| {
        RariError::js_execution(format!("Failed to load streaming module '{script_name}': {e}"))
    })?;

    runtime.mod_evaluate(module_id).await.map_err(|e| {
        RariError::js_execution(format!("Failed to evaluate streaming module '{script_name}': {e}"))
    })?;

    runtime.run_event_loop(PollEventLoopOptions::default()).await.map_err(|e| {
        RariError::js_execution(format!(
            "Event loop error after streaming module '{script_name}': {e}"
        ))
    })?;

    Ok(())
}
