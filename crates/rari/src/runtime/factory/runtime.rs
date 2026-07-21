use std::{
    env,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::engine::general_purpose::STANDARD;
use deno_core::{ModuleSpecifier, v8};
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde_json::Value;
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
    time,
};

use crate::{
    runtime::{
        factory::{
            component_ops::pending_component_id,
            executor::{self, execute_script},
            interface::{AsyncBatchResult, JsRuntimeInterface, QueueStreamingScriptFuture},
            runtime_builder::build_js_runtime,
            utils::{
                self,
                constants::{
                    CHANNEL_CAPACITY, JS_EXECUTOR_CHANNEL_CLOSED_ERROR, JS_EXECUTOR_FAILED_ERROR,
                    MODULE_ALREADY_EVALUATED_ERROR, RUNTIME_QUICK_RESTART_DELAY_MS,
                    RUNTIME_RESTART_DELAY_MS, create_already_evaluated_response,
                    create_graceful_error, is_critical_error, is_runtime_restart_needed,
                },
                v8::{get_module_namespace_as_json, v8_to_json},
            },
        },
        module_loader::RariModuleLoader,
        ops::{RequestContextStore, StreamOpState},
    },
    server::{actions::action_form_state_sync_script, middleware::request_context::RequestContext},
    with_scope,
};

fn sync_action_form_state_for_context(
    js_runtime: &mut deno_core::JsRuntime,
    request_context: &RequestContext,
) {
    let script = action_form_state_sync_script(request_context.action_form_state.as_ref());
    let _ = js_runtime.execute_script("sync_action_form_state".to_string(), script);
}

fn clear_action_form_state(js_runtime: &mut deno_core::JsRuntime) {
    let script = action_form_state_sync_script(None);
    let _ = js_runtime.execute_script("clear_action_form_state".to_string(), script);
}

const RESET_USE_CACHE_DYNAMIC_DEPTH_SCRIPT: &str =
    "if (globalThis['~rari']) globalThis['~rari'].useCacheDynamicDepth = 0;";

const CLEAR_PAGE_CACHE_TAGS_SCRIPT: &str = r"
if (globalThis['~rari']) {
    globalThis['~rari'].pageCacheTags ??= new Set();
    globalThis['~rari'].pageCacheTags.clear();
}
";

fn reset_use_cache_dynamic_context(js_runtime: &mut deno_core::JsRuntime) {
    let _ = js_runtime.execute_script(
        "reset_use_cache_dynamic_context".to_string(),
        RESET_USE_CACHE_DYNAMIC_DEPTH_SCRIPT.to_string(),
    );
}

fn clear_page_cache_tags(js_runtime: &mut deno_core::JsRuntime) {
    let _ = js_runtime.execute_script(
        "clear_page_cache_tags".to_string(),
        CLEAR_PAGE_CACHE_TAGS_SCRIPT.to_string(),
    );
}

type ScriptBatchItem = (String, String, oneshot::Sender<Result<Value, RariError>>);
type BatchResultSender = mpsc::UnboundedSender<(usize, Result<Value, RariError>)>;
type PendingScript =
    (oneshot::Sender<Result<Value, RariError>>, String, Option<v8::Global<v8::Value>>);

struct PendingBatch {
    senders: Vec<Option<oneshot::Sender<Result<Value, RariError>>>>,
    names: Vec<String>,
    sent: Vec<bool>,
    remaining: usize,
    start: Instant,
    timeout: Duration,
    batch_id: u64,
}

struct PendingStream {
    stream_id: String,
    slot_key: String,
    result_tx: Option<oneshot::Sender<Result<(), RariError>>>,
    start: Instant,
    timeout: Duration,
    done: bool,
}

