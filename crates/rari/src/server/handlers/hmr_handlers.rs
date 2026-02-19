use crate::server::ServerState;
use crate::server::config::Config;
use crate::server::handlers::rsc_handlers::{
    immediate_component_reregistration, reload_component_from_dist,
};
use crate::server::utils::component_utils::extract_component_id;
use axum::{extract::State, http::StatusCode, response::Json};
use cow_utils::CowUtils;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::error;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum HmrRequest {
    Register {
        #[serde(rename = "file_path")]
        file_path: String,
    },
    Invalidate {
        #[serde(rename = "componentId")]
        component_id: String,
        #[serde(rename = "filePath")]
        file_path: Option<String>,
    },
    Reload {
        #[serde(rename = "componentId")]
        component_id: String,
        #[serde(rename = "filePath")]
        file_path: String,
    },
    InvalidateApiRoute {
        #[serde(rename = "filePath")]
        file_path: String,
    },
    ReloadComponent {
        #[serde(rename = "component_id")]
        component_id: String,
        #[serde(rename = "bundle_path")]
        bundle_path: String,
    },
}

#[derive(Debug, Serialize)]
pub struct HmrResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(flatten)]
    pub data: Option<Value>,
}

#[axum::debug_handler]
pub async fn handle_hmr_action(
    State(state): State<ServerState>,
    Json(request): Json<HmrRequest>,
) -> Result<Json<Value>, StatusCode> {
    match request {
        HmrRequest::Register { file_path } => handle_register(state, file_path).await,
        HmrRequest::Invalidate { component_id, file_path } => {
            handle_invalidate(state, component_id, file_path).await
        }
        HmrRequest::Reload { component_id, file_path } => {
            handle_reload(state, component_id, file_path).await
        }
        HmrRequest::InvalidateApiRoute { file_path } => {
            handle_invalidate_api_route(state, file_path).await
        }
        HmrRequest::ReloadComponent { component_id, bundle_path } => {
            handle_reload_component(state, component_id, bundle_path).await
        }
    }
}

