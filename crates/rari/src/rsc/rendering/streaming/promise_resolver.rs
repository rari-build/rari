use cow_utils::CowUtils;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::error;

use crate::runtime::JsExecutionRuntime;

use super::constants::PROMISE_RESOLUTION_SCRIPT;
use super::types::{BoundaryError, BoundaryUpdate, PendingSuspensePromise, RscWireFormatTag};

fn process_client_components(
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
                && (type_str.contains('/') || type_str.contains('#'))
                && !type_str.starts_with("$L")
                && !component_map.contains_key(type_str)
            {
                *row_counter += 1;
                let module_ref = format!("$L{}", row_counter);

                let (file_path, export_name) = if let Some(idx) = type_str.find('#') {
                    (&type_str[..idx], &type_str[idx + 1..])
                } else {
                    (type_str, "default")
                };

                #[allow(clippy::disallowed_methods)]
                let import_data = serde_json::json!([file_path, ["default"], export_name]);
                let import_row = RscWireFormatTag::ModuleImport
                    .format_row(*row_counter, &import_data.to_string());
                import_rows.push(import_row.trim_end().to_string());

                component_map.insert(type_str.to_string(), module_ref);
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
                && let Some(module_ref) = component_map.get(type_str)
            {
                arr[1] = serde_json::Value::String(module_ref.clone());
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

pub struct BackgroundPromiseResolver {
    runtime: Arc<JsExecutionRuntime>,
    active_promises: Arc<Mutex<FxHashMap<String, PendingSuspensePromise>>>,
    update_sender: mpsc::UnboundedSender<BoundaryUpdate>,
    error_sender: mpsc::UnboundedSender<BoundaryError>,
    shared_row_counter: Arc<Mutex<u32>>,
}

impl BackgroundPromiseResolver {
    pub fn new(
        runtime: Arc<JsExecutionRuntime>,
        update_sender: mpsc::UnboundedSender<BoundaryUpdate>,
        error_sender: mpsc::UnboundedSender<BoundaryError>,
        shared_row_counter: Arc<Mutex<u32>>,
    ) -> Self {
        Self {
            runtime,
            active_promises: Arc::new(Mutex::new(FxHashMap::default())),
            update_sender,
            error_sender,
            shared_row_counter,
        }
    }

    pub async fn resolve_async(&self, promise: PendingSuspensePromise) {
        let promise_id = promise.id.clone();
        let boundary_id = promise.boundary_id.clone();

        {
            let mut active = self.active_promises.lock().await;
            active.insert(promise_id.clone(), promise.clone());
        }

        let runtime = Arc::clone(&self.runtime);
        let update_sender = self.update_sender.clone();
        let error_sender = self.error_sender.clone();
        let shared_row_counter = Arc::clone(&self.shared_row_counter);
        let active_promises = Arc::clone(&self.active_promises);

        tokio::spawn(async move {
            let resolution_script = PROMISE_RESOLUTION_SCRIPT
                .cow_replace("{promise_id}", &promise_id)
                .cow_replace("{boundary_id}", &boundary_id)
                .cow_replace("{component_path}", &promise.component_path)
                .into_owned();

            let script_name = format!("<promise_resolution_{promise_id}>");

            match runtime.execute_script(script_name.clone(), resolution_script).await {
                Ok(result) => {
                    let result_string = result.to_string();

                    match serde_json::from_str::<serde_json::Value>(&result_string) {
                        Ok(result_data) => {
                            if result_data["success"].as_bool().unwrap_or(false) {
                                let mut content = result_data["content"].clone();

                                let (row_id, import_rows) = {
                                    let mut counter = shared_row_counter.lock().await;
                                    let import_rows =
                                        process_client_components(&mut content, &mut counter);
                                    *counter += 1;
                                    (*counter, import_rows)
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
                                let error_message =
                                    result_data["error"].as_str().unwrap_or("Unknown error");
                                let error_name =
                                    result_data["errorName"].as_str().unwrap_or("UnknownError");
                                let error_stack =
                                    result_data["errorStack"].as_str().unwrap_or("No stack trace");
                                let error_context = &result_data["errorContext"];

                                error!(
                                    "Promise resolution failed for boundary {}: {} (Name: {}, Phase: {}, Component: {}, Promise: {}, Stack: {})",
                                    boundary_id,
                                    error_message,
                                    error_name,
                                    error_context["phase"].as_str().unwrap_or("unknown"),
                                    error_context["componentPath"].as_str().unwrap_or("unknown"),
                                    error_context["promiseId"].as_str().unwrap_or("unknown"),
                                    error_stack
                                );

                                let row_id = {
                                    let mut counter = shared_row_counter.lock().await;
                                    *counter += 1;
                                    *counter
                                };

                                let error_update = BoundaryError {
                                    boundary_id: boundary_id.clone(),
                                    error_message: error_message.to_string(),
                                    row_id,
                                };

                                if let Err(e) = error_sender.send(error_update) {
                                    error!(
                                        "Failed to send boundary error for {}: {}",
                                        boundary_id, e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to parse promise resolution result for {}: {} - Raw result: {} - Script: {}",
                                boundary_id, e, result_string, script_name
                            );

                            let row_id = {
                                let mut counter = shared_row_counter.lock().await;
                                *counter += 1;
                                *counter
                            };

                            let error_update = BoundaryError {
                                boundary_id: boundary_id.clone(),
                                error_message: format!("Failed to parse promise result: {}", e),
                                row_id,
                            };

                            let _ = error_sender.send(error_update);
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to execute promise resolution script {} for boundary {}: {}",
                        script_name, boundary_id, e
                    );

                    let row_id = {
                        let mut counter = shared_row_counter.lock().await;
                        *counter += 1;
                        *counter
                    };

                    let error_update = BoundaryError {
                        boundary_id: boundary_id.clone(),
                        error_message: format!("Failed to execute promise: {}", e),
                        row_id,
                    };

                    let _ = error_sender.send(error_update);
                }
            }

            {
                let mut active = active_promises.lock().await;
                active.remove(&promise_id);
            }
        });
    }

    pub async fn active_count(&self) -> usize {
        self.active_promises.lock().await.len()
    }
}
