use crate::error::RariError;
use crate::runtime::module_loader::RariModuleLoader;
use crate::runtime::ops::StreamOpState;
use crate::runtime::runtime_factory::constants::*;
use crate::runtime::runtime_factory::executor::{execute_script, execute_script_for_streaming};
use crate::runtime::runtime_factory::interface::{AsyncBatchResult, JsRuntimeInterface};
use crate::runtime::runtime_factory::runtime_builder::create_deno_runtime;
use crate::runtime::runtime_factory::v8_utils::get_module_namespace_as_json;
use deno_core::{ModuleSpecifier, PollEventLoopOptions};
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::{mpsc, oneshot};

type ScriptBatchItem = (String, String, oneshot::Sender<Result<JsonValue, RariError>>);
type BatchResultSender = mpsc::UnboundedSender<(usize, Result<JsonValue, RariError>)>;
type PendingScript = (
    oneshot::Sender<Result<JsonValue, RariError>>,
    String,
    Option<deno_core::v8::Global<deno_core::v8::Value>>,
);

enum JsRequest {
    ExecuteScript {
        script_name: String,
        script_code: String,
        result_tx: oneshot::Sender<Result<JsonValue, RariError>>,
    },
    ExecuteScriptBatch {
        scripts: Vec<(String, String)>,
        result_tx: BatchResultSender,
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
    ClearRequestContext {
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
                                    JsRequest::ExecuteScriptBatch {
                                        scripts,
                                        result_tx,
                                    } => {
                                        let batch: Vec<ScriptBatchItem> = scripts
                                            .into_iter()
                                            .enumerate()
                                            .map(|(i, (name, code))| {
                                                let (tx, rx) = oneshot::channel::<Result<JsonValue, RariError>>();
                                                let result_tx_clone = result_tx.clone();
                                                tokio::spawn(async move {
                                                    let result = rx.await.unwrap_or_else(|_| Err(create_graceful_error()));
                                                    let _ = result_tx_clone.send((i, result));
                                                });
                                                (name, code, tx)
                                            })
                                            .collect();

                                        let needs_restart = execute_scripts_concurrent(
                                            &mut deno_runtime,
                                            &module_loader,
                                            batch,
                                        )
                                        .await;
                                        if needs_restart {
                                            break;
                                        }
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

                                        let is_hmr_specifier = specifier.contains("/rari_hmr/");

                                        let existing_specifier = module_loader
                                            .component_specifiers
                                            .get(&component_id)
                                            .map(|entry| entry.clone());

                                        let has_existing_hmr_mapping = existing_specifier
                                            .as_ref()
                                            .map(|spec| spec.contains("/rari_hmr/"))
                                            .unwrap_or(false);

                                        if is_hmr_specifier || !has_existing_hmr_mapping {
                                            module_loader
                                                .component_specifiers
                                                .insert(component_id.clone(), specifier.clone());
                                        }

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
                                    JsRequest::ClearRequestContext {
                                        result_tx,
                                    } => {
                                        use crate::runtime::ops::FetchOpState;
                                        let op_state = deno_runtime.op_state();
                                        let mut op_state_borrow = op_state.borrow_mut();
                                        if let Some(fetch_state) =
                                            op_state_borrow.try_borrow_mut::<FetchOpState>()
                                        {
                                            fetch_state.request_context = None;
                                        }
                                        let _ = result_tx.send(Ok(()));
                                        Ok::<(), RariError>(())
                                    }
                                };

                                if let Err(e) = result {
                                    eprintln!("[rari] Error processing request: {e}");
                                    break;
                                }

                                if let Err(e) = deno_runtime
                                    .run_event_loop(PollEventLoopOptions::default())
                                    .await
                                {
                                    eprintln!("[rari] Event loop error: {e}. Restarting runtime.");
                                }
                            }
                            None => {
                                return Ok::<(), RariError>(());
                            }
                        }
                    }

                    println!("[rari] Restarting JS runtime due to error or forced restart");
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
                eprintln!("[rari] {err_msg}");
                let _ = result_tx.send(Err(RariError::js_execution(err_msg)));
            }
        }
    } else {
        let err_msg = format!("Component specifier not found for LoadEsModule: {component_id}");
        eprintln!("[rari] {err_msg}");
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
        println!("[rari] Critical error detected in module evaluation: {e}");
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

async fn execute_scripts_concurrent(
    deno_runtime: &mut deno_core::JsRuntime,
    module_loader: &std::rc::Rc<RariModuleLoader>,
    batch: Vec<ScriptBatchItem>,
) -> bool /* needs_restart */ {
    use crate::runtime::runtime_factory::v8_utils::{
        run_event_loop_with_error_handling, v8_to_json,
    };
    use crate::with_scope;

    if batch.len() == 1 {
        let (name, code, tx) = batch.into_iter().next().expect("batch has exactly one item");
        let result = execute_script(deno_runtime, module_loader, &name, &code).await;
        let needs_restart = result.as_ref().err().map(is_runtime_restart_needed).unwrap_or(false);
        if needs_restart {
            let _ = tx.send(Err(create_graceful_error()));
        } else {
            let _ = tx.send(result);
        }
        return needs_restart;
    }

    let mut pending: Vec<PendingScript> = Vec::with_capacity(batch.len());
    let mut needs_immediate_restart = false;

    for (script_name, script_code, tx) in batch {
        match deno_runtime.execute_script(script_name.clone(), script_code.clone()) {
            Ok(v8_val) => {
                let slot_key = format!("__rari_concurrent_{}__", pending.len());
                let store_result = with_scope!(deno_runtime, |scope| {
                    let local_val = deno_core::v8::Local::new(scope, &v8_val);
                    let context = scope.get_current_context();
                    let global = context.global(scope);
                    if let Some(key_str) = deno_core::v8::String::new(scope, &slot_key) {
                        global.set(scope, key_str.into(), local_val);
                        Ok::<(), RariError>(())
                    } else {
                        Err(RariError::internal("Failed to create V8 key string".to_string()))
                    }
                });
                if store_result.is_err() {
                    let _ = tx.send(Err(RariError::internal(
                        "Failed to store V8 value in global slot".to_string(),
                    )));
                } else {
                    pending.push((tx, script_name, Some(v8_val)));
                }
            }
            Err(e) => {
                let err = RariError::js_execution(format!(
                    "Failed to execute script '{}': {}",
                    script_name, e
                ));
                if is_runtime_restart_needed(&err) {
                    needs_immediate_restart = true;
                    let _ = tx.send(Err(create_graceful_error()));
                } else {
                    let _ = tx.send(Err(err));
                }
            }
        }
    }

    if needs_immediate_restart {
        let _ = deno_runtime.execute_script(
            "cleanup_concurrent",
            r#"(function(){Object.keys(globalThis).forEach(function(k){if(/^__rari_concurrent_\d+__$/.test(k))delete globalThis[k];});delete globalThis['~rari_concurrent'];})()"#.to_string(),
        );
        return true;
    }

    for (i, (_, _, v8_val_opt)) in pending.iter().enumerate() {
        if v8_val_opt.is_none() {
            continue;
        }
        let slot_key = format!("__rari_concurrent_{}__", i);
        let setup = format!(
            r#"(function() {{
                if (!globalThis['~rari_concurrent']) globalThis['~rari_concurrent'] = {{}};
                const val = globalThis['{}'];
                if (val && typeof val.then === 'function') {{
                    globalThis['~rari_concurrent']['{}'] = {{ done: false, result: null, error: null }};
                    val.then(function(r) {{
                        globalThis['~rari_concurrent']['{}'] = {{ done: true, result: r, error: null }};
                    }}).catch(function(e) {{
                        globalThis['~rari_concurrent']['{}'] = {{ done: true, result: null, error: String(e) }};
                    }});
                }} else {{
                    globalThis['~rari_concurrent']['{}'] = {{ done: true, result: val, error: null }};
                }}
            }})()"#,
            slot_key, slot_key, slot_key, slot_key, slot_key
        );
        if let Err(e) = deno_runtime.execute_script(format!("setup_concurrent_{i}"), setup) {
            eprintln!("[rari] Failed to setup concurrent tracking for slot {i}: {e}");
        }
    }

    let promise_timeout_ms: u64 = std::env::var("RARI_PROMISE_RESOLUTION_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000);

    let timeout_duration = std::time::Duration::from_millis(promise_timeout_ms);
    let start = std::time::Instant::now();

    let mut sent = vec![false; pending.len()];
    let mut remaining = pending.len();

    let (senders, names): (Vec<_>, Vec<_>) =
        pending.into_iter().map(|(tx, name, _)| (Some(tx), name)).unzip();
    let mut senders: Vec<Option<oneshot::Sender<Result<JsonValue, RariError>>>> = senders;

    while remaining > 0 && start.elapsed() < timeout_duration {
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            run_event_loop_with_error_handling(deno_runtime, "concurrent batch"),
        )
        .await;

        for i in 0..sent.len() {
            if sent[i] {
                continue;
            }

            let slot_key = format!("__rari_concurrent_{}__", i);
            let check = format!(
                r#"(function() {{
                    const e = globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{}'];
                    return e && e.done ? e : null;
                }})()"#,
                slot_key
            );

            let is_done = match deno_runtime.execute_script(format!("check_slot_{i}"), check) {
                Ok(result) => {
                    with_scope!(deno_runtime, |scope| {
                        let local = deno_core::v8::Local::new(scope, result);
                        !local.is_null_or_undefined()
                    })
                }
                Err(_) => false,
            };

            if is_done {
                let extract = format!(
                    r#"(function() {{
                        const entry = globalThis['~rari_concurrent']['{}'];
                        if (entry.error) return {{ '~concurrent_error': entry.error }};
                        return entry.result;
                    }})()"#,
                    slot_key
                );
                let result =
                    match deno_runtime.execute_script(format!("extract_concurrent_{i}"), extract) {
                        Ok(extracted) => {
                            let json_result = with_scope!(deno_runtime, |scope| {
                                let local = deno_core::v8::Local::new(scope, extracted);
                                v8_to_json(scope, local)
                            });
                            match json_result {
                                Ok(json) => {
                                    if let JsonValue::Object(ref obj) = json
                                        && let Some(JsonValue::String(err)) =
                                            obj.get("~concurrent_error")
                                    {
                                        Err(RariError::js_execution(err.clone()))
                                    } else {
                                        Ok(json)
                                    }
                                }
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(RariError::js_execution(format!(
                            "Failed to extract result for '{}': {}",
                            names[i], e
                        ))),
                    };

                if let Some(tx) = senders[i].take() {
                    let _ = tx.send(result);
                }
                sent[i] = true;
                remaining -= 1;
            }
        }

        if remaining > 0 {
            tokio::task::yield_now().await;
        }
    }

    for i in 0..sent.len() {
        if !sent[i]
            && let Some(tx) = senders[i].take()
        {
            let timeout_err =
                Err(RariError::timeout(format!("Promise timed out for '{}'", names[i])));
            let needs_restart =
                timeout_err.as_ref().err().map(is_runtime_restart_needed).unwrap_or(false);
            if needs_restart {
                let _ = tx.send(Err(create_graceful_error()));
            } else {
                let _ = tx.send(timeout_err);
            }
        }
    }

    let _ = deno_runtime.execute_script(
            "cleanup_concurrent",
            r#"(function(){Object.keys(globalThis).forEach(function(k){if(/^__rari_concurrent_\d+__$/.test(k))delete globalThis[k];});delete globalThis['~rari_concurrent'];})()"#.to_string(),
        );

    false
}

