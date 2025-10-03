use crate::error::RariError;
use crate::runtime::module_loader::{RariModuleLoader, RuntimeConfig};
use crate::runtime::ops::StreamOpState;
use deno_core::{Extension, JsRuntime, ModuleSpecifier, PollEventLoopOptions, RuntimeOptions, v8};
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::error;

const CHANNEL_CAPACITY: usize = 32;
const RUNTIME_RESTART_DELAY_MS: u64 = 1000;
const RUNTIME_QUICK_RESTART_DELAY_MS: u64 = 100;

const COMPONENT_PREFIX: &str = "component_";
const VERIFY_REGISTRATION_PREFIX: &str = "verify_registration_";
const RARI_COMPONENT_PREFIX: &str = "file:///rari_component/";
const RARI_REGISTER_FUNCTION: &str = "__rari_register";
const JS_EXTENSION: &str = ".js";

const MODULE_ALREADY_EVALUATED_ERROR: &str = "Module already evaluated";
const JS_EXECUTOR_FAILED_ERROR: &str = "JS executor failed to respond";
const JS_EXECUTOR_CHANNEL_CLOSED_ERROR: &str = "JS executor channel closed";
const RUNTIME_RESTART_MESSAGE: &str =
    "Runtime is being restarted for stability. Please retry your request.";

const ENV_INJECTION_SCRIPT: &str = r#"
(() => {
    if (!globalThis.process.env) {
        globalThis.process.env = {};
    }

    const envVars = {};
    Object.assign(globalThis.process.env, envVars);

    return Object.keys(envVars).length;
})();
"#;

const MODULE_CHECK_SCRIPT: &str = r#"
(function() {
    if (!globalThis.RscModuleManager) {
        return { available: false, extension: 'rsc_modules' };
    }
    return { available: true, extension: 'rsc_modules' };
})()
"#;

type LazyRuntimeInner = Arc<tokio::sync::Mutex<Option<Box<dyn JsRuntimeInterface>>>>;

fn is_critical_error(error: &RariError) -> bool {
    let error_str = error.to_string();
    error_str.contains("assertion") || error_str.contains("panicked")
}

fn is_runtime_restart_needed(error: &RariError) -> bool {
    let error_str = error.to_string();
    error_str.contains(MODULE_ALREADY_EVALUATED_ERROR)
        || error_str.contains(JS_EXECUTOR_FAILED_ERROR)
        || error_str.contains(JS_EXECUTOR_CHANNEL_CLOSED_ERROR)
}

fn create_graceful_error() -> RariError {
    RariError::js_runtime(RUNTIME_RESTART_MESSAGE.to_string())
}

fn get_module_namespace_as_json(
    runtime: &mut JsRuntime,
    module_id: deno_core::ModuleId,
) -> Result<JsonValue, RariError> {
    match runtime.get_module_namespace(module_id) {
        Ok(namespace) => {
            let mut scope = runtime.handle_scope();
            let local_namespace = v8::Local::new(&mut scope, namespace);
            let local_value: v8::Local<v8::Value> = local_namespace.into();
            v8_to_json(&mut scope, local_value)
        }
        Err(e) => Err(RariError::js_execution(format!("Failed to get module namespace: {e}"))),
    }
}

#[allow(clippy::disallowed_methods)]
fn create_already_evaluated_response(component_name: &str) -> JsonValue {
    serde_json::json!({
        "status": "already_evaluated",
        "component": component_name
    })
}

#[allow(clippy::disallowed_methods)]
fn create_already_loaded_response(component_name: &str) -> JsonValue {
    serde_json::json!({
        "status": "already_loaded",
        "component": component_name
    })
}

fn extract_component_id_from_specifier(specifier: &str) -> &str {
    if specifier.starts_with(RARI_COMPONENT_PREFIX) {
        specifier
            .strip_prefix(RARI_COMPONENT_PREFIX)
            .unwrap_or(specifier)
            .strip_suffix(JS_EXTENSION)
            .unwrap_or(specifier)
    } else {
        specifier
    }
}

fn create_registration_script(specifier_str: &str, script_name: &str) -> String {
    format!(
        r#"
        (async function() {{
            try {{
                const module = await import("{specifier_str}");
                if (typeof module.{RARI_REGISTER_FUNCTION} === 'function') {{
                    const result = module.{RARI_REGISTER_FUNCTION}.call(module);
                    return {{ success: true, result }};
                }} else {{
                    return {{ success: false, error: 'No {RARI_REGISTER_FUNCTION} function found' }};
                }}
            }} catch (e) {{
                console.error("[RARI_ERROR] Failed to call {RARI_REGISTER_FUNCTION} for '{script_name}': " + e.message);
                return {{ success: false, error: e.message }};
            }}
        }})()
        "#
    )
}

async fn run_event_loop_with_error_handling(
    runtime: &mut JsRuntime,
    context: &str,
) -> Result<(), RariError> {
    match runtime.run_event_loop(PollEventLoopOptions::default()).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("assertion") || error_str.contains("panicked") {
                Err(RariError::js_runtime(format!(
                    "Critical runtime error in {context}: {error_str}"
                )))
            } else {
                Err(RariError::js_execution(format!("Event loop error in {context}: {error_str}")))
            }
        }
    }
}

pub trait JsRuntimeInterface: Send + Sync {
    fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>>;

