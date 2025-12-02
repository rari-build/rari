use crate::server::config::Config;
use crate::server::handlers::rsc_handlers::{
    immediate_component_reregistration, reload_component_from_dist,
};
use crate::server::utils::component_utils::extract_component_id;
use crate::server::{
    HmrRegisterRequest, ReloadComponentRequest, ReloadComponentResponse, ServerState,
};
use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, error, info, warn};

#[derive(Debug, Deserialize)]
pub struct HmrInvalidateRequest {
    #[serde(rename = "componentId")]
    component_id: String,
    #[serde(rename = "filePath")]
    #[allow(dead_code)]
    file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HmrInvalidateApiRouteRequest {
    #[serde(rename = "filePath")]
    file_path: String,
}

#[derive(Debug, Deserialize)]
pub struct HmrReloadRequest {
    #[serde(rename = "componentId")]
    component_id: String,
    #[serde(rename = "filePath")]
    file_path: String,
}

#[axum::debug_handler]
pub async fn hmr_register_component(
    State(state): State<ServerState>,
    Json(request): Json<HmrRegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    let file_path = request.file_path.clone();

    let component_id = match extract_component_id(&file_path) {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to extract component ID from {}: {}", file_path, e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    info!("HMR register request for component: {} from file: {}", component_id, file_path);
    debug!("Extracted component ID: {} from path: {}", component_id, file_path);

    let path = std::path::Path::new(&file_path);

    {
        let renderer = state.renderer.lock().await;
        let mut registry = renderer.component_registry.lock();
        registry.mark_module_stale(&component_id);
        debug!("Marked component {} as stale", component_id);
    }

    let reload_result = reload_component_from_dist(&state, &file_path, &component_id).await;

    let mut reload_error_details: Option<serde_json::Value> = None;

    match &reload_result {
        Ok(_) => {
            info!(
                component_id = component_id,
                file_path = file_path,
                "Successfully reloaded component from dist"
            );
        }
        Err(e) => {
            error!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to reload component from dist, preserving last known good version"
            );

            #[allow(clippy::disallowed_methods)]
            {
                reload_error_details = Some(serde_json::json!({
                    "stage": "dist_reload",
                    "message": e.to_string(),
                    "component_id": component_id,
                    "file_path": file_path,
                    "preserved_last_good": true
                }));
            }
        }
    }

    if reload_result.is_err() {
        debug!(
            component_id = component_id,
            "Attempting fallback re-registration after dist reload failure"
        );

        if let Err(e) = immediate_component_reregistration(&state, &file_path).await {
            error!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to immediately re-register component, preserving last known good version"
            );

            #[allow(clippy::disallowed_methods)]
            return Ok(Json(serde_json::json!({
                "success": false,
                "file_path": request.file_path,
                "component_id": component_id,
                "reloaded": false,
                "preserved_last_good": true,
                "error": {
                    "stage": "fallback_registration",
                    "message": e.to_string(),
                    "previous_error": reload_error_details,
                    "suggestion": "Component reload failed. Last known good version is still available. Consider checking for syntax errors or manual page refresh."
                }
            })));
        } else {
            info!(component_id = component_id, "Fallback re-registration succeeded");
        }
    }

    let mut reloaded = reload_result.is_ok();
    let mut module_reload_error: Option<String> = None;

    if state.config.hmr_reload_enabled() {
        debug!("HMR reload is enabering debounced module reload for {}", component_id);

        match state.module_reload_manager.reload_module_debounced(&component_id, path).await {
            Ok(()) => {
                debug!("Scheduled debounced reload for component: {}", component_id);
                reloaded = true;
            }
            Err(e) => {
                error!(
                    component_id = component_id,
                    error = %e,
                    "Failed to schedule module reload, preserving last known good version"
                );
                module_reload_error = Some(e.to_string());
            }
        }
    } else {
        debug!("HMR reload is disabled, skipping module reload");
    }

