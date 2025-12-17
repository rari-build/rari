use crate::error::RariError;
use crate::runtime::module_loader::RariModuleLoader;
use crate::runtime::ops::StreamOpState;
use crate::runtime::runtime_factory::constants::*;
use crate::runtime::runtime_factory::executor::{execute_script, execute_script_for_streaming};
use crate::runtime::runtime_factory::interface::JsRuntimeInterface;
use crate::runtime::runtime_factory::runtime_builder::create_deno_runtime;
use crate::runtime::runtime_factory::v8_utils::get_module_namespace_as_json;
use deno_core::{ModuleSpecifier, PollEventLoopOptions};
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::{mpsc, oneshot};

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
        result_tx: oneshot::Sender<Result<JsonValue, RariError>>,
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
        module_id: deno_core::ModuleId,
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
    SetRequestContext {
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
}

pub struct DenoRuntime {
    request_sender: mpsc::Sender<JsRequest>,
}

impl DenoRuntime {
    pub fn new(env_vars: Option<FxHashMap<String, String>>) -> Self {
        let (request_sender, mut request_receiver) = mpsc::channel(CHANNEL_CAPACITY);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");

            let _ = runtime.block_on(async {
                loop {
                    let (mut deno_runtime, module_loader) =
                        match create_deno_runtime(env_vars.clone()) {
                            Ok(rt) => rt,
                            Err(_) => {
                                tokio::time::sleep(std::time::Duration::from_millis(
                                    RUNTIME_RESTART_DELAY_MS,
                                ))
                                .await;
                                continue;
                            }
                        };

                    let mut continue_processing = true;
                    while continue_processing {
                        match request_receiver.recv().await {
                            Some(request) => {
                                let result = match request {
                                    JsRequest::ExecuteScript {
                                        script_name,
                                        script_code,
                                        result_tx,
                                    } => {
                                        let result = execute_script(
                                            &mut deno_runtime,
                                            &module_loader,
                                            &script_name,
                                            &script_code,
                                        )
                                        .await;

                                        if let Err(e) = &result
                                            && is_runtime_restart_needed(e)
                                        {
                                            let graceful_error = create_graceful_error();
                                            let _ = result_tx.send(Err(graceful_error));
                                            break;
                                        }

                                        let _ = result_tx.send(result);
                                        Ok::<(), RariError>(())
                                    }
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

                                        let _ = result_tx.send(result.map(|_| JsonValue::Null));
                                        Ok::<(), RariError>(())
                                    }
                                    JsRequest::AddModuleToLoader {
                                        component_id,
                                        result_tx,
                                    } => {
                                        let specifier_opt =
                                            module_loader.get_component_specifier(&component_id);

                                        if specifier_opt.is_some() {
                                            let _ = result_tx.send(Ok(()));
                                        } else {
                                            let _ = result_tx.send(Err(RariError::js_execution(
                                                format!(
                                                    "Component specifier not found in loader for AddModuleToLoader: {component_id}"
                                                ),
                                            )));
                                        }
                                        Ok::<(), RariError>(())
                                    }
                                    JsRequest::LoadEsModule {
                                        component_id,
                                        result_tx,
                                    } => {
                                        handle_load_es_module(
                                            &mut deno_runtime,
                                            &module_loader,
                                            &component_id,
                                            result_tx,
                                        )
                                        .await
                                    }
                                    JsRequest::EvaluateModule {
                                        module_id,
                                        result_tx,
                                    } => {
                                        handle_evaluate_module(
                                            &mut deno_runtime,
                                            &module_loader,
                                            module_id,
                                            result_tx,
                                            &mut continue_processing,
                                        )
                                        .await
                                    }
                                    JsRequest::GetModuleNamespace {
                                        module_id,
                                        result_tx,
                                    } => {
                                        handle_get_module_namespace(
                                            &mut deno_runtime,
                                            &module_loader,
                                            module_id,
                                            result_tx,
                                        )
                                        .await
                                    }
                                    JsRequest::AddModuleToLoaderOnly {
                                        specifier,
                                        code,
                                        result_tx,
                                    } => {
                                        module_loader.set_module_code(specifier.clone(), code.clone());

                                        let component_id =
                                            extract_component_id_from_specifier(&specifier);

                                        module_loader
                                            .component_specifiers
                                            .insert(component_id, specifier.clone());

                                        let _ = result_tx.send(Ok(()));
                                        Ok::<(), RariError>(())
                                    }
                                    JsRequest::ClearModuleLoaderCaches {
                                        component_id,
                                        result_tx,
                                    } => {
                                        module_loader.clear_component_caches(&component_id);
                                        let _ = result_tx.send(Ok(()));
                                        Ok::<(), RariError>(())
                                    }
                                    JsRequest::SetRequestContext {
                                        request_context,
                                        result_tx,
                                    } => {
                                        use crate::runtime::ops::FetchOpState;
                                        let op_state = deno_runtime.op_state();
                                        let mut op_state_borrow = op_state.borrow_mut();
                                        if let Some(fetch_state) =
                                            op_state_borrow.try_borrow_mut::<FetchOpState>()
                                        {
                                            fetch_state.request_context = Some(request_context);
                                        }
                                        let _ = result_tx.send(Ok(()));
                                        Ok::<(), RariError>(())
                                    }
                                };

                                if let Err(e) = result {
                                    eprintln!("[RARI_ERROR] Error processing request: {e}");
                                    break;
                                }

                                if let Err(e) = deno_runtime
                                    .run_event_loop(PollEventLoopOptions::default())
                                    .await
                                {
                                    eprintln!("[RARI_WARN] Event loop error: {e}. Restarting runtime.");
                                }
                            }
                            None => {
                                return Ok::<(), RariError>(());
                            }
                        }
                    }

                    println!("[RARI_RUNTIME] Restarting JS runtime due to error or forced restart");
                    tokio::time::sleep(std::time::Duration::from_millis(
                        RUNTIME_QUICK_RESTART_DELAY_MS,
                    ))
                    .await;
                }
            });
        });

        Self { request_sender }
    }
}

