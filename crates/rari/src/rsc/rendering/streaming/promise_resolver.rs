use cow_utils::CowUtils;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{error, info, warn};

use crate::runtime::JsExecutionRuntime;

use super::constants::PROMISE_RESOLUTION_SCRIPT;
use super::types::{BoundaryError, BoundaryUpdate, PendingSuspensePromise, RscWireFormatTag};

pub(super) fn process_client_components(
    content: &mut serde_json::Value,
    row_counter: &mut u32,
) -> Vec<String> {
    let mut import_rows = Vec::new();
    let mut component_map: FxHashMap<String, String> = FxHashMap::default();

    collect_client_components(content, &mut component_map, row_counter, &mut import_rows);

    replace_client_component_paths(content, &component_map);

    import_rows
}

fn collect_client_components(
    value: &serde_json::Value,
    component_map: &mut FxHashMap<String, String>,
    row_counter: &mut u32,
    import_rows: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Array(arr) => {
            if arr.len() >= 4
                && arr[0].as_str() == Some("$")
                && let Some(type_str) = arr[1].as_str()
                && (type_str.contains('/') || type_str.contains('\\') || type_str.contains('#'))
                && !type_str.starts_with("$L")
            {
                let normalized_type_str = type_str.cow_replace('\\', "/");
                let type_str_normalized = normalized_type_str.as_ref();

                if !component_map.contains_key(type_str)
                    && !component_map.contains_key(type_str_normalized)
                {
                    *row_counter += 1;
                    let module_ref = format!("$L{row_counter:x}");

                    let (file_path, export_name) = if let Some(idx) = type_str_normalized.find('#')
                    {
                        (&type_str_normalized[..idx], &type_str_normalized[idx + 1..])
                    } else {
                        (type_str_normalized, "default")
                    };

                    let import_data = serde_json::json!({
                        "id": file_path,
                        "chunks": [],
                        "name": export_name
                    });
                    let import_row = RscWireFormatTag::ModuleImport
                        .format_row(*row_counter, &import_data.to_string());
                    import_rows.push(import_row.trim_end().to_string());

                    component_map.insert(type_str.to_string(), module_ref.clone());
                    component_map.insert(type_str_normalized.to_string(), module_ref);
                }
            }

            for item in arr {
                collect_client_components(item, component_map, row_counter, import_rows);
            }
        }
        serde_json::Value::Object(obj) => {
            for value in obj.values() {
                collect_client_components(value, component_map, row_counter, import_rows);
            }
        }
        _ => {}
    }
}

fn replace_client_component_paths(
    value: &mut serde_json::Value,
    component_map: &FxHashMap<String, String>,
) {
    match value {
        serde_json::Value::Array(arr) => {
            if arr.len() >= 4
                && arr[0].as_str() == Some("$")
                && let Some(type_str) = arr[1].as_str()
            {
                let module_ref = component_map.get(type_str).or_else(|| {
                    let normalized = type_str.cow_replace('\\', "/");
                    component_map.get(normalized.as_ref())
                });

                if let Some(module_ref) = module_ref {
                    arr[1] = serde_json::Value::String(module_ref.clone());
                }
            }

            for item in arr {
                replace_client_component_paths(item, component_map);
            }
        }
        serde_json::Value::Object(obj) => {
            for value in obj.values_mut() {
                replace_client_component_paths(value, component_map);
            }
        }
        _ => {}
    }
}

fn replace_lazy_markers(value: &mut serde_json::Value, promise_to_row: &FxHashMap<String, u32>) {
    match value {
        serde_json::Value::Array(arr) => {
            for item in arr {
                replace_lazy_markers(item, promise_to_row);
            }
        }

        serde_json::Value::Object(obj) => {
            let is_lazy = obj.get("~rari_lazy").and_then(serde_json::Value::as_bool) == Some(true);

            let promise_id = obj.get("~rari_promise_id").and_then(|v| v.as_str());

            if is_lazy && let Some(promise_id) = promise_id {
                let Some(row_id) = promise_to_row.get(promise_id) else {
                    warn!(
                        "Lazy marker missing from promise_to_row in streaming: {}",
                        promise_id
                    );
                    *value = serde_json::Value::Null;
                    return;
                };

                *value = serde_json::Value::String(format!("${row_id:x}"));

                return;
            }

            for child in obj.values_mut() {
                replace_lazy_markers(child, promise_to_row);
            }
        }

        _ => {}
    }
}

fn nested_pending_promises(
    result_data: &serde_json::Value,
    render_generation: u32,
) -> Vec<PendingSuspensePromise> {
    result_data["pending_promises"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|p| {
            let promise_id = p["id"].as_str()?;
            let boundary_id = p["boundaryId"]
                .as_str()
                .or_else(|| p["~boundaryId"].as_str())
                .unwrap_or("root")
                .to_string();

            Some(PendingSuspensePromise {
                id: promise_id.to_string(),
                boundary_id,
                component_path: p["componentPath"]
                    .as_str()
                    .unwrap_or("AsyncComponent")
                    .to_string(),
                promise_handle: promise_id.to_string(),
                render_generation,
            })
        })
        .collect()
}

