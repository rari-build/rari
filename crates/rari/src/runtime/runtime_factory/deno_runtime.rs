use crate::error::RariError;
use crate::runtime::module_loader::RariModuleLoader;
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

struct PendingBatch {
    senders: Vec<Option<oneshot::Sender<Result<JsonValue, RariError>>>>,
    names: Vec<String>,
    sent: Vec<bool>,
    remaining: usize,
    start: std::time::Instant,
    timeout: std::time::Duration,
    batch_id: u64,
}

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

            runtime.block_on(async {
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
                    let mut pending_batches: Vec<PendingBatch> = Vec::new();
                    let mut batch_id_counter: u64 = 0;

                    while continue_processing {
                        if !pending_batches.is_empty() {
                            tokio::select! {
                                biased;
                                request = request_receiver.recv() => {
                                    match request {
                                        Some(req) => {
                                            let result = handle_js_request(
                                                req,
                                                &mut deno_runtime,
                                                &module_loader,
                                                &mut continue_processing,
                                                &mut pending_batches,
                                                &mut batch_id_counter,
                                            ).await;
                                            if let Err(e) = result {
                                                eprintln!("[rari] Error processing request: {e}");
                                                break;
                                            }
                                        }
                                        None => {
                                            continue_processing = false;
                                        }
                                    }
                                }
                                event_loop_result = tokio::time::timeout(
                                    std::time::Duration::from_millis(50),
                                    crate::runtime::runtime_factory::v8_utils::run_event_loop_with_error_handling(
                                        &mut deno_runtime, "concurrent batch"
                                    ),
                                ) => {
                                    if let Ok(Err(e)) = event_loop_result {
                                        eprintln!("[rari] Event loop error: {e}");
                                        if is_runtime_restart_needed(&e) {
                                            for batch in pending_batches.drain(..) {
                                                for sender in batch.senders.into_iter().flatten() {
                                                    let _ = sender.send(Err(create_graceful_error()));
                                                }
                                            }
                                            break;
                                        }
                                    }
                                }
                            }

                            check_pending_batches(&mut deno_runtime, &mut pending_batches);
                            pending_batches.retain(|b| b.remaining > 0 && b.start.elapsed() < b.timeout);
                        } else {
                            match request_receiver.recv().await {
                                Some(req) => {
                                    let result = handle_js_request(
                                        req,
                                        &mut deno_runtime,
                                        &module_loader,
                                        &mut continue_processing,
                                        &mut pending_batches,
                                        &mut batch_id_counter,
                                    ).await;
                                    if let Err(e) = result {
                                        eprintln!("[rari] Error processing request: {e}");
                                        break;
                                    }
                                }
                                None => {
                                    continue_processing = false;
                                }
                            }
                        }

                        let _ = tokio::time::timeout(
                            std::time::Duration::from_millis(10),
                            deno_runtime.run_event_loop(PollEventLoopOptions::default()),
                        ).await;
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

async fn handle_js_request(
    request: JsRequest,
    deno_runtime: &mut deno_core::JsRuntime,
    module_loader: &std::rc::Rc<RariModuleLoader>,
    continue_processing: &mut bool,
    pending_batches: &mut Vec<PendingBatch>,
    batch_id_counter: &mut u64,
) -> Result<(), RariError> {
    match request {
        JsRequest::ExecuteScript { script_name, script_code, result_tx } => {
            let result =
                execute_script(deno_runtime, module_loader, &script_name, &script_code).await;
            if let Err(e) = &result
                && is_runtime_restart_needed(e)
            {
                let _ = result_tx.send(Err(create_graceful_error()));
                return Err(RariError::internal("Runtime restart needed".to_string()));
            }
            let _ = result_tx.send(result);
        }
        JsRequest::ExecuteScriptBatch { scripts, result_tx } => {
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

            if let Some(pending_batch) =
                setup_concurrent_batch(deno_runtime, batch, batch_id_counter).await
            {
                pending_batches.push(pending_batch);
            }
        }
        JsRequest::ExecuteScriptForStreaming {
            script_name,
            script_code,
            result_tx,
            chunk_sender,
        } => {
            let sender_for_op_state = chunk_sender.clone();
            deno_runtime.op_state().borrow_mut().put(crate::runtime::ops::StreamOpState {
                chunk_sender: Some(sender_for_op_state),
                current_stream_id: None,
                row_counter: 0,
            });
            let result = execute_script_for_streaming(
                deno_runtime,
                module_loader,
                &script_name,
                &script_code,
                chunk_sender,
            )
            .await;
            let _ = result_tx.send(result.map(|_| JsonValue::Null));
        }
        JsRequest::AddModuleToLoader { component_id, result_tx } => {
            let specifier_opt = module_loader.get_component_specifier(&component_id);
            if specifier_opt.is_some() {
                let _ = result_tx.send(Ok(()));
            } else {
                let _ = result_tx.send(Err(RariError::js_execution(format!(
                    "Component specifier not found in loader for AddModuleToLoader: {component_id}"
                ))));
            }
        }
        JsRequest::LoadEsModule { component_id, result_tx } => {
            handle_load_es_module(deno_runtime, module_loader, &component_id, result_tx).await?;
        }
        JsRequest::EvaluateModule { module_id, result_tx } => {
            handle_evaluate_module(
                deno_runtime,
                module_loader,
                module_id,
                result_tx,
                continue_processing,
            )
            .await?;
        }
        JsRequest::GetModuleNamespace { module_id, result_tx } => {
            handle_get_module_namespace(deno_runtime, module_loader, module_id, result_tx).await?;
        }
        JsRequest::AddModuleToLoaderOnly { specifier, code, result_tx } => {
            module_loader.set_module_code(specifier.clone(), code.clone());
            let component_id = extract_component_id_from_specifier(&specifier);
            let is_hmr_specifier = specifier.contains("/rari_hmr/");
            let existing_specifier =
                module_loader.component_specifiers.get(&component_id).map(|entry| entry.clone());
            let has_existing_hmr_mapping = existing_specifier
                .as_ref()
                .map(|spec| spec.contains("/rari_hmr/"))
                .unwrap_or(false);
            if is_hmr_specifier || !has_existing_hmr_mapping {
                module_loader.component_specifiers.insert(component_id.clone(), specifier.clone());
            }
            let _ = result_tx.send(Ok(()));
        }
        JsRequest::ClearModuleLoaderCaches { component_id, result_tx } => {
            module_loader.clear_component_caches(&component_id);
            let _ = result_tx.send(Ok(()));
        }
        JsRequest::SetRequestContext { request_context, result_tx } => {
            deno_runtime.op_state().borrow_mut().put(request_context);
            let _ = result_tx.send(Ok(()));
        }
        JsRequest::ClearRequestContext { result_tx } => {
            deno_runtime.op_state().borrow_mut().try_take::<std::sync::Arc<crate::server::middleware::request_context::RequestContext>>();
            let _ = result_tx.send(Ok(()));
        }
    }
    Ok(())
}

async fn setup_concurrent_batch(
    deno_runtime: &mut deno_core::JsRuntime,
    batch: Vec<ScriptBatchItem>,
    batch_id_counter: &mut u64,
) -> Option<PendingBatch> {
    use crate::with_scope;

    *batch_id_counter += 1;
    let batch_id = *batch_id_counter;

    let mut pending: Vec<PendingScript> = Vec::with_capacity(batch.len());

    for (script_name, script_code, tx) in batch {
        match deno_runtime.execute_script(script_name.clone(), script_code.clone()) {
            Ok(v8_val) => {
                let slot_key = format!("__rari_b{}_{}__", batch_id, pending.len());
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
                let _ = tx.send(Err(err));
            }
        }
    }

    if pending.is_empty() {
        return None;
    }

    let promise_timeout_ms: u64 = std::env::var("RARI_PROMISE_RESOLUTION_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000);

    let mut sent = vec![false; pending.len()];
    let remaining = pending.len();

    let (senders, names): (Vec<_>, Vec<_>) =
        pending.into_iter().map(|(tx, name, _)| (Some(tx), name)).unzip();

    for (i, sent_item) in sent.iter_mut().enumerate().take(senders.len()) {
        let slot_key = format!("__rari_b{}_{}__", batch_id, i);
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
            *sent_item = true;
        }
    }

    Some(PendingBatch {
        senders,
        names,
        sent,
        remaining,
        start: std::time::Instant::now(),
        timeout: std::time::Duration::from_millis(promise_timeout_ms),
        batch_id,
    })
}

fn check_pending_batches(
    deno_runtime: &mut deno_core::JsRuntime,
    pending_batches: &mut [PendingBatch],
) {
    use crate::runtime::runtime_factory::v8_utils::v8_to_json;
    use crate::with_scope;

    for batch in pending_batches.iter_mut() {
        for i in 0..batch.sent.len() {
            if batch.sent[i] {
                continue;
            }

            let slot_key = format!("__rari_b{}_{}__", batch.batch_id, i);
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
                        delete globalThis['~rari_concurrent']['{}'];
                        delete globalThis['{}'];
                        return {{
                            ok: entry.error === null,
                            value: entry.result,
                            error: entry.error
                        }};
                    }})()"#,
                    slot_key, slot_key, slot_key
                );
                let result =
                    match deno_runtime.execute_script(format!("extract_concurrent_{i}"), extract) {
                        Ok(extracted) => {
                            let json_result = with_scope!(deno_runtime, |scope| {
                                let local = deno_core::v8::Local::new(scope, extracted);
                                v8_to_json(scope, local)
                            });
                            match json_result {
                                Ok(JsonValue::Object(obj)) => {
                                    if matches!(obj.get("ok"), Some(JsonValue::Bool(false))) {
                                        Err(RariError::js_execution(
                                            obj.get("error")
                                                .and_then(JsonValue::as_str)
                                                .unwrap_or("Unknown concurrent error")
                                                .to_string(),
                                        ))
                                    } else {
                                        Ok(obj.get("value").cloned().unwrap_or(JsonValue::Null))
                                    }
                                }
                                Ok(_) => Err(RariError::internal(
                                    "Concurrent extraction wrapper was not an object".to_string(),
                                )),
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(RariError::js_execution(format!(
                            "Failed to extract result for '{}': {}",
                            batch.names[i], e
                        ))),
                    };

                if let Some(tx) = batch.senders[i].take() {
                    let _ = tx.send(result);
                }
                batch.sent[i] = true;
                batch.remaining -= 1;
            }
        }

        if batch.start.elapsed() >= batch.timeout {
            for i in 0..batch.sent.len() {
                if !batch.sent[i] {
                    if let Some(tx) = batch.senders[i].take() {
                        let _ = tx.send(Err(RariError::timeout(format!(
                            "Promise timed out for '{}'",
                            batch.names[i]
                        ))));
                    }
                    batch.sent[i] = true;
                    batch.remaining -= 1;
                }
            }
        }
    }
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

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::ExecuteScriptForStreaming {
                    script_name,
                    script_code,
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