enum JsRequest {
    ExecuteScript {
        script_name: String,
        script_code: String,
        result_tx: oneshot::Sender<Result<Value, RariError>>,
    },
    ExecuteScriptBatch {
        scripts: Vec<(String, String)>,
        result_tx: BatchResultSender,
    },
    LoadEsModule {
        component_id: String,
        result_tx: oneshot::Sender<Result<deno_core::ModuleId, RariError>>,
    },
    EvaluateModule {
        module_id: deno_core::ModuleId,
        result_tx: oneshot::Sender<Result<Value, RariError>>,
    },
    GetModuleNamespace {
        module_id: deno_core::ModuleId,
        result_tx: oneshot::Sender<Result<Value, RariError>>,
    },
    AddModuleToLoader {
        specifier: String,
        code: String,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
    ClearModuleLoaderCaches {
        component_id: String,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
    SetRequestContext {
        request_context: Arc<RequestContext>,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
    ClearRequestContextIfMatches {
        expected_context: Arc<RequestContext>,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
    ExecuteScriptForStreaming {
        stream_id: String,
        script_name: String,
        script_code: String,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
        request_context: Option<Arc<RequestContext>>,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
    RegisterRequestContext {
        request_context: Arc<RequestContext>,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
    UnregisterRequestContext {
        request_id: String,
        result_tx: oneshot::Sender<Result<(), RariError>>,
    },
}

#[derive(Clone)]
pub struct RariRuntime {
    request_sender: mpsc::Sender<JsRequest>,
    priority_sender: mpsc::Sender<JsRequest>,
}

fn is_priority_js_request(req: &JsRequest) -> bool {
    match req {
        JsRequest::ExecuteScriptForStreaming { .. }
        | JsRequest::RegisterRequestContext { .. }
        | JsRequest::UnregisterRequestContext { .. } => true,
        JsRequest::ExecuteScript { script_name, .. } => script_name.starts_with("execute_action_"),
        _ => false,
    }
}

/// After this many consecutive priority receives while streams/batches are pending,
/// force an event-loop pump so timers/chunk ops are not starved.
const PRIORITY_FAIRNESS_QUOTA: u32 = 8;

async fn recv_js_request(
    priority_receiver: &mut mpsc::Receiver<JsRequest>,
    request_receiver: &mut mpsc::Receiver<JsRequest>,
) -> Option<JsRequest> {
    tokio::select! {
        biased;
        req = priority_receiver.recv() => req,
        req = request_receiver.recv() => req,
    }
}

impl RariRuntime {
    async fn send_js_request(&self, req: JsRequest) -> Result<(), RariError> {
        let sender =
            if is_priority_js_request(&req) { &self.priority_sender } else { &self.request_sender };

        sender
            .send(req)
            .await
            .map_err(|_| RariError::js_runtime(JS_EXECUTOR_CHANNEL_CLOSED_ERROR.to_string()))
    }

    #[expect(clippy::too_many_lines)]
    pub fn new(env_vars: Option<FxHashMap<String, String>>) -> Self {
        let (request_sender, mut request_receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let (priority_sender, mut priority_receiver) = mpsc::channel(CHANNEL_CAPACITY);

        thread::spawn(move || {
            #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
            let runtime = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");

            runtime.block_on(async {
                loop {
                    let Ok((mut js_runtime, module_loader)) = build_js_runtime(env_vars.clone())
                    else {
                        time::sleep(Duration::from_millis(
                            RUNTIME_RESTART_DELAY_MS,
                        ))
                        .await;
                        continue;
                    };

                    let mut continue_processing = true;
                    let mut pending_batches: Vec<PendingBatch> = Vec::new();
                    let mut pending_streams: Vec<PendingStream> = Vec::new();
                    let mut batch_id_counter: u64 = 0;
                    let mut priority_streak: u32 = 0;

                    while continue_processing {
                        let has_pending =
                            !pending_batches.is_empty() || !pending_streams.is_empty();
                        if has_pending {
                            // Short polls keep Suspense timers and chunk ops progressing under load.
                            // Longer polls delay timer wakeups and stretch stream latency.
                            let pump_budget_ms = if pending_streams.is_empty() { 50 } else { 2 };
                            if priority_streak >= PRIORITY_FAIRNESS_QUOTA {
                                priority_streak = 0;
                                let event_loop_result = time::timeout(
                                    Duration::from_millis(pump_budget_ms),
                                    utils::v8::run_event_loop_with_error_handling(
                                        &mut js_runtime,
                                        "priority fairness pump",
                                    ),
                                )
                                .await;
                                if let Ok(Err(e)) = event_loop_result {
                                    eprintln!("[rari] Event loop error: {e}");
                                    if is_runtime_restart_needed(&e) {
                                        break;
                                    }
                                }
                            } else {
                                tokio::select! {
                                    biased;
                                    request = recv_js_request(&mut priority_receiver, &mut request_receiver) => {
                                        match request {
                                            Some(req) => {
                                                if is_priority_js_request(&req) {
                                                    priority_streak =
                                                        priority_streak.saturating_add(1);
                                                } else {
                                                    priority_streak = 0;
                                                }
                                                let result = handle_js_request(
                                                    req,
                                                    &mut js_runtime,
                                                    &module_loader,
                                                    &mut continue_processing,
                                                    &mut pending_batches,
                                                    &mut pending_streams,
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
                                    event_loop_result = time::timeout(
                                        Duration::from_millis(pump_budget_ms),
                                        utils::v8::run_event_loop_with_error_handling(
                                            &mut js_runtime, "concurrent pending"
                                        ),
                                    ) => {
                                        if let Ok(Err(e)) = event_loop_result {
                                            eprintln!("[rari] Event loop error: {e}");
                                            if is_runtime_restart_needed(&e) {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }

                            check_pending_batches(&mut js_runtime, &mut pending_batches);
                            pending_batches.retain(|b| b.remaining > 0 && b.start.elapsed() < b.timeout);
                            check_pending_streams(&mut js_runtime, &mut pending_streams);
                            pending_streams.retain(|s| !s.done);
                        } else {
                            priority_streak = 0;
                            match recv_js_request(&mut priority_receiver, &mut request_receiver).await {
                                Some(req) => {
                                    let result = handle_js_request(
                                        req,
                                        &mut js_runtime,
                                        &module_loader,
                                        &mut continue_processing,
                                        &mut pending_batches,
                                        &mut pending_streams,
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

                            let event_loop_result = time::timeout(
                                Duration::from_millis(10),
                                utils::v8::run_event_loop_with_error_handling(
                                    &mut js_runtime,
                                    "idle pump",
                                ),
                            )
                            .await;
                            if let Ok(Err(e)) = event_loop_result {
                                eprintln!("[rari] Event loop error: {e}");
                                if is_runtime_restart_needed(&e) {
                                    break;
                                }
                            }
                        }
                    }

                    for batch in pending_batches.drain(..) {
                        for sender in batch.senders.into_iter().flatten() {
                            let _ = sender.send(Err(create_graceful_error()));
                        }
                    }
                    for mut stream in pending_streams.drain(..) {
                        fail_pending_stream(
                            &mut js_runtime,
                            &mut stream,
                            create_graceful_error(),
                        );
                    }

                    if !continue_processing {
                        break;
                    }

                    #[expect(clippy::print_stdout, reason = "Runtime restart notification for debugging")]
                    {
                        println!("[rari] Restarting JS runtime due to error or forced restart");
                    }
                    time::sleep(Duration::from_millis(
                        RUNTIME_QUICK_RESTART_DELAY_MS,
                    ))
                    .await;
                }
            });
        });

        Self { request_sender, priority_sender }
    }
}

async fn handle_load_es_module(
    js_runtime: &mut deno_core::JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
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
                                js_runtime.load_side_es_module(&versioned_module_specifier).await
                            }
                            Err(_) => js_runtime.load_side_es_module(&module_specifier).await,
                        }
                    } else {
                        js_runtime.load_side_es_module(&module_specifier).await
                    }
                } else {
                    js_runtime.load_side_es_module(&module_specifier).await
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
    js_runtime: &mut deno_core::JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    module_id: deno_core::ModuleId,
    result_tx: oneshot::Sender<Result<Value, RariError>>,
    continue_processing: &mut bool,
) -> Result<(), RariError> {
    let module_registered = module_loader.is_already_evaluated(&module_id.to_string());

    let result = if module_registered {
        match get_module_namespace_as_json(js_runtime, module_id) {
            Ok(json_result) => Ok(json_result),
            Err(_) => Ok(create_already_evaluated_response("get_module_namespace")),
        }
    } else {
        match js_runtime.mod_evaluate(module_id).await {
            Ok(()) => {
                module_loader.mark_module_evaluated(&module_id.to_string());
                get_module_namespace_as_json(js_runtime, module_id)
            }
            Err(e) => {
                if e.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR) {
                    module_loader.mark_module_evaluated(&module_id.to_string());

                    match get_module_namespace_as_json(js_runtime, module_id) {
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
        #[expect(clippy::print_stdout, reason = "Critical error logging before shutdown")]
        {
            println!("[rari] Critical error detected in module evaluation: {e}");
        }
        *continue_processing = false;
    }

    let _ = result_tx.send(result);
    Ok::<(), RariError>(())
}

async fn handle_get_module_namespace(
    js_runtime: &mut deno_core::JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    module_id: deno_core::ModuleId,
    result_tx: oneshot::Sender<Result<Value, RariError>>,
) -> Result<(), RariError> {
    let module_evaluated = module_loader.is_already_evaluated(&module_id.to_string());

    if module_evaluated {
        let json_result =
            get_module_namespace_as_json(js_runtime, module_id as deno_core::ModuleId);
        let _ = result_tx.send(json_result);
    } else {
        match js_runtime.mod_evaluate(module_id as deno_core::ModuleId).await {
            Ok(()) => {
                module_loader.mark_module_evaluated(&module_id.to_string());
                let json_result =
                    get_module_namespace_as_json(js_runtime, module_id as deno_core::ModuleId);
                let _ = result_tx.send(json_result);
            }
            Err(e) => {
                if e.to_string().contains(MODULE_ALREADY_EVALUATED_ERROR) {
                    module_loader.mark_module_evaluated(&module_id.to_string());
                    let json_result =
                        get_module_namespace_as_json(js_runtime, module_id as deno_core::ModuleId);
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

#[expect(clippy::too_many_lines)]
async fn handle_js_request(
    request: JsRequest,
    js_runtime: &mut deno_core::JsRuntime,
    module_loader: &Rc<RariModuleLoader>,
    continue_processing: &mut bool,
    pending_batches: &mut Vec<PendingBatch>,
    pending_streams: &mut Vec<PendingStream>,
    batch_id_counter: &mut u64,
) -> Result<(), RariError> {
    match request {
        JsRequest::ExecuteScript { script_name, script_code, result_tx } => {
            let result =
                execute_script(js_runtime, module_loader, &script_name, &script_code).await;
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
                    let (tx, rx) = oneshot::channel::<Result<Value, RariError>>();
                    let result_tx_clone = result_tx.clone();
                    tokio::spawn(async move {
                        let result = rx.await.unwrap_or_else(|_| Err(create_graceful_error()));
                        let _ = result_tx_clone.send((i, result));
                    });
                    (name, code, tx)
                })
                .collect();

            if let Some(pending_batch) = setup_concurrent_batch(js_runtime, batch, batch_id_counter)
            {
                pending_batches.push(pending_batch);
            }
        }
        JsRequest::LoadEsModule { component_id, result_tx } => {
            handle_load_es_module(js_runtime, module_loader, &component_id, result_tx).await?;
        }
        JsRequest::EvaluateModule { module_id, result_tx } => {
            handle_evaluate_module(
                js_runtime,
                module_loader,
                module_id,
                result_tx,
                continue_processing,
            )
            .await?;
        }
        JsRequest::GetModuleNamespace { module_id, result_tx } => {
            handle_get_module_namespace(js_runtime, module_loader, module_id, result_tx).await?;
        }
        JsRequest::AddModuleToLoader { specifier, code, result_tx } => {
            module_loader.set_module_code(specifier.clone(), code);
            let component_id = extract_component_id_from_specifier(&specifier);
            let is_pending_hmr = specifier.contains("/rari_hmr/pending/");
            let is_hmr_specifier = specifier.contains("/rari_hmr/");
            if is_pending_hmr {
                let pending_id = pending_component_id(&component_id);
                module_loader.register_component_specifier(&pending_id, &specifier);
            } else {
                let existing_specifier = module_loader
                    .component_specifiers
                    .get(&component_id)
                    .map(|entry| entry.clone());
                let has_existing_hmr_mapping = existing_specifier
                    .as_ref()
                    .map(|spec| spec.contains("/rari_hmr/"))
                    .unwrap_or(false);
                if is_hmr_specifier || !has_existing_hmr_mapping {
                    module_loader.register_component_specifier(&component_id, &specifier);
                }
            }
            let _ = result_tx.send(Ok(()));
        }
        JsRequest::ClearModuleLoaderCaches { component_id, result_tx } => {
            module_loader.clear_component_caches(&component_id);
            let _ = result_tx.send(Ok(()));
        }
        JsRequest::SetRequestContext { request_context, result_tx } => {
            clear_page_cache_tags(js_runtime);
            reset_use_cache_dynamic_context(js_runtime);
            sync_action_form_state_for_context(js_runtime, &request_context);
            js_runtime.op_state().borrow_mut().put(request_context);
            let _ = result_tx.send(Ok(()));
        }
        JsRequest::ClearRequestContextIfMatches { expected_context, result_tx } => {
            let should_clear = {
                let op_state = js_runtime.op_state();
                let borrowed = op_state.borrow();
                if let Some(current_context) = borrowed.try_borrow::<Arc<RequestContext>>() {
                    Arc::ptr_eq(current_context, &expected_context)
                } else {
                    false
                }
            };

            if should_clear {
                js_runtime.op_state().borrow_mut().try_take::<Arc<RequestContext>>();
                clear_action_form_state(js_runtime);
            }
            let _ = result_tx.send(Ok(()));
        }
        JsRequest::ExecuteScriptForStreaming {
            stream_id,
            script_name,
            script_code,
            chunk_sender,
            request_context,
            result_tx,
        } => {
            if let Some(request_context) = request_context {
                let op_state = js_runtime.op_state();
                let mut borrowed = op_state.borrow_mut();
                if let Some(store) = borrowed.try_borrow_mut::<RequestContextStore>() {
                    store.insert(request_context);
                } else {
                    let mut store = RequestContextStore::default();
                    store.insert(request_context);
                    borrowed.put(store);
                }
            }
            match executor::start_streaming_script(
                js_runtime,
                &script_name,
                &script_code,
                &stream_id,
                chunk_sender,
            ) {
                Ok(slot_key) => {
                    pending_streams.push(PendingStream {
                        stream_id,
                        slot_key,
                        result_tx: Some(result_tx),
                        start: Instant::now(),
                        timeout: Duration::from_millis(executor::streaming_promise_timeout_ms()),
                        done: false,
                    });
                }
                Err(e) => {
                    if is_runtime_restart_needed(&e) {
                        let _ = result_tx.send(Err(create_graceful_error()));
                        return Err(RariError::internal("Runtime restart needed".to_string()));
                    }
                    let _ = result_tx.send(Err(e));
                }
            }
        }
        JsRequest::RegisterRequestContext { request_context, result_tx } => {
            let op_state = js_runtime.op_state();
            let mut borrowed = op_state.borrow_mut();
            if let Some(store) = borrowed.try_borrow_mut::<RequestContextStore>() {
                store.insert(request_context);
            } else {
                let mut store = RequestContextStore::default();
                store.insert(request_context);
                borrowed.put(store);
            }
            let _ = result_tx.send(Ok(()));
        }
        JsRequest::UnregisterRequestContext { request_id, result_tx } => {
            let op_state = js_runtime.op_state();
            let mut borrowed = op_state.borrow_mut();
            if let Some(store) = borrowed.try_borrow_mut::<RequestContextStore>() {
                store.remove(&request_id);
            }
            let _ = result_tx.send(Ok(()));
        }
    }
    Ok(())
}

fn setup_concurrent_batch(
    js_runtime: &mut deno_core::JsRuntime,
    batch: Vec<ScriptBatchItem>,
    batch_id_counter: &mut u64,
) -> Option<PendingBatch> {
    *batch_id_counter += 1;
    let batch_id = *batch_id_counter;

    let mut pending: Vec<PendingScript> = Vec::with_capacity(batch.len());

    for (script_name, script_code, tx) in batch {
        match js_runtime.execute_script(script_name.clone(), script_code.clone()) {
            Ok(v8_val) => {
                let slot_key = format!("__rari_b{}_{}__", batch_id, pending.len());
                let store_result = with_scope!(js_runtime, |scope| {
                    let local_val = deno_core::v8::Local::new(scope, &v8_val);
                    let context = scope.get_current_context();
                    let global = context.global(scope);
                    if let Some(key_str) = v8::String::new(scope, &slot_key) {
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
                    "Failed to execute script '{script_name}': {e}"
                ));
                let _ = tx.send(Err(err));
            }
        }
    }

    if pending.is_empty() {
        return None;
    }

    let promise_timeout_ms: u64 = env::var("RARI_PROMISE_RESOLUTION_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000);

    let mut sent = vec![false; pending.len()];
    let mut remaining = pending.len();

    let (mut senders, names): (Vec<_>, Vec<_>) =
        pending.into_iter().map(|(tx, name, _)| (Some(tx), name)).unzip();

    for (i, sent_item) in sent.iter_mut().enumerate().take(senders.len()) {
        let slot_key = format!("__rari_b{batch_id}_{i}__");
        let setup = format!(
            r"(function() {{
                if (!globalThis['~rari_concurrent']) globalThis['~rari_concurrent'] = {{}};
                const val = globalThis['{slot_key}'];
                if (val && typeof val.then === 'function') {{
                    globalThis['~rari_concurrent']['{slot_key}'] = {{ done: false, result: null, error: null }};
                    val.then(function(r) {{
                        globalThis['~rari_concurrent']['{slot_key}'] = {{ done: true, result: r, error: null }};
                    }}).catch(function(e) {{
                        globalThis['~rari_concurrent']['{slot_key}'] = {{ done: true, result: null, error: String(e) }};
                    }});
                }} else {{
                    globalThis['~rari_concurrent']['{slot_key}'] = {{ done: true, result: val, error: null }};
                }}
            }})()"
        );
        if let Err(e) = js_runtime.execute_script(format!("setup_concurrent_{i}"), setup) {
            eprintln!("[rari] Failed to setup concurrent tracking for slot {i}: {e}");
            let cleanup = format!(
                "delete globalThis['{slot_key}']; delete globalThis['~rari_concurrent']['{slot_key}']"
            );
            let _ = js_runtime.execute_script(format!("cleanup_setup_{i}"), cleanup);

            *sent_item = true;

            remaining = remaining.saturating_sub(1);

            if let Some(sender) = senders[i].take() {
                let _ = sender.send(Err(RariError::internal(format!(
                    "Failed to setup concurrent tracking: {e}"
                ))));
            }
        }
    }

    Some(PendingBatch {
        senders,
        names,
        sent,
        remaining,
        start: Instant::now(),
        timeout: Duration::from_millis(promise_timeout_ms),
        batch_id,
    })
}

#[expect(clippy::too_many_lines)]
fn check_pending_batches(
    js_runtime: &mut deno_core::JsRuntime,
    pending_batches: &mut [PendingBatch],
) {
    for batch in pending_batches.iter_mut() {
        for i in 0..batch.sent.len() {
            if batch.sent[i] {
                continue;
            }

            let slot_key = format!("__rari_b{}_{}__", batch.batch_id, i);
            let check = format!(
                r"(function() {{
                    const e = globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{slot_key}'];
                    return e && e.done ? e : null;
                }})()"
            );

            let check_result = js_runtime.execute_script(format!("check_slot_{i}"), check);

            if let Err(e) = check_result {
                let cleanup = format!(
                    r"(function() {{
                        if (globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{slot_key}']) {{
                            delete globalThis['~rari_concurrent']['{slot_key}'];
                        }}
                        if (globalThis['{slot_key}']) {{
                            delete globalThis['{slot_key}'];
                        }}
                    }})()"
                );
                let _ = js_runtime.execute_script(format!("cleanup_check_error_{i}"), cleanup);

                if let Some(tx) = batch.senders[i].take() {
                    let _ = tx.send(Err(RariError::js_execution(format!(
                        "Failed to check status for '{}': {}",
                        batch.names[i], e
                    ))));
                }
                batch.sent[i] = true;
                batch.remaining -= 1;
                continue;
            }

            #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
            let check_value = check_result.expect("check_result is Ok after error check");
            let is_done = with_scope!(js_runtime, |scope| {
                let local = deno_core::v8::Local::new(scope, check_value);
                !local.is_null_or_undefined()
            });

            if is_done {
                let extract = format!(
                    r"(function() {{
                        const entry = globalThis['~rari_concurrent']['{slot_key}'];
                        delete globalThis['~rari_concurrent']['{slot_key}'];
                        delete globalThis['{slot_key}'];
                        return {{
                            ok: entry.error === null,
                            value: entry.result,
                            error: entry.error
                        }};
                    }})()"
                );
                let result = match js_runtime
                    .execute_script(format!("extract_concurrent_{i}"), extract)
                {
                    Ok(extracted) => {
                        let json_result = with_scope!(js_runtime, |scope| {
                            let local = deno_core::v8::Local::new(scope, extracted);
                            v8_to_json(scope, local)
                        });
                        match json_result {
                            Ok(Value::Object(obj)) => {
                                if matches!(obj.get("ok"), Some(Value::Bool(false))) {
                                    Err(RariError::js_execution(
                                        obj.get("error")
                                            .and_then(Value::as_str)
                                            .unwrap_or("Unknown concurrent error")
                                            .to_string(),
                                    ))
                                } else {
                                    Ok(obj.get("value").cloned().unwrap_or(Value::Null))
                                }
                            }
                            Ok(_) => {
                                let cleanup = format!(
                                    r"(function() {{
                                            if (globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{slot_key}']) {{
                                                delete globalThis['~rari_concurrent']['{slot_key}'];
                                            }}
                                            if (globalThis['{slot_key}']) {{
                                                delete globalThis['{slot_key}'];
                                            }}
                                        }})()"
                                );
                                let _ = js_runtime
                                    .execute_script(format!("cleanup_extract_shape_{i}"), cleanup);
                                Err(RariError::internal(
                                    "Concurrent extraction wrapper was not an object".to_string(),
                                ))
                            }
                            Err(e) => {
                                let cleanup = format!(
                                    r"(function() {{
                                            if (globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{slot_key}']) {{
                                                delete globalThis['~rari_concurrent']['{slot_key}'];
                                            }}
                                            if (globalThis['{slot_key}']) {{
                                                delete globalThis['{slot_key}'];
                                            }}
                                        }})()"
                                );
                                let _ = js_runtime
                                    .execute_script(format!("cleanup_extract_json_{i}"), cleanup);
                                Err(e)
                            }
                        }
                    }
                    Err(e) => {
                        let cleanup = format!(
                            r"(function() {{
                                    if (globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{slot_key}']) {{
                                        delete globalThis['~rari_concurrent']['{slot_key}'];
                                    }}
                                    if (globalThis['{slot_key}']) {{
                                        delete globalThis['{slot_key}'];
                                    }}
                                }})()"
                        );
                        let _ = js_runtime
                            .execute_script(format!("cleanup_extract_error_{i}"), cleanup);
                        Err(RariError::js_execution(format!(
                            "Failed to extract result for '{}': {}",
                            batch.names[i], e
                        )))
                    }
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
                    let slot_key = format!("__rari_b{}_{}__", batch.batch_id, i);
                    let cleanup = format!(
                        r"(function() {{
                            if (globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{slot_key}']) {{
                                delete globalThis['~rari_concurrent']['{slot_key}'];
                            }}
                            if (globalThis['{slot_key}']) {{
                                delete globalThis['{slot_key}'];
                            }}
                        }})()"
                    );
                    let _ = js_runtime.execute_script(format!("cleanup_timeout_{i}"), cleanup);

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