    #[allow(clippy::disallowed_methods)]
    let response = if reloaded {
        serde_json::json!({
            "success": true,
            "file_path": request.file_path,
            "component_id": component_id,
            "reloaded": true,
            "error": null
        })
    } else if reload_error_details.is_some() || module_reload_error.is_some() {
        serde_json::json!({
            "success": true,
            "file_path": request.file_path,
            "component_id": component_id,
            "reloaded": false,
            "preserved_last_good": true,
            "error": {
                "dist_reload": reload_error_details,
                "module_reload": module_reload_error,
                "suggestion": "Component reload encountered errors. Last known good version is still available. Check console for details or try a manual page refresh."
            }
        })
    } else {
        serde_json::json!({
            "success": true,
            "file_path": request.file_path,
            "component_id": component_id,
            "reloaded": false,
            "error": null
        })
    };

    Ok(Json(response))
}

#[axum::debug_handler]
pub async fn hmr_invalidate_component(
    State(state): State<ServerState>,
    Json(payload): Json<HmrInvalidateRequest>,
) -> Json<Value> {
    info!("HMR invalidate request for component: {}", payload.component_id);

    let result = {
        let renderer = state.renderer.lock().await;

        {
            let mut registry = renderer.component_registry.lock();
            registry.mark_module_stale(&payload.component_id);
            debug!("Marked component {} as stale during invalidation", payload.component_id);
        }

        renderer.clear_component_cache(&payload.component_id);
        debug!("Cleared component cache for {}", payload.component_id);

        {
            let mut registry = renderer.component_registry.lock();
            registry.remove_component(&payload.component_id);
            debug!("Removed component {} from registry", payload.component_id);
        }

        if let Err(e) = renderer.runtime.clear_module_loader_caches(&payload.component_id).await {
            warn!("Failed to clear module loader caches for {}: {}", payload.component_id, e);
        }

        let clear_script = format!(
            r#"
            (function() {{
                let clearedCount = 0;
                const componentId = "{}";

                if (typeof globalThis[componentId] !== 'undefined') {{
                    delete globalThis[componentId];
                    clearedCount++;
                }}

                if (globalThis.__rsc_modules && globalThis.__rsc_modules[componentId]) {{
                    delete globalThis.__rsc_modules[componentId];
                    clearedCount++;
                }}

                if (globalThis.__rsc_functions && globalThis.__rsc_functions[componentId]) {{
                    delete globalThis.__rsc_functions[componentId];
                    clearedCount++;
                }}

                if (globalThis.__rsc_component_functions && globalThis.__rsc_component_functions.has(componentId)) {{
                    globalThis.__rsc_component_functions.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis.__rsc_component_server_functions && globalThis.__rsc_component_server_functions.has(componentId)) {{
                    globalThis.__rsc_component_server_functions.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis.__rsc_component_data && globalThis.__rsc_component_data.has(componentId)) {{
                    globalThis.__rsc_component_data.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis.__rsc_component_namespaces && globalThis.__rsc_component_namespaces.has(componentId)) {{
                    globalThis.__rsc_component_namespaces.delete(componentId);
                    clearedCount++;
                }}

                return {{
                    success: true,
                    clearedCount: clearedCount,
                    componentId: componentId
                }};
            }})()
            "#,
            payload.component_id
        );

        renderer
            .runtime
            .execute_script(
                format!("hmr_clear_cache_{}.js", payload.component_id.replace('/', "_")),
                clear_script,
            )
            .await
    };

    match result {
        Ok(clear_result) => {
            info!("Successfully invalidated component cache for: {}", payload.component_id);
            #[allow(clippy::disallowed_methods)]
            Json(serde_json::json!({
                "success": true,
                "componentId": payload.component_id,
                "cleared": clear_result
            }))
        }
        Err(e) => {
            error!("Failed to invalidate component cache for {}: {}", payload.component_id, e);
            #[allow(clippy::disallowed_methods)]
            Json(serde_json::json!({
                "success": false,
                "componentId": payload.component_id,
                "error": e.to_string()
            }))
        }
    }
}

#[axum::debug_handler]
pub async fn hmr_invalidate_api_route(
    State(state): State<ServerState>,
    Json(payload): Json<HmrInvalidateApiRouteRequest>,
) -> Json<Value> {
    info!("HMR invalidate request for API route: {}", payload.file_path);

    let api_handler = match &state.api_route_handler {
        Some(handler) => handler,
        None => {
            warn!("No API route handler available for HMR invalidation");
            #[allow(clippy::disallowed_methods)]
            return Json(serde_json::json!({
                "success": false,
                "filePath": payload.file_path,
                "error": "API route handler not available"
            }));
        }
    };

    api_handler.invalidate_handler(&payload.file_path);
    info!("Invalidated API route handler cache for: {}", payload.file_path);

    #[allow(clippy::disallowed_methods)]
    Json(serde_json::json!({
        "success": true,
        "filePath": payload.file_path,
        "message": "API route handler cache invalidated"
    }))
}

#[axum::debug_handler]
pub async fn hmr_reload_component(
    State(state): State<ServerState>,
    Json(payload): Json<HmrReloadRequest>,
) -> Json<Value> {
    debug!(
        "HMR reload request for component: {} from file: {}",
        payload.component_id, payload.file_path
    );

    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for HMR reload");
            #[allow(clippy::disallowed_methods)]
            return Json(serde_json::json!({
                "success": false,
                "componentId": payload.component_id,
                "error": "Configuration not available"
            }));
        }
    };

    let client = reqwest::Client::new();
    let vite_base_url = format!("http://{}", config.vite_address());

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let file_path = if payload.file_path.starts_with('/') {
        payload.file_path.clone()
    } else {
        format!("/{}", payload.file_path)
    };

    let vite_url = format!("{}{}?t={}", vite_base_url, file_path, timestamp);

    debug!("Fetching transpiled code from Vite: {}", vite_url);

    let transpiled_code = match client.get(&vite_url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                error!("Vite returned error status: {}", response.status());
                #[allow(clippy::disallowed_methods)]
                return Json(serde_json::json!({
                    "success": false,
                    "componentId": payload.component_id,
                    "error": format!("Vite returned status: {}", response.status())
                }));
            }

            match response.text().await {
                Ok(code) => code,
                Err(e) => {
                    error!("Failed to read response from Vite: {}", e);
                    #[allow(clippy::disallowed_methods)]
                    return Json(serde_json::json!({
                        "success": false,
                        "componentId": payload.component_id,
                        "error": format!("Failed to read response: {}", e)
                    }));
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch from Vite dev server: {}", e);
            #[allow(clippy::disallowed_methods)]
            return Json(serde_json::json!({
                "success": false,
                "componentId": payload.component_id,
                "error": format!("Failed to fetch from Vite: {}", e)
            }));
        }
    };

    debug!("Fetched {} bytes of transpiled code", transpiled_code.len());

    let result = {
        state
            .renderer
            .lock()
            .await
            .register_component(&payload.component_id, &transpiled_code)
            .await
    };

    match result {
        Ok(()) => {
            info!("Successfully reloaded component: {}", payload.component_id);
            #[allow(clippy::disallowed_methods)]
            Json(serde_json::json!({
                "success": true,
                "componentId": payload.component_id,
                "codeSize": transpiled_code.len()
            }))
        }
        Err(e) => {
            error!("Failed to reload component {}: {}", payload.component_id, e);
            #[allow(clippy::disallowed_methods)]
            Json(serde_json::json!({
                "success": false,
                "componentId": payload.component_id,
                "error": e.to_string()
            }))
        }
    }
}

#[axum::debug_handler]
pub async fn reload_component(
    State(state): State<ServerState>,
    Json(payload): Json<ReloadComponentRequest>,
) -> Result<Json<ReloadComponentResponse>, StatusCode> {
    info!(
        "Reload component request for: {} from bundle: {}",
        payload.component_id, payload.bundle_path
    );

    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let bundle_full_path = project_root.join(&payload.bundle_path);

    let invalidate_result = {
        let renderer = state.renderer.lock().await;
        renderer.runtime.invalidate_component(&payload.component_id).await
    };

    if let Err(e) = invalidate_result {
        warn!("Failed to invalidate component (non-fatal): {}", e);
    }

    let load_result = {
        let renderer = state.renderer.lock().await;
        renderer.runtime.load_component(&payload.component_id, &bundle_full_path).await
    };

    match load_result {
        Ok(()) => {
            info!("Component reloaded successfully: {}", payload.component_id);
            Ok(Json(ReloadComponentResponse {
                success: true,
                message: format!("Component {} reloaded successfully", payload.component_id),
            }))
        }
        Err(e) => {
            error!("Failed to reload component {}: {}", payload.component_id, e);
            Ok(Json(ReloadComponentResponse {
                success: false,
                message: format!("Failed to reload component: {}", e),
            }))
        }
    }
}