    fn execute_function(
        &self,
        function_name: &str,
        args: Vec<JsonValue>,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send + 'static>>;

    fn execute_script_for_streaming(
        &self,
        script_name: String,
        script_code: String,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn add_module_to_loader(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn load_es_module(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>>;

    fn evaluate_module(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>>;

    fn get_module_namespace(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>>;

    fn add_module_to_loader_only(
        &self,
        specifier: &str,
        code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn clear_module_loader_caches(
        &self,
        component_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;
}

enum JsRequest {
    ExecuteScript {
        script_name: String,
        script_code: String,
        result_tx: oneshot::Sender<Result<JsonValue, RariError>>,
    },
    ExecuteScriptForStreaming {
        script_name: String,
        script_code: String,
        stream_id: String,
        result_tx: oneshot::Sender<Result<(), RariError>>,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
    },
    AddModuleToLoader {
        component_id: String,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
    LoadEsModule {
        component_id: String,
        result_tx: oneshot::Sender<Result<deno_core::ModuleId, RariError>>,
    },
    EvaluateModule {
        module_id: deno_core::ModuleId,
        result_tx: oneshot::Sender<Result<JsonValue, RariError>>,
    },
    GetModuleNamespace {
        module_id: usize,
        result_tx: oneshot::Sender<Result<JsonValue, RariError>>,
    },
    AddModuleToLoaderOnly {
        specifier: String,
        code: String,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
    ClearModuleLoaderCaches {
        component_id: String,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
}

struct DenoRuntime {
    request_sender: mpsc::Sender<JsRequest>,
}

impl DenoRuntime {
    fn new(env_vars: Option<FxHashMap<String, String>>) -> Self {
        let (request_sender, mut request_receiver) = mpsc::channel(CHANNEL_CAPACITY);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");

            let _ = runtime.block_on(async {
                                    loop {
                        let (mut deno_runtime, module_loader) = match create_deno_runtime(env_vars.clone()) {
                            Ok(rt) => rt,
                            Err(_) => {
                                tokio::time::sleep(std::time::Duration::from_millis(RUNTIME_RESTART_DELAY_MS)).await;
                                continue;
                            }
                        };

                    let _evaluated_modules: FxHashMap<deno_core::ModuleId, bool> = FxHashMap::default();

                    let mut continue_processing = true;
                    while continue_processing {
                        match request_receiver.recv().await {
                            Some(request) => {
                                let result = match request {
                                    JsRequest::ExecuteScript { script_name, script_code, result_tx } => {
                                        let result = execute_script(&mut deno_runtime, &module_loader, &script_name, &script_code).await;

                                        if let Err(e) = &result
                                            && is_runtime_restart_needed(e) {
                                                let graceful_error = create_graceful_error();
                                                let _ = result_tx.send(Err(graceful_error));
                                                break;
                                            }

                                        let _ = result_tx.send(result);
                                        Ok::<(), RariError>(())
                                    },
                                    JsRequest::ExecuteScriptForStreaming {
                                        script_name,
                                        script_code,
                                        stream_id,
                                        result_tx,
                                        chunk_sender,
                                    } => {
                                        let sender_for_op_state = chunk_sender.clone();
                                                            deno_runtime.op_state().borrow_mut().put(StreamOpState {
                        chunk_sender: Some(sender_for_op_state),
                        current_stream_id: Some(stream_id.clone()),
                        row_counter: 0,
                    });

                                        let result = execute_script_for_streaming(
                                            &mut deno_runtime,
                                            &module_loader,
                                            &script_name,
                                            &script_code,
                                            chunk_sender,
                                        )
                                        .await;

                                        let _ = result_tx.send(result);
                                        Ok::<(), RariError>(())
                                    },
                                    JsRequest::AddModuleToLoader { component_id, result_tx } => {
                                        let specifier_opt = module_loader.get_component_specifier(&component_id);

                                        if let Some(_specifier) = specifier_opt {
                                            let _ = result_tx.send(Ok(()));
                                        } else {
                                            let _ = result_tx.send(Err(RariError::js_execution(format!(
                                                "Component specifier not found in loader for AddModuleToLoader: {component_id}"
                                            ))));
                                        }
                                        Ok::<(), RariError>(())
                                    },
                                    JsRequest::LoadEsModule { component_id, result_tx } => {
                                        let specifier_opt = module_loader.get_component_specifier(&component_id);

                                        if let Some(specifier_str) = specifier_opt {
                                            let is_hmr_update = module_loader.is_hmr_module(&specifier_str);

                                            match ModuleSpecifier::parse(&specifier_str) {
                                                Ok(module_specifier) => {
                                                    let module_load_result = if is_hmr_update {
                                                        if let Some(versioned_specifier) = module_loader.get_versioned_specifier(&component_id) {
                                                            match ModuleSpecifier::parse(&versioned_specifier) {
                                                                Ok(versioned_module_specifier) => {
                                                                    let load_result = deno_runtime.load_side_es_module(&versioned_module_specifier).await;

                                                                    if let Ok(_module_id) = load_result {
                                                                    }

                                                                    load_result
                                                                },
                                                                Err(_) => {
                                                                    deno_runtime.load_side_es_module(&module_specifier).await
                                                                }
                                                            }
                                                        } else {
                                                            let load_result = deno_runtime.load_side_es_module(&module_specifier).await;

                                                            if let Ok(_module_id) = load_result {
                                                            }

                                                            load_result
                                                        }
                                                    } else {
                                                        deno_runtime.load_side_es_module(&module_specifier).await
                                                    };

                                                    let module_load_result = module_load_result.map_err(|e| {
                                                        RariError::js_execution(format!(
                                                            "Failed to load module '{component_id}' (specifier: '{module_specifier}'): {e}"
                                                        ))
                                                    });

                                                    let _ = result_tx.send(module_load_result);
                                                }
                                                Err(e) => {
                                                    let err_msg = format!(
                                                        "Invalid module specifier string '{specifier_str}' for component '{component_id}': {e}"
                                                    );
                                                    eprintln!("[RARI_ERROR] {err_msg}");
                                                    let _ = result_tx.send(Err(RariError::js_execution(err_msg)));
                                                }
                                            }
                                        } else {
                                            let err_msg =
                                                format!("Component specifier not found for LoadEsModule: {component_id}");
                                            eprintln!("[RARI_ERROR] {err_msg}");
                                            let _ = result_tx.send(Err(RariError::js_execution(err_msg)));
                                        }
                                        Ok::<(), RariError>(())
                                    },
                                    JsRequest::EvaluateModule { module_id, result_tx } => {
                                        let module_registered = module_loader.is_already_evaluated(&module_id.to_string());

                                        let result = if module_registered {
                                            println!("[RARI_HMR] Module {module_id} already registered, getting namespace");
                                            match get_module_namespace_as_json(&mut deno_runtime, module_id) {
                                                Ok(json_result) => Ok(json_result),
                                                Err(_) => {
                                                    Ok(create_already_evaluated_response("get_module_namespace"))
                                                }
                                            }
                                        } else {
                                            match deno_runtime.mod_evaluate(module_id).await {
                                                Ok(_) => {
                                                    module_loader.mark_module_evaluated(&module_id.to_string());

                                                    get_module_namespace_as_json(&mut deno_runtime, module_id)
                                                },
                                                Err(e) => {
                                                    if e.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR) {
                                                        println!("[RARI_HMR] Module '{module_id}' already evaluated during evaluation attempt");
                                                        module_loader.mark_module_evaluated(&module_id.to_string());

                                                        match get_module_namespace_as_json(&mut deno_runtime, module_id) {
                                                            Ok(json_result) => Ok(json_result),
                                                            Err(_) => {
                                                                Ok(create_already_evaluated_response("get_module_namespace"))
                                                            }
                                                        }
                                                    } else {
                                                        Err(RariError::js_execution(format!(
                                                            "Failed to evaluate module {module_id}: {e}"
                                                        )))
                                                    }
                                                }
                                            }
                                        };

                                        if let Err(e) = &result
                                            && is_critical_error(e) {
                                                println!("[RARI_RUNTIME] Critical error detected in module evaluation: {e}");
                                                continue_processing = false;
                                            }

                                        let _ = result_tx.send(result);
                                        Ok::<(), RariError>(())
                                    },
                                    JsRequest::GetModuleNamespace { module_id, result_tx } => {
                                        let module_evaluated = module_loader.is_already_evaluated(&module_id.to_string());

                                        if module_evaluated {
                                            let json_result = get_module_namespace_as_json(&mut deno_runtime, module_id as deno_core::ModuleId);
                                            let _ = result_tx.send(json_result);
                                        } else {
                                            match deno_runtime.mod_evaluate(module_id as deno_core::ModuleId).await {
                                                Ok(_) => {
                                                    module_loader.mark_module_evaluated(&module_id.to_string());
                                                    let json_result = get_module_namespace_as_json(&mut deno_runtime, module_id as deno_core::ModuleId);
                                                    let _ = result_tx.send(json_result);
                                                },
                                                Err(e) => {
                                                    if e.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR) {
                                                        module_loader.mark_module_evaluated(&module_id.to_string());
                                                        let json_result = get_module_namespace_as_json(&mut deno_runtime, module_id as deno_core::ModuleId);
                                                        let _ = result_tx.send(json_result);
                                                    } else {
                                                        let _ = result_tx.send(Err(RariError::js_execution(format!(
                                                            "Failed to evaluate module: {e}"
                                                        ))));
                                                    }
                                                }
                                            }
                                        }
                                        Ok::<(), RariError>(())
                                    },
                                                        JsRequest::AddModuleToLoaderOnly { specifier, code, result_tx } => {
                        module_loader.set_module_code(specifier.clone(), code.clone());

                        let component_id = extract_component_id_from_specifier(&specifier);

                        module_loader.component_specifiers.insert(component_id.to_string(), specifier.clone());

                        let _ = result_tx.send(Ok(()));
                        Ok::<(), RariError>(())
                    },
                    JsRequest::ClearModuleLoaderCaches { component_id, result_tx } => {
                        module_loader.clear_component_caches(&component_id);
                        let _ = result_tx.send(Ok(()));
                        Ok::<(), RariError>(())
                    },
                                };

                                if let Err(e) = result {
                                    eprintln!("[RARI_ERROR] Error processing request: {e}");
                                    break;
                                }

                                if let Err(e) = deno_runtime.run_event_loop(PollEventLoopOptions::default()).await {
                                    eprintln!("[RARI_WARN] Event loop error: {e}. Restarting runtime.");
                                }
                            },
                            None => {
                                return Ok::<(), RariError>(());
                            }
                        }
                    }

                    println!("[RARI_RUNTIME] Restarting JS runtime due to error or forced restart");
                    tokio::time::sleep(std::time::Duration::from_millis(RUNTIME_QUICK_RESTART_DELAY_MS)).await;
                }
            });
        });

        Self { request_sender }
    }
}

fn create_deno_runtime(
    env_vars: Option<FxHashMap<String, String>>,
) -> Result<(JsRuntime, Rc<RariModuleLoader>), RariError> {
    let module_loader = Rc::new(RariModuleLoader::new());

    let streaming_ops = get_streaming_ops();

    let ext_options = crate::runtime::ext::ExtensionOptions::default();
    let mut extensions = crate::runtime::ext::extensions(&ext_options, false);

    extensions.push(Extension {
        name: "rari:streaming",
        ops: Cow::Owned(streaming_ops),
        op_state_fn: Some(Box::new(|state| {
            state.put(StreamOpState::default());
            let mut feature_checker = deno_features::FeatureChecker::default();
            feature_checker.enable_feature("broadcast-channel");
            feature_checker.enable_feature("webgpu");
            feature_checker.enable_feature("kv");
            feature_checker.enable_feature("cron");
            feature_checker.enable_feature("net");
            feature_checker.enable_feature("worker-options");
            state.put(std::sync::Arc::new(feature_checker));
        })),
        ..Default::default()
    });

    let options = RuntimeOptions {
        module_loader: Some(module_loader.clone()),
        extensions,
        extension_transpiler: Some(module_loader.as_extension_transpiler()),
        ..Default::default()
    };

    let mut runtime = JsRuntime::new(options);

    if let Some(env_vars) = env_vars {
        let env_script = ENV_INJECTION_SCRIPT.replace(
            "const envVars = {};",
            &format!(
                "const envVars = {};",
                serde_json::to_string(&env_vars).unwrap_or_else(|_| "{}".to_string())
            ),
        );

        if let Err(err) = runtime.execute_script("env_vars.js", env_script) {
            eprintln!("[RARI_WARN] Failed to inject environment variables: {err}");
        }
    }

    if let Err(err) =
        runtime.execute_script("module_registration_check.js", MODULE_CHECK_SCRIPT.to_string())
    {
        eprintln!("[RARI_WARN] Failed to check module registration extension: {err}");
    }

    Ok((runtime, module_loader))
}

fn get_streaming_ops() -> Vec<deno_core::OpDecl> {
    crate::runtime::ops::get_streaming_ops()
}

fn v8_to_json(
    scope: &mut v8::HandleScope,
    value: v8::Local<v8::Value>,
) -> Result<JsonValue, RariError> {
    let try_json_stringify =
        |scope: &mut v8::HandleScope, value: v8::Local<v8::Value>| -> Option<JsonValue> {
            let global = scope.get_current_context().global(scope);
            let json_key = v8::String::new(scope, "JSON")?;
            let json_obj = global.get(scope, json_key.into())?.to_object(scope)?;
            let stringify_key = v8::String::new(scope, "stringify")?;
            let stringify_value = json_obj.get(scope, stringify_key.into())?;
            let stringify_fn = stringify_value.to_object(scope)?.cast::<v8::Function>();

            let args = [value];
            let result = stringify_fn.call(scope, json_obj.into(), &args)?;
            let json_string = result.to_string(scope)?.to_rust_string_lossy(scope);

            serde_json::from_str(&json_string).ok()
        };

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        deno_core::serde_v8::from_v8(scope, value)
    })) {
        Ok(Ok(json_value)) => Ok(json_value),
        Ok(Err(err)) => {
            if let Some(json_value) = try_json_stringify(scope, value) {
                return Ok(json_value);
            }

            let v8_type_str = value.type_of(scope).to_rust_string_lossy(scope);
            let detailed_err_msg = format!(
                "Failed to convert V8 value of type '{}' to JSON: {}. V8 value details: {}",
                v8_type_str,
                err,
                value
                    .to_detail_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "<unable to get detailed string for V8 value>".to_string())
            );
            Err(RariError::js_execution(detailed_err_msg))
        }
        Err(_panic) => {
            if let Some(json_value) = try_json_stringify(scope, value) {
                return Ok(json_value);
            }

            let v8_type_str = value.type_of(scope).to_rust_string_lossy(scope);
            let fallback_msg = format!(
                "V8 serialization panicked for type '{}', using fallback. V8 value details: {}",
                v8_type_str,
                value
                    .to_detail_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "<unable to get detailed string for V8 value>".to_string())
            );

            let mut error_obj = serde_json::Map::new();
            error_obj.insert("__serialization_error".to_string(), serde_json::Value::Bool(true));
            error_obj.insert(
                "error".to_string(),
                serde_json::Value::String("V8 value could not be serialized".to_string()),
            );
            error_obj.insert("type".to_string(), serde_json::Value::String(v8_type_str));
            error_obj.insert("details".to_string(), serde_json::Value::String(fallback_msg));
            Ok(serde_json::Value::Object(error_obj))
        }
    }
}

fn is_promise(scope: &mut v8::HandleScope, value: v8::Local<v8::Value>) -> bool {
    if !value.is_object() {
        return false;
    }

    if let Some(string_rep) = value.to_string(scope) {
        let string_val = string_rep.to_rust_string_lossy(scope);

        if string_val == "[object Promise]"
            && let Ok(obj) = v8::Local::<v8::Object>::try_from(value)
        {
            let then_key = match v8::String::new(scope, "then") {
                Some(key) => key.into(),
                None => return false,
            };
            let catch_key = match v8::String::new(scope, "catch") {
                Some(key) => key.into(),
                None => return false,
            };

            if let Some(then_val) = obj.get(scope, then_key)
                && let Some(catch_val) = obj.get(scope, catch_key)
            {
                let result = then_val.is_function() && catch_val.is_function();
                return result;
            }
        }
    }

    false
}

fn should_resolve_promises(script_name: &str) -> bool {
    script_name.starts_with("promise_test_")
        || script_name.starts_with("streaming_sim_")
        || script_name.starts_with("execute_action_")
        || script_name.starts_with("exec_func_")
        || script_name == "<streaming_init>"
        || script_name.starts_with("<partial_render_")
        || script_name.starts_with("<promise_resolution_")
        || script_name.contains("streaming")
        || script_name.contains("async")
        || (script_name.starts_with("render_") && script_name.contains("Suspense"))
        || (script_name.starts_with("render_") && script_name.contains("Streaming"))
        || (script_name.starts_with("extract_rsc_") && script_name.contains("Suspense"))
        || (script_name.starts_with("extract_rsc_") && script_name.contains("Streaming"))
        || (script_name.starts_with("render_") && script_name.contains("Fetch"))
        || (script_name.starts_with("render_") && script_name.contains("Async"))
        || (script_name.starts_with("render_")
            && (script_name.contains("Test")
                || script_name.contains("Component")
                || script_name.contains("Example")))
}

fn get_promise_resolution_timeout_ms(script_name: &str) -> u64 {
    let base_timeout = std::env::var("RARI_PROMISE_RESOLUTION_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000); // Default 2 second timeout

    if script_name.contains("Suspense") {
        base_timeout * 3
    } else if script_name.contains("Streaming") || script_name.contains("Fetch") {
        base_timeout * 2
    } else {
        base_timeout
    }
}

fn check_promise_completion(runtime: &mut JsRuntime) -> Result<bool, RariError> {
    let check_script = r#"
        (function() {
            return globalThis.__promise_resolution_complete === true;
        })()
    "#;

    match runtime.execute_script("promise_completion_check", check_script.to_string()) {
        Ok(result_val) => {
            let mut scope = runtime.handle_scope();
            let local_v8_val = v8::Local::new(&mut scope, result_val);

            let boolean_val = local_v8_val.to_boolean(&mut scope);
            Ok(boolean_val.is_true())
        }
        Err(_) => Ok(false),
    }
}

async fn run_event_loop_with_promise_timeout(
    runtime: &mut JsRuntime,
    script_name: &str,
    timeout_ms: u64,
) -> Result<(), RariError> {
    let timeout_duration = std::time::Duration::from_millis(timeout_ms);
    let start_time = std::time::Instant::now();
    let check_interval = std::time::Duration::from_millis(5);

    while start_time.elapsed() < timeout_duration {
        run_event_loop_with_error_handling(
            runtime,
            &format!("promise resolution iteration for '{script_name}'"),
        )
        .await?;

        if let Ok(is_complete) = check_promise_completion(runtime)
            && is_complete
        {
            break;
        }

        tokio::time::sleep(check_interval).await;
    }
    Ok(())
}

async fn execute_script(
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

    if is_registrable_module
        || script_code.trim().starts_with("import ")
        || script_code.trim().contains("export ")
        || script_code.contains("\"use module\"")
    {
        let specifier_str = module_loader.create_specifier(script_name, "rari_internal");

        module_loader.add_module(&specifier_str, script_name, script_code_string.clone());

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
            let registration_script = create_registration_script(&specifier_str, script_name);

            let reg_result =
                match runtime.execute_script("registration_script", registration_script) {
                    Ok(result) => result,
                    Err(err) => {
                        let error_str = err.to_string();
                        if error_str.contains(MODULE_ALREADY_EVALUATED_ERROR)
                            || error_str.contains("assertion")
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

            let mut scope = runtime.handle_scope();
            let local_result = v8::Local::new(&mut scope, reg_result);
            let json_result = v8_to_json(&mut scope, local_result)?;

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
        }

        module_loader.mark_module_evaluated(script_name);

        return Ok(JsonValue::Null);
    }

    match runtime.execute_script("script", script_code_string.clone()) {
        Ok(_global_v8_val) => {
            run_event_loop_with_error_handling(
                runtime,
                &format!("initial simple exec for '{script_name}'"),
            )
            .await?;

            let should_resolve = should_resolve_promises(script_name);

            let is_promise_result = if should_resolve {
                let mut scope = runtime.handle_scope();
                let local_v8_val = v8::Local::new(&mut scope, &_global_v8_val);

                is_promise(&mut scope, local_v8_val)
            } else {
                false
            };

            if should_resolve && is_promise_result {
                let _store_script = r#"
                    (function() {
                        globalThis.__current_promise_object = arguments[0];
                        return { stored: true };
                    })(globalThis.__current_promise_object)
                "#;

                {
                    let mut scope = runtime.handle_scope();
                    let local_v8_val = v8::Local::new(&mut scope, &_global_v8_val);

                    let global = scope.get_current_context().global(&mut scope);
                    let key = match v8::String::new(&mut scope, "__current_promise_object") {
                        Some(key) => key,
                        None => {
                            error!("Failed to create V8 string for __current_promise_object");
                            return Err(RariError::internal(
                                "Failed to create V8 string".to_string(),
                            ));
                        }
                    };
                    global.set(&mut scope, key.into(), local_v8_val);
                }

                let setup_script = r#"
                    (function() {
                        try {
                            const promise = globalThis.__current_promise_object;

                            // Verify it's a Promise
                            if (!promise || typeof promise.then !== 'function') {
                                globalThis.__promise_resolved_value = {
                                    __error: "Not a valid promise",
                                    received: typeof promise,
                                    promiseToString: String(promise)
                                };
                                globalThis.__promise_resolution_complete = true;
                                return;
                            }

                            // Set up Promise resolution with global variable capture
                            globalThis.__promise_resolved_value = null;
                            globalThis.__promise_resolution_complete = false;

                            promise.then(function(resolvedValue) {
                                globalThis.__promise_resolved_value = resolvedValue;
                                globalThis.__promise_resolution_complete = true;
                            }).catch(function(error) {
                                globalThis.__promise_resolved_value = {
                                    __promise_error: true,
                                    error: String(error),
                                    stack: error.stack || "No stack trace"
                                };
                                globalThis.__promise_resolution_complete = true;
                            });
                        } catch (error) {
                            globalThis.__promise_resolved_value = {
                                __promise_error: true,
                                error: String(error),
                                stack: error.stack || "No stack trace"
                            };
                            globalThis.__promise_resolution_complete = true;
                        }
                    })()
                "#;

                match runtime.execute_script(
                    format!("{script_name}_promise_setup"),
                    setup_script.to_string(),
                ) {
                    Ok(_) => {
                        let promise_timeout_ms = get_promise_resolution_timeout_ms(script_name);
                        run_event_loop_with_promise_timeout(
                            runtime,
                            script_name,
                            promise_timeout_ms,
                        )
                        .await?;

                        let extract_script = r#"
                            (function() {
                                if (globalThis.__promise_resolved_value !== null && globalThis.__promise_resolved_value !== undefined) {
                                    return globalThis.__promise_resolved_value;
                                } else if (globalThis.__promise_resolution_complete === true) {
                                    // Completed but no value (shouldn't happen)
                                    return { __completion_error: "Promise completed but no value stored" };
                                } else {
                                    return {
                                        __timeout_error: "Promise did not resolve in time",
                                        __debug_info: {
                                            completion_flag: globalThis.__promise_resolution_complete,
                                            resolved_value: globalThis.__promise_resolved_value
                                        }
                                    };
                                }
                            })()
                        "#;

                        match runtime.execute_script(
                            format!("{script_name}_extract_value"),
                            extract_script.to_string(),
                        ) {
                            Ok(extracted_value) => {
                                let mut scope = runtime.handle_scope();
                                let local_v8_val = v8::Local::new(&mut scope, extracted_value);
                                v8_to_json(&mut scope, local_v8_val)
                            }
                            Err(_) => {
                                let mut scope = runtime.handle_scope();
                                let local_v8_val = v8::Local::new(&mut scope, _global_v8_val);
                                v8_to_json(&mut scope, local_v8_val)
                            }
                        }
                    }
                    Err(_) => {
                        let mut scope = runtime.handle_scope();
                        let local_v8_val = v8::Local::new(&mut scope, _global_v8_val);
                        v8_to_json(&mut scope, local_v8_val)
                    }
                }
            } else {
                let mut scope = runtime.handle_scope();
                let local_v8_val = v8::Local::new(&mut scope, _global_v8_val);
                v8_to_json(&mut scope, local_v8_val)
            }
        }
        Err(e) => {
            let error_string = e.to_string();

            if error_string.contains("SyntaxError")
                && (error_string.contains("Unexpected end of input")
                    || error_string.contains("Unexpected token")
                    || error_string.contains("Invalid left-hand side")
                    || error_string.contains("Missing")
                    || error_string.contains("Unexpected character")
                    || error_string.contains("Unexpected identifier"))
            {
                return Err(RariError::js_execution(format!(
                    "Syntax error in script '{script_name}': {error_string}"
                )));
            }

            if error_string.contains("assertion") || error_string.contains("panicked") {
                return Err(RariError::js_runtime(format!(
                    "Critical runtime error: {error_string}"
                )));
            }

            if error_string.contains("Error")
                && (script_code.contains("throw")
                    || error_string.contains("Error:")
                    || error_string.contains("TypeError")
                    || error_string.contains("RangeError"))
            {
                if error_string.contains("module.exports is not supported")
                    && script_code.contains("typeof module")
                {
                } else {
                    return Err(RariError::js_execution(format!(
                        "Runtime error in script '{script_name}': {error_string}"
                    )));
                }
            }

            if error_string.contains("Cannot use import statement")
                || error_string.contains("Unexpected token 'export'")
                || (error_string.contains("ReferenceError")
                    && !script_code.contains("throw")
                    && !error_string.contains("Error:")
                    && (script_code.contains("URL")
                        || script_code.contains("fetch")
                        || script_code.contains("TextEncoder")
                        || script_code.contains("crypto")
                        || script_code.contains("process")
                        || script_code.contains("global")
                        || script_code.contains("Buffer")
                        || script_code.contains("require")
                        || script_code.contains("module")
                        || script_code.contains("exports")
                        || script_code.contains("__dirname")
                        || script_code.contains("__filename")))
                || (error_string.contains("module.exports is not supported")
                    && script_code.contains("typeof"))
            {
                let specifier_str = module_loader.create_specifier(script_name, "rari_internal");

                let module_code = module_loader.transform_to_esmodule(script_code, script_name);

                module_loader.add_module(&specifier_str, script_name, module_code);

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
                        if load_err.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR)
                            || load_err.to_string().contains("assertion")
                        {
                            return Err(RariError::js_runtime(format!(
                                "Runtime error loading module: {load_err}"
                            )));
                        }

                        return Err(RariError::js_execution(format!(
                            "Failed to load module '{script_name}': {load_err}"
                        )));
                    }
                };

                let eval_completion_future = runtime.mod_evaluate(module_id);
                let eval_result = eval_completion_future.await;

                if let Err(eval_err) = eval_result {
                    if eval_err.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR)
                        || eval_err.to_string().contains("assertion")
                    {
                        return Err(RariError::js_runtime(format!(
                            "Runtime error evaluating module: {eval_err}"
                        )));
                    }

                    return Err(RariError::js_execution(format!(
                        "Failed to evaluate module '{script_name}': {eval_err}"
                    )));
                }

                run_event_loop_with_error_handling(
                    runtime,
                    &format!("module exec for '{script_name}'"),
                )
                .await?;

                Ok(JsonValue::Null)
            } else {
                Err(RariError::js_execution(format!(
                    "Failed to execute script '{script_name}': {error_string}"
                )))
            }
        }
    }
}

