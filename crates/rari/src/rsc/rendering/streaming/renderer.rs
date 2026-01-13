use cow_utils::CowUtils;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::error;

use crate::error::RariError;
use crate::runtime::JsExecutionRuntime;

use super::constants::*;
use super::promise_resolver::BackgroundPromiseResolver;
use super::stream::RscStream;
use super::types::*;
use super::validation::validate_suspense_boundaries;

pub struct StreamingRenderer {
    runtime: Arc<JsExecutionRuntime>,
    promise_resolver: Option<Arc<BackgroundPromiseResolver>>,
    row_counter: u32,
    module_path: Option<String>,
    shared_row_counter: Arc<Mutex<u32>>,
    boundary_row_ids: Arc<Mutex<FxHashMap<String, u32>>>,
    rendered_skeleton_ids: Arc<Mutex<FxHashSet<String>>>,
    resolved_boundary_ids: Arc<Mutex<FxHashSet<String>>>,
}

impl StreamingRenderer {
    pub fn new(runtime: Arc<JsExecutionRuntime>) -> Self {
        Self {
            runtime,
            promise_resolver: None,
            row_counter: 0,
            module_path: None,
            shared_row_counter: Arc::new(Mutex::new(0)),
            boundary_row_ids: Arc::new(Mutex::new(FxHashMap::default())),
            rendered_skeleton_ids: Arc::new(Mutex::new(FxHashSet::default())),
            resolved_boundary_ids: Arc::new(Mutex::new(FxHashSet::default())),
        }
    }