async fn get_or_allocate_row_id(
    promise_id: &str,
    promise_to_row_map: &Arc<Mutex<FxHashMap<String, u32>>>,
    shared_row_counter: &Arc<Mutex<u32>>,
) -> u32 {
    if let Some(row_id) = {
        let map = promise_to_row_map.lock().await;
        map.get(promise_id).copied()
    } {
        return row_id;
    }

    let new_row_id = {
        let mut counter = shared_row_counter.lock().await;
        *counter += 1;
        *counter
    };

    let mut map = promise_to_row_map.lock().await;
    match map.get(promise_id).copied() {
        Some(row_id) => row_id,
        None => {
            map.insert(promise_id.to_string(), new_row_id);
            new_row_id
        }
    }
}

pub struct BackgroundPromiseResolver {
    runtime: Arc<JsExecutionRuntime>,
    active_promises: Arc<Mutex<FxHashMap<String, PendingSuspensePromise>>>,
    update_sender: mpsc::UnboundedSender<BoundaryUpdate>,
    error_sender: mpsc::UnboundedSender<BoundaryError>,
    shared_row_counter: Arc<Mutex<u32>>,
    promise_to_row: Arc<Mutex<FxHashMap<String, u32>>>,
}

impl BackgroundPromiseResolver {
    pub fn new(
        runtime: Arc<JsExecutionRuntime>,
        update_sender: mpsc::UnboundedSender<BoundaryUpdate>,
        error_sender: mpsc::UnboundedSender<BoundaryError>,
        shared_row_counter: Arc<Mutex<u32>>,
        promise_to_row: Arc<Mutex<FxHashMap<String, u32>>>,
    ) -> Self {
        Self {
            runtime,
            active_promises: Arc::new(Mutex::new(FxHashMap::default())),
            update_sender,
            error_sender,
            shared_row_counter,
            promise_to_row,
        }
    }

    pub fn resolve_async(&self, promise: PendingSuspensePromise) {
        self.resolve_all(vec![promise]);
    }