fn extract_component_id_from_specifier(specifier: &str) -> String {
    if let Some(server_idx) = specifier.rfind("/server/") {
        let after_server = &specifier[server_idx + 8..];
        after_server.split('?').next().unwrap_or(after_server).trim_end_matches(".js").to_string()
    } else if let Some(component_idx) = specifier.rfind("/rari_component/") {
        let after_component = &specifier[component_idx + 16..];
        after_component
            .split('?')
            .next()
            .unwrap_or(after_component)
            .trim_end_matches(".js")
            .to_string()
    } else if let Some(hmr_idx) = specifier.rfind("/rari_hmr/server/") {
        let after_hmr = &specifier[hmr_idx + 17..];
        after_hmr.split('?').next().unwrap_or(after_hmr).trim_end_matches(".js").to_string()
    } else {
        specifier
            .split('/')
            .next_back()
            .unwrap_or(specifier)
            .split('?')
            .next()
            .unwrap_or(specifier)
            .trim_end_matches(".js")
            .to_string()
    }
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

    fn execute_script_batch(&self, scripts: Vec<(String, String)>) -> AsyncBatchResult {
        let request_sender = self.request_sender.clone();

        Box::pin(async move {
            let (result_tx, result_rx) = mpsc::unbounded_channel();
            let script_count = scripts.len();

            if request_sender
                .send(JsRequest::ExecuteScriptBatch { scripts, result_tx })
                .await
                .is_err()
            {
                let (err_tx, err_rx) = mpsc::unbounded_channel();
                for i in 0..script_count {
                    let _ = err_tx.send((
                        i,
                        Err(RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string())),
                    ));
                }
                return err_rx;
            }

            result_rx
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
                    const argsBase64 = "{}";
                    const argsBinary = atob(argsBase64);
                    const argsBytes = new Uint8Array(argsBinary.length);
                    for (let i = 0; i < argsBinary.length; i++) {{
                        argsBytes[i] = argsBinary.charCodeAt(i);
                    }}
                    const argsJson = new TextDecoder('utf-8').decode(argsBytes);
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

    fn clear_request_context(&self) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let request_sender = self.request_sender.clone();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();
            request_sender
                .send(JsRequest::ClearRequestContext { result_tx: response_sender })
                .await
                .map_err(|_| {
                    RariError::js_runtime(
                        "JS executor channel closed (clear_request_context)".to_string(),
                    )
                })?;
            response_receiver.await.map_err(|_| {
                RariError::js_runtime(
                    "JS executor failed to respond (clear_request_context)".to_string(),
                )
            })?
        })
    }
}