    pub async fn start_streaming_with_composition(
        &mut self,
        composition_script: String,
        layout_structure: crate::rsc::rendering::layout::LayoutStructure,
    ) -> Result<RscStream, RariError> {
        if !layout_structure.is_valid() {
            error!(
                "StreamingRenderer: Invalid layout structure detected, streaming should not have been initiated"
            );

            error!(
                "Layout structure details: has_navigation={}, navigation_position={:?}, content_position={:?}, suspense_boundaries={}",
                layout_structure.has_navigation,
                layout_structure.navigation_position,
                layout_structure.content_position,
                layout_structure.suspense_boundaries.len()
            );

            for boundary in &layout_structure.suspense_boundaries {
                error!(
                    "  Suspense boundary '{}': parent_path={:?}, is_in_content_area={}",
                    boundary.boundary_id, boundary.parent_path, boundary.is_in_content_area
                );
            }

            return Err(RariError::internal(
                "Cannot start streaming with invalid layout structure. Navigation must precede content, and Suspense boundaries must be in content area.",
            ));
        }

        let boundary_positions: Arc<Mutex<FxHashMap<String, Vec<usize>>>> = Arc::new(Mutex::new(
            layout_structure
                .suspense_boundaries
                .iter()
                .map(|b| (b.boundary_id.clone(), b.dom_path.clone()))
                .collect(),
        ));

        let (update_sender, update_receiver) = mpsc::unbounded_channel::<BoundaryUpdate>();
        let (error_sender, error_receiver) = mpsc::unbounded_channel::<BoundaryError>();
        let (chunk_sender, chunk_receiver) = mpsc::channel::<RscStreamChunk>(64);

        self.promise_resolver = Some(Arc::new(BackgroundPromiseResolver::new(
            Arc::clone(&self.runtime),
            update_sender,
            error_sender,
            Arc::clone(&self.shared_row_counter),
        )));

        let partial_result = self.render_partial_from_composition(composition_script).await?;

        self.send_initial_shell(&chunk_sender, &partial_result).await?;

        {
            let mut shared_counter = self.shared_row_counter.lock().await;
            *shared_counter = self.row_counter;
        }

        let stream_complete_chunk = RscStreamChunk {
            data: b"STREAM_COMPLETE\n".to_vec(),
            chunk_type: RscChunkType::StreamComplete,
            row_id: u32::MAX,
            is_final: false,
            boundary_id: None,
        };
        chunk_sender.send(stream_complete_chunk).await.map_err(|e| {
            RariError::internal(format!("Failed to send initial stream complete: {}", e))
        })?;

        if let Some(resolver) = &self.promise_resolver {
            let runtime = Arc::clone(&self.runtime);
            let resolver_clone = Arc::clone(resolver);
            let pending_promises = partial_result.pending_promises.clone();

            tokio::spawn(async move {
                let execute_script = DEFERRED_EXECUTION_SCRIPT;

                match runtime
                    .execute_script(
                        "<execute_deferred_components>".to_string(),
                        execute_script.to_string(),
                    )
                    .await
                {
                    Ok(result) => {
                        let result_str = result.to_string();
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&result_str)
                            && let Some(results) = data["results"].as_array()
                        {
                            for result in results {
                                if !result["success"].as_bool().unwrap_or(false) {
                                    let error_msg = result["error"].as_str().unwrap_or("unknown");
                                    let error_name =
                                        result["errorName"].as_str().unwrap_or("UnknownError");
                                    let component_path =
                                        result["componentPath"].as_str().unwrap_or("unknown");
                                    let promise_id =
                                        result["promiseId"].as_str().unwrap_or("unknown");

                                    error!(
                                        "Deferred component failed: promiseId={}, component={}, error={} ({})",
                                        promise_id, component_path, error_msg, error_name
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute deferred components: {}", e);
                    }
                }

                for promise in pending_promises {
                    resolver_clone.resolve_async(promise).await;
                }
            });
        } else {
            return Err(RariError::internal(
                "No promise resolver available - this should not happen",
            ));
        }

        let chunk_sender_clone = chunk_sender.clone();
        let boundary_rows_map = Arc::clone(&self.boundary_row_ids);
        let boundary_positions_clone = Arc::clone(&boundary_positions);
        let rendered_skeleton_ids = Arc::clone(&self.rendered_skeleton_ids);
        let resolved_boundary_ids = Arc::clone(&self.resolved_boundary_ids);

        tokio::spawn(async move {
            let mut update_receiver = update_receiver;
            let mut error_receiver = error_receiver;

            loop {
                tokio::select! {
                    Some(mut update) = update_receiver.recv() => {
                        let is_duplicate_resolution = {
                            let mut skeleton_ids = rendered_skeleton_ids.lock().await;
                            let mut resolved_ids = resolved_boundary_ids.lock().await;

                            skeleton_ids.remove(&update.boundary_id);
                            let is_first_resolution = resolved_ids.insert(update.boundary_id.clone());

                            !is_first_resolution
                        };

                        if is_duplicate_resolution {
                            continue;
                        }

                        if let Some(dom_path) = boundary_positions_clone.lock().await.get(&update.boundary_id) {
                            update.dom_path = dom_path.clone();
                        } else {
                            error!(
                                "DOM path not found for boundary '{}' in boundary_positions map. This may cause incorrect skeleton replacement.",
                                update.boundary_id
                            );
                        }

                        if update.dom_path.is_empty() {
                            error!(
                                "DOM path is empty for boundary '{}'. Skeleton replacement may fail without proper targeting.",
                                update.boundary_id
                            );
                        }

                        Self::send_boundary_update_with_map(
                            &chunk_sender_clone,
                            update,
                            Arc::clone(&boundary_rows_map),
                        )
                        .await;
                    }
                    Some(error) = error_receiver.recv() => {
                        Self::send_boundary_error(
                            &chunk_sender_clone,
                            error,
                        )
                        .await;
                    }
                    else => break,
                }
            }

            let _ = rendered_skeleton_ids.lock().await;

            let final_chunk = RscStreamChunk {
                data: b"STREAM_COMPLETE\n".to_vec(),
                chunk_type: RscChunkType::StreamComplete,
                row_id: u32::MAX,
                is_final: true,
                boundary_id: None,
            };

            if let Err(e) = chunk_sender_clone.send(final_chunk).await {
                error!("Failed to send stream completion signal: {}", e);
            }
        });

        Ok(RscStream::new(chunk_receiver))
    }

    pub async fn start_streaming_with_precomputed_data(
        &mut self,
        rsc_data: serde_json::Value,
        boundaries: Vec<crate::rsc::rendering::layout::BoundaryInfo>,
        layout_structure: crate::rsc::rendering::layout::LayoutStructure,
        pending_promises: Vec<PendingSuspensePromise>,
    ) -> Result<RscStream, RariError> {
        if !layout_structure.is_valid() {
            error!(
                "StreamingRenderer: Invalid layout structure detected, streaming should not have been initiated"
            );
            return Err(RariError::internal(
                "Cannot start streaming with invalid layout structure",
            ));
        }

        let boundary_positions: Arc<Mutex<FxHashMap<String, Vec<usize>>>> = Arc::new(Mutex::new(
            layout_structure
                .suspense_boundaries
                .iter()
                .map(|b| (b.boundary_id.clone(), b.dom_path.clone()))
                .collect(),
        ));

        let (update_sender, update_receiver) = mpsc::unbounded_channel::<BoundaryUpdate>();
        let (error_sender, error_receiver) = mpsc::unbounded_channel::<BoundaryError>();
        let (chunk_sender, chunk_receiver) = mpsc::channel::<RscStreamChunk>(64);

        self.promise_resolver = Some(Arc::new(BackgroundPromiseResolver::new(
            Arc::clone(&self.runtime),
            update_sender,
            error_sender,
            Arc::clone(&self.shared_row_counter),
        )));

        let suspense_boundaries =
            Self::extract_suspense_boundaries(&rsc_data, &boundaries, &layout_structure);

        let partial_result = PartialRenderResult {
            initial_content: rsc_data,
            pending_promises,
            boundaries: suspense_boundaries,
            has_suspense: !boundaries.is_empty(),
        };

        self.send_initial_shell(&chunk_sender, &partial_result).await?;

        {
            let mut shared_counter = self.shared_row_counter.lock().await;
            *shared_counter = self.row_counter;
        }

        let chunk_sender_clone = chunk_sender.clone();

        drop(chunk_sender);

        if let Some(resolver) = &self.promise_resolver {
            let resolver_clone = Arc::clone(resolver);
            let pending_promises = partial_result.pending_promises.clone();

            tokio::spawn(async move {
                for promise in pending_promises {
                    resolver_clone.resolve_async(promise).await;
                }
            });
        }

        let boundary_rows_map = Arc::clone(&self.boundary_row_ids);
        let boundary_positions_clone = Arc::clone(&boundary_positions);
        let rendered_skeleton_ids = Arc::clone(&self.rendered_skeleton_ids);
        let resolved_boundary_ids = Arc::clone(&self.resolved_boundary_ids);

        tokio::spawn(async move {
            let mut update_receiver = update_receiver;
            let mut error_receiver = error_receiver;

            loop {
                tokio::select! {
                    Some(mut update) = update_receiver.recv() => {
                        let is_duplicate_resolution = {
                            let mut skeleton_ids = rendered_skeleton_ids.lock().await;
                            let mut resolved_ids = resolved_boundary_ids.lock().await;
                            skeleton_ids.remove(&update.boundary_id);
                            let is_first_resolution = resolved_ids.insert(update.boundary_id.clone());
                            !is_first_resolution
                        };

                        if is_duplicate_resolution {
                            continue;
                        }

                        if let Some(dom_path) = boundary_positions_clone.lock().await.get(&update.boundary_id) {
                            update.dom_path = dom_path.clone();
                        }

                        Self::send_boundary_update_with_map(
                            &chunk_sender_clone,
                            update,
                            Arc::clone(&boundary_rows_map),
                        )
                        .await;
                    }
                    Some(error) = error_receiver.recv() => {
                        Self::send_boundary_error(
                            &chunk_sender_clone,
                            error,
                        )
                        .await;
                    }
                    else => break,
                }
            }

            let final_chunk = RscStreamChunk {
                data: b"STREAM_COMPLETE\n".to_vec(),
                chunk_type: RscChunkType::StreamComplete,
                row_id: u32::MAX,
                is_final: true,
                boundary_id: None,
            };

            if let Err(e) = chunk_sender_clone.send(final_chunk).await {
                error!("Failed to send stream completion signal: {}", e);
            }
        });

        Ok(RscStream::new(chunk_receiver))
    }

    pub async fn start_streaming_from_rsc(
        &mut self,
        rsc_wire_format: String,
    ) -> Result<RscStream, RariError> {
        let (update_sender, update_receiver) = mpsc::unbounded_channel::<BoundaryUpdate>();
        let (error_sender, error_receiver) = mpsc::unbounded_channel::<BoundaryError>();
        let (chunk_sender, chunk_receiver) = mpsc::channel::<RscStreamChunk>(64);

        self.promise_resolver = Some(Arc::new(BackgroundPromiseResolver::new(
            Arc::clone(&self.runtime),
            update_sender,
            error_sender,
            Arc::clone(&self.shared_row_counter),
        )));

        let partial_result = self.parse_rsc_wire_format(&rsc_wire_format).await?;

        self.send_initial_shell(&chunk_sender, &partial_result).await?;

        if let Some(resolver) = &self.promise_resolver {
            let runtime = Arc::clone(&self.runtime);
            let resolver_clone = Arc::clone(resolver);
            let pending_promises = partial_result.pending_promises.clone();
            let has_promises = !pending_promises.is_empty();

            tokio::spawn(async move {
                if let Err(e) = runtime
                    .execute_script(
                        "<init_promise_tracking>".to_string(),
                        PROMISE_TRACKING_INIT_SCRIPT.to_string(),
                    )
                    .await
                {
                    error!("Failed to initialize promise tracking: {}", e);
                }

                match runtime
                    .execute_script(
                        "<execute_deferred_components>".to_string(),
                        DEFERRED_EXECUTION_SCRIPT.to_string(),
                    )
                    .await
                {
                    Ok(result) => {
                        let result_str = result.to_string();
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&result_str)
                            && let Some(results) = data["results"].as_array()
                        {
                            for result in results {
                                if !result["success"].as_bool().unwrap_or(false) {
                                    error!(
                                        "Deferred component failed: promiseId={}, error={}",
                                        result["promiseId"].as_str().unwrap_or("unknown"),
                                        result["error"].as_str().unwrap_or("unknown")
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute deferred components: {}", e);
                    }
                }

                if has_promises {
                    for promise in pending_promises {
                        resolver_clone.resolve_async(promise).await;
                    }
                }
            });
        } else {
            return Err(RariError::internal(
                "No promise resolver available - this should not happen",
            ));
        }

        let chunk_sender_clone = chunk_sender.clone();
        let boundary_rows_map = Arc::clone(&self.boundary_row_ids);

        tokio::spawn(async move {
            let mut update_receiver = update_receiver;
            let mut error_receiver = error_receiver;

            loop {
                tokio::select! {
                             Some(update) = update_receiver.recv() => {

                                 {
                                     let mut map = boundary_rows_map.lock().await;
                                     map.insert(update.boundary_id.clone(), update.row_id);
                                 }

                                 let update_str = format!(
                                     "{}:{}\n",
                                     update.row_id,
                                     serde_json::to_string(&update.content).unwrap_or_else(|_| "null".to_string())
                                 );

                                 let chunk = RscStreamChunk {
                                     data: update_str.into_bytes(),
                                     chunk_type: RscChunkType::BoundaryUpdate,
                                     row_id: update.row_id,
                                     is_final: false,
                                     boundary_id: Some(update.boundary_id.clone()),
                                 };

                                 if chunk_sender_clone.send(chunk).await.is_err() {
                                     break;
                                 }

                             }
                             Some(error) = error_receiver.recv() => {

                                 error!(
                                     "Streaming boundary error: boundary_id={}, error={}, row_id={}",
                                     error.boundary_id,
                                     error.error_message,
                                     error.row_id
                                 );

                                 #[allow(clippy::disallowed_methods)]
                                 let error_json = serde_json::to_string(&serde_json::json!({
                                     "message": error.error_message,
                                     "boundaryId": error.boundary_id
                                 })).unwrap_or_else(|_| "{}".to_string());

                                 let error_str = format!(
                                     "{}:E{}\n",
                                     error.row_id,
                error_json
                                 );

                                 let chunk = RscStreamChunk {
                                     data: error_str.into_bytes(),
                                     chunk_type: RscChunkType::BoundaryError,
                                     row_id: error.row_id,
                                     is_final: false,
                boundary_id: None,
                                 };

                                 if chunk_sender_clone.send(chunk).await.is_err() {
                                     break;
                                 }
                             }
                             else => {
                                 break;
                             }
                         }
            }

            let final_chunk = RscStreamChunk {
                data: Vec::new(),
                chunk_type: RscChunkType::StreamComplete,
                row_id: 0,
                is_final: true,
                boundary_id: None,
            };

            let _ = chunk_sender_clone.send(final_chunk).await;
        });

        Ok(RscStream::new(chunk_receiver))
    }

    pub async fn start_streaming(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscStream, RariError> {
        let (update_sender, update_receiver) = mpsc::unbounded_channel::<BoundaryUpdate>();
        let (error_sender, error_receiver) = mpsc::unbounded_channel::<BoundaryError>();
        let (chunk_sender, chunk_receiver) = mpsc::channel::<RscStreamChunk>(64);

        self.promise_resolver = Some(Arc::new(BackgroundPromiseResolver::new(
            Arc::clone(&self.runtime),
            update_sender,
            error_sender,
            Arc::clone(&self.shared_row_counter),
        )));

        self.module_path = Some(format!("{component_id}.js"));

        let partial_result = self.render_partial(component_id, props).await?;

        self.send_initial_shell(&chunk_sender, &partial_result).await?;

        if let Some(resolver) = &self.promise_resolver {
            let runtime = Arc::clone(&self.runtime);
            let resolver_clone = Arc::clone(resolver);
            let pending_promises = partial_result.pending_promises.clone();

            tokio::spawn(async move {
                match runtime
                    .execute_script(
                        "<execute_deferred_components>".to_string(),
                        DEFERRED_EXECUTION_SCRIPT.to_string(),
                    )
                    .await
                {
                    Ok(result) => {
                        let result_str = result.to_string();
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&result_str)
                            && let Some(results) = data["results"].as_array()
                        {
                            for result in results {
                                if !result["success"].as_bool().unwrap_or(false) {
                                    error!(
                                        "Deferred component failed: promiseId={}, error={}",
                                        result["promiseId"].as_str().unwrap_or("unknown"),
                                        result["error"].as_str().unwrap_or("unknown")
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute deferred components: {}", e);
                    }
                }

                for promise in pending_promises {
                    resolver_clone.resolve_async(promise).await;
                }
            });
        } else {
            return Err(RariError::internal(
                "No promise resolver available - this should not happen",
            ));
        }

        let chunk_sender_clone = chunk_sender.clone();
        let boundary_rows_map = Arc::clone(&self.boundary_row_ids);
        tokio::spawn(async move {
            let mut update_receiver = update_receiver;
            let mut error_receiver = error_receiver;

            loop {
                tokio::select! {
                    Some(update) = update_receiver.recv() => {
                        Self::send_boundary_update_with_map(
                            &chunk_sender_clone,
                            update,
                            Arc::clone(&boundary_rows_map),
                        )
                        .await;
                    }
                    Some(error) = error_receiver.recv() => {
                        Self::send_boundary_error(
                            &chunk_sender_clone,
                            error,
                        )
                        .await;
                    }
                    else => break,
                }
            }

            let final_chunk = RscStreamChunk {
                data: b"STREAM_COMPLETE\n".to_vec(),
                chunk_type: RscChunkType::StreamComplete,
                row_id: u32::MAX,
                is_final: true,
                boundary_id: None,
            };

            if let Err(e) = chunk_sender_clone.send(final_chunk).await {
                error!("Failed to send stream completion signal: {}", e);
            }
        });

        Ok(RscStream::new(chunk_receiver))
    }

    async fn render_partial(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<PartialRenderResult, RariError> {
        let react_init_result = self
            .runtime
            .execute_script("streaming-react-init".to_string(), REACT_INIT_SCRIPT.to_string())
            .await?;

        if let Some(available) = react_init_result.get("available").and_then(|v| v.as_bool()) {
            if !available {
                return Err(RariError::internal("Failed to initialize React in streaming context"));
            }
        } else {
            return Err(RariError::internal("Failed to check React initialization"));
        }

        self.runtime
            .execute_script("<streaming_init>".to_string(), STREAMING_INIT_SCRIPT.to_string())
            .await
            .map_err(|e| RariError::internal(format!("Streaming init failed: {e}")))?;

        let setup_script = COMPONENT_RENDER_SETUP_SCRIPT
            .cow_replace("{component_id}", component_id)
            .cow_replace("{props_json}", props.unwrap_or("{}"))
            .into_owned();

        self.runtime
            .execute_script(format!("<setup_render_{component_id}>"), setup_script)
            .await
            .map_err(|e| RariError::internal(format!("Setup render failed: {e}")))?;
        self.runtime
            .execute_script(
                format!("<start_render_{component_id}>"),
                RENDER_INIT_SCRIPT.to_string(),
            )
            .await
            .map_err(|e| RariError::internal(format!("Partial render failed: {e}")))?;

        let result_data = {
            let (completion_tx, completion_rx) = tokio::sync::oneshot::channel::<()>();

            let completion_tx = Arc::new(tokio::sync::Mutex::new(Some(completion_tx)));
            let completion_tx_clone = Arc::clone(&completion_tx);

            self.runtime
                .execute_script(
                    format!("<start_render_{component_id}>"),
                    RENDER_START_SCRIPT.to_string(),
                )
                .await
                .map_err(|e| RariError::internal(format!("Failed to start render: {e}")))?;

            let runtime_clone = Arc::clone(&self.runtime);
            let component_id_clone = component_id.to_string();
            tokio::spawn(async move {
                loop {
                    match runtime_clone
                        .execute_script(
                            format!("<check_complete_{}>", component_id_clone),
                            RENDER_CHECK_COMPLETE_SCRIPT.to_string(),
                        )
                        .await
                    {
                        Ok(check_result) => {
                            let check_str = check_result.to_string();
                            if let Ok(check_data) =
                                serde_json::from_str::<serde_json::Value>(&check_str)
                            {
                                if let Some(json_str) = check_data.as_str() {
                                    if let Ok(parsed) =
                                        serde_json::from_str::<serde_json::Value>(json_str)
                                        && (parsed
                                            .get("complete")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false)
                                            || parsed
                                                .get("initialComplete")
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(false))
                                    {
                                        let mut tx = completion_tx_clone.lock().await;
                                        if let Some(sender) = tx.take() {
                                            let _ = sender.send(());
                                        }
                                        break;
                                    }
                                } else if check_data
                                    .get("complete")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                                    || check_data
                                        .get("initialComplete")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false)
                                {
                                    let mut tx = completion_tx_clone.lock().await;
                                    if let Some(sender) = tx.take() {
                                        let _ = sender.send(());
                                    }
                                    break;
                                }
                            }
                        }
                        Err(_) => break,
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                }
            });

            match tokio::time::timeout(tokio::time::Duration::from_secs(3), completion_rx).await {
                Ok(Ok(())) => {
                    let result = self
                        .runtime
                        .execute_script(
                            format!("<fetch_result_{component_id}>"),
                            RENDER_FETCH_RESULT_SCRIPT.to_string(),
                        )
                        .await
                        .map_err(|e| RariError::internal(format!("Failed to fetch result: {e}")))?;

                    let result_str = result.to_string();
                    let result_value: serde_json::Value = serde_json::from_str(&result_str)
                        .map_err(|e| RariError::internal(format!("Failed to parse result: {e}")))?;

                    if let Some(json_str) = result_value.as_str() {
                        serde_json::from_str(json_str).map_err(|e| {
                            RariError::internal(format!("Failed to parse result JSON: {e}"))
                        })?
                    } else {
                        result_value
                    }
                }
                Ok(Err(_)) => {
                    return Err(RariError::internal(
                        "Render completion channel closed unexpectedly".to_string(),
                    ));
                }
                Err(_) => {
                    return Err(RariError::internal(
                        "Initial render timed out after 3 seconds".to_string(),
                    ));
                }
            }
        };

        if !result_data["success"].as_bool().unwrap_or(false) {
            return Err(RariError::internal(format!(
                "Component render failed: {}",
                result_data["error"].as_str().unwrap_or("Unknown error")
            )));
        }

        let mut pending_counts: FxHashMap<String, usize> = FxHashMap::default();
        if let Some(pending) = result_data["pending_promises"].as_array() {
            for p in pending {
                if let Some(bid) = p["~boundaryId"].as_str() {
                    *pending_counts.entry(bid.to_string()).or_insert(0) += 1;
                }
            }
        }

        let boundaries = result_data["boundaries"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|b| {
                let id = b["id"].as_str().unwrap_or("unknown").to_string();
                let count = pending_counts.get(&id).cloned().unwrap_or(0);
                if count == 0 {
                    return None;
                }

                let parent_path = b["parentPath"]
                    .as_array()
                    .map(|arr| {
                        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                    })
                    .unwrap_or_else(Vec::new);

                let is_in_content_area = b["isInContentArea"].as_bool().unwrap_or(false);

                let position_hints =
                    b.get("positionHints").and_then(|h| h.as_object()).map(|hints| PositionHints {
                        in_content_area: hints
                            .get("inContentArea")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        dom_path: hints
                            .get("domPath")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_else(Vec::new),
                        is_stable: hints.get("isStable").and_then(|v| v.as_bool()).unwrap_or(false),
                    });

                Some(SuspenseBoundaryInfo {
                    id,
                    fallback_content: b["fallback"].clone(),
                    parent_boundary_id: b["parentId"].as_str().map(|s| s.to_string()),
                    pending_promise_count: count,
                    parent_path,
                    is_in_content_area,
                    skeleton_rendered: false,
                    is_resolved: false,
                    position_hints,
                })
            })
            .collect();

        let pending_promises = result_data["pending_promises"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|p| PendingSuspensePromise {
                id: p["id"].as_str().unwrap_or("unknown").to_string(),
                boundary_id: p["~boundaryId"].as_str().unwrap_or("root").to_string(),
                component_path: p["componentPath"].as_str().unwrap_or(component_id).to_string(),
                promise_handle: p["id"].as_str().unwrap_or("unknown").to_string(),
            })
            .collect();

        Ok(PartialRenderResult {
            initial_content: result_data["rsc_data"].clone(),
            pending_promises,
            boundaries,
            has_suspense: result_data["has_suspense"].as_bool().unwrap_or(false),
        })
    }

    async fn render_partial_from_composition(
        &mut self,
        composition_script: String,
    ) -> Result<PartialRenderResult, RariError> {
        let react_init_result = self
            .runtime
            .execute_script("streaming-react-init".to_string(), REACT_INIT_SCRIPT.to_string())
            .await
            .map_err(|e| {
                error!("Failed to execute React initialization script: {}", e);
                RariError::internal(format!(
                    "Failed to initialize React for streaming context: {}",
                    e
                ))
            })?;

        if let Some(available) = react_init_result.get("available").and_then(|v| v.as_bool()) {
            if !available {
                error!("React initialization reported as unavailable");
                error!("React init result: {:?}", react_init_result);
                return Err(RariError::internal(
                    "Failed to initialize React in streaming context - React not available after initialization",
                ));
            }
        } else {
            error!("Failed to check React initialization status");
            error!("React init result: {:?}", react_init_result);
            return Err(RariError::internal(
                "Failed to check React initialization - unexpected result format",
            ));
        }

        self.runtime
            .execute_script("<streaming_init>".to_string(), STREAMING_INIT_SCRIPT.to_string())
            .await
            .map_err(|e| {
                error!("Streaming initialization script failed: {}", e);
                RariError::internal(format!(
                    "Failed to initialize streaming globals and helpers: {}",
                    e
                ))
            })?;

        let wrapped_script = COMPOSITION_WRAPPER_SCRIPT
            .cow_replace("{composition_script}", &composition_script)
            .into_owned();

        let result = self
            .runtime
            .execute_script("<composition_script>".to_string(), wrapped_script)
            .await
            .map_err(|e| {
                error!("Failed to execute composition script: {}", e);

                RariError::internal(format!(
                    "Failed to execute composition script (length: {} bytes): {}",
                    composition_script.len(),
                    e
                ))
            })?;

        let result_string = result.to_string();

        let result_data: serde_json::Value = serde_json::from_str(&result_string).map_err(|e| {
            error!("Failed to parse composition result: {}", e);
            RariError::internal(format!("Failed to parse composition result: {}", e))
        })?;

        if !result_data["success"].as_bool().unwrap_or(false) {
            let error_msg = result_data["error"].as_str().unwrap_or("Unknown error");
            let error_stack = result_data["error_stack"].as_str().unwrap_or("No stack available");

            error!("Composition script execution failed: {}", error_msg);
            error!("Error stack trace: {}", error_stack);

            return Err(RariError::internal(format!(
                "Composition script execution failed: {} (Stack: {})",
                error_msg, error_stack
            )));
        }

        let mut pending_counts: FxHashMap<String, usize> = FxHashMap::default();
        if let Some(pending) = result_data["pending_promises"].as_array() {
            for p in pending {
                if let Some(bid) = p["~boundaryId"].as_str() {
                    *pending_counts.entry(bid.to_string()).or_insert(0) += 1;
                }
            }
        }

        let boundaries: Vec<SuspenseBoundaryInfo> = result_data["boundaries"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|b| {
                let id = b["id"].as_str().unwrap_or("unknown").to_string();
                let count = pending_counts.get(&id).cloned().unwrap_or(0);
                if count == 0 {
                    return None;
                }

                let parent_path = b["parentPath"]
                    .as_array()
                    .map(|arr| {
                        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                    })
                    .unwrap_or_else(Vec::new);

                let is_in_content_area = b["isInContentArea"].as_bool().unwrap_or(false);

                let position_hints =
                    b.get("positionHints").and_then(|h| h.as_object()).map(|hints| PositionHints {
                        in_content_area: hints
                            .get("inContentArea")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        dom_path: hints
                            .get("domPath")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_else(Vec::new),
                        is_stable: hints.get("isStable").and_then(|v| v.as_bool()).unwrap_or(false),
                    });

                Some(SuspenseBoundaryInfo {
                    id,
                    fallback_content: b["fallback"].clone(),
                    parent_boundary_id: b["parentId"].as_str().map(|s| s.to_string()),
                    pending_promise_count: count,
                    parent_path,
                    is_in_content_area,
                    skeleton_rendered: false,
                    is_resolved: false,
                    position_hints,
                })
            })
            .collect();

        let pending_promises: Vec<PendingSuspensePromise> = result_data["pending_promises"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|p| PendingSuspensePromise {
                id: p["id"].as_str().unwrap_or("unknown").to_string(),
                boundary_id: p["~boundaryId"].as_str().unwrap_or("root").to_string(),
                component_path: p["componentPath"].as_str().unwrap_or("unknown").to_string(),
                promise_handle: p["id"].as_str().unwrap_or("unknown").to_string(),
            })
            .collect();

        if let Err(validation_error) = validate_suspense_boundaries(&result_data["rsc_data"]) {
            error!(
                "RSC structure validation failed after composition script execution: {}",
                validation_error
            );
        }

        Ok(PartialRenderResult {
            initial_content: result_data["rsc_data"].clone(),
            pending_promises,
            boundaries,
            has_suspense: result_data["has_suspense"].as_bool().unwrap_or(false),
        })
    }
    async fn parse_rsc_wire_format(
        &mut self,
        rsc_wire_format: &str,
    ) -> Result<PartialRenderResult, RariError> {
        let mut parser = crate::rsc::wire_format::parser::RscWireFormatParser::new(rsc_wire_format);

        parser.parse().map_err(|e| {
            error!("Failed to parse RSC wire format: {}", e);
            RariError::internal(format!("RSC parsing failed: {}", e))
        })?;

        let boundaries = parser.find_suspense_boundaries();
        let promises = parser.find_promises();

        let (linked_boundaries, linked_promises) =
            parser.link_promises_to_boundaries(boundaries, promises);

        let mut pending_promises = Vec::new();
        let mut boundary_infos = Vec::new();

        for boundary in &linked_boundaries {
            #[allow(clippy::disallowed_methods)]
            let fallback_content = serde_json::json!({
                "type": "div",
                "props": {
                    "children": "Loading..."
                }
            });

            let boundary_info = SuspenseBoundaryInfo {
                id: boundary.boundary_id.clone(),
                fallback_content,
                parent_boundary_id: None,
                pending_promise_count: boundary.promise_ids.len(),
                parent_path: Vec::new(),
                is_in_content_area: true,
                skeleton_rendered: false,
                is_resolved: false,
                position_hints: None,
            };

            boundary_infos.push(boundary_info);
        }

        for promise in &linked_promises {
            if !promise.boundary_id.is_empty() {
                let pending_promise = PendingSuspensePromise {
                    id: promise.promise_id.clone(),
                    boundary_id: promise.boundary_id.clone(),
                    component_path: format!("async_component_{}", promise.promise_id),
                    promise_handle: promise.element_ref.clone(),
                };

                pending_promises.push(pending_promise);
            }
        }

        let mut initial_content = serde_json::Value::Null;

        for line in rsc_wire_format.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let row_id = &line[..colon_pos];
                let data = &line[colon_pos + 1..];

                if row_id.starts_with('M') || row_id.starts_with('S') || row_id.starts_with('I') {
                    continue;
                }

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if row_id == "0" {
                        initial_content = parsed;
                        break;
                    }
                    if initial_content.is_null() {
                        initial_content = parsed;
                    }
                }
            }
        }

        if initial_content.is_null() {
            #[allow(clippy::disallowed_methods)]
            {
                initial_content = serde_json::json!(rsc_wire_format);
            }
        }

        let has_suspense = !boundary_infos.is_empty();

        Ok(PartialRenderResult {
            initial_content,
            pending_promises,
            boundaries: boundary_infos,
            has_suspense,
        })
    }

    fn extract_suspense_boundaries(
        rsc_data: &serde_json::Value,
        boundaries: &[crate::rsc::rendering::layout::BoundaryInfo],
        layout_structure: &crate::rsc::rendering::layout::LayoutStructure,
    ) -> Vec<SuspenseBoundaryInfo> {
        let mut result = Vec::new();

        let boundary_ids: FxHashSet<String> = boundaries.iter().map(|b| b.id.clone()).collect();

        let boundary_positions: FxHashMap<
            String,
            &crate::rsc::rendering::layout::BoundaryPosition,
        > = layout_structure
            .suspense_boundaries
            .iter()
            .map(|bp| (bp.boundary_id.clone(), bp))
            .collect();

        fn traverse(
            value: &serde_json::Value,
            boundary_ids: &FxHashSet<String>,
            boundary_positions: &FxHashMap<
                String,
                &crate::rsc::rendering::layout::BoundaryPosition,
            >,
            result: &mut Vec<SuspenseBoundaryInfo>,
            parent_path: &mut Vec<String>,
        ) {
            match value {
                serde_json::Value::Array(arr) => {
                    if arr.len() >= 4
                        && arr[0].as_str() == Some("$")
                        && (arr[1].as_str() == Some("$Sreact.suspense")
                            || arr[1].as_str() == Some("react.suspense")
                            || arr[1].as_str() == Some("Suspense"))
                        && let Some(props) = arr.get(3).and_then(|v| v.as_object())
                        && let Some(boundary_id) = props.get("~boundaryId").and_then(|v| v.as_str())
                        && boundary_ids.contains(boundary_id)
                    {
                        let fallback_content =
                            props.get("fallback").cloned().unwrap_or(serde_json::Value::Null);

                        let position = boundary_positions.get(boundary_id);
                        let is_in_content_area =
                            position.map(|p| p.is_in_content_area).unwrap_or(false);

                        let dom_path = position
                            .map(|p| p.dom_path.iter().map(|n| n.to_string()).collect())
                            .unwrap_or_default();

                        result.push(SuspenseBoundaryInfo {
                            id: boundary_id.to_string(),
                            fallback_content,
                            parent_boundary_id: None,
                            pending_promise_count: 1,
                            parent_path: parent_path.clone(),
                            is_in_content_area,
                            skeleton_rendered: false,
                            is_resolved: false,
                            position_hints: Some(PositionHints {
                                in_content_area: is_in_content_area,
                                dom_path,
                                is_stable: true,
                            }),
                        });
                    }

                    for item in arr {
                        traverse(item, boundary_ids, boundary_positions, result, parent_path);
                    }
                }
                serde_json::Value::Object(obj) => {
                    if obj.contains_key("~preSerializedSuspense") {
                        if let Some(rsc_array) = obj.get("rscArray") {
                            traverse(
                                rsc_array,
                                boundary_ids,
                                boundary_positions,
                                result,
                                parent_path,
                            );
                        }
                    } else {
                        for value in obj.values() {
                            traverse(value, boundary_ids, boundary_positions, result, parent_path);
                        }
                    }
                }
                _ => {}
            }
        }

        let mut parent_path = Vec::new();
        traverse(rsc_data, &boundary_ids, &boundary_positions, &mut result, &mut parent_path);

        result
    }

    async fn send_initial_shell(
        &mut self,
        sender: &mpsc::Sender<RscStreamChunk>,
        partial_result: &PartialRenderResult,
    ) -> Result<(), RariError> {
        self.validate_lazy_marker_structure(partial_result)?;

        let symbol_row_id = if partial_result.has_suspense {
            self.row_counter += 1;
            let symbol_row_id = self.row_counter;
            let symbol_chunk =
                self.create_symbol_reference_chunk(symbol_row_id, "react.suspense")?;
            sender
                .send(symbol_chunk)
                .await
                .map_err(|e| RariError::internal(format!("Failed to send symbol chunk: {e}")))?;
            Some(symbol_row_id)
        } else {
            None
        };

        self.row_counter += 1;

        let shell_chunk = self.create_shell_chunk_with_module(
            &partial_result.initial_content,
            0,
            symbol_row_id,
            &FxHashMap::default(),
        )?;
        sender
            .send(shell_chunk)
            .await
            .map_err(|e| RariError::internal(format!("Failed to send shell chunk: {e}")))?;

        Ok(())
    }

    async fn send_boundary_update_with_map(
        sender: &mpsc::Sender<RscStreamChunk>,
        update: BoundaryUpdate,
        _boundary_rows_map: Arc<Mutex<FxHashMap<String, u32>>>,
    ) {
        for import_row in &update.import_rows {
            let import_chunk = RscStreamChunk {
                data: format!("{}\n", import_row).into_bytes(),
                chunk_type: RscChunkType::ModuleImport,
                row_id: 0,
                is_final: false,
                boundary_id: None,
            };

            if let Err(e) = sender.send(import_chunk).await {
                error!("Failed to send import row chunk: {}", e);
            }
        }

        let update_row = format!("{}:{}\n", update.row_id, update.content);

        let chunk = RscStreamChunk {
            data: update_row.into_bytes(),
            chunk_type: RscChunkType::BoundaryUpdate,
            row_id: update.row_id,
            is_final: false,
            boundary_id: Some(update.boundary_id.clone()),
        };

        match sender.send(chunk).await {
            Ok(_) => {}
            Err(e) => {
                error!(
                    "Failed to send boundary update chunk, boundary_id={}, row_id={}, error={}",
                    update.boundary_id, update.row_id, e
                );
            }
        }
    }

    async fn send_boundary_error(sender: &mpsc::Sender<RscStreamChunk>, error: BoundaryError) {
        #[allow(clippy::disallowed_methods)]
        let error_data = serde_json::json!({
            "boundary_id": error.boundary_id,
            "error": error.error_message,
        });

        let error_row = format!("{}:E{}\n", error.row_id, error_data);

        let chunk = RscStreamChunk {
            data: error_row.into_bytes(),
            chunk_type: RscChunkType::BoundaryError,
            row_id: error.row_id,
            is_final: false,
            boundary_id: None,
        };

        if let Err(e) = sender.send(chunk).await {
            error!(
                "Failed to send boundary error chunk, boundary_id={}, row_id={}, error={}",
                error.boundary_id, error.row_id, e
            );
        }
    }

    fn create_shell_chunk_with_module(
        &self,
        content: &serde_json::Value,
        _module_row_id: u32,
        symbol_row_id: Option<u32>,
        boundary_lazy_refs: &FxHashMap<String, u32>,
    ) -> Result<RscStreamChunk, RariError> {
        let rsc_element = self.json_to_rsc_element(content, symbol_row_id, boundary_lazy_refs)?;
        let row = format!("{}:{}\n", self.row_counter, rsc_element);

        Ok(RscStreamChunk {
            data: row.into_bytes(),
            chunk_type: RscChunkType::InitialShell,
            row_id: self.row_counter,
            is_final: false,
            boundary_id: None,
        })
    }

    fn json_to_rsc_element(
        &self,
        json: &serde_json::Value,
        symbol_row_id: Option<u32>,
        boundary_lazy_refs: &FxHashMap<String, u32>,
    ) -> Result<serde_json::Value, RariError> {
        if let Some(arr) = json.as_array() {
            if arr.len() >= 4 && arr[0].as_str() == Some("$") {
                let element_type = &arr[1];
                let key = &arr[2];
                let props = &arr[3];

                let final_element_type = if let Some(type_str) = element_type.as_str()
                    && (type_str == "react.suspense" || type_str == "$Sreact.suspense")
                    && let Some(row_id) = symbol_row_id
                {
                    serde_json::Value::String(format!("${}", row_id))
                } else {
                    element_type.clone()
                };

                let final_props = if let Some(props_obj) = props.as_object() {
                    let mut new_props = serde_json::Map::new();
                    for (k, v) in props_obj {
                        if k == "children" {
                            new_props.insert(
                                k.clone(),
                                self.convert_children(v, symbol_row_id, boundary_lazy_refs)?,
                            );
                        } else {
                            new_props.insert(k.clone(), v.clone());
                        }
                    }

                    if let Some(type_str) = element_type.as_str()
                        && (type_str == "react.suspense" || type_str == "$Sreact.suspense")
                        && let Some(boundary_id) =
                            props_obj.get("~boundaryId").and_then(|v| v.as_str())
                    {
                        if let Some(fallback) = props_obj.get("fallback") {
                            new_props.insert(
                                "fallback".to_string(),
                                self.convert_children(fallback, symbol_row_id, boundary_lazy_refs)?,
                            );
                        }

                        new_props.insert(
                            "~boundaryId".to_string(),
                            serde_json::Value::String(boundary_id.to_string()),
                        );
                    }

                    serde_json::Value::Object(new_props)
                } else {
                    props.clone()
                };

                return Ok(serde_json::Value::Array(vec![
                    serde_json::Value::String("$".to_string()),
                    final_element_type,
                    key.clone(),
                    final_props,
                ]));
            } else {
                let mut converted = Vec::new();
                for item in arr {
                    converted.push(self.json_to_rsc_element(
                        item,
                        symbol_row_id,
                        boundary_lazy_refs,
                    )?);
                }
                return Ok(serde_json::Value::Array(converted));
            }
        }

        if let Some(obj) = json.as_object()
            && let (Some(element_type), Some(props)) = (obj.get("type"), obj.get("props"))
        {
            let mut converted_props = serde_json::Map::new();

            if let Some(props_obj) = props.as_object() {
                for (key, value) in props_obj {
                    if key == "children" {
                        converted_props.insert(
                            key.clone(),
                            self.convert_children(value, symbol_row_id, boundary_lazy_refs)?,
                        );
                    } else {
                        converted_props.insert(key.clone(), value.clone());
                    }
                }
            }

            let final_element_type = if let Some(type_str) = element_type.as_str()
                && (type_str == "react.suspense" || type_str == "$Sreact.suspense")
                && let Some(row_id) = symbol_row_id
            {
                serde_json::Value::String(format!("${}", row_id))
            } else {
                element_type.clone()
            };

            return Ok(serde_json::Value::Array(vec![
                serde_json::Value::String("$".to_string()),
                final_element_type,
                serde_json::Value::Null,
                serde_json::Value::Object(converted_props),
            ]));
        }

        Ok(json.clone())
    }

    fn convert_children(
        &self,
        children: &serde_json::Value,
        symbol_row_id: Option<u32>,
        boundary_lazy_refs: &FxHashMap<String, u32>,
    ) -> Result<serde_json::Value, RariError> {
        match children {
            serde_json::Value::Array(arr) => {
                let mut converted = Vec::new();
                for child in arr {
                    converted.push(self.json_to_rsc_element(
                        child,
                        symbol_row_id,
                        boundary_lazy_refs,
                    )?);
                }
                Ok(serde_json::Value::Array(converted))
            }
            _ => self.json_to_rsc_element(children, symbol_row_id, boundary_lazy_refs),
        }
    }

    pub(crate) fn create_symbol_reference_chunk(
        &self,
        row_id: u32,
        symbol_name: &str,
    ) -> Result<RscStreamChunk, RariError> {
        let symbol_row = format!("{}:\"$S{}\"\n", row_id, symbol_name);

        Ok(RscStreamChunk {
            data: symbol_row.into_bytes(),
            chunk_type: RscChunkType::ModuleImport,
            row_id,
            is_final: false,
            boundary_id: None,
        })
    }

    fn validate_lazy_marker_structure(
        &self,
        partial_result: &PartialRenderResult,
    ) -> Result<(), RariError> {
        let mut promise_ids = FxHashSet::default();
        for promise in &partial_result.pending_promises {
            if !promise_ids.insert(promise.id.clone()) {
                return Err(RariError::internal(format!(
                    "Duplicate promise ID detected: {}",
                    promise.id
                )));
            }
        }

        let mut boundary_ids = FxHashSet::default();
        for boundary in &partial_result.boundaries {
            if !boundary_ids.insert(boundary.id.clone()) {
                return Err(RariError::internal(format!(
                    "Duplicate boundary ID detected: {}",
                    boundary.id
                )));
            }
        }

        for boundary in &partial_result.boundaries {
            if boundary.id.is_empty() {
                return Err(RariError::internal("Boundary ID cannot be empty".to_string()));
            }
        }

        Ok(())
    }
}