fn take_stream_sender(
    js_runtime: &deno_core::JsRuntime,
    stream_id: &str,
) -> Option<mpsc::Sender<Result<Vec<u8>, RariError>>> {
    let op_state = js_runtime.op_state();
    let mut borrowed = op_state.borrow_mut();
    borrowed.try_borrow_mut::<StreamOpState>().and_then(|state| state.take_sender(stream_id))
}

fn fail_pending_stream(
    js_runtime: &mut deno_core::JsRuntime,
    stream: &mut PendingStream,
    err: RariError,
) {
    let slot_key = &stream.slot_key;
    let cleanup = format!(
        r"(function() {{
            if (globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{slot_key}']) {{
                delete globalThis['~rari_concurrent']['{slot_key}'];
            }}
            if (globalThis['{slot_key}']) {{
                delete globalThis['{slot_key}'];
            }}
        }})()"
    );
    let _ = js_runtime.execute_script(format!("cleanup_stream_fail_{slot_key}"), cleanup);

    if let Some(sender) = take_stream_sender(js_runtime, &stream.stream_id) {
        let _ = sender.try_send(Err(err.clone()));
    }
    {
        let op_state = js_runtime.op_state();
        let mut borrowed = op_state.borrow_mut();
        if let Some(state) = borrowed.try_borrow_mut::<StreamOpState>() {
            let _ = state.take_settled(&stream.stream_id);
        }
    }
    if let Some(tx) = stream.result_tx.take() {
        let _ = tx.send(Err(err));
    }
    stream.done = true;
}