async fn handle_register(state: ServerState, file_path: String) -> Result<Json<Value>, StatusCode> {
    use crate::server::utils::path_validation::{
        normalize_component_path, validate_component_path,
    };

    let file_path = normalize_component_path(&file_path);

    if let Err(e) = validate_component_path(&file_path) {
        error!(
            file_path = %file_path,
            error = %e,
            "Component path validation failed"
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    let component_id = match extract_component_id(&file_path) {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to extract component ID from {}: {}", file_path, e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let path = std::path::Path::new(&file_path);

    {
        let renderer = state.renderer.lock().await;
        {
            let mut registry = renderer.component_registry.lock();
            registry.mark_module_stale(&component_id);
        }

        let clear_cache_script = r#"
            if (typeof globalThis.__RARI_CLEAR_RESOLVED_CACHE__ === 'function') {
                globalThis.__RARI_CLEAR_RESOLVED_CACHE__();
            }
        "#;

        if let Err(e) = renderer
            .runtime
            .execute_script("clear_resolved_cache".to_string(), clear_cache_script.to_string())
            .await
        {
            error!("Failed to clear resolved cache: {}", e);
        }
    }

    let reload_result = reload_component_from_dist(&state, &file_path, &component_id).await;

    let mut reload_error_details: Option<serde_json::Value> = None;

    match &reload_result {
        Ok(_) => {}
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

    if reload_result.is_err()
        && let Err(e) = immediate_component_reregistration(&state, &file_path).await
    {
        error!(
            component_id = component_id,
            file_path = file_path,
            error = %e,
            "Failed to immediately re-register component, preserving last known good version"
        );

        #[allow(clippy::disallowed_methods)]
        return Ok(Json(serde_json::json!({
            "success": false,
            "file_path": file_path,
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
    }

    let mut reloaded = reload_result.is_ok();
    let mut module_reload_error: Option<String> = None;

    if state.config.hmr_reload_enabled() {
        match state.module_reload_manager.reload_module_debounced(&component_id, path).await {
            Ok(()) => {
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
    }

    #[allow(clippy::disallowed_methods)]
    let response = if reloaded {
        let cache_key_prefix = format!("/_rari/stream/{}", component_id);
        state.response_cache.invalidate(&cache_key_prefix).await;

        let all_keys = state.response_cache.get_all_keys();
        for key in all_keys {
            if key.starts_with(&cache_key_prefix) {
                state.response_cache.invalidate(&key).await;
            }
        }

        let route_cache_patterns = vec![
            format!("/?"),
            format!("/"),
            file_path.replace("src/app/", "/").replace("/page.tsx", ""),
            file_path.replace("src/app/", "/").replace("/page.ts", ""),
        ];

        for pattern in route_cache_patterns {
            let all_keys = state.response_cache.get_all_keys();
            for key in all_keys {
                if key.starts_with(&pattern) || key.contains(&pattern) {
                    state.response_cache.invalidate(&key).await;
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        serde_json::json!({
            "success": true,
            "file_path": file_path,
            "component_id": component_id,
            "reloaded": true,
            "error": null
        })
    } else if reload_error_details.is_some() || module_reload_error.is_some() {
        serde_json::json!({
            "success": true,
            "file_path": file_path,
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
            "file_path": file_path,
            "component_id": component_id,
            "reloaded": false,
            "error": null
        })
    };

    Ok(Json(response))
}

async fn handle_invalidate(
    state: ServerState,
    component_id: String,
    _file_path: Option<String>,
) -> Result<Json<Value>, StatusCode> {
    let result = {
        let renderer = state.renderer.lock().await;

        {
            let mut registry = renderer.component_registry.lock();
            registry.mark_module_stale(&component_id);
        }

        renderer.clear_component_cache(&component_id);

        if let Err(e) = renderer.runtime.clear_module_loader_caches(&component_id).await {
            error!("Failed to clear module loader caches for {}: {}", component_id, e);
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

                if (globalThis['~rsc'].modules && globalThis['~rsc'].modules[componentId]) {{
                    delete globalThis['~rsc'].modules[componentId];
                    clearedCount++;
                }}

                if (globalThis['~rsc'].functions && globalThis['~rsc'].functions[componentId]) {{
                    delete globalThis['~rsc'].functions[componentId];
                    clearedCount++;
                }}

                if (globalThis['~rsc'].componentFunctions && globalThis['~rsc'].componentFunctions.has(componentId)) {{
                    globalThis['~rsc'].componentFunctions.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis['~rsc'].serverFunctions && globalThis['~rsc'].serverFunctions.has(componentId)) {{
                    globalThis['~rsc'].serverFunctions.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis['~rsc'].componentData && globalThis['~rsc'].componentData.has(componentId)) {{
                    globalThis['~rsc'].componentData.delete(componentId);
                    clearedCount++;
                }}

                if (globalThis['~rsc'].componentNamespaces && globalThis['~rsc'].componentNamespaces.has(componentId)) {{
                    globalThis['~rsc'].componentNamespaces.delete(componentId);
                    clearedCount++;
                }}

                return {{
                    success: true,
                    clearedCount: clearedCount,
                    componentId: componentId
                }};
            }})()
            "#,
            component_id
        );

        renderer
            .runtime
            .execute_script(
                format!("hmr_clear_cache_{}.js", component_id.cow_replace('/', "_")),
                clear_script,
            )
            .await
    };

    match result {
        Ok(clear_result) =>
        {
            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": true,
                "componentId": component_id,
                "cleared": clear_result
            })))
        }
        Err(e) => {
            error!("Failed to invalidate component cache for {}: {}", component_id, e);
            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": false,
                "componentId": component_id,
                "error": e.to_string()
            })))
        }
    }
}

async fn handle_reload(
    state: ServerState,
    component_id: String,
    file_path: String,
) -> Result<Json<Value>, StatusCode> {
    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for HMR reload");
            #[allow(clippy::disallowed_methods)]
            return Ok(Json(serde_json::json!({
                "success": false,
                "componentId": component_id,
                "error": "Configuration not available"
            })));
        }
    };

    if file_path.contains("://") {
        error!("Invalid file path: contains URL scheme");
        #[allow(clippy::disallowed_methods)]
        return Ok(Json(serde_json::json!({
            "success": false,
            "componentId": component_id,
            "error": "Invalid file path: URL schemes not allowed"
        })));
    }

    let client = reqwest::Client::new();
    let vite_base_url = format!("http://{}", config.vite_address());

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let file_path =
        if file_path.starts_with('/') { file_path.clone() } else { format!("/{}", file_path) };

    let vite_url = format!("{}{}?t={}", vite_base_url, file_path, timestamp);

    let transpiled_code = match client.get(&vite_url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                error!("Vite returned error status: {}", response.status());
                #[allow(clippy::disallowed_methods)]
                return Ok(Json(serde_json::json!({
                    "success": false,
                    "componentId": component_id,
                    "error": format!("Vite returned status: {}", response.status())
                })));
            }

            match response.text().await {
                Ok(code) => code,
                Err(e) => {
                    error!("Failed to read response from Vite: {}", e);
                    #[allow(clippy::disallowed_methods)]
                    return Ok(Json(serde_json::json!({
                        "success": false,
                        "componentId": component_id,
                        "error": format!("Failed to read response: {}", e)
                    })));
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch from Vite dev server: {}", e);
            #[allow(clippy::disallowed_methods)]
            return Ok(Json(serde_json::json!({
                "success": false,
                "componentId": component_id,
                "error": format!("Failed to fetch from Vite: {}", e)
            })));
        }
    };

    let result =
        { state.renderer.lock().await.register_component(&component_id, &transpiled_code).await };

    match result {
        Ok(()) =>
        {
            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": true,
                "componentId": component_id,
                "codeSize": transpiled_code.len()
            })))
        }
        Err(e) => {
            error!("Failed to reload component {}: {}", component_id, e);
            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": false,
                "componentId": component_id,
                "error": e.to_string()
            })))
        }
    }
}