    pub fn resolve_all(&self, promises: Vec<PendingSuspensePromise>) {
        if promises.is_empty() {
            return;
        }

        let runtime = Arc::clone(&self.runtime);
        let update_sender = self.update_sender.clone();
        let error_sender = self.error_sender.clone();
        let shared_row_counter = Arc::clone(&self.shared_row_counter);
        let active_promises = Arc::clone(&self.active_promises);
        let promise_to_row_map = Arc::clone(&self.promise_to_row);

        tokio::spawn(async move {
            let mut queue: VecDeque<PendingSuspensePromise> = promises.into_iter().collect();
            let mut scheduled_ids = FxHashSet::default();

            {
                let mut active = active_promises.lock().await;
                for p in &queue {
                    scheduled_ids.insert(p.id.clone());
                    active.insert(p.id.clone(), p.clone());
                }
            }

            while !queue.is_empty() {
                let batch: Vec<PendingSuspensePromise> = queue.drain(..).collect();
                let scripts: Vec<(String, String)> = batch
                    .iter()
                    .map(|p| {
                        let script = PROMISE_RESOLUTION_SCRIPT
                            .cow_replace("{promise_id}", &p.id)
                            .cow_replace("{boundary_id}", &p.boundary_id)
                            .cow_replace("{component_path}", &p.component_path)
                            .cow_replace("{render_generation}", &p.render_generation.to_string())
                            .into_owned();
                        (format!("<promise_resolution_{}>", p.id), script)
                    })
                    .collect();

                let mut result_rx = runtime.execute_script_batch(scripts).await;
                let n = batch.len();
                let mut received = 0;
                let mut received_indices = FxHashSet::default();

                while received < n {
                    match result_rx.recv().await {
                        Some((idx, result)) => {
                            if idx >= batch.len() || !received_indices.insert(idx) {
                                error!("Ignoring invalid or duplicate batch result index {}", idx);
                                continue;
                            }
                            received += 1;
                            let promise = &batch[idx];
                            let promise_id = &promise.id;
                            let boundary_id = &promise.boundary_id;

                            match result {
                                Ok(result_val) => {
                                    let result_string = result_val.to_string();
                                    match serde_json::from_str::<serde_json::Value>(&result_string)
                                    {
                                        Ok(result_data) => {
                                            if result_data
                                                .get("stale")
                                                .and_then(serde_json::Value::as_bool)
                                                .unwrap_or(false)
                                            {
                                                continue;
                                            }

                                            if result_data["success"].as_bool().unwrap_or(false) {
                                                let mut content = result_data["content"].clone();
                                                let nested_promises = nested_pending_promises(
                                                    &result_data,
                                                    promise.render_generation,
                                                );

                                                {
                                                    let mut active = active_promises.lock().await;
                                                    for nested in &nested_promises {
                                                        if scheduled_ids.insert(nested.id.clone()) {
                                                            active.insert(
                                                                nested.id.clone(),
                                                                nested.clone(),
                                                            );
                                                            queue.push_back(nested.clone());
                                                        }
                                                    }
                                                }

                                                for nested in &nested_promises {
                                                    get_or_allocate_row_id(
                                                        &nested.id,
                                                        &promise_to_row_map,
                                                        &shared_row_counter,
                                                    )
                                                    .await;
                                                }

                                                let map = {
                                                    let map = promise_to_row_map.lock().await;
                                                    map.clone()
                                                };

                                                replace_lazy_markers(&mut content, &map);

                                                let row_id = get_or_allocate_row_id(
                                                    promise_id,
                                                    &promise_to_row_map,
                                                    &shared_row_counter,
                                                )
                                                .await;
                                                let import_rows = {
                                                    let mut counter =
                                                        shared_row_counter.lock().await;
                                                    process_client_components(
                                                        &mut content,
                                                        &mut counter,
                                                    )
                                                };
                                                let update = BoundaryUpdate {
                                                    boundary_id: boundary_id.clone(),
                                                    content,
                                                    row_id,
                                                    dom_path: Vec::new(),
                                                    import_rows,
                                                };
                                                if let Err(e) = update_sender.send(update) {
                                                    error!(
                                                        "Failed to send boundary update for {}: {}",
                                                        boundary_id, e
                                                    );
                                                }
                                            } else {
                                                let error_message = result_data["error"]
                                                    .as_str()
                                                    .unwrap_or("Unknown error");
                                                let error_name = result_data["errorName"]
                                                    .as_str()
                                                    .unwrap_or("UnknownError");
                                                let error_stack = result_data["errorStack"]
                                                    .as_str()
                                                    .unwrap_or("No stack trace");
                                                let error_context = &result_data["errorContext"];
                                                info!(
                                                    "Promise resolution failed for boundary {}: {} (Name: {}, Phase: {}, Component: {}, Promise: {}, Stack: {})",
                                                    boundary_id,
                                                    error_message,
                                                    error_name,
                                                    error_context["phase"]
                                                        .as_str()
                                                        .unwrap_or("unknown"),
                                                    error_context["componentPath"]
                                                        .as_str()
                                                        .unwrap_or("unknown"),
                                                    error_context["promiseId"]
                                                        .as_str()
                                                        .unwrap_or("unknown"),
                                                    error_stack
                                                );
                                                let row_id = get_or_allocate_row_id(
                                                    promise_id,
                                                    &promise_to_row_map,
                                                    &shared_row_counter,
                                                )
                                                .await;
                                                if let Err(e) = error_sender.send(BoundaryError {
                                                    boundary_id: boundary_id.clone(),
                                                    error_message: error_message.to_string(),
                                                    row_id,
                                                }) {
                                                    error!(
                                                        "Failed to send boundary error for {}: {}",
                                                        boundary_id, e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to parse promise resolution result for {}: {} - Raw: {}",
                                                boundary_id, e, result_string
                                            );
                                            let row_id = get_or_allocate_row_id(
                                                promise_id,
                                                &promise_to_row_map,
                                                &shared_row_counter,
                                            )
                                            .await;
                                            if let Err(e) = error_sender.send(BoundaryError {
                                                boundary_id: boundary_id.clone(),
                                                error_message: format!(
                                                    "Failed to parse promise result: {e}"
                                                ),
                                                row_id,
                                            }) {
                                                error!(
                                                    "Failed to send boundary error for {}: {}",
                                                    boundary_id, e
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to execute promise resolution script for boundary {}: {}",
                                        boundary_id, e
                                    );
                                    let row_id = get_or_allocate_row_id(
                                        promise_id,
                                        &promise_to_row_map,
                                        &shared_row_counter,
                                    )
                                    .await;
                                    if let Err(e) = error_sender.send(BoundaryError {
                                        boundary_id: boundary_id.clone(),
                                        error_message: format!("Failed to execute promise: {e}"),
                                        row_id,
                                    }) {
                                        error!(
                                            "Failed to send boundary error for {}: {}",
                                            boundary_id, e
                                        );
                                    }
                                }
                            }
                        }
                        None => break,
                    }
                }

                if received < n {
                    for (idx, promise) in batch.iter().enumerate() {
                        if !received_indices.contains(&idx) {
                            let row_id = get_or_allocate_row_id(
                                &promise.id,
                                &promise_to_row_map,
                                &shared_row_counter,
                            )
                            .await;
                            if let Err(e) = error_sender.send(BoundaryError {
                                boundary_id: promise.boundary_id.clone(),
                                error_message: "Promise channel closed before result arrived"
                                    .to_string(),
                                row_id,
                            }) {
                                error!(
                                    "Failed to send boundary error for {}: {}",
                                    promise.boundary_id, e
                                );
                            }
                        }
                    }
                }

                {
                    let mut active = active_promises.lock().await;
                    for p in &batch {
                        active.remove(&p.id);
                    }
                }
            }
        });
    }

    pub async fn active_count(&self) -> usize {
        self.active_promises.lock().await.len()
    }
}