fn check_pending_streams(
    js_runtime: &mut deno_core::JsRuntime,
    pending_streams: &mut [PendingStream],
) {
    for stream in pending_streams.iter_mut() {
        if stream.done {
            continue;
        }

        let settled = {
            let op_state = js_runtime.op_state();
            let mut borrowed = op_state.borrow_mut();
            borrowed
                .try_borrow_mut::<StreamOpState>()
                .and_then(|state| state.take_settled(&stream.stream_id))
        };

        let Some(settled) = settled else {
            if stream.start.elapsed() >= stream.timeout {
                let err = RariError::timeout(format!(
                    "Streaming script timed out for '{}'",
                    stream.stream_id
                ));
                fail_pending_stream(js_runtime, stream, err);
            }
            continue;
        };

        let slot_key = &stream.slot_key;
        let cleanup = format!(
            r"(function() {{
                if (globalThis['~rari_concurrent'] && globalThis['~rari_concurrent']['{slot_key}']) {{
                    delete globalThis['~rari_concurrent']['{slot_key}'];
                }}
                if (globalThis['{slot_key}']) {{
                    delete globalThis['{slot_key}'];
                }}
            }})()"
        );
        let _ = js_runtime.execute_script(format!("cleanup_stream_{slot_key}"), cleanup);

        let result = settled.map_err(RariError::js_execution);

        let leftover = take_stream_sender(js_runtime, &stream.stream_id);
        if let Some(sender) = leftover {
            let stream_err = match &result {
                Err(err) => err.clone(),
                Ok(()) => RariError::js_execution(format!(
                    "Streaming script for '{}' ended without completing the stream",
                    stream.stream_id
                )),
            };
            let _ = sender.try_send(Err(stream_err.clone()));
            if result.is_ok() {
                if let Some(tx) = stream.result_tx.take() {
                    let _ = tx.send(Err(stream_err));
                }
                stream.done = true;
                continue;
            }
        }

        if let Some(tx) = stream.result_tx.take() {
            let _ = tx.send(result);
        }
        stream.done = true;
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
    } else if let Some(hmr_idx) = specifier.rfind("/rari_hmr/pending/") {
        let after_hmr = &specifier[hmr_idx + "/rari_hmr/pending/".len()..];
        after_hmr.split('?').next().unwrap_or(after_hmr).trim_end_matches(".js").to_string()
    } else if let Some(hmr_idx) = specifier.rfind("/rari_hmr/server/") {
        let after_hmr = &specifier[hmr_idx + "/rari_hmr/server/".len()..];
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

impl JsRuntimeInterface for RariRuntime {
    fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
        let runtime = self.clone();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            runtime
                .send_js_request(JsRequest::ExecuteScript {
                    script_name,
                    script_code,
                    result_tx: response_sender,
                })
                .await?;

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
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send + 'static>> {
        let request_sender = self.request_sender.clone();
        let function_name = function_name.to_string();

        Box::pin(async move {
            let args_json = serde_json::to_string(&args)
                .map_err(|e| RariError::js_runtime(format!("Failed to serialize args: {e}")))?;

            let unique_id =
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();

            let args_base64 = base64::Engine::encode(&STANDARD, args_json.as_bytes());

            let escaped_function_name = function_name.replace('\\', "\\\\").replace('"', "\\\"");
            let script = format!(
                r#"
                (function() {{
                    const argsBase64 = "{args_base64}";
                    const argsBinary = atob(argsBase64);
                    const argsBytes = new Uint8Array(argsBinary.length);
                    for (let i = 0; i < argsBinary.length; i++) {{
                        argsBytes[i] = argsBinary.charCodeAt(i);
                    }}
                    const argsJson = new TextDecoder('utf-8').decode(argsBytes);
                    const args = JSON.parse(argsJson);

                    if (typeof globalThis["{escaped_function_name}"] !== 'function') {{
                        throw new Error("Function not found: {escaped_function_name}");
                    }}

                    return globalThis["{escaped_function_name}"](...args);
                }})();
                "#
            );

            let (response_sender, response_receiver) = oneshot::channel();

            request_sender
                .send(JsRequest::ExecuteScript {
                    script_name: format!("exec_func_{function_name}_{unique_id}.js"),
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
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
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
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>> {
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

    fn add_module_to_loader(
        &self,
        specifier: &str,
        code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let request_sender = self.request_sender.clone();
        let specifier_str = specifier.to_string();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();
            request_sender
                .send(JsRequest::AddModuleToLoader {
                    specifier: specifier_str,
                    code,
                    result_tx: response_sender,
                })
                .await
                .map_err(|_| {
                    RariError::js_runtime("JS executor channel closed (add_module)".to_string())
                })?;
            response_receiver.await.map_err(|_| {
                RariError::js_runtime("JS executor failed to respond (add_module)".to_string())
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
        request_context: Arc<RequestContext>,
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

    fn clear_request_context_if_matches(
        &self,
        expected_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let request_sender = self.request_sender.clone();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();
            request_sender
                .send(JsRequest::ClearRequestContextIfMatches {
                    expected_context,
                    result_tx: response_sender,
                })
                .await
                .map_err(|_| {
                    RariError::js_runtime(
                        "JS executor channel closed (clear_request_context_if_matches)".to_string(),
                    )
                })?;
            response_receiver.await.map_err(|_| {
                RariError::js_runtime(
                    "JS executor failed to respond (clear_request_context_if_matches)".to_string(),
                )
            })?
        })
    }

    fn execute_script_for_streaming(
        &self,
        stream_id: String,
        script_name: String,
        script_code: String,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let runtime = Self {
            request_sender: self.request_sender.clone(),
            priority_sender: self.priority_sender.clone(),
        };

        Box::pin(async move {
            let completion = runtime
                .queue_script_for_streaming(stream_id, script_name, script_code, chunk_sender, None)
                .await?;
            completion.await
        })
    }

    fn queue_script_for_streaming(
        &self,
        stream_id: String,
        script_name: String,
        script_code: String,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
        request_context: Option<Arc<RequestContext>>,
    ) -> QueueStreamingScriptFuture {
        let runtime = Self {
            request_sender: self.request_sender.clone(),
            priority_sender: self.priority_sender.clone(),
        };

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();

            runtime
                .send_js_request(JsRequest::ExecuteScriptForStreaming {
                    stream_id,
                    script_name,
                    script_code,
                    chunk_sender,
                    request_context,
                    result_tx: response_sender,
                })
                .await?;

            let completion: Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> =
                Box::pin(async move {
                    response_receiver.await.map_err(|_| {
                        RariError::js_runtime(
                            "JS executor failed to respond (execute_script_for_streaming)"
                                .to_string(),
                        )
                    })?
                });
            Ok(completion)
        })
    }

    fn register_request_context(
        &self,
        request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let runtime = Self {
            request_sender: self.request_sender.clone(),
            priority_sender: self.priority_sender.clone(),
        };

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();
            runtime
                .send_js_request(JsRequest::RegisterRequestContext {
                    request_context,
                    result_tx: response_sender,
                })
                .await?;
            response_receiver.await.map_err(|_| {
                RariError::js_runtime(
                    "JS executor failed to respond (register_request_context)".to_string(),
                )
            })?
        })
    }

    fn unregister_request_context(
        &self,
        request_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let runtime = Self {
            request_sender: self.request_sender.clone(),
            priority_sender: self.priority_sender.clone(),
        };
        let request_id = request_id.to_string();

        Box::pin(async move {
            let (response_sender, response_receiver) = oneshot::channel();
            runtime
                .send_js_request(JsRequest::UnregisterRequestContext {
                    request_id,
                    result_tx: response_sender,
                })
                .await?;
            response_receiver.await.map_err(|_| {
                RariError::js_runtime(
                    "JS executor failed to respond (unregister_request_context)".to_string(),
                )
            })?
        })
    }
}