async fn execute_script_for_streaming(
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

    let module_id_future = runtime.load_side_es_module(&specifier);
    let module_id: usize = module_id_future.await.map_err(|e| {
        RariError::js_execution(format!("Failed to load streaming module '{script_name}': {e}"))
    })?;

    let eval_completion_future = runtime.mod_evaluate(module_id);
    eval_completion_future.await.map_err(|e| {
        RariError::js_execution(format!("Failed to evaluate streaming module '{script_name}': {e}"))
    })?;

    runtime.run_event_loop(PollEventLoopOptions::default()).await.map_err(|e| {
        RariError::js_execution(format!(
            "Event loop error after streaming module '{script_name}': {e}"
        ))
    })?;

    Ok(())
}

impl JsRuntimeInterface for DenoRuntime {
    fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
        let request_sender = self.request_sender.clone();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::ExecuteScript {
                    script_name,
                    script_code,
                    result_tx: response_sender,
                })
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string()))?;

            response_receiver
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_FAILED_ERROR.to_string()))?
        })
    }

    fn execute_function(
        &self,
        function_name: &str,
        args: Vec<JsonValue>,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send + 'static>> {
        let request_sender = self.request_sender.clone();
        let function_name = function_name.to_string();

        Box::pin(async move {
            let args_json = serde_json::to_string(&args)
                .map_err(|e| RariError::js_runtime(format!("Failed to serialize args: {e}")))?;

            let script = format!(
                r#"
                (function() {{
                    try {{
                        const args = JSON.parse('{}');

                        if (typeof {} !== 'function') {{
                            throw new Error("Function not found: {}");
                        }}

                        return {}(...args);
                    }} catch (e) {{
                        throw new Error(`Error executing function: ${{e.message}}`);
                    }}
                }})();
                "#,
                args_json.replace('\'', "\\'"),
                function_name,
                function_name,
                function_name
            );

            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::ExecuteScript {
                    script_name: format!("exec_func_{function_name}.js"),
                    script_code: script,
                    result_tx: response_sender,
                })
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string()))?;

            response_receiver
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_FAILED_ERROR.to_string()))?
        })
    }

    fn execute_script_for_streaming(
        &self,
        script_name: String,
        script_code: String,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let request_sender = self.request_sender.clone();
        let stream_id = format!(
            "stream_{}",
            std::time::SystemTime::now().elapsed().unwrap_or_default().as_millis()
        );

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::ExecuteScriptForStreaming {
                    script_name,
                    script_code,
                    stream_id,
                    result_tx: response_sender,
                    chunk_sender,
                })
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string()))?;

            response_receiver
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_FAILED_ERROR.to_string()))?
                .map(|_| ())
        })
    }

    fn add_module_to_loader(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let request_sender = self.request_sender.clone();
        let specifier = specifier.to_string();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::AddModuleToLoader {
                    component_id: specifier,
                    result_tx: response_sender,
                })
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string()))?;

            response_receiver
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_FAILED_ERROR.to_string()))?
        })
    }

    fn load_es_module(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>> {
        let request_sender = self.request_sender.clone();
        let specifier = specifier.to_string();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::LoadEsModule {
                    component_id: specifier,
                    result_tx: response_sender,
                })
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string()))?;

            response_receiver
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_FAILED_ERROR.to_string()))?
        })
    }

    fn evaluate_module(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
        let request_sender = self.request_sender.clone();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::EvaluateModule { module_id, result_tx: response_sender })
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string()))?;

            response_receiver
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_FAILED_ERROR.to_string()))?
        })
    }

    fn get_module_namespace(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
        let request_sender = self.request_sender.clone();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::GetModuleNamespace { module_id, result_tx: response_sender })
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string()))?;

            response_receiver
                .await
                .map_err(|_| RariError::js_runtime(JS_EXECUTOR_FAILED_ERROR.to_string()))?
        })
    }

    fn add_module_to_loader_only(
        &self,
        specifier: &str,
        code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let request_sender = self.request_sender.clone();
        let specifier_str = specifier.to_string();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();
            request_sender
                .send(JsRequest::AddModuleToLoaderOnly {
                    specifier: specifier_str,
                    code,
                    result_tx: response_sender,
                })
                .await
                .map_err(|_| {
                    RariError::js_runtime(
                        "JS executor channel closed (add_module_only)".to_string(),
                    )
                })?;
            response_receiver.await.map_err(|_| {
                RariError::js_runtime("JS executor failed to respond (add_module_only)".to_string())
            })?
        })
    }

    fn clear_module_loader_caches(
        &self,
        component_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let request_sender = self.request_sender.clone();
        let component_id_str = component_id.to_string();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();
            request_sender
                .send(JsRequest::ClearModuleLoaderCaches {
                    component_id: component_id_str,
                    result_tx: response_sender,
                })
                .await
                .map_err(|_| {
                    RariError::js_runtime("JS executor channel closed (clear_caches)".to_string())
                })?;
            response_receiver.await.map_err(|_| {
                RariError::js_runtime("JS executor failed to respond (clear_caches)".to_string())
            })?
        })
    }
}

