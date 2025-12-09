use crate::server::utils::component_utils::{
    get_dist_path_for_component, wrap_server_action_module,
};
use crate::server::utils::http_utils::get_memory_usage;
use crate::server::{
    RegisterClientRequest, RegisterRequest, RenderRequest, ServerState, StatusResponse,
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Response},
};
use rustc_hash::FxHashMap;
use serde_json::Value;
use tracing::{debug, error, info, warn};

const RSC_CONTENT_TYPE: &str = "text/x-component";
const CHUNKED_ENCODING: &str = "chunked";

#[axum::debug_handler]
pub async fn stream_component(
    State(state): State<ServerState>,
    Json(request): Json<RenderRequest>,
) -> Result<Response, StatusCode> {
    debug!("Streaming component: {}", request.component_id);

    let props_str = request.props.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let stream_result = {
        let renderer = state.renderer.lock().await;
        renderer.render_with_streaming(&request.component_id, props_str.as_deref()).await
    };

    match stream_result {
        Ok(mut rsc_stream) => {
            debug!("Successfully created true streaming for component: {}", request.component_id);

            let byte_stream = async_stream::stream! {
                while let Some(chunk) = rsc_stream.next_chunk().await {
                    yield Ok::<Vec<u8>, std::io::Error>(chunk.data);
                }
            };

            let body = Body::from_stream(byte_stream);

            let cache_control = state.config.get_cache_control_for_route("/api/rsc/stream");

            Ok(Response::builder()
                .header("content-type", RSC_CONTENT_TYPE)
                .header("cache-control", cache_control)
                .header("transfer-encoding", CHUNKED_ENCODING)
                .body(body)
                .expect("Valid streaming response"))
        }
        Err(e) => {
            error!("Failed to create true streaming for component {}: {}", request.component_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[axum::debug_handler]
pub async fn register_component(
    State(state): State<ServerState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    let is_app_router = request.component_id.starts_with("app/");

    if is_app_router {
        debug!("Registering app router component: {}", request.component_id);
    } else {
        debug!("Registering component: {}", request.component_id);
    }

    if let Some(cache_config) = &request.cache_config {
        let mut cache_configs = state.component_cache_configs.write().await;
        cache_configs.insert(request.component_id.clone(), cache_config.clone());
        debug!("Stored cache config for component: {}", request.component_id);
    }

    let result = {
        let mut renderer = state.renderer.lock().await;
        renderer.register_component(&request.component_id, &request.component_code).await
    };

    match result {
        Ok(_) => {
            if is_app_router {
                info!("Successfully registered app router component: {}", request.component_id);
            } else {
                info!("Successfully registered component: {}", request.component_id);
            }

            let renderer = state.renderer.lock().await;
            let is_client =
                renderer.serializer.lock().is_client_component_registered(&request.component_id);

            if is_client {
                let mark_script = format!(
                    r#"(function() {{
                        const comp = globalThis["{}"];
                        if (comp && typeof comp === 'function') {{
                            comp.__isClientComponent = true;
                            comp.__clientComponentId = "{}";
                        }}
                    }})()"#,
                    request.component_id, request.component_id
                );

                if let Err(e) = renderer
                    .runtime
                    .execute_script(
                        format!("mark_client_{}.js", request.component_id.replace('/', "_")),
                        mark_script,
                    )
                    .await
                {
                    error!("Failed to mark {} as client component: {}", request.component_id, e);
                } else {
                    debug!("Marked {} as client component", request.component_id);
                }
            }

            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": true,
                "component_id": request.component_id
            })))
        }
        Err(e) => {
            error!("Failed to register component {}: {}", request.component_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[axum::debug_handler]
pub async fn register_client_component(
    State(state): State<ServerState>,
    Json(request): Json<RegisterClientRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!(
        "Registering client component: {} from path: {} with export: {}",
        request.component_id, request.file_path, request.export_name
    );

    {
        let renderer = state.renderer.lock().await;
        renderer.register_client_component(
            &request.component_id,
            &request.file_path,
            &request.export_name,
        );

        let serializer = renderer.serializer.lock();
        let is_registered = serializer.is_client_component_registered(&request.component_id);
        info!("Client component {} registration status: {}", request.component_id, is_registered);
    }

    info!("Successfully registered client component: {}", request.component_id);
    #[allow(clippy::disallowed_methods)]
    Ok(Json(serde_json::json!({
        "success": true,
        "component_id": request.component_id
    })))
}

#[axum::debug_handler]
pub async fn list_components(State(state): State<ServerState>) -> Json<Value> {
    let components = {
        let renderer = state.renderer.lock().await;
        renderer.list_components()
    };

    #[allow(clippy::disallowed_methods)]
    Json(serde_json::json!({
        "success": true,
        "components": components
    }))
}

#[axum::debug_handler]
pub async fn health_check() -> Json<Value> {
    #[allow(clippy::disallowed_methods)]
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[axum::debug_handler]
pub async fn rsc_status_handler() -> Json<Value> {
    #[allow(clippy::disallowed_methods)]
    Json(serde_json::json!({
        "status": "ready"
    }))
}

#[axum::debug_handler]
pub async fn rsc_render_handler(
    State(state): State<ServerState>,
    Path(component_id): Path<String>,
    Query(params): Query<FxHashMap<String, String>>,
) -> Result<Response, StatusCode> {
    let start_time = std::time::Instant::now();

    state.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    debug!("RSC rendering component: {}", component_id);

    let props: Option<Value> = params.get("props").and_then(|p| {
        if p.trim().is_empty() || p == "{}" { None } else { serde_json::from_str(p).ok() }
    });

    let props_str = props.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let result = {
        let mut renderer = state.renderer.lock().await;
        renderer.render_to_rsc_format(&component_id, props_str.as_deref()).await
    };

    match result {
        Ok(rsc_data) => {
            let _render_time = start_time.elapsed().as_millis() as u64;

            let cache_configs = state.component_cache_configs.read().await;
            let mut response_builder = Response::builder().header("content-type", RSC_CONTENT_TYPE);

            if let Some(component_cache_config) = cache_configs.get(&component_id) {
                for (key, value) in component_cache_config {
                    response_builder = response_builder.header(key.to_lowercase(), value);
                }
                debug!("Applied component-specific cache headers for: {}", component_id);
            } else {
                let cache_control = state
                    .config
                    .get_cache_control_for_route(&format!("/rsc/render/{}", component_id));
                response_builder = response_builder.header("cache-control", cache_control);
            }

            Ok(response_builder.body(Body::from(rsc_data)).expect("Valid RSC response"))
        }
        Err(e) => {
            let _render_time = start_time.elapsed().as_millis() as u64;

            error!("Failed to render RSC component {}: {}", component_id, e);

            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[axum::debug_handler]
pub async fn server_status(State(state): State<ServerState>) -> Json<StatusResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let request_count = state.request_count.load(std::sync::atomic::Ordering::Relaxed);
    let components = {
        let renderer = state.renderer.lock().await;
        renderer.list_components()
    };

    Json(StatusResponse {
        status: "running".to_string(),
        mode: state.config.mode.to_string(),
        uptime_seconds: uptime,
        request_count,
        components_registered: components.len(),
        memory_usage: get_memory_usage(),
    })
}

pub async fn reload_component_from_dist(
    state: &ServerState,
    file_path: &str,
    component_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::server::utils::path_validation::validate_component_path;

    if let Err(e) = validate_component_path(file_path) {
        error!(
            component_id = component_id,
            file_path = file_path,
            error = %e,
            "Component path validation failed"
        );
        return Err(format!("Path validation error: {}", e).into());
    }

    let dist_path = match get_dist_path_for_component(file_path) {
        Ok(path) => path,
        Err(e) => {
            error!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to resolve dist path for component"
            );
            return Err(format!("Path resolution error: {}", e).into());
        }
    };

    debug!("Reloading component {} from dist path: {}", component_id, dist_path.display());

    if !dist_path.exists() {
        warn!(
            component_id = component_id,
            dist_path = %dist_path.display(),
            source_path = file_path,
            "Dist file does not exist, Vite may not have finished building"
        );
        return Err(format!(
            "Dist file not found: {}. Vite may not have finished building yet. Last known good version will be preserved.",
            dist_path.display()
        )
        .into());
    }

    debug!("Found dist file at: {}", dist_path.display());

    let dist_code = match tokio::fs::read_to_string(&dist_path).await {
        Ok(code) => code,
        Err(e) => {
            error!(
                component_id = component_id,
                dist_path = %dist_path.display(),
                error = %e,
                error_kind = ?e.kind(),
                "Failed to read dist file. Last known good version will be preserved."
            );
            return Err(format!(
                "Failed to read dist file {}: {}. Last known good version will be preserved.",
                dist_path.display(),
                e
            )
            .into());
        }
    };

    debug!("Read {} bytes from dist file", dist_code.len());

    let wrapped_code = wrap_server_action_module(&dist_code, component_id);

    let renderer = state.renderer.lock().await;

    let execution_result = renderer
        .runtime
        .execute_script(
            format!("hmr_reload_{}.js", component_id.replace('/', "_")),
            wrapped_code.clone(),
        )
        .await;

    if let Err(e) = execution_result {
        error!(
            component_id = component_id,
            dist_path = %dist_path.display(),
            error = %e,
            code_length = dist_code.len(),
            "Failed to execute component code during reload. Last known good version will be preserved."
        );
        return Err(format!(
            "Failed to execute component code: {}. Last known good version will be preserved.",
            e
        )
        .into());
    }

    let verification_script = format!(
        r#"(function() {{
            const expectedKey = '{}';
            const exists = typeof globalThis[expectedKey] !== 'undefined';
            const type = typeof globalThis[expectedKey];

            const allKeys = Object.keys(globalThis).filter(key =>
                typeof globalThis[key] === 'function' ||
                typeof globalThis[key] === 'object'
            );

            return {{
                success: exists,
                componentId: expectedKey,
                type: type,
                hasDefault: exists,
                expectedKey: expectedKey,
                actualKeys: allKeys
            }};
        }})()"#,
        component_id
    );

    let result_json = match renderer
        .runtime
        .execute_script(
            format!("verify_{}.js", component_id.replace('/', "_")),
            verification_script,
        )
        .await
    {
        Ok(json) => json,
        Err(e) => {
            error!(
                component_id = component_id,
                error = %e,
                "Failed to execute verification script. Last known good version will be preserved."
            );
            return Err(format!(
                "Failed to verify component reload: {}. Last known good version will be preserved.",
                e
            )
            .into());
        }
    };

    if let Some(success) = result_json.get("success").and_then(|v| v.as_bool()) {
        if success {
            info!(
                component_id = component_id,
                dist_path = %dist_path.display(),
                "Component successfully reloaded from dist"
            );

            if let Some(comp_type) = result_json.get("type").and_then(|v| v.as_str()) {
                debug!("Component type: {}", comp_type);
            }

            Ok(())
        } else {
            let actual_keys = result_json
                .get("actualKeys")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_else(|| "unknown".to_string());

            let expected_key =
                result_json.get("expectedKey").and_then(|v| v.as_str()).unwrap_or(component_id);

            error!(
                component_id = component_id,
                expected_key = expected_key,
                actual_keys = actual_keys,
                verification_result = ?result_json,
                "Component not found in globalThis after reload. Expected key '{}' not found. Available keys: [{}]. Last known good version will be preserved.",
                expected_key,
                actual_keys
            );
            Err(format!(
                "Component '{}' not found in globalThis after reload. Expected key '{}' but found keys: [{}]. Last known good version will be preserved.",
                component_id,
                expected_key,
                actual_keys
            )
            .into())
        }
    } else {
        error!(
            component_id = component_id,
            verification_result = ?result_json,
            "Invalid verification result format. Last known good version will be preserved."
        );
        Err("Invalid verification result format. Last known good version will be preserved.".into())
    }
}

pub async fn immediate_component_reregistration(
    state: &ServerState,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::server::utils::path_validation::validate_component_path;

    if let Err(e) = validate_component_path(file_path) {
        error!(
            file_path = file_path,
            error = %e,
            "Component path validation failed"
        );
        return Err(format!("Path validation error: {}", e).into());
    }

    let path = std::path::Path::new(file_path);
    let component_name =
        path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("UnknownComponent");

    debug!(
        component_name = component_name,
        file_path = file_path,
        "Starting immediate component re-registration"
    );

    {
        let mut renderer = state.renderer.lock().await;
        renderer.clear_script_cache();

        if let Err(e) = renderer.clear_component_module_cache(component_name).await {
            warn!(
                component_name = component_name,
                error = %e,
                "Failed to clear component module cache, continuing anyway"
            );
        }
    }

    let content = match tokio::fs::read_to_string(file_path).await {
        Ok(c) => c,
        Err(e) => {
            error!(
                component_name = component_name,
                file_path = file_path,
                error = %e,
                error_kind = ?e.kind(),
                "Failed to read source file for immediate re-registration"
            );
            return Err(format!("Failed to read source file: {}", e).into());
        }
    };

    {
        if let Err(e) =
            state.renderer.lock().await.register_component(component_name, &content).await
        {
            error!(
                component_name = component_name,
                error = %e,
                "Failed to register component directly, preserving last known good version"
            );
            Err(format!("Failed to register component: {}", e).into())
        } else {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let mut renderer = state.renderer.lock().await;
            if let Err(e) = renderer.clear_component_module_cache(component_name).await {
                warn!(
                    component_name = component_name,
                    error = %e,
                    "Failed to clear component module cache after initial registration"
                );
            }
            drop(renderer);

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            if let Err(e) =
                state.renderer.lock().await.register_component(component_name, &content).await
            {
                error!(
                    component_name = component_name,
                    error = %e,
                    "Failed to re-register component after cache clear, preserving last known good version"
                );
                return Err(
                    format!("Failed to re-register component after cache clear: {}", e).into()
                );
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let renderer = state.renderer.lock().await;

            let verification_attempts = 3;
            for attempt in 1..=verification_attempts {
                let _unique_id = format!(
                    "{}_{}",
                    component_name,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
                let verification_script = format!(
                    r#"
                    (function() {{
                        const componentName = "{}";
                        const hashedComponentName = "Component_" + componentName;
                        const componentExists = (
                            typeof globalThis[componentName] === 'function' ||
                            typeof globalThis[hashedComponentName] === 'function' ||
                            (globalThis.__rsc_modules && globalThis.__rsc_modules[componentName] &&
                             (typeof globalThis.__rsc_modules[componentName].default === 'function' ||
                              typeof Object.values(globalThis.__rsc_modules[componentName])[0] === 'function'))
                        );

                        const debugInfo = {{
                            globalExists: typeof globalThis[componentName],
                            moduleRegistryExists: !!globalThis.__rsc_modules,
                            moduleExists: globalThis.__rsc_modules ? !!globalThis.__rsc_modules[componentName] : false,
                            moduleDefaultExists: globalThis.__rsc_modules && globalThis.__rsc_modules[componentName] ? typeof globalThis.__rsc_modules[componentName].default : 'module_not_found',
                            moduleFirstExportExists: globalThis.__rsc_modules && globalThis.__rsc_modules[componentName] ? typeof Object.values(globalThis.__rsc_modules[componentName])[0] : 'module_not_found',
                            hashedExists: typeof globalThis[hashedComponentName],
                            availableGlobals: Object.keys(globalThis).filter(k => typeof globalThis[k] === 'function' && k.match(/^[A-Z]/)).slice(0, 20),
                            moduleKeys: globalThis.__rsc_modules ? Object.keys(globalThis.__rsc_modules).slice(0, 20) : []
                        }};

                        return {{ success: componentExists, componentName: componentName, debugInfo: debugInfo }};
                    }})()
                    "#,
                    component_name
                );

                match renderer
                    .runtime
                    .execute_script(
                        format!("hmr_verify_{}.js", component_name),
                        verification_script,
                    )
                    .await
                {
                    Ok(result) => {
                        if let Some(success) = result.get("success").and_then(|v| v.as_bool())
                            && success
                        {
                            return Ok(());
                        }

                        if attempt == verification_attempts {
                            warn!(
                                component_name = component_name,
                                attempts = verification_attempts,
                                "Component verification failed after all attempts, but component may still be available"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            component_name = component_name,
                            attempt = attempt,
                            error = %e,
                            "Component verification script execution failed"
                        );
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            }

            warn!(
                component_name = component_name,
                attempts = verification_attempts,
                "Component verification failed after all attempts, but component may still be available"
            );
            Ok(())
        }
    }
}
