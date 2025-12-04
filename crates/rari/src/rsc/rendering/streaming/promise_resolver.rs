use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::error;

use crate::runtime::JsExecutionRuntime;

use super::constants::PROMISE_RESOLUTION_SCRIPT;
use super::types::{BoundaryError, BoundaryUpdate, PendingSuspensePromise};

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
                .replace("{promise_id}", &promise_id)
                .replace("{boundary_id}", &boundary_id)
                .replace("{component_path}", &promise.component_path);

            let script_name = format!("<promise_resolution_{promise_id}>");

            match runtime.execute_script(script_name.clone(), resolution_script).await {
                Ok(result) => {
                    let result_string = result.to_string();

                    match serde_json::from_str::<serde_json::Value>(&result_string) {
                        Ok(result_data) => {
                            if result_data["success"].as_bool().unwrap_or(false) {
                                let row_id = {
                                    let mut counter = shared_row_counter.lock().await;
                                    *counter += 1;
                                    *counter
                                };

                                let update = BoundaryUpdate {
                                    boundary_id: boundary_id.clone(),
                                    content: result_data["content"].clone(),
                                    row_id,
                                    dom_path: Vec::new(),
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