pub fn create_lazy_runtime() -> Box<dyn JsRuntimeInterface> {
    let runtime_instance: LazyRuntimeInner = Arc::new(tokio::sync::Mutex::new(None));

    struct LazyRuntime {
        inner: Arc<tokio::sync::Mutex<Option<Box<dyn JsRuntimeInterface>>>>,
    }

    impl LazyRuntime {
        fn new(inner: Arc<tokio::sync::Mutex<Option<Box<dyn JsRuntimeInterface>>>>) -> Self {
            Self { inner }
        }

        async fn with_runtime<F, R>(&self, operation: F) -> Result<R, RariError>
        where
            F: FnOnce(&Box<dyn JsRuntimeInterface>) -> R + Send,
            R: Send,
        {
            self.ensure_initialized().await?;
            let runtime = self.inner.lock().await;
            if let Some(runtime) = &*runtime {
                Ok(operation(runtime))
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        }

        async fn ensure_initialized(&self) -> Result<(), RariError> {
            let needs_init = {
                let guard = self.inner.lock().await;
                guard.is_none()
            };

            if needs_init {
                let runtime = DenoRuntime::new(None);
                let mut guard = self.inner.lock().await;
                if guard.is_none() {
                    *guard = Some(Box::new(runtime));
                }
            }
            Ok(())
        }
    }

    let lazy_runtime = LazyRuntime::new(runtime_instance);

    impl JsRuntimeInterface for LazyRuntime {
        fn execute_script(
            &self,
            script_name: String,
            script_code: String,
        ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
            let self_copy = LazyRuntime { inner: self.inner.clone() };

            Box::pin(async move {
                self_copy
                    .with_runtime(|runtime| runtime.execute_script(script_name, script_code))
                    .await?
                    .await
            })
        }

        fn execute_function(
            &self,
            function_name: &str,
            args: Vec<JsonValue>,
        ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send + 'static>> {
            let inner = self.inner.clone();
            let function_name = function_name.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntime { inner: inner.clone() };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.execute_function(&function_name, args).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn execute_script_for_streaming(
            &self,
            script_name: String,
            script_code: String,
            chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
            let inner = self.inner.clone();
            let script_name = script_name.to_string();
            let script_code = script_code.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntime { inner: inner.clone() };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime
                        .execute_script_for_streaming(script_name, script_code, chunk_sender)
                        .await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn add_module_to_loader(
            &self,
            specifier: &str,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
            let self_copy = LazyRuntime { inner: self.inner.clone() };
            let specifier = specifier.to_string();

            Box::pin(async move {
                self_copy
                    .with_runtime(|runtime| runtime.add_module_to_loader(&specifier))
                    .await?
                    .await
            })
        }

        fn load_es_module(
            &self,
            specifier: &str,
        ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>> {
            let inner = self.inner.clone();
            let specifier = specifier.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntime { inner: inner.clone() };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.load_es_module(&specifier).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn evaluate_module(
            &self,
            module_id: deno_core::ModuleId,
        ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
            let inner = self.inner.clone();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntime { inner: inner.clone() };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.evaluate_module(module_id).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn get_module_namespace(
            &self,
            module_id: deno_core::ModuleId,
        ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
            let inner = self.inner.clone();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntime { inner: inner.clone() };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.get_module_namespace(module_id).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn add_module_to_loader_only(
            &self,
            specifier: &str,
            code: String,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
            let inner = self.inner.clone();
            let specifier = specifier.to_string();
            let code = code.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntime { inner: inner.clone() };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.add_module_to_loader_only(&specifier, code).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn clear_module_loader_caches(
            &self,
            component_id: &str,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
            let inner = self.inner.clone();
            let component_id = component_id.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntime { inner: inner.clone() };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.clear_module_loader_caches(&component_id).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }
    }

    Box::new(lazy_runtime)
}

#[allow(unused_variables)]
pub fn create_runtime_with_config(config: RuntimeConfig) -> Box<dyn JsRuntimeInterface> {
    Box::new(DenoRuntime::new(None))
}

pub fn create_runtime() -> Box<dyn JsRuntimeInterface> {
    create_runtime_with_config(RuntimeConfig::default())
}

pub fn create_lazy_runtime_with_env(
    env_vars: FxHashMap<String, String>,
) -> Box<dyn JsRuntimeInterface> {
    let inner = Arc::new(tokio::sync::Mutex::new(None));

    struct LazyRuntimeWithEnv {
        inner: Arc<tokio::sync::Mutex<Option<Box<dyn JsRuntimeInterface>>>>,
        env_vars: Option<FxHashMap<String, String>>,
    }

    impl LazyRuntimeWithEnv {
        fn new(
            inner: Arc<tokio::sync::Mutex<Option<Box<dyn JsRuntimeInterface>>>>,
            env_vars: Option<FxHashMap<String, String>>,
        ) -> Self {
            Self { inner, env_vars }
        }

        async fn ensure_initialized(&self) -> Result<(), RariError> {
            let mut runtime = self.inner.lock().await;
            if runtime.is_none() {
                *runtime = Some(Box::new(DenoRuntime::new(self.env_vars.clone())));
            }
            Ok(())
        }
    }

    impl JsRuntimeInterface for LazyRuntimeWithEnv {
        fn execute_script(
            &self,
            script_name: String,
            script_code: String,
        ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
            let self_copy =
                LazyRuntimeWithEnv { inner: self.inner.clone(), env_vars: self.env_vars.clone() };

            Box::pin(async move {
                self_copy.ensure_initialized().await?;

                let runtime = self_copy.inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.execute_script(script_name, script_code).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn execute_function(
            &self,
            function_name: &str,
            args: Vec<JsonValue>,
        ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send + 'static>> {
            let inner = self.inner.clone();
            let function_name = function_name.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntimeWithEnv { inner: inner.clone(), env_vars: None };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.execute_function(&function_name, args).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn execute_script_for_streaming(
            &self,
            script_name: String,
            script_code: String,
            chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
            let inner = self.inner.clone();
            let script_name = script_name.to_string();
            let script_code = script_code.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntimeWithEnv { inner: inner.clone(), env_vars: None };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime
                        .execute_script_for_streaming(script_name, script_code, chunk_sender)
                        .await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn add_module_to_loader(
            &self,
            specifier: &str,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
            let inner = self.inner.clone();
            let specifier = specifier.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntimeWithEnv { inner: inner.clone(), env_vars: None };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.add_module_to_loader(&specifier).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn load_es_module(
            &self,
            specifier: &str,
        ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>> {
            let inner = self.inner.clone();
            let specifier = specifier.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntimeWithEnv { inner: inner.clone(), env_vars: None };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.load_es_module(&specifier).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn evaluate_module(
            &self,
            module_id: deno_core::ModuleId,
        ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
            let inner = self.inner.clone();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntimeWithEnv { inner: inner.clone(), env_vars: None };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.evaluate_module(module_id).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn get_module_namespace(
            &self,
            module_id: deno_core::ModuleId,
        ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
            let inner = self.inner.clone();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntimeWithEnv { inner: inner.clone(), env_vars: None };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.get_module_namespace(module_id).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn add_module_to_loader_only(
            &self,
            specifier: &str,
            code: String,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
            let inner = self.inner.clone();
            let specifier = specifier.to_string();
            let code = code.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntimeWithEnv { inner: inner.clone(), env_vars: None };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.add_module_to_loader_only(&specifier, code).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }

        fn clear_module_loader_caches(
            &self,
            component_id: &str,
        ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
            let inner = self.inner.clone();
            let component_id = component_id.to_string();

            Box::pin(async move {
                let initialized_result = {
                    let this = LazyRuntimeWithEnv { inner: inner.clone(), env_vars: None };
                    this.ensure_initialized().await
                };

                initialized_result?;

                let runtime = inner.lock().await;

                if let Some(runtime) = &*runtime {
                    runtime.clear_module_loader_caches(&component_id).await
                } else {
                    Err(RariError::js_execution("Runtime not initialized".to_string()))
                }
            })
        }
    }

    let lazy_runtime = LazyRuntimeWithEnv::new(inner, Some(env_vars));
    Box::new(lazy_runtime)
}
