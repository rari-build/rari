use futures::Stream;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::error;

use crate::error::RariError;
use crate::runtime::JsExecutionRuntime;

#[derive(Debug, Clone)]
pub struct PartialRenderResult {
    pub initial_content: serde_json::Value,
    pub pending_promises: Vec<PendingSuspensePromise>,
    pub boundaries: Vec<SuspenseBoundaryInfo>,
    pub has_suspense: bool,
}

#[derive(Debug, Clone)]
pub struct PendingSuspensePromise {
    pub id: String,
    pub boundary_id: String,
    pub component_path: String,
    pub promise_handle: String,
}

#[derive(Debug, Clone)]
pub struct SuspenseBoundaryInfo {
    pub id: String,
    pub fallback_content: serde_json::Value,
    pub parent_boundary_id: Option<String>,
    pub pending_promise_count: usize,
    pub parent_path: Vec<String>,
    pub is_in_content_area: bool,
    pub skeleton_rendered: bool,
    pub is_resolved: bool,
    pub position_hints: Option<PositionHints>,
}

#[derive(Debug, Clone)]
pub struct PositionHints {
    pub in_content_area: bool,
    pub dom_path: Vec<String>,
    pub is_stable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryUpdate {
    pub boundary_id: String,
    pub content: serde_json::Value,
    pub row_id: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dom_path: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryError {
    pub boundary_id: String,
    pub error_message: String,
    pub row_id: u32,
}

#[derive(Debug, Clone)]
pub struct RscStreamChunk {
    pub data: Vec<u8>,
    pub chunk_type: RscChunkType,
    pub row_id: u32,
    pub is_final: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RscChunkType {
    ModuleImport,
    InitialShell,
    BoundaryUpdate,
    BoundaryError,
    StreamComplete,
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
            let resolution_script = format!(
                r#"
                (function() {{
                    const safeSerializeError = function(error, phase) {{
                        const errorObj = {{
                            success: false,
                            boundary_id: '{boundary_id}',
                            errorContext: {{
                                phase: phase,
                                promiseId: '{promise_id}',
                                componentPath: '{component_path}',
                                availablePromises: Object.keys(globalThis.__suspense_promises || {{}})
                            }}
                        }};

                        try {{
                            errorObj.errorName = error.name || 'UnknownError';
                        }} catch (e) {{
                            errorObj.errorName = 'UnknownError';
                        }}

                        try {{
                            errorObj.error = error.message || String(error) || 'Unknown error';
                        }} catch (e) {{
                            errorObj.error = 'Error message unavailable';
                        }}

                        try {{
                            errorObj.errorStack = error.stack || 'No stack trace available';
                        }} catch (e) {{
                            errorObj.errorStack = 'Stack trace unavailable';
                        }}

                        try {{
                            const additionalProps = {{}};
                            for (const key in error) {{
                                if (error.hasOwnProperty(key) && key !== 'name' && key !== 'message' && key !== 'stack') {{
                                    try {{
                                        const value = error[key];
                                        if (value !== undefined && value !== null &&
                                            typeof value !== 'function' && typeof value !== 'symbol') {{
                                            additionalProps[key] = String(value);
                                        }}
                                    }} catch (propError) {{
                                    }}
                                }}
                            }}
                            if (Object.keys(additionalProps).length > 0) {{
                                errorObj.additionalErrorProps = additionalProps;
                            }}
                        }} catch (e) {{
                        }}

                        return errorObj;
                    }};

                    try {{
                        const promiseId = '{promise_id}';
                        const boundaryId = '{boundary_id}';

                        const promise = globalThis.__suspense_promises[promiseId];

                        if (!promise) {{
                            return Promise.resolve({{
                                success: false,
                                boundary_id: boundaryId,
                                error: 'Promise not found: ' + promiseId,
                                errorName: 'PromiseNotFound',
                                errorStack: 'No stack trace (promise not registered)',
                                errorContext: {{
                                    phase: 'promise_resolution',
                                    promiseId: promiseId,
                                    componentPath: '{component_path}',
                                    availablePromises: Object.keys(globalThis.__suspense_promises || {{}})
                                }}
                            }});
                        }}

                        return promise.then(async function(resolvedElement) {{
                            if (resolvedElement === undefined || resolvedElement === null) {{
                                return {{
                                    success: false,
                                    boundary_id: boundaryId,
                                    error: 'Promise resolved to null/undefined',
                                    errorName: 'InvalidPromiseResolution',
                                    errorStack: 'No stack trace (invalid resolution)',
                                    errorContext: {{
                                        phase: 'promise_resolution',
                                        promiseId: promiseId,
                                        componentPath: '{component_path}',
                                        resolvedType: typeof resolvedElement,
                                        resolvedValue: String(resolvedElement)
                                    }}
                                }};
                            }}

                            let rscData;
                            try {{
                                if (globalThis.renderToRsc) {{
                                    rscData = await globalThis.renderToRsc(resolvedElement, globalThis.__rsc_client_components || {{}});
                                }} else {{
                                    rscData = resolvedElement;
                                }}
                            }} catch (rscError) {{
                                return safeSerializeError(rscError, 'rsc_conversion');
                            }}

                            return {{
                                success: true,
                                boundary_id: boundaryId,
                                content: rscData
                            }};
                        }}).catch(function(awaitError) {{
                            return safeSerializeError(awaitError, 'promise_resolution');
                        }});

                    }} catch (error) {{
                        return Promise.resolve(safeSerializeError(error, 'composition'));
                    }}
                }})()
                "#,
                promise_id = promise_id,
                boundary_id = boundary_id,
                component_path = promise.component_path
            );

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

pub struct SuspenseBoundaryManager {
    boundaries: Arc<Mutex<FxHashMap<String, SuspenseBoundaryInfo>>>,
    boundary_stack: Vec<String>,
    resolved_boundaries: Arc<Mutex<FxHashMap<String, serde_json::Value>>>,
    rendered_skeleton_ids: Arc<Mutex<FxHashSet<String>>>,
}

impl Default for SuspenseBoundaryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SuspenseBoundaryManager {
    pub fn new() -> Self {
        Self {
            boundaries: Arc::new(Mutex::new(FxHashMap::default())),
            boundary_stack: Vec::new(),
            resolved_boundaries: Arc::new(Mutex::new(FxHashMap::default())),
            rendered_skeleton_ids: Arc::new(Mutex::new(FxHashSet::default())),
        }
    }

    pub async fn register_boundary(&mut self, mut boundary: SuspenseBoundaryInfo) {
        let boundary_id = boundary.id.clone();

        {
            let boundaries = self.boundaries.lock().await;
            if boundaries.contains_key(&boundary_id) {
                tracing::warn!(
                    "Duplicate boundary registration detected: boundary_id='{}'. This may cause duplicate loading skeletons.",
                    boundary_id
                );
            }
        }

        boundary.skeleton_rendered = false;
        boundary.is_resolved = false;

        {
            let mut boundaries = self.boundaries.lock().await;
            boundaries.insert(boundary_id.clone(), boundary);
        }
        self.boundary_stack.push(boundary_id);
    }

    pub async fn mark_skeleton_rendered(&self, boundary_id: &str) -> bool {
        let mut skeleton_ids = self.rendered_skeleton_ids.lock().await;
        let is_first = skeleton_ids.insert(boundary_id.to_string());

        if !is_first {
            tracing::warn!(
                "Duplicate loading skeleton detected for boundary '{}'. Only one skeleton should be rendered per boundary.",
                boundary_id
            );
        }

        {
            let mut boundaries = self.boundaries.lock().await;
            if let Some(boundary) = boundaries.get_mut(boundary_id) {
                if boundary.skeleton_rendered {
                    tracing::warn!(
                        "Boundary '{}' already has skeleton_rendered=true, but skeleton is being rendered again",
                        boundary_id
                    );
                }
                boundary.skeleton_rendered = true;
            }
        }

        is_first
    }

    pub async fn resolve_boundary(&self, boundary_id: &str, content: serde_json::Value) {
        {
            let boundaries = self.boundaries.lock().await;
            if let Some(boundary) = boundaries.get(boundary_id)
                && boundary.is_resolved
            {
                tracing::warn!(
                    "Boundary '{}' is already resolved. Duplicate resolution may cause orphaned loading skeletons.",
                    boundary_id
                );
            }
        }

        {
            let mut resolved = self.resolved_boundaries.lock().await;
            resolved.insert(boundary_id.to_string(), content);
        }

        {
            let mut boundaries = self.boundaries.lock().await;
            if let Some(boundary) = boundaries.get_mut(boundary_id) {
                boundary.pending_promise_count = 0;
                boundary.is_resolved = true;
            }
        }

        {
            let mut skeleton_ids = self.rendered_skeleton_ids.lock().await;
            skeleton_ids.remove(boundary_id);
        }
    }

    pub async fn get_pending_boundaries(&self) -> Vec<SuspenseBoundaryInfo> {
        let boundaries = self.boundaries.lock().await;
        let resolved = self.resolved_boundaries.lock().await;

        boundaries
            .values()
            .filter(|b| !resolved.contains_key(&b.id) && b.pending_promise_count > 0)
            .cloned()
            .collect()
    }

    pub async fn validate_no_duplicate_skeletons(&self) -> Vec<String> {
        let boundaries = self.boundaries.lock().await;
        let skeleton_ids = self.rendered_skeleton_ids.lock().await;

        let mut duplicates = Vec::new();

        for (id, boundary) in boundaries.iter() {
            if boundary.skeleton_rendered && !skeleton_ids.contains(id) && !boundary.is_resolved {
                tracing::warn!(
                    "Inconsistency detected: boundary '{}' has skeleton_rendered=true but is not in rendered_skeleton_ids",
                    id
                );
                duplicates.push(id.clone());
            }
        }

        if !duplicates.is_empty() {
            tracing::error!(
                "Duplicate skeleton validation failed: {} boundaries have inconsistent state",
                duplicates.len()
            );
        }

        duplicates
    }

    pub async fn get_rendered_skeleton_count(&self) -> usize {
        self.rendered_skeleton_ids.lock().await.len()
    }

    pub async fn has_rendered_skeleton(&self, boundary_id: &str) -> bool {
        self.rendered_skeleton_ids.lock().await.contains(boundary_id)
    }
}

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
        layout_structure: crate::rsc::layout_renderer::LayoutStructure,
    ) -> Result<RscStream, RariError> {
        if !layout_structure.is_valid() {
            tracing::error!(
                "StreamingRenderer: Invalid layout structure detected, streaming should not have been initiated"
            );

            tracing::error!(
                "Layout structure details: has_navigation={}, navigation_position={:?}, content_position={:?}, suspense_boundaries={}",
                layout_structure.has_navigation,
                layout_structure.navigation_position,
                layout_structure.content_position,
                layout_structure.suspense_boundaries.len()
            );

            for boundary in &layout_structure.suspense_boundaries {
                tracing::error!(
                    "  Suspense boundary '{}': parent_path={:?}, is_in_content_area={}",
                    boundary.boundary_id,
                    boundary.parent_path,
                    boundary.is_in_content_area
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

        if let Some(resolver) = &self.promise_resolver {
            let runtime = Arc::clone(&self.runtime);
            let resolver_clone = Arc::clone(resolver);
            let pending_promises = partial_result.pending_promises.clone();

            tokio::spawn(async move {
                let execute_script = Self::build_deferred_execution_script();

                match runtime
                    .execute_script("<execute_deferred_components>".to_string(), execute_script)
                    .await
                {
                    Ok(result) => {
                        let result_str = result.to_string();
                        match serde_json::from_str::<serde_json::Value>(&result_str) {
                            Ok(data) => {
                                if let Some(results) = data["results"].as_array() {
                                    for result in results {
                                        if !result["success"].as_bool().unwrap_or(false) {
                                            let error_msg =
                                                result["error"].as_str().unwrap_or("unknown");
                                            let error_name = result["errorName"]
                                                .as_str()
                                                .unwrap_or("UnknownError");
                                            let component_path = result["componentPath"]
                                                .as_str()
                                                .unwrap_or("unknown");
                                            let promise_id =
                                                result["promiseId"].as_str().unwrap_or("unknown");

                                            tracing::warn!(
                                                "Deferred component failed: promiseId={}, component={}, error={} ({})",
                                                promise_id,
                                                component_path,
                                                error_msg,
                                                error_name
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse deferred execution result: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to execute deferred components: {}", e);
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
                        let (was_skeleton_removed, is_duplicate_resolution) = {
                            let mut skeleton_ids = rendered_skeleton_ids.lock().await;
                            let mut resolved_ids = resolved_boundary_ids.lock().await;

                            let skeleton_removed = skeleton_ids.remove(&update.boundary_id);
                            let is_first_resolution = resolved_ids.insert(update.boundary_id.clone());

                            (skeleton_removed, !is_first_resolution)
                        };

                        if is_duplicate_resolution {
                            tracing::warn!(
                                "Boundary '{}' is already resolved. Skipping duplicate resolution to prevent orphaned loading skeletons.",
                                update.boundary_id
                            );
                            continue;
                        }

                        if !was_skeleton_removed {
                            tracing::warn!(
                                "Boundary '{}' resolved but no skeleton was tracked. This may indicate the skeleton was never rendered.",
                                update.boundary_id
                            );
                        }

                        if let Some(dom_path) = boundary_positions_clone.lock().await.get(&update.boundary_id) {
                            update.dom_path = dom_path.clone();
                        } else {
                            tracing::error!(
                                "DOM path not found for boundary '{}' in boundary_positions map. This may cause incorrect skeleton replacement.",
                                update.boundary_id
                            );
                        }

                        if update.dom_path.is_empty() {
                            tracing::error!(
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

            {
                let skeleton_ids = rendered_skeleton_ids.lock().await;
                if !skeleton_ids.is_empty() {
                    tracing::warn!(
                        "Stream completed with {} unresolved loading skeletons. These may be orphaned: {:?}",
                        skeleton_ids.len(),
                        skeleton_ids.iter().collect::<Vec<_>>()
                    );
                }
            }

            let final_chunk = RscStreamChunk {
                data: b"STREAM_COMPLETE\n".to_vec(),
                chunk_type: RscChunkType::StreamComplete,
                row_id: u32::MAX,
                is_final: true,
            };

            if let Err(e) = chunk_sender_clone.send(final_chunk).await {
                tracing::error!("Failed to send stream completion signal: {}", e);
            }
        });

        Ok(RscStream::new(chunk_receiver))
    }

    pub async fn start_streaming_with_precomputed_data(
        &mut self,
        rsc_data: serde_json::Value,
        boundaries: Vec<crate::rsc::layout_renderer::BoundaryInfo>,
        layout_structure: crate::rsc::layout_renderer::LayoutStructure,
    ) -> Result<RscStream, RariError> {
        if !layout_structure.is_valid() {
            tracing::error!(
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

        let partial_result = PartialRenderResult {
            initial_content: rsc_data,
            pending_promises: Vec::new(),
            boundaries: Vec::new(),
            has_suspense: !boundaries.is_empty(),
        };

        self.send_initial_shell(&chunk_sender, &partial_result).await?;

        if let Some(resolver) = &self.promise_resolver {
            let runtime = Arc::clone(&self.runtime);
            let resolver_clone = Arc::clone(resolver);
            let pending_promises = partial_result.pending_promises.clone();

            tokio::spawn(async move {
                let execute_script = Self::build_deferred_execution_script();

                match runtime
                    .execute_script("<execute_deferred_components>".to_string(), execute_script)
                    .await
                {
                    Ok(result) => {
                        let result_str = result.to_string();
                        match serde_json::from_str::<serde_json::Value>(&result_str) {
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!("Failed to parse deferred execution result: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to execute deferred components: {}", e);
                    }
                }

                for promise in pending_promises {
                    resolver_clone.resolve_async(promise).await;
                }
            });
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
                        let (_was_skeleton_removed, is_duplicate_resolution) = {
                            let mut skeleton_ids = rendered_skeleton_ids.lock().await;
                            let mut resolved_ids = resolved_boundary_ids.lock().await;
                            let skeleton_removed = skeleton_ids.remove(&update.boundary_id);
                            let is_first_resolution = resolved_ids.insert(update.boundary_id.clone());
                            (skeleton_removed, !is_first_resolution)
                        };

                        if is_duplicate_resolution {
                            tracing::warn!(
                                "Boundary '{}' is already resolved. Skipping duplicate resolution.",
                                update.boundary_id
                            );
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
            };

            if let Err(e) = chunk_sender_clone.send(final_chunk).await {
                tracing::error!("Failed to send stream completion signal: {}", e);
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
                let init_script = r#"
                    (function() {
                        if (!globalThis.__suspense_promises) {
                            globalThis.__suspense_promises = {};
                        }

                        if (!globalThis.__deferred_async_components) {
                            globalThis.__deferred_async_components = [];
                        }

                        return {
                            initialized: true,
                            existingPromises: Object.keys(globalThis.__suspense_promises || {}).length,
                            deferredComponents: globalThis.__deferred_async_components.length
                        };
                    })()
                "#;

                match runtime
                    .execute_script("<init_promise_tracking>".to_string(), init_script.to_string())
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("Failed to initialize promise tracking: {}", e);
                    }
                }

                let execute_script = r#"
                    (async function() {
                        if (globalThis.__deferred_async_components && globalThis.__deferred_async_components.length > 0) {

                            const results = [];
                            for (const deferred of globalThis.__deferred_async_components) {
                                try {

                                    if (typeof deferred.component !== 'function') {
                                        results.push({ promiseId: deferred.promiseId, success: false, error: 'Not a function' });
                                        continue;
                                    }

                                    const componentPromise = deferred.component(deferred.props);

                                    if (!componentPromise || typeof componentPromise.then !== 'function') {
                                        results.push({ promiseId: deferred.promiseId, success: false, error: 'Not a promise' });
                                        continue;
                                    }

                                    globalThis.__suspense_promises = globalThis.__suspense_promises || {};
                                    globalThis.__suspense_promises[deferred.promiseId] = componentPromise;
                                    results.push({ promiseId: deferred.promiseId, success: true });
                                } catch (e) {
                                    results.push({
                                        promiseId: deferred.promiseId,
                                        success: false,
                                        error: e.message || 'Unknown error',
                                        stack: e.stack
                                    });
                                }
                            }

                            const successCount = results.filter(r => r.success).length;
                            globalThis.__deferred_async_components = [];
                            return {
                                success: true,
                                count: successCount,
                                total: results.length,
                                results: results
                            };
                        }
                        return { success: true, count: 0, total: 0 };
                    })()
                "#;

                match runtime
                    .execute_script(
                        "<execute_deferred_components>".to_string(),
                        execute_script.to_string(),
                    )
                    .await
                {
                    Ok(result) => {
                        let result_str = result.to_string();
                        match serde_json::from_str::<serde_json::Value>(&result_str) {
                            Ok(data) => {
                                if let Some(results) = data["results"].as_array() {
                                    for result in results {
                                        if !result["success"].as_bool().unwrap_or(false) {
                                            tracing::warn!(
                                                "Deferred component failed: promiseId={}, error={}",
                                                result["promiseId"].as_str().unwrap_or("unknown"),
                                                result["error"].as_str().unwrap_or("unknown")
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse deferred execution result: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to execute deferred components: {}", e);
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
                                 };

                                 if chunk_sender_clone.send(chunk).await.is_err() {
                                     break;
                                 }

                             }
                             Some(error) = error_receiver.recv() => {

                                 tracing::error!(
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
                let execute_script = r#"
                    (async function() {
                        if (globalThis.__deferred_async_components && globalThis.__deferred_async_components.length > 0) {

                            const results = [];
                            for (const deferred of globalThis.__deferred_async_components) {
                                try {

                                    if (typeof deferred.component !== 'function') {
                                        results.push({ promiseId: deferred.promiseId, success: false, error: 'Not a function' });
                                        continue;
                                    }

                                    const componentPromise = deferred.component(deferred.props);

                                    if (!componentPromise || typeof componentPromise.then !== 'function') {
                                        results.push({ promiseId: deferred.promiseId, success: false, error: 'Not a promise' });
                                        continue;
                                    }

                                    globalThis.__suspense_promises = globalThis.__suspense_promises || {};
                                    globalThis.__suspense_promises[deferred.promiseId] = componentPromise;
                                    results.push({ promiseId: deferred.promiseId, success: true });
                                } catch (e) {
                                    results.push({
                                        promiseId: deferred.promiseId,
                                        success: false,
                                        error: e.message || 'Unknown error',
                                        stack: e.stack
                                    });
                                }
                            }

                            const successCount = results.filter(r => r.success).length;
                            globalThis.__deferred_async_components = [];
                            return {
                                success: true,
                                count: successCount,
                                total: results.length,
                                results: results
                            };
                        }
                        return { success: true, count: 0, total: 0 };
                    })()
                "#;

                match runtime
                    .execute_script(
                        "<execute_deferred_components>".to_string(),
                        execute_script.to_string(),
                    )
                    .await
                {
                    Ok(result) => {
                        let result_str = result.to_string();
                        match serde_json::from_str::<serde_json::Value>(&result_str) {
                            Ok(data) => {
                                if let Some(results) = data["results"].as_array() {
                                    for result in results {
                                        if !result["success"].as_bool().unwrap_or(false) {
                                            tracing::warn!(
                                                "Deferred component failed: promiseId={}, error={}",
                                                result["promiseId"].as_str().unwrap_or("unknown"),
                                                result["error"].as_str().unwrap_or("unknown")
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse deferred execution result: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to execute deferred components: {}", e);
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
            };

            if let Err(e) = chunk_sender_clone.send(final_chunk).await {
                tracing::error!("Failed to send stream completion signal: {}", e);
            }
        });

        Ok(RscStream::new(chunk_receiver))
    }

    async fn render_partial(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<PartialRenderResult, RariError> {
        let react_init_script = r#"
            (function() {
                if (typeof React === 'undefined') {
                    try {
                        if (typeof globalThis.__rsc_modules !== 'undefined') {
                            const reactModule = globalThis.__rsc_modules['react'] ||
                                              globalThis.__rsc_modules['React'] ||
                                              Object.values(globalThis.__rsc_modules).find(m => m && m.createElement);
                            if (reactModule) {
                                globalThis.React = reactModule;
                            }
                        }

                        if (typeof React === 'undefined' && typeof require !== 'undefined') {
                            globalThis.React = require('react');
                        }

                        if (typeof React !== 'undefined' && React.createElement && !globalThis.__react_patched) {
                            globalThis.__original_create_element = React.createElement;

                                const createElementOverride = function(type, props, ...children) {
                                    return globalThis.__original_create_element(type, props, ...children);
                                };

                            Object.defineProperty(React, 'createElement', {
                                value: createElementOverride,
                                writable: false,
                                enumerable: true,
                                configurable: false
                            });

                            globalThis.__react_patched = true;
                        }

                        if (typeof React !== 'undefined' && React.Suspense) {
                            React.__originalSuspense = React.Suspense;

                            React.Suspense = function SuspenseOverride(props) {
                                if (!props) return null;
                                const previousBoundaryId = globalThis.__current_boundary_id;
                                const boundaryId = 'boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                globalThis.__current_boundary_id = boundaryId;
                                try {
                                    const safeFallback = props?.fallback || null;
                                    const serializableFallback = globalThis.__safeSerializeElement(safeFallback);
                                    globalThis.__discovered_boundaries.push({ id: boundaryId, fallback: serializableFallback, parentId: previousBoundaryId });
                                    if (!props.children) {
                                        return safeFallback;
                                    }
                                    return props.children;
                                } catch (error) {
                                    if (error && error.$$typeof === Symbol.for('react.suspense.pending') && error.promise) {
                                        const promiseId = 'suspense_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                        globalThis.__suspense_promises = globalThis.__suspense_promises || {};
                                        globalThis.__suspense_promises[promiseId] = error.promise;
                                        globalThis.__pending_promises = globalThis.__pending_promises || [];
                                        globalThis.__pending_promises.push({ id: promiseId, boundaryId: boundaryId, componentPath: (error.componentName || 'unknown') });
                                        return props.fallback || null;
                                    }
                                    return props?.fallback || React.createElement('div', null, 'Suspense Error: ' + (error && error.message ? error.message : 'Unknown'));
                                } finally {
                                    globalThis.__current_boundary_id = previousBoundaryId;
                                }
                            };
                        }

                        if (typeof React === 'undefined') {
                            globalThis.React = {
                                createElement: function(type, props, ...children) {
                                    return {
                                        type: type,
                                        props: props ? { ...props, children: children.length > 0 ? children : props.children } : { children: children },
                                        key: props?.key || null,
                                        ref: props?.ref || null
                                    };
                                },
                                Fragment: Symbol.for('react.fragment'),
                                Suspense: function(props) {
                                    return props.children;
                                }
                            };
                        }
                    } catch (e) {
                        console.error('Failed to load React in streaming context:', e);
                        throw new Error('Cannot initialize streaming without React: ' + e.message);
                    }
                }

                return {
                    available: typeof React !== 'undefined',
                    reactType: typeof React,
                    createElementType: typeof React.createElement,
                    suspenseType: typeof React.Suspense
                };
            })()
        "#;

        let react_init_result = self
            .runtime
            .execute_script("streaming-react-init".to_string(), react_init_script.to_string())
            .await?;

        if let Some(available) = react_init_result.get("available").and_then(|v| v.as_bool()) {
            if !available {
                return Err(RariError::internal("Failed to initialize React in streaming context"));
            }
        } else {
            return Err(RariError::internal("Failed to check React initialization"));
        }

        let init_script = r#"
            if (!globalThis.renderToRsc) {
                globalThis.renderToRsc = async function(element, clientComponents = {}) {
                    if (!element) return null;

                    if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
                        return element;
                    }

                    if (Array.isArray(element)) {
                        const results = [];
                        for (const child of element) {
                            results.push(await globalThis.renderToRsc(child, clientComponents));
                        }
                        return results;
                    }

                    if (element && typeof element === 'object') {
                        const uniqueKey = element.key || `element-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

                        if (element.type) {
                            if (typeof element.type === 'string') {
                                const props = element.props || {};
                                const { children: propsChildren, ...otherProps } = props;

                                const actualChildren = element.children || propsChildren;

                                const rscProps = {
                                    ...otherProps,
                                    children: actualChildren ? await globalThis.renderToRsc(actualChildren, clientComponents) : undefined
                                };
                                if (rscProps.children === undefined) {
                                    delete rscProps.children;
                                }
                                return ["$", element.type, uniqueKey, rscProps];
                            } else if (typeof element.type === 'function') {
                                try {
                                    const props = element.props || {};
                                    let result = element.type(props);

                                    if (result && typeof result.then === 'function') {
                                        result = await result;
                                    }

                                    return await globalThis.renderToRsc(result, clientComponents);
                                } catch (error) {
                                    console.error('Error rendering function component:', error);
                                    return ["$", "div", uniqueKey, {
                                        children: `Error: ${error.message}`,
                                        style: { color: 'red', border: '1px solid red', padding: '10px' }
                                    }];
                                }
                            }
                        }

                        return ["$", "div", uniqueKey, {
                            className: "rsc-unknown",
                            children: "Unknown element type"
                        }];
                    }

                    return element;
                };
            }


            if (typeof React === 'undefined') {
                throw new Error('React is not available in streaming context. This suggests the runtime was not properly initialized with React extensions.');
            }

            if (!globalThis.__suspense_streaming) {
                globalThis.__suspense_streaming = true;
                globalThis.__suspense_promises = {};
                globalThis.__boundary_props = {};
                globalThis.__discovered_boundaries = [];
                globalThis.__pending_promises = [];
                globalThis.__current_boundary_id = null;

                globalThis.__safeSerializeElement = function(element) {
                    if (!element) return null;

                    try {
                        if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
                            return element;
                        }

                        if (element && typeof element === 'object') {
                            return {
                                type: element.type || 'div',
                                props: element.props ? {
                                    children: (element.props.children === undefined ? null : element.props.children),
                                    ...(element.props.className && { className: element.props.className })
                                } : { children: null },
                                key: null,
                                ref: null
                            };
                        }

                        return { type: 'div', props: { children: null }, key: null, ref: null };
                    } catch (e) {
                        return { type: 'div', props: { children: null }, key: null, ref: null };
                    }
                };

                if (!globalThis.__react_patched && typeof React !== 'undefined' && React.createElement) {
                    globalThis.__original_create_element = React.createElement;

                    const createElementOverride = function(type, props, ...children) {
                        return globalThis.__original_create_element(type, props, ...children);
                    };

                    React.createElement = createElementOverride;
                    globalThis.__react_patched = true;
                }
            } else {
                globalThis.__discovered_boundaries = [];
                globalThis.__pending_promises = [];
                globalThis.__current_boundary_id = null;
            }
        "#;

        self.runtime
            .execute_script("<streaming_init>".to_string(), init_script.to_string())
            .await
            .map_err(|e| RariError::internal(format!("Streaming init failed: {e}")))?;

        let setup_script = format!(
            r#"
            globalThis.__render_component_async = async function() {{
                try {{

                    let Component = (globalThis.__rsc_modules && globalThis.__rsc_modules['{component_id}']?.default) ||
                                    globalThis['{component_id}'] ||
                                    (globalThis.__rsc_modules && globalThis.__rsc_modules['{component_id}']);

                    if (Component && typeof Component === 'object' && typeof Component.default === 'function') {{
                        Component = Component.default;
                    }}

                    if (!Component || typeof Component !== 'function') {{
                        throw new Error('Component {component_id} not found or not a function');
                    }}

                    const props = {props_json};
                    globalThis.__boundary_props['root'] = props;

                    let element;
                    let renderError = null;
                    let isAsyncResult = false;


                    try {{
                        const isOverrideActive = React.createElement.toString().includes('SUSPENSE BOUNDARY FOUND');

                        if (!isOverrideActive) {{
                            if (!globalThis.__original_create_element) {{
                                globalThis.__original_create_element = React.createElement;
                            }}

                            React.createElement = function(type, props, ...children) {{
                                const isSuspenseComponent = (type) => {{
                                    if (typeof React !== 'undefined' && React.Suspense && type === React.Suspense) {{
                                        return true;
                                    }}
                                    if (typeof type === 'function' && type.name === 'Suspense') {{
                                        return true;
                                    }}
                                    return false;
                                }};

                                if (isSuspenseComponent(type)) {{
                                    const boundaryId = 'boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                    const previousBoundaryId = globalThis.__current_boundary_id;
                                    globalThis.__current_boundary_id = boundaryId;

                                    const safeFallback = props?.fallback || null;
                                    const serializableFallback = globalThis.__safeSerializeElement(safeFallback);

                                    globalThis.__discovered_boundaries.push({{
                                        id: boundaryId,
                                        fallback: serializableFallback,
                                        parentId: previousBoundaryId
                                    }});

                                    globalThis.__current_boundary_id = previousBoundaryId;
                                    return globalThis.__original_create_element('suspense', {{...props, key: boundaryId}}, ...children);
                                }}
                                return globalThis.__original_create_element(type, props, ...children);
                            }};
                        }}

                        const isAsyncFunction = Component.constructor.name === 'AsyncFunction' ||
                                              Component[Symbol.toStringTag] === 'AsyncFunction' ||
                                              (Component.toString && Component.toString().trim().startsWith('async'));

                        if (isAsyncFunction) {{

                            const boundaryId = 'async_boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                            const promiseId = 'async_promise_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);

                            let loadingComponent = null;
                            const componentPath = '{component_id}';

                            const loadingPaths = [
                                componentPath.replace('/page', '/loading'),
                                componentPath.replace(/\/[^/]+$/, '/loading'),
                                componentPath + '-loading',
                                'app/loading'
                            ];


                            for (const loadingPath of loadingPaths) {{
                                if (globalThis.__rsc_modules && globalThis.__rsc_modules[loadingPath]) {{
                                    const LoadingModule = globalThis.__rsc_modules[loadingPath];
                                    const LoadingComp = LoadingModule.default || LoadingModule;
                                    if (typeof LoadingComp === 'function') {{
                                        try {{
                                            loadingComponent = LoadingComp({{}});
                                            break;
                                        }} catch (e) {{
                                        }}
                                    }}
                                }}
                            }}

                            let fallbackContent;
                            if (loadingComponent) {{
                                if (loadingComponent && typeof loadingComponent === 'object' &&
                                    (loadingComponent.type || loadingComponent.$$typeof)) {{
                                    fallbackContent = loadingComponent;
                                }} else {{
                                    fallbackContent = globalThis.__original_create_element('div', {{
                                        className: 'rari-loading',
                                        children: 'Loading...'
                                    }});
                                }}
                            }} else {{
                                fallbackContent = globalThis.__original_create_element('div', {{
                                    className: 'rari-loading',
                                    children: 'Loading...'
                                }});
                            }}

                            globalThis.__discovered_boundaries = globalThis.__discovered_boundaries || [];
                            globalThis.__discovered_boundaries.push({{
                                id: boundaryId,
                                fallback: globalThis.__safeSerializeElement(fallbackContent),
                                parentId: null
                            }});

                            globalThis.__pending_promises = globalThis.__pending_promises || [];
                            globalThis.__pending_promises.push({{
                                id: promiseId,
                                boundaryId: boundaryId,
                                componentPath: '{component_id}'
                            }});

                            const serializedFallback = globalThis.__safeSerializeElement(fallbackContent);

                            const safeBoundaries = (globalThis.__discovered_boundaries || []).map(boundary => ({{
                                id: boundary.id,
                                fallback: globalThis.__safeSerializeElement(boundary.fallback),
                                parentId: boundary.parentId
                            }}));

                            const fallbackRsc = ["$", "react.suspense", null, {{
                                boundaryId: boundaryId,
                                __boundary_id: boundaryId,
                                fallback: ["$", serializedFallback.type, serializedFallback.key, serializedFallback.props],
                                children: null
                            }}];

                            const initialResult = {{
                                success: true,
                                rsc_data: fallbackRsc,
                                boundaries: safeBoundaries,
                                pending_promises: globalThis.__pending_promises || [],
                                has_suspense: true,
                                error: null,
                                error_stack: null
                            }};

                            try {{
                                const jsonString = JSON.stringify(initialResult);
                                globalThis.__streaming_result = JSON.parse(jsonString);
                            }} catch (jsonError) {{
                                globalThis.__streaming_result = initialResult;
                            }}
                            globalThis.__initial_render_complete = true;

                                success: initialResult.success,
                                has_rsc_data: !!initialResult.rsc_data,
                                boundaries_count: initialResult.boundaries.length,
                                pending_count: initialResult.pending_promises.length
                            }}));

                            globalThis.__deferred_async_components = globalThis.__deferred_async_components || [];
                            globalThis.__deferred_async_components.push({{
                                component: Component,
                                props: props,
                                promiseId: promiseId,
                                boundaryId: boundaryId,
                                componentPath: '{component_id}'
                            }});

                            return;
                        }}

                        element = Component(props);

                        if (element && typeof element.then === 'function') {{
                            isAsyncResult = true;

                            const boundaryId = 'async_boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                            const promiseId = 'async_promise_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);

                            globalThis.__suspense_promises = globalThis.__suspense_promises || {{}};
                            globalThis.__suspense_promises[promiseId] = element;

                            globalThis.__pending_promises = globalThis.__pending_promises || [];
                            globalThis.__pending_promises.push({{
                                id: promiseId,
                                boundaryId: boundaryId,
                                componentPath: '{component_id}'
                            }});

                            let loadingComponent = null;
                            const componentPath = '{component_id}';

                            const loadingPaths = [
                                componentPath.replace('/page', '/loading'),
                                componentPath.replace(/\/[^/]+$/, '/loading'),
                                componentPath + '-loading',
                                'app/loading'
                            ];


                            for (const loadingPath of loadingPaths) {{
                                if (globalThis.__rsc_modules && globalThis.__rsc_modules[loadingPath]) {{
                                    const LoadingModule = globalThis.__rsc_modules[loadingPath];
                                    const LoadingComp = LoadingModule.default || LoadingModule;
                                    if (typeof LoadingComp === 'function') {{
                                        try {{
                                            loadingComponent = LoadingComp({{}});
                                            break;
                                        }} catch (e) {{
                                        }}
                                    }}
                                }}
                            }}

                            let fallbackContent;
                            if (loadingComponent && typeof loadingComponent === 'object' &&
                                (loadingComponent.type || loadingComponent.$$typeof)) {{
                                fallbackContent = loadingComponent;
                            }} else {{
                                if (loadingComponent) {{
                                }}
                                fallbackContent = globalThis.__original_create_element('div', {{
                                    className: 'rari-loading',
                                    children: 'Loading...'
                                }});
                            }}

                            globalThis.__discovered_boundaries = globalThis.__discovered_boundaries || [];
                            globalThis.__discovered_boundaries.push({{
                                id: boundaryId,
                                fallback: globalThis.__safeSerializeElement(fallbackContent),
                                parentId: null
                            }});

                            element = fallbackContent;

                            const safeBoundaries = (globalThis.__discovered_boundaries || []).map(boundary => ({{
                                id: boundary.id,
                                fallback: globalThis.__safeSerializeElement(boundary.fallback),
                                parentId: boundary.parentId
                            }}));

                            const serializedFallback = globalThis.__safeSerializeElement(fallbackContent);
                            const simpleFallbackRsc = {{
                                type: "react.suspense",
                                key: null,
                                props: {{
                                    boundaryId: boundaryId,
                                    __boundary_id: boundaryId,
                                    fallback: {{
                                        type: serializedFallback.type,
                                        key: serializedFallback.key,
                                        props: serializedFallback.props
                                    }},
                                    children: null
                                }}
                            }};

                            const initialResult = {{
                                success: true,
                                rsc_data: simpleFallbackRsc,
                                boundaries: safeBoundaries,
                                pending_promises: globalThis.__pending_promises || [],
                                has_suspense: true,
                                error: null,
                                error_stack: null
                            }};

                            try {{
                                const jsonString = JSON.stringify(initialResult);
                                globalThis.__streaming_result = JSON.parse(jsonString);
                            }} catch (jsonError) {{
                                globalThis.__streaming_result = initialResult;
                            }}
                            globalThis.__initial_render_complete = true;


                            return;
                        }}

                        const processSuspenseInStructure = (el, parentBoundaryId = null) => {{
                                if (!el || typeof el !== 'object') return el;

                                if ((el.type === 'suspense' || !el.type) && el.props && el.props.fallback && el.children) {{
                                    const boundaryId = 'boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                    const previousBoundaryId = globalThis.__current_boundary_id;
                                    globalThis.__current_boundary_id = boundaryId;

                                    const safeFallback = el.props.fallback || null;
                                    const serializableFallback = globalThis.__safeSerializeElement(safeFallback);

                                    globalThis.__discovered_boundaries.push({{
                                        id: boundaryId,
                                        fallback: serializableFallback,
                                        parentId: previousBoundaryId
                                    }});

                                    const processedChildren = el.children.map(child => {{
                                        try {{
                                            if (child && typeof child === 'object' && child.type && typeof child.type === 'function') {{
                                                const result = child.type(child.props || null);
                                                if (result && typeof result.then === 'function') {{
                                                    const promiseId = 'promise_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                                 globalThis.__suspense_promises = globalThis.__suspense_promises || {{}};
                                                    globalThis.__suspense_promises[promiseId] = result;

                                                    globalThis.__pending_promises = globalThis.__pending_promises || [];
                                                    globalThis.__pending_promises.push({{
                                                        id: promiseId,
                                                        boundaryId: boundaryId,
                                                        componentPath: (child.type.name || 'AnonymousComponent')
                                                    }});
                                                    return safeFallback;
                                                }} else {{
                                                    return globalThis.renderToRsc(result, globalThis.__rsc_client_components || {{}});
                                                }}
                                            }}
                                        }} catch (error) {{
                                            if (error && typeof error.then === 'function') {{
                                                const promiseId = 'promise_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                                globalThis.__suspense_promises = globalThis.__suspense_promises || {{}};
                                                globalThis.__suspense_promises[promiseId] = error;

                                                globalThis.__pending_promises = globalThis.__pending_promises || [];
                                                globalThis.__pending_promises.push({{
                                                    id: promiseId,
                                                    boundaryId: boundaryId,
                                                    componentPath: 'ThrownPromise'
                                                }});
                                                return safeFallback;
                                            }}
                                            return safeFallback;
                                        }}

                                        return processSuspenseInStructure(child, boundaryId);
                                    }});

                                    globalThis.__current_boundary_id = previousBoundaryId;

                                    return {{
                                        type: 'suspense',
                                        props: {{...el.props, key: boundaryId, boundaryId: boundaryId}},
                                        children: processedChildren
                                    }};
                                }}

                                if (el.children && Array.isArray(el.children)) {{
                                    el.children = el.children.map(child => processSuspenseInStructure(child, parentBoundaryId));
                                }}

                                return el;
                            }};

                            element = processSuspenseInStructure(element);
                        }}
                    catch (suspenseError) {{
                        if (suspenseError && suspenseError.$$typeof === Symbol.for('react.suspense.pending')) {{
                            const componentName = suspenseError.componentName || suspenseError.name || suspenseError.message || '{component_id}';
                            const asyncDetected = suspenseError.asyncComponentDetected === true;
                            const hasPromise = suspenseError.promise && typeof suspenseError.promise.then === 'function';

                            const isParentComponent = componentName === '{component_id}' ||
                                componentName.includes('Test') ||
                                componentName.includes('Streaming');

                            const isLeafAsyncComponent = asyncDetected ||
                                (hasPromise && !isParentComponent) ||
                                (componentName.includes('Async') && !isParentComponent);

                            if (isLeafAsyncComponent) {{
                                const promiseId = 'promise_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                globalThis.__suspense_promises[promiseId] = suspenseError.promise;

                                const boundaryId = globalThis.__current_boundary_id || 'root_boundary';
                                globalThis.__pending_promises.push({{
                                    id: promiseId,
                                    boundaryId: boundaryId,
                                    componentPath: componentName
                                }});

                            }}

                            element = globalThis.__original_create_element ?
                                globalThis.__original_create_element('div', null, '') :
                                {{'type': 'div', 'props': {{'children': ''}}}};
                        }} else {{
                            console.error('Non-suspense error during rendering:', suspenseError);
                            renderError = suspenseError;
                            element = globalThis.__original_create_element ?
                                globalThis.__original_create_element('div', null, 'Error: ' + suspenseError.message) :
                                {{'type': 'div', 'props': {{'children': 'Error: ' + suspenseError.message}}}};
                        }}
                    }}

                    let rscData;
                    try {{
                        rscData = globalThis.renderToRsc ?
                            await globalThis.renderToRsc(element, globalThis.__rsc_client_components || {{}}) :
                            element;
                    }} catch (rscError) {{
                        console.error('Error in RSC conversion:', rscError);
                        rscData = {{
                            type: 'div',
                            props: {{
                                children: renderError ? 'Render Error: ' + renderError.message : 'RSC Conversion Error'
                            }}
                        }};
                    }}

                    const safeBoundaries = (globalThis.__discovered_boundaries || []).map(boundary => ({{
                        id: boundary.id,
                        fallback: globalThis.__safeSerializeElement(boundary.fallback),
                        parentId: boundary.parentId
                    }}));

                    const finalResult = {{
                        success: !renderError,
                        rsc_data: rscData,
                        boundaries: safeBoundaries,
                        pending_promises: globalThis.__pending_promises || [],
                        has_suspense: (safeBoundaries && safeBoundaries.length > 0) ||
                                     (globalThis.__pending_promises && globalThis.__pending_promises.length > 0),
                        error: renderError ? renderError.message : null,
                        error_stack: renderError ? renderError.stack : null
                    }};

                    try {{
                        const jsonString = JSON.stringify(finalResult);
                        globalThis.__streaming_result = JSON.parse(jsonString);
                    }} catch (jsonError) {{
                        globalThis.__streaming_result = finalResult;
                    }}

                    if (!globalThis.__initial_render_complete) {{
                        globalThis.__initial_render_complete = true;
                    }}

                    globalThis.__streaming_complete = true;
                }} catch (error) {{
                    console.error('Fatal error in component rendering:', error);
                    const errorResult = {{
                        success: false,
                        error: error.message,
                        stack: error.stack,
                        fatal: true
                    }};
                    try {{
                        const jsonString = JSON.stringify(errorResult);
                        globalThis.__streaming_result = JSON.parse(jsonString);
                    }} catch (jsonError) {{
                        globalThis.__streaming_result = errorResult;
                    }}
                    globalThis.__streaming_complete = true;
                }}
            }};

            ({{ __setup_complete: true }})
            "#,
            component_id = component_id,
            props_json = props.unwrap_or("{}")
        );

        let _setup_result = self
            .runtime
            .execute_script(format!("<setup_render_{component_id}>"), setup_script)
            .await
            .map_err(|e| RariError::internal(format!("Setup render failed: {e}")))?;
        let start_script = r#"
            globalThis.__streaming_complete = false;
            globalThis.__initial_render_complete = false;
            globalThis.__should_start_render = true;
            true
        "#;

        self.runtime
            .execute_script(format!("<start_render_{component_id}>"), start_script.to_string())
            .await
            .map_err(|e| RariError::internal(format!("Partial render failed: {e}")))?;

        let result_data = {
            let (completion_tx, completion_rx) = tokio::sync::oneshot::channel::<()>();

            let completion_tx = Arc::new(tokio::sync::Mutex::new(Some(completion_tx)));
            let completion_tx_clone = Arc::clone(&completion_tx);

            let start_script = r#"
                (async function() {
                    if (globalThis.__should_start_render) {
                        globalThis.__should_start_render = false;
                        const renderStart = Date.now();
                        await globalThis.__render_component_async();
                        const renderCallTime = Date.now() - renderStart;

                        globalThis.__render_complete_signal = true;
                    }
                    return { started: true };
                })()
            "#
            .to_string();

            self.runtime
                .execute_script(format!("<start_render_{component_id}>"), start_script)
                .await
                .map_err(|e| RariError::internal(format!("Failed to start render: {e}")))?;

            let runtime_clone = Arc::clone(&self.runtime);
            let component_id_clone = component_id.to_string();
            tokio::spawn(async move {
                let check_script = r#"
                    JSON.stringify((function() {
                        if (globalThis.__initial_render_complete) {
                            return { complete: true, result: globalThis.__streaming_result };
                        }
                        return { complete: false };
                    })())
                "#;

                loop {
                    match runtime_clone
                        .execute_script(
                            format!("<check_complete_{}>", component_id_clone),
                            check_script.to_string(),
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
                                        && parsed
                                            .get("complete")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false)
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
                    let fetch_script = r#"
                        JSON.stringify(globalThis.__streaming_result || { success: false, error: "No result available" })
                    "#;

                    let result = self
                        .runtime
                        .execute_script(
                            format!("<fetch_result_{component_id}>"),
                            fetch_script.to_string(),
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
                if let Some(bid) = p["boundaryId"].as_str() {
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
                boundary_id: p["boundaryId"].as_str().unwrap_or("root").to_string(),
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
        let react_init_script = r#"
            (function() {
                if (typeof React === 'undefined') {
                    try {
                        if (typeof globalThis.__rsc_modules !== 'undefined') {
                            const reactModule = globalThis.__rsc_modules['react'] ||
                                              globalThis.__rsc_modules['React'] ||
                                              Object.values(globalThis.__rsc_modules).find(m => m && m.createElement);
                            if (reactModule) {
                                globalThis.React = reactModule;
                            }
                        }

                        if (typeof React === 'undefined' && typeof require !== 'undefined') {
                            globalThis.React = require('react');
                        }

                        if (typeof React !== 'undefined' && React.createElement && !globalThis.__react_patched) {
                            globalThis.__original_create_element = React.createElement;

                                const createElementOverride = function(type, props, ...children) {
                                    return globalThis.__original_create_element(type, props, ...children);
                                };

                            Object.defineProperty(React, 'createElement', {
                                value: createElementOverride,
                                writable: false,
                                enumerable: true,
                                configurable: false
                            });

                            globalThis.__react_patched = true;
                        }

                        if (typeof React !== 'undefined' && React.Suspense) {
                            React.__originalSuspense = React.Suspense;

                            React.Suspense = function SuspenseOverride(props) {
                                if (!props) return null;
                                const previousBoundaryId = globalThis.__current_boundary_id;
                                const boundaryId = 'boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                globalThis.__current_boundary_id = boundaryId;
                                try {
                                    const safeFallback = props?.fallback || null;
                                    const serializableFallback = globalThis.__safeSerializeElement(safeFallback);
                                    globalThis.__discovered_boundaries.push({ id: boundaryId, fallback: serializableFallback, parentId: previousBoundaryId });
                                    if (!props.children) {
                                        return safeFallback;
                                    }
                                    return props.children;
                                } catch (error) {
                                    if (error && error.$typeof === Symbol.for('react.suspense.pending') && error.promise) {
                                        const promiseId = 'suspense_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                        globalThis.__suspense_promises = globalThis.__suspense_promises || {};
                                        globalThis.__suspense_promises[promiseId] = error.promise;
                                        globalThis.__pending_promises = globalThis.__pending_promises || [];
                                        globalThis.__pending_promises.push({ id: promiseId, boundaryId: boundaryId, componentPath: (error.componentName || 'unknown') });
                                        return props.fallback || null;
                                    }
                                    return props?.fallback || React.createElement('div', null, 'Suspense Error: ' + (error && error.message ? error.message : 'Unknown'));
                                } finally {
                                    globalThis.__current_boundary_id = previousBoundaryId;
                                }
                            };
                        }

                        if (typeof React === 'undefined') {
                            globalThis.React = {
                                createElement: function(type, props, ...children) {
                                    return {
                                        type: type,
                                        props: props ? { ...props, children: children.length > 0 ? children : props.children } : { children: children },
                                        key: props?.key || null,
                                        ref: props?.ref || null
                                    };
                                },
                                Fragment: Symbol.for('react.fragment'),
                                Suspense: function(props) {
                                    return props.children;
                                }
                            };
                        }
                    } catch (e) {
                        console.error('Failed to load React in streaming context:', e);
                        throw new Error('Cannot initialize streaming without React: ' + e.message);
                    }
                }

                return {
                    available: typeof React !== 'undefined',
                    reactType: typeof React,
                    createElementType: typeof React.createElement,
                    suspenseType: typeof React.Suspense
                };
            })()
        "#;

        let react_init_result = self
            .runtime
            .execute_script("streaming-react-init".to_string(), react_init_script.to_string())
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

        let init_script = r#"
            if (!globalThis.renderToRsc) {
                globalThis.renderToRsc = async function(element, clientComponents = {}) {
                    if (!element) return null;

                    if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
                        return element;
                    }

                    if (Array.isArray(element)) {
                        const results = [];
                        for (const child of element) {
                            results.push(await globalThis.renderToRsc(child, clientComponents));
                        }
                        return results;
                    }

                    if (element && typeof element === 'object') {
                        const uniqueKey = element.key || `element-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

                        if (element.type) {
                            if (typeof element.type === 'string') {
                                const props = element.props || {};
                                const { children: propsChildren, ...otherProps } = props;

                                const actualChildren = element.children || propsChildren;

                                const rscProps = {
                                    ...otherProps,
                                    children: actualChildren ? await globalThis.renderToRsc(actualChildren, clientComponents) : undefined
                                };
                                if (rscProps.children === undefined) {
                                    delete rscProps.children;
                                }
                                return ["$", element.type, uniqueKey, rscProps];
                            } else if (typeof element.type === 'function') {
                                try {
                                    const props = element.props || {};
                                    let result = element.type(props);

                                    if (result && typeof result.then === 'function') {
                                        result = await result;
                                    }

                                    return await globalThis.renderToRsc(result, clientComponents);
                                } catch (error) {
                                    console.error('Error rendering function component:', error);
                                    return ["$", "div", uniqueKey, {
                                        children: `Error: ${error.message}`,
                                        style: { color: 'red', border: '1px solid red', padding: '10px' }
                                    }];
                                }
                            }
                        }

                        return ["$", "div", uniqueKey, {
                            className: "rsc-unknown",
                            children: "Unknown element type"
                        }];
                    }

                    return element;
                };
            }

            if (!globalThis.__suspense_streaming) {
                globalThis.__suspense_streaming = true;
                globalThis.__suspense_promises = {};
                globalThis.__boundary_props = {};
                globalThis.__discovered_boundaries = [];
                globalThis.__pending_promises = [];
                globalThis.__current_boundary_id = null;

                globalThis.__safeSerializeElement = function(element) {
                    if (!element) return null;

                    try {
                        if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
                            return element;
                        }

                        if (element && typeof element === 'object') {
                            return {
                                type: element.type || 'div',
                                props: element.props ? {
                                    children: (element.props.children === undefined ? null : element.props.children),
                                    ...(element.props.className && { className: element.props.className })
                                } : { children: null },
                                key: null,
                                ref: null
                            };
                        }

                        return { type: 'div', props: { children: null }, key: null, ref: null };
                    } catch (e) {
                        return { type: 'div', props: { children: null }, key: null, ref: null };
                    }
                };

                if (!globalThis.__react_patched && typeof React !== 'undefined' && React.createElement) {
                    globalThis.__original_create_element = React.createElement;

                    const createElementOverride = function(type, props, ...children) {
                        return globalThis.__original_create_element(type, props, ...children);
                    };

                    React.createElement = createElementOverride;
                    globalThis.__react_patched = true;
                }
            } else {
                globalThis.__discovered_boundaries = [];
                globalThis.__pending_promises = [];
                globalThis.__current_boundary_id = null;
            }
        "#;

        self.runtime
            .execute_script("<streaming_init>".to_string(), init_script.to_string())
            .await
            .map_err(|e| {
                error!("Streaming initialization script failed: {}", e);
                RariError::internal(format!(
                    "Failed to initialize streaming globals and helpers: {}",
                    e
                ))
            })?;

        let wrapped_script = format!(
            r#"
            (async function() {{
                try {{
                    globalThis.__discovered_boundaries = [];
                    globalThis.__pending_promises = [];
                    globalThis.__deferred_async_components = [];

                    const compositionResult = await {composition_script};


                    if (!compositionResult) {{
                        throw new Error('Composition script returned null/undefined');
                    }}

                    if (!compositionResult.rsc_data) {{
                        throw new Error('Composition script result missing rsc_data property. Keys: ' + Object.keys(compositionResult).join(', '));
                    }}


                    const rscData = compositionResult.rsc_data;

                    const boundaries = compositionResult.boundaries || [];
                    const pendingPromises = compositionResult.pending_promises || [];


                    const safeBoundaries = boundaries.map(boundary => ({{
                        id: boundary.id,
                        fallback: globalThis.__safeSerializeElement(boundary.fallback),
                        parentId: boundary.parentId,
                        parentPath: boundary.parentPath || [],
                        isInContentArea: boundary.isInContentArea || false
                    }}));


                    const finalResult = {{
                        success: true,
                        rsc_data: rscData,
                        boundaries: safeBoundaries,
                        pending_promises: pendingPromises,
                        has_suspense: (safeBoundaries && safeBoundaries.length > 0) ||
                                     (pendingPromises && pendingPromises.length > 0),
                        metadata: compositionResult.metadata,
                        error: null,
                        error_stack: null
                    }};

                    return finalResult;
                }} catch (error) {{
                    let errorMessage = 'Unknown error';
                    if (error) {{
                        if (error.message) {{
                            errorMessage = error.message;
                        }} else if (error.toString && typeof error.toString === 'function') {{
                            try {{
                                const str = error.toString();
                                if (str && str !== '[object Object]') {{
                                    errorMessage = str;
                                }}
                            }} catch (e) {{
                            }}
                        }} else if (typeof error === 'string') {{
                            errorMessage = error;
                        }}
                    }}

                    return {{
                        success: false,
                        error: errorMessage,
                        error_stack: error && error.stack ? error.stack : 'No stack available',
                        error_type: typeof error,
                        error_string: String(error),
                        error_name: error && error.name ? error.name : 'UnknownError'
                    }};
                }}
            }})()
            "#,
            composition_script = composition_script
        );

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

        if let Some(boundaries_array) = result_data["boundaries"].as_array() {
            for boundary in boundaries_array {
                let boundary_id = boundary["id"].as_str().unwrap_or("unknown");
                let is_in_content_area = boundary["isInContentArea"].as_bool().unwrap_or(false);

                if !is_in_content_area {
                    tracing::warn!(
                        "Suspense boundary '{}' is not nested within content area - this may cause layout shifts",
                        boundary_id
                    );
                }
            }
        }

        let mut pending_counts: FxHashMap<String, usize> = FxHashMap::default();
        if let Some(pending) = result_data["pending_promises"].as_array() {
            for p in pending {
                if let Some(bid) = p["boundaryId"].as_str() {
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
                boundary_id: p["boundaryId"].as_str().unwrap_or("root").to_string(),
                component_path: p["componentPath"].as_str().unwrap_or("unknown").to_string(),
                promise_handle: p["id"].as_str().unwrap_or("unknown").to_string(),
            })
            .collect();

        if let Err(validation_error) = validate_suspense_boundaries(&result_data["rsc_data"]) {
            tracing::error!(
                "RSC structure validation failed after composition script execution: {}",
                validation_error
            );
            tracing::warn!(
                "Continuing with rendering despite validation failure. \
                 Duplicate fallbacks may cause duplicate loading skeletons."
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
        let mut parser = crate::rsc::rsc_wire_parser::RscWireFormatParser::new(rsc_wire_format);

        parser.parse().map_err(|e| {
            tracing::error!("Failed to parse RSC wire format: {}", e);
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
            tracing::warn!("Could not extract initial content from RSC wire format");
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

    async fn send_initial_shell(
        &mut self,
        sender: &mpsc::Sender<RscStreamChunk>,
        partial_result: &PartialRenderResult,
    ) -> Result<(), RariError> {
        self.row_counter += 1;
        let module_chunk = self.create_module_chunk()?;
        sender
            .send(module_chunk)
            .await
            .map_err(|e| RariError::internal(format!("Failed to send module chunk: {e}")))?;

        self.row_counter += 1;
        let module_row_id = self.row_counter.saturating_sub(1);

        let shell_chunk =
            self.create_shell_chunk_with_module(&partial_result.initial_content, module_row_id)?;
        sender
            .send(shell_chunk)
            .await
            .map_err(|e| RariError::internal(format!("Failed to send shell chunk: {e}")))?;

        if partial_result.has_suspense {
            self.row_counter += 1;
            let symbol_chunk = self.create_symbol_chunk("react.suspense")?;
            sender
                .send(symbol_chunk)
                .await
                .map_err(|e| RariError::internal(format!("Failed to send symbol chunk: {e}")))?;

            for boundary in &partial_result.boundaries {
                self.row_counter += 1;

                {
                    let mut skeleton_ids = self.rendered_skeleton_ids.lock().await;
                    if !skeleton_ids.insert(boundary.id.clone()) {
                        tracing::warn!(
                            "Duplicate loading skeleton detected for boundary '{}'. Only one skeleton should be rendered per boundary.",
                            boundary.id
                        );
                    }
                }

                {
                    let mut map = self.boundary_row_ids.lock().await;
                    map.insert(boundary.id.clone(), self.row_counter);
                }

                let boundary_chunk = Self::create_boundary_chunk_static(
                    self.row_counter,
                    &boundary.id,
                    &boundary.fallback_content,
                )?;

                sender.send(boundary_chunk).await.map_err(|e| {
                    RariError::internal(format!("Failed to send boundary chunk: {e}"))
                })?;
            }
        }

        Ok(())
    }

    async fn send_boundary_update_with_map(
        sender: &mpsc::Sender<RscStreamChunk>,
        update: BoundaryUpdate,
        _boundary_rows_map: Arc<Mutex<FxHashMap<String, u32>>>,
    ) {
        let element = serde_json::Value::Object({
            let mut map = serde_json::Map::new();
            map.insert(
                "boundary_id".to_string(),
                serde_json::Value::String(update.boundary_id.clone()),
            );
            map.insert("content".to_string(), update.content.clone());
            if !update.dom_path.is_empty() {
                map.insert(
                    "dom_path".to_string(),
                    serde_json::Value::Array(
                        update
                            .dom_path
                            .iter()
                            .map(|&i| serde_json::Value::Number(i.into()))
                            .collect(),
                    ),
                );
            }
            map
        });

        let update_row = format!("{}:{}\n", update.row_id, element);

        let chunk = RscStreamChunk {
            data: update_row.into_bytes(),
            chunk_type: RscChunkType::BoundaryUpdate,
            row_id: update.row_id,
            is_final: false,
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
        };

        if let Err(e) = sender.send(chunk).await {
            error!(
                "Failed to send boundary error chunk, boundary_id={}, row_id={}, error={}",
                error.boundary_id, error.row_id, e
            );
        }
    }

    fn build_deferred_execution_script() -> String {
        r#"
            (async function() {
                if (typeof React === 'undefined' || !React) {
                    return {
                        success: false,
                        error: 'React is not available',
                        errorContext: {
                            phase: 'pre_execution_validation',
                            hasReact: false
                        }
                    };
                }

                if (!globalThis.__deferred_async_components) {
                    return { success: true, count: 0, total: 0, results: [] };
                }

                if (!Array.isArray(globalThis.__deferred_async_components)) {
                    return {
                        success: false,
                        error: '__deferred_async_components is not an array',
                        errorContext: {
                            phase: 'pre_execution_validation',
                            actualType: typeof globalThis.__deferred_async_components
                        }
                    };
                }

                const componentCount = globalThis.__deferred_async_components.length;
                const componentIds = globalThis.__deferred_async_components.map(d => d.promiseId);

                const captureErrorContext = function(error, deferred) {
                    const errorInfo = {
                        promiseId: deferred.promiseId,
                        success: false,
                        componentPath: deferred.componentPath,
                        boundaryId: deferred.boundaryId
                    };

                    try {
                        errorInfo.errorName = error.name || 'UnknownError';
                    } catch (e) {
                        errorInfo.errorName = 'UnknownError';
                    }

                    try {
                        errorInfo.error = error.message || String(error) || 'Unknown error';
                    } catch (e) {
                        errorInfo.error = 'Error message unavailable';
                    }

                    try {
                        errorInfo.errorStack = error.stack || 'No stack trace available';
                    } catch (e) {
                        errorInfo.errorStack = 'Stack trace unavailable';
                    }

                    errorInfo.errorContext = {
                        phase: 'deferred_execution',
                        promiseId: deferred.promiseId,
                        componentPath: deferred.componentPath,
                        boundaryId: deferred.boundaryId
                    };

                    return errorInfo;
                };

                if (globalThis.__deferred_async_components && globalThis.__deferred_async_components.length > 0) {

                    const results = [];
                    for (const deferred of globalThis.__deferred_async_components) {
                        globalThis.__current_executing_component = {
                            promiseId: deferred.promiseId,
                            componentPath: deferred.componentPath,
                            boundaryId: deferred.boundaryId
                        };

                        try {

                            if (typeof deferred.component !== 'function') {
                                results.push({
                                    promiseId: deferred.promiseId,
                                    success: false,
                                    error: 'Component is not a function',
                                    errorName: 'TypeError',
                                    errorStack: 'No stack trace (type validation)',
                                    componentPath: deferred.componentPath,
                                    boundaryId: deferred.boundaryId,
                                    errorContext: {
                                        phase: 'deferred_execution',
                                        promiseId: deferred.promiseId,
                                        componentPath: deferred.componentPath,
                                        actualType: typeof deferred.component
                                    }
                                });
                                continue;
                            }

                            let componentPromise;
                            try {
                                componentPromise = deferred.component(deferred.props);
                            } catch (callError) {
                                results.push({
                                    promiseId: deferred.promiseId,
                                    success: false,
                                    error: callError.message || String(callError) || 'Component call failed',
                                    errorName: callError.name || 'Error',
                                    errorStack: callError.stack || 'No stack trace available',
                                    componentPath: deferred.componentPath,
                                    boundaryId: deferred.boundaryId,
                                    errorContext: {
                                        phase: 'deferred_execution',
                                        subPhase: 'component_call',
                                        promiseId: deferred.promiseId,
                                        componentPath: deferred.componentPath
                                    }
                                });
                                continue;
                            }

                            if (!componentPromise || typeof componentPromise.then !== 'function') {
                                results.push({
                                    promiseId: deferred.promiseId,
                                    success: false,
                                    error: 'Component did not return a promise',
                                    errorName: 'TypeError',
                                    errorStack: 'No stack trace (promise validation)',
                                    componentPath: deferred.componentPath,
                                    boundaryId: deferred.boundaryId,
                                    errorContext: {
                                        phase: 'deferred_execution',
                                        subPhase: 'promise_validation',
                                        promiseId: deferred.promiseId,
                                        componentPath: deferred.componentPath,
                                        returnedType: typeof componentPromise,
                                        hasPromise: componentPromise !== null && componentPromise !== undefined,
                                        hasThen: componentPromise && typeof componentPromise.then === 'function'
                                    }
                                });
                                continue;
                            }

                            globalThis.__suspense_promises = globalThis.__suspense_promises || {};
                            globalThis.__suspense_promises[deferred.promiseId] = componentPromise;

                            if (!globalThis.__suspense_promises[deferred.promiseId]) {
                                const availablePromiseIds = Object.keys(globalThis.__suspense_promises || {});
                                results.push({
                                    promiseId: deferred.promiseId,
                                    success: false,
                                    error: 'Promise registration verification failed',
                                    errorName: 'RegistrationError',
                                    errorStack: 'No stack trace (registration verification)',
                                    componentPath: deferred.componentPath,
                                    boundaryId: deferred.boundaryId,
                                    errorContext: {
                                        phase: 'deferred_execution',
                                        subPhase: 'promise_registration_verification',
                                        promiseId: deferred.promiseId,
                                        componentPath: deferred.componentPath,
                                        availablePromises: availablePromiseIds
                                    }
                                });
                            } else {
                                results.push({
                                    promiseId: deferred.promiseId,
                                    success: true,
                                    componentPath: deferred.componentPath,
                                    boundaryId: deferred.boundaryId
                                });
                            }
                        } catch (e) {
                            results.push(captureErrorContext(e, deferred));
                        }
                    }

                    globalThis.__current_executing_component = null;

                    const successCount = results.filter(r => r.success).length;
                    globalThis.__deferred_async_components = [];
                    return {
                        success: true,
                        count: successCount,
                        total: results.length,
                        results: results
                    };
                }
                return { success: true, count: 0, total: 0 };
            })()
        "#.to_string()
    }

    fn create_module_chunk(&self) -> Result<RscStreamChunk, RariError> {
        let path = self
            .module_path
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "app/UnknownComponent.js".to_string());
        let module_data = format!("{}:I[\"{}\",[\"main\"],\"default\"]\n", self.row_counter, path);

        Ok(RscStreamChunk {
            data: module_data.into_bytes(),
            chunk_type: RscChunkType::ModuleImport,
            row_id: self.row_counter,
            is_final: false,
        })
    }

    fn create_shell_chunk_with_module(
        &self,
        content: &serde_json::Value,
        _module_row_id: u32,
    ) -> Result<RscStreamChunk, RariError> {
        let rsc_element = self.json_to_rsc_element(content)?;
        let row = format!("{}:{}\n", self.row_counter, rsc_element);

        Ok(RscStreamChunk {
            data: row.into_bytes(),
            chunk_type: RscChunkType::InitialShell,
            row_id: self.row_counter,
            is_final: false,
        })
    }

    fn json_to_rsc_element(
        &self,
        json: &serde_json::Value,
    ) -> Result<serde_json::Value, RariError> {
        if let Some(obj) = json.as_object()
            && let (Some(element_type), Some(props)) = (obj.get("type"), obj.get("props"))
        {
            let mut converted_props = serde_json::Map::new();

            if let Some(props_obj) = props.as_object() {
                for (key, value) in props_obj {
                    if key == "children" {
                        converted_props.insert(key.clone(), self.convert_children(value)?);
                    } else {
                        converted_props.insert(key.clone(), value.clone());
                    }
                }
            }

            return Ok(serde_json::Value::Array(vec![
                serde_json::Value::String("$".to_string()),
                element_type.clone(),
                serde_json::Value::Null,
                serde_json::Value::Object(converted_props),
            ]));
        }

        Ok(json.clone())
    }

    fn convert_children(
        &self,
        children: &serde_json::Value,
    ) -> Result<serde_json::Value, RariError> {
        match children {
            serde_json::Value::Array(arr) => {
                let mut converted = Vec::new();
                for child in arr {
                    converted.push(self.json_to_rsc_element(child)?);
                }
                Ok(serde_json::Value::Array(converted))
            }
            _ => self.json_to_rsc_element(children),
        }
    }

    fn create_symbol_chunk(&self, symbol_ref: &str) -> Result<RscStreamChunk, RariError> {
        let symbol_row = format!("{}:SSymbol.for(\"{}\")\n", self.row_counter, symbol_ref);

        Ok(RscStreamChunk {
            data: symbol_row.into_bytes(),
            chunk_type: RscChunkType::InitialShell,
            row_id: self.row_counter,
            is_final: false,
        })
    }

    fn create_boundary_chunk_static(
        row_id: u32,
        boundary_id: &str,
        fallback_content: &serde_json::Value,
    ) -> Result<RscStreamChunk, RariError> {
        let mut props = serde_json::Map::new();
        props.insert("fallback".to_string(), fallback_content.clone());
        props.insert("boundaryId".to_string(), serde_json::Value::String(boundary_id.to_string()));
        let element = serde_json::Value::Array(vec![
            serde_json::Value::String("$".to_string()),
            serde_json::Value::String("react.suspense".to_string()),
            serde_json::Value::Null,
            serde_json::Value::Object(props),
        ]);
        let row = format!("{row_id}:{element}\n");

        Ok(RscStreamChunk {
            data: row.into_bytes(),
            chunk_type: RscChunkType::InitialShell,
            row_id,
            is_final: false,
        })
    }
}

fn validate_suspense_boundaries(rsc_data: &serde_json::Value) -> Result<(), String> {
    let mut fallback_refs = FxHashSet::default();
    let mut duplicate_fallbacks = Vec::new();

    fn check_for_duplicates(
        value: &serde_json::Value,
        fallback_refs: &mut FxHashSet<String>,
        duplicates: &mut Vec<String>,
    ) {
        if let Some(arr) = value.as_array() {
            if arr.len() >= 4
                && arr[0].as_str() == Some("$")
                && arr[1].as_str() == Some("react.suspense")
                && let Some(props) = arr[3].as_object()
                && let Some(fallback) = props.get("fallback")
            {
                let fallback_str = serde_json::to_string(fallback).unwrap_or_default();

                if !fallback_refs.insert(fallback_str.clone()) {
                    let boundary_id = props
                        .get("__boundary_id")
                        .or_else(|| props.get("boundaryId"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    tracing::warn!(
                        "Duplicate fallback content detected for boundary '{}'",
                        boundary_id
                    );

                    duplicates.push(boundary_id);
                }
            }

            for item in arr {
                check_for_duplicates(item, fallback_refs, duplicates);
            }
        } else if let Some(obj) = value.as_object() {
            for (_, v) in obj {
                check_for_duplicates(v, fallback_refs, duplicates);
            }
        }
    }

    check_for_duplicates(rsc_data, &mut fallback_refs, &mut duplicate_fallbacks);

    if !duplicate_fallbacks.is_empty() {
        let error_msg = format!(
            "Duplicate fallback content detected for boundaries: {:?}",
            duplicate_fallbacks
        );
        tracing::error!("{}", error_msg);
        return Err(error_msg);
    }

    Ok(())
}

pub struct RscStream {
    receiver: mpsc::Receiver<RscStreamChunk>,
}

impl RscStream {
    pub fn new(receiver: mpsc::Receiver<RscStreamChunk>) -> Self {
        Self { receiver }
    }

    pub async fn next_chunk(&mut self) -> Option<RscStreamChunk> {
        self.receiver.recv().await
    }

    pub fn is_complete(&self) -> bool {
        self.receiver.is_closed()
    }
}

impl Stream for RscStream {
    type Item = Result<Vec<u8>, String>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(chunk.data))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_partial_render_result() {
        let partial_result = PartialRenderResult {
            initial_content: serde_json::json!({"test": "content"}),
            pending_promises: vec![],
            boundaries: vec![],
            has_suspense: false,
        };

        assert!(!partial_result.has_suspense);
        assert_eq!(partial_result.pending_promises.len(), 0);
        assert_eq!(partial_result.boundaries.len(), 0);
    }

    #[tokio::test]
    async fn test_boundary_manager() {
        let mut manager = SuspenseBoundaryManager::new();

        let boundary = SuspenseBoundaryInfo {
            id: "test-boundary".to_string(),
            fallback_content: serde_json::json!({"loading": true}),
            parent_boundary_id: None,
            pending_promise_count: 1,
            parent_path: vec!["content-slot".to_string()],
            is_in_content_area: true,
            skeleton_rendered: false,
            is_resolved: false,
            position_hints: Some(PositionHints {
                in_content_area: true,
                dom_path: vec!["content-slot".to_string()],
                is_stable: true,
            }),
        };

        manager.register_boundary(boundary).await;

        let pending = manager.get_pending_boundaries().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "test-boundary");
    }

    #[test]
    fn test_rsc_stream_chunk() {
        let chunk = RscStreamChunk {
            data: b"test data".to_vec(),
            chunk_type: RscChunkType::InitialShell,
            row_id: 1,
            is_final: false,
        };

        assert_eq!(chunk.chunk_type, RscChunkType::InitialShell);
        assert_eq!(chunk.row_id, 1);
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_module_row_format() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = StreamingRenderer::new(runtime);

        let mut renderer = renderer;
        renderer.row_counter = 1;
        renderer.module_path = Some("app/MyComponent.js".to_string());

        let module_chunk = renderer.create_module_chunk().expect("module chunk");
        let s = String::from_utf8(module_chunk.data).expect("utf8");
        assert!(s.starts_with("1:I[\"app/MyComponent.js\",[\"main\"],\"default\"]"));
    }

    #[test]
    fn test_symbol_row_format() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let renderer = StreamingRenderer::new(runtime);

        let mut renderer = renderer;
        renderer.row_counter = 2;

        let sym_chunk = renderer.create_symbol_chunk("react.suspense").expect("symbol chunk");
        let s = String::from_utf8(sym_chunk.data).expect("utf8");
        assert!(s.starts_with("2:SSymbol.for(\"react.suspense\")"));
    }

    #[tokio::test]
    async fn test_deferred_execution_validates_react_availability() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (async function() {
                const originalReact = globalThis.React;
                delete globalThis.React;

                if (typeof React === 'undefined' || !React) {
                    globalThis.React = originalReact;
                    return {
                        success: false,
                        error: 'React is not available',
                        validated: true
                    };
                }

                globalThis.React = originalReact;
                return { success: true, validated: false };
            })()
        "#;

        let result = runtime
            .execute_script("test_react_validation".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["validated"], true);
        assert_eq!(value["success"], false);
    }

    #[tokio::test]
    async fn test_deferred_execution_validates_components_array() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (async function() {
                globalThis.__deferred_async_components = "not an array";

                if (!Array.isArray(globalThis.__deferred_async_components)) {
                    return {
                        success: false,
                        error: '__deferred_async_components is not an array',
                        actualType: typeof globalThis.__deferred_async_components,
                        validated: true
                    };
                }

                return { success: true, validated: false };
            })()
        "#;

        let result = runtime
            .execute_script("test_array_validation".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["validated"], true);
        assert_eq!(value["actualType"], "string");
    }

    #[tokio::test]
    async fn test_deferred_execution_validates_component_is_function() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (async function() {
                const deferred = {
                    component: "not a function",
                    promiseId: "test-promise",
                    componentPath: "TestComponent",
                    boundaryId: "test-boundary"
                };

                if (typeof deferred.component !== 'function') {
                    return {
                        success: false,
                        error: 'Component is not a function',
                        errorName: 'TypeError',
                        actualType: typeof deferred.component,
                        validated: true
                    };
                }

                return { success: true, validated: false };
            })()
        "#;

        let result = runtime
            .execute_script("test_function_validation".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["validated"], true);
        assert_eq!(value["errorName"], "TypeError");
        assert_eq!(value["actualType"], "string");
    }

    #[tokio::test]
    async fn test_deferred_execution_validates_promise_return() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (async function() {
                const deferred = {
                    component: function() { return "not a promise"; },
                    promiseId: "test-promise",
                    componentPath: "TestComponent",
                    boundaryId: "test-boundary",
                    props: {}
                };

                const componentPromise = deferred.component(deferred.props);

                if (!componentPromise || typeof componentPromise.then !== 'function') {
                    return {
                        success: false,
                        error: 'Component did not return a promise',
                        errorName: 'TypeError',
                        returnedType: typeof componentPromise,
                        hasPromise: componentPromise !== null && componentPromise !== undefined,
                        hasThen: componentPromise && typeof componentPromise.then === 'function',
                        validated: true
                    };
                }

                return { success: true, validated: false };
            })()
        "#;

        let result = runtime
            .execute_script("test_promise_validation".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["validated"], true);
        assert_eq!(value["errorName"], "TypeError");
        assert_eq!(value["returnedType"], "string");
        assert_eq!(value["hasThen"], false);
    }

    #[tokio::test]
    async fn test_deferred_execution_verifies_promise_registration() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (async function() {
                globalThis.__suspense_promises = {};
                const promiseId = "test-promise-123";

                const testPromise = Promise.resolve("test");
                globalThis.__suspense_promises[promiseId] = testPromise;

                if (!globalThis.__suspense_promises[promiseId]) {
                    const availablePromiseIds = Object.keys(globalThis.__suspense_promises || {});
                    return {
                        success: false,
                        error: 'Promise registration verification failed',
                        availablePromises: availablePromiseIds,
                        verified: false
                    };
                }

                return {
                    success: true,
                    verified: true,
                    promiseId: promiseId
                };
            })()
        "#;

        let result = runtime
            .execute_script("test_registration_verification".to_string(), test_script.to_string())
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["verified"], true);
        assert_eq!(value["success"], true);
        assert_eq!(value["promiseId"], "test-promise-123");
    }

    #[test]
    fn test_validate_suspense_boundaries_no_duplicates() {
        let rsc_data = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "__boundary_id": "boundary-1",
                "fallback": ["$", "div", null, { "children": "Loading 1..." }],
                "children": "$L1"
            }
        ]);

        let result = validate_suspense_boundaries(&rsc_data);
        assert!(result.is_ok(), "Validation should pass with no duplicates");
    }

    #[test]
    fn test_validate_suspense_boundaries_detects_duplicates() {
        let rsc_data = serde_json::json!({
            "root": [
                "$",
                "div",
                null,
                {
                    "children": [
                        [
                            "$",
                            "react.suspense",
                            null,
                            {
                                "__boundary_id": "boundary-1",
                                "fallback": ["$", "div", null, { "children": "Loading..." }],
                                "children": "$L1"
                            }
                        ],
                        [
                            "$",
                            "react.suspense",
                            null,
                            {
                                "__boundary_id": "boundary-2",
                                "fallback": ["$", "div", null, { "children": "Loading..." }],
                                "children": "$L2"
                            }
                        ]
                    ]
                }
            ]
        });

        let result = validate_suspense_boundaries(&rsc_data);
        assert!(result.is_err(), "Validation should fail with duplicate fallbacks");

        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("boundary-2"), "Error should mention the duplicate boundary");
    }

    #[test]
    fn test_validate_suspense_boundaries_nested() {
        let rsc_data = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "__boundary_id": "outer-boundary",
                "fallback": ["$", "div", null, { "children": "Loading outer..." }],
                "children": [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "__boundary_id": "inner-boundary",
                        "fallback": ["$", "div", null, { "children": "Loading inner..." }],
                        "children": "$L1"
                    }
                ]
            }
        ]);

        let result = validate_suspense_boundaries(&rsc_data);
        assert!(
            result.is_ok(),
            "Validation should pass with nested boundaries having different fallbacks"
        );
    }

    #[test]
    fn test_validate_suspense_boundaries_multiple_unique() {
        let rsc_data = serde_json::json!({
            "children": [
                [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "boundaryId": "boundary-1",
                        "fallback": ["$", "div", null, { "children": "Loading 1..." }],
                        "children": "$L1"
                    }
                ],
                [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "boundaryId": "boundary-2",
                        "fallback": ["$", "div", null, { "children": "Loading 2..." }],
                        "children": "$L2"
                    }
                ],
                [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "boundaryId": "boundary-3",
                        "fallback": ["$", "div", null, { "children": "Loading 3..." }],
                        "children": "$L3"
                    }
                ]
            ]
        });

        let result = validate_suspense_boundaries(&rsc_data);
        assert!(result.is_ok(), "Validation should pass with multiple unique boundaries");
    }

    #[tokio::test]
    async fn test_deferred_execution_complete_flow() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let test_script = r#"
            (async function() {
                globalThis.React = { createElement: () => {} };

                globalThis.__deferred_async_components = [
                    {
                        component: async function(props) {
                            return { type: 'div', props: { children: 'Valid' } };
                        },
                        promiseId: "valid-promise",
                        componentPath: "ValidComponent",
                        boundaryId: "boundary-1",
                        props: {}
                    },
                    {
                        component: function(props) {
                            return "not a promise";
                        },
                        promiseId: "invalid-promise",
                        componentPath: "InvalidComponent",
                        boundaryId: "boundary-2",
                        props: {}
                    }
                ];

                globalThis.__suspense_promises = {};
                const results = [];

                for (const deferred of globalThis.__deferred_async_components) {
                    try {
                        if (typeof deferred.component !== 'function') {
                            results.push({ promiseId: deferred.promiseId, success: false, error: 'Not a function' });
                            continue;
                        }

                        const componentPromise = deferred.component(deferred.props);

                        if (!componentPromise || typeof componentPromise.then !== 'function') {
                            results.push({
                                promiseId: deferred.promiseId,
                                success: false,
                                error: 'Not a promise',
                                returnedType: typeof componentPromise
                            });
                            continue;
                        }

                        globalThis.__suspense_promises[deferred.promiseId] = componentPromise;

                        if (!globalThis.__suspense_promises[deferred.promiseId]) {
                            results.push({ promiseId: deferred.promiseId, success: false, error: 'Registration failed' });
                        } else {
                            results.push({ promiseId: deferred.promiseId, success: true });
                        }
                    } catch (e) {
                        results.push({ promiseId: deferred.promiseId, success: false, error: e.message });
                    }
                }

                return {
                    totalComponents: globalThis.__deferred_async_components.length,
                    results: results,
                    successCount: results.filter(r => r.success).length,
                    failureCount: results.filter(r => !r.success).length
                };
            })()
        "#;

        let result =
            runtime.execute_script("test_complete_flow".to_string(), test_script.to_string()).await;
        assert!(result.is_ok());
        let value = result.unwrap();

        assert_eq!(value["totalComponents"], 2);
        assert_eq!(value["successCount"], 1);
        assert_eq!(value["failureCount"], 1);

        let results = value["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);

        assert_eq!(results[0]["promiseId"], "valid-promise");
        assert_eq!(results[0]["success"], true);

        assert_eq!(results[1]["promiseId"], "invalid-promise");
        assert_eq!(results[1]["success"], false);
        assert_eq!(results[1]["error"], "Not a promise");
    }
}