async fn handle_load_es_module(
    deno_runtime: &mut deno_core::JsRuntime,
    module_loader: &std::rc::Rc<RariModuleLoader>,
    component_id: &str,
    result_tx: oneshot::Sender<Result<deno_core::ModuleId, RariError>>,
) -> Result<(), RariError> {
    let specifier_opt = module_loader.get_component_specifier(component_id);

    if let Some(specifier_str) = specifier_opt {
        let is_hmr_update = module_loader.is_hmr_module(&specifier_str);

        match ModuleSpecifier::parse(&specifier_str) {
            Ok(module_specifier) => {
                let module_load_result = if is_hmr_update {
                    if let Some(versioned_specifier) =
                        module_loader.get_versioned_specifier(component_id)
                    {
                        match ModuleSpecifier::parse(&versioned_specifier) {
                            Ok(versioned_module_specifier) => {
                                deno_runtime.load_side_es_module(&versioned_module_specifier).await
                            }
                            Err(_) => deno_runtime.load_side_es_module(&module_specifier).await,
                        }
                    } else {
                        deno_runtime.load_side_es_module(&module_specifier).await
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
        let err_msg = format!("Component specifier not found for LoadEsModule: {component_id}");
        eprintln!("[RARI_ERROR] {err_msg}");
        let _ = result_tx.send(Err(RariError::js_execution(err_msg)));
    }
    Ok::<(), RariError>(())
}

async fn handle_evaluate_module(
    deno_runtime: &mut deno_core::JsRuntime,
    module_loader: &std::rc::Rc<RariModuleLoader>,
    module_id: deno_core::ModuleId,
    result_tx: oneshot::Sender<Result<JsonValue, RariError>>,
    continue_processing: &mut bool,
) -> Result<(), RariError> {
    let module_registered = module_loader.is_already_evaluated(&module_id.to_string());

    let result = if module_registered {
        println!("[RARI_HMR] Module {module_id} already registered, getting namespace");
        match get_module_namespace_as_json(deno_runtime, module_id) {
            Ok(json_result) => Ok(json_result),
            Err(_) => Ok(create_already_evaluated_response("get_module_namespace")),
        }
    } else {
        match deno_runtime.mod_evaluate(module_id).await {
            Ok(_) => {
                module_loader.mark_module_evaluated(&module_id.to_string());
                get_module_namespace_as_json(deno_runtime, module_id)
            }
            Err(e) => {
                if e.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR) {
                    println!(
                        "[RARI_HMR] Module '{module_id}' already evaluated during evaluation attempt"
                    );
                    module_loader.mark_module_evaluated(&module_id.to_string());

                    match get_module_namespace_as_json(deno_runtime, module_id) {
                        Ok(json_result) => Ok(json_result),
                        Err(_) => Ok(create_already_evaluated_response("get_module_namespace")),
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
        && is_critical_error(e)
    {
        println!("[RARI_RUNTIME] Critical error detected in module evaluation: {e}");
        *continue_processing = false;
    }

    let _ = result_tx.send(result);
    Ok::<(), RariError>(())
}

async fn handle_get_module_namespace(
    deno_runtime: &mut deno_core::JsRuntime,
    module_loader: &std::rc::Rc<RariModuleLoader>,
    module_id: deno_core::ModuleId,
    result_tx: oneshot::Sender<Result<JsonValue, RariError>>,
) -> Result<(), RariError> {
    let module_evaluated = module_loader.is_already_evaluated(&module_id.to_string());

    if module_evaluated {
        let json_result =
            get_module_namespace_as_json(deno_runtime, module_id as deno_core::ModuleId);
        let _ = result_tx.send(json_result);
    } else {
        match deno_runtime.mod_evaluate(module_id as deno_core::ModuleId).await {
            Ok(_) => {
                module_loader.mark_module_evaluated(&module_id.to_string());
                let json_result =
                    get_module_namespace_as_json(deno_runtime, module_id as deno_core::ModuleId);
                let _ = result_tx.send(json_result);
            }
            Err(e) => {
                if e.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR) {
                    module_loader.mark_module_evaluated(&module_id.to_string());
                    let json_result = get_module_namespace_as_json(
                        deno_runtime,
                        module_id as deno_core::ModuleId,
                    );
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
}

fn extract_component_id_from_specifier(specifier: &str) -> String {
    if let Some(server_idx) = specifier.rfind("/server/") {
        let after_server = &specifier[server_idx + 8..];
        return after_server.trim_end_matches(".js").to_string();
    }

    // Fallback to old behavior for non-file:// URLs
    specifier
        .split('/')
        .next_back()
        .unwrap_or(specifier)
        .split('?')
        .next()
        .unwrap_or(specifier)
        .to_string()
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

            let unique_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();

            let args_base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                args_json.as_bytes(),
            );

            let script = format!(
                r#"
                (function() {{
                    const argsJson = atob("{}");
                    const args = JSON.parse(argsJson);

                    if (typeof globalThis["{}"] !== 'function') {{
                        throw new Error("Function not found: {}");
                    }}

                    return globalThis["{}"](...args);
                }})();
                "#,
                args_base64, function_name, function_name, function_name
            );

            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::ExecuteScript {
                    script_name: format!("exec_func_{}_{}.js", function_name, unique_id),
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

    fn set_request_context(
        &self,
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let request_sender = self.request_sender.clone();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();
            request_sender
                .send(JsRequest::SetRequestContext { request_context, result_tx: response_sender })
                .await
                .map_err(|_| {
                    RariError::js_runtime(
                        "JS executor channel closed (set_request_context)".to_string(),
                    )
                })?;
            response_receiver.await.map_err(|_| {
                RariError::js_runtime(
                    "JS executor failed to respond (set_request_context)".to_string(),
                )
            })?
        })
    }
}