async fn handle_invalidate_api_route(
    state: ServerState,
    file_path: String,
) -> Result<Json<Value>, StatusCode> {
    let api_handler = match &state.api_route_handler {
        Some(handler) => handler,
        None => {
            #[allow(clippy::disallowed_methods)]
            return Ok(Json(serde_json::json!({
                "success": false,
                "filePath": file_path,
                "error": "API route handler not available"
            })));
        }
    };

    api_handler.invalidate_handler(&file_path);

    #[allow(clippy::disallowed_methods)]
    Ok(Json(serde_json::json!({
        "success": true,
        "filePath": file_path,
        "message": "API route handler cache invalidated"
    })))
}

async fn handle_reload_component(
    state: ServerState,
    component_id: String,
    bundle_path: String,
) -> Result<Json<Value>, StatusCode> {
    use crate::server::utils::path_validation::validate_safe_path;

    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let bundle_full_path = match validate_safe_path(&project_root, &bundle_path) {
        Ok(path) => path,
        Err(e) => {
            error!(
                bundle_path = %bundle_path,
                error = %e,
                "Bundle path validation failed"
            );
            #[allow(clippy::disallowed_methods)]
            return Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("Invalid bundle path: {}", e)
            })));
        }
    };

    let mut bundle_code = String::new();
    let mut last_error = None;
    for attempt in 0..3 {
        match tokio::fs::read_to_string(&bundle_full_path).await {
            Ok(code) => {
                bundle_code = code;
                last_error = None;
                break;
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < 2 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
            }
        }
    }

    if let Some(e) = last_error {
        error!("Failed to read bundle file {}: {}", bundle_full_path.display(), e);
        #[allow(clippy::disallowed_methods)]
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": format!("Failed to read bundle: {}", e)
        })));
    }

    if let Err(e) = {
        let renderer = state.renderer.lock().await;
        renderer.runtime.invalidate_component(&component_id).await
    } {
        error!("Failed to invalidate component {}: {}", component_id, e);
    }

    let load_result = {
        let renderer = state.renderer.lock().await;
        renderer.runtime.load_component_code(&component_id, bundle_code.clone()).await
    };

    match load_result {
        Ok(()) => {
            {
                let renderer = state.renderer.lock().await;
                let mut registry = renderer.component_registry.lock();

                registry.remove_component(&component_id);

                let dependencies =
                    crate::rsc::utils::dependency_utils::extract_dependencies(&bundle_code);
                let _ = registry.register_component(
                    &component_id,
                    &bundle_code,
                    bundle_code.clone(),
                    dependencies.into_iter().collect(),
                );

                registry.mark_component_loaded(&component_id);
                registry.mark_component_initially_loaded(&component_id);
            }

            let cache_key_prefix = format!("/_rari/stream/{}", component_id);
            state.response_cache.invalidate(&cache_key_prefix).await;

            let all_keys = state.response_cache.get_all_keys();
            for key in all_keys {
                if key.starts_with(&cache_key_prefix) {
                    state.response_cache.invalidate(&key).await;
                }
            }

            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("Component {} reloaded successfully", component_id)
            })))
        }
        Err(e) => {
            error!("Failed to reload component {}: {}", component_id, e);
            #[allow(clippy::disallowed_methods)]
            Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to reload component: {}", e)
            })))
        }
    }
}
