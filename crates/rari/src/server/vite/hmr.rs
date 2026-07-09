#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{extract::State, http::StatusCode, response::Json};
use cow_utils::CowUtils;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{fs, time};

use crate::{
    rsc::extract_dependencies,
    server::{
        ServerState,
        cache::response::ResponseCache,
        config::Config,
        core::utils::{
            component::extract_component_id,
            path_validation::{
                normalize_component_path, validate_component_path, validate_safe_path,
            },
        },
        vite::rsc::{immediate_component_reregistration, reload_component_from_dist},
    },
};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
#[non_exhaustive]
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
#[non_exhaustive]
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
            Ok(handle_invalidate_api_route(&state, &file_path))
        }
        HmrRequest::ReloadComponent { component_id, bundle_path } => {
            handle_reload_component(state, component_id, bundle_path).await
        }
    }
}

async fn invalidate_component_cache(cache: &ResponseCache, component_id: &str) {
    let cache_key_prefix = format!("/_rari/stream/{component_id}");

    let all_keys = cache.get_all_keys();
    for key in all_keys {
        if key.starts_with(&cache_key_prefix) {
            cache.invalidate(&key).await;
        }
    }
}

async fn handle_register(state: ServerState, file_path: String) -> Result<Json<Value>, StatusCode> {
    let file_path = normalize_component_path(&file_path);

    if let Err(e) = validate_component_path(&file_path) {
        tracing::error!(
            file_path = %file_path,
            error = %e,
            "Component path validation failed"
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    let component_id = match extract_component_id(&file_path) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to extract component ID from {}: {}", file_path, e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    {
        let renderer = state.renderer.lock().await;
        {
            let mut registry = renderer.component_registry.lock();
            registry.mark_module_stale(&component_id);
        }
    }

    let reload_result = reload_component_from_dist(&state, &file_path, &component_id).await;

    let mut reload_error_details: Option<serde_json::Value> = None;

    match &reload_result {
        Ok(()) => {}
        Err(e) => {
            tracing::error!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to reload component from dist, preserving last known good version"
            );

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
        tracing::error!(
            component_id = component_id,
            file_path = file_path,
            error = %e,
            "Failed to immediately re-register component, preserving last known good version"
        );

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

    let reloaded = reload_result.is_ok();

    let response = if reloaded {
        invalidate_component_cache(&state.response_cache, &component_id).await;

        let route_cache_patterns: Vec<String> = vec![
            file_path.cow_replace("src/app/", "/").cow_replace("/page.tsx", "").into_owned(),
            file_path.cow_replace("src/app/", "/").cow_replace("/page.ts", "").into_owned(),
        ]
        .into_iter()
        .filter(|p| p.len() > 1)
        .collect();
        for pattern in route_cache_patterns {
            let all_keys = state.response_cache.get_all_keys();
            for key in all_keys {
                if key.starts_with(&pattern) {
                    state.response_cache.invalidate(&key).await;
                }
            }
        }

        if let Err(e) = state.layout_html_cache.clear().await {
            tracing::warn!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to clear layout_html_cache during HMR reload"
            );
            return Ok(Json(serde_json::json!({
                "success": false,
                "file_path": file_path,
                "component_id": component_id,
                "reloaded": false,
                "preserved_last_good": true,
                "error": {
                    "stage": "layout_cache_clear",
                    "message": e.to_string(),
                    "suggestion": "Layout cache clear failed during HMR. Last known good version is still available. Try a manual page refresh."
                }
            })));
        }

        serde_json::json!({
            "success": true,
            "file_path": file_path,
            "component_id": component_id,
            "reloaded": true,
            "error": null
        })
    } else if reload_error_details.is_some() {
        serde_json::json!({
            "success": true,
            "file_path": file_path,
            "component_id": component_id,
            "reloaded": false,
            "preserved_last_good": true,
            "error": {
                "dist_reload": reload_error_details,
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
            tracing::error!("Failed to clear module loader caches for {}: {}", component_id, e);
        }

        let clear_script = format!(
            r#"
            (function() {{
                let clearedCount = 0;
                const componentId = "{component_id}";

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

                if (globalThis['~rari']?.ssrModules) {{
                    const colonPrefix = componentId + ':';
                    const hashPrefix = componentId + '#';
                    for (const key in globalThis['~rari'].ssrModules) {{
                        if (key === componentId || key.startsWith(colonPrefix) || key.startsWith(hashPrefix)) {{
                            delete globalThis['~rari'].ssrModules[key];
                            clearedCount++;
                        }}
                    }}
                }}

                if (globalThis['~rari']?.serverManifest) {{
                    const colonPrefix = componentId + ':';
                    const hashPrefix = componentId + '#';
                    for (const key in globalThis['~rari'].serverManifest) {{
                        if (key === componentId || key.startsWith(colonPrefix) || key.startsWith(hashPrefix)) {{
                            delete globalThis['~rari'].serverManifest[key];
                            clearedCount++;
                        }}
                    }}
                }}

                if (globalThis['~rari']?.registeredServerFunctions) {{
                    const colonPrefix = componentId + ':';
                    const hashPrefix = componentId + '#';
                    for (const key of globalThis['~rari'].registeredServerFunctions) {{
                        if (key === componentId || key.startsWith(colonPrefix) || key.startsWith(hashPrefix)) {{
                            globalThis['~rari'].registeredServerFunctions.delete(key);
                            clearedCount++;
                        }}
                    }}
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
            "#
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
        Ok(clear_result) => Ok(Json(serde_json::json!({
            "success": true,
            "componentId": component_id,
            "cleared": clear_result
        }))),
        Err(e) => {
            tracing::error!("Failed to invalidate component cache for {}: {}", component_id, e);
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
    let Some(config) = Config::get() else {
        tracing::error!("Failed to get global configuration for HMR reload");
        return Ok(Json(serde_json::json!({
            "success": false,
            "componentId": component_id,
            "error": "Configuration not available"
        })));
    };

    if file_path.contains("://") {
        tracing::error!("Invalid file path: contains URL scheme");
        return Ok(Json(serde_json::json!({
            "success": false,
            "componentId": component_id,
            "error": "Invalid file path: URL schemes not allowed"
        })));
    }

    let client = Client::new();
    let vite_base_url = format!("http://{}", config.vite_address());

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();

    let file_path =
        if file_path.starts_with('/') { file_path.clone() } else { format!("/{file_path}") };

    let vite_url = format!("{vite_base_url}{file_path}?t={timestamp}");

    let transpiled_code = match client.get(&vite_url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                tracing::error!("Vite returned error status: {}", response.status());
                return Ok(Json(serde_json::json!({
                    "success": false,
                    "componentId": component_id,
                    "error": format!("Vite returned status: {}", response.status())
                })));
            }

            match response.text().await {
                Ok(code) => code,
                Err(e) => {
                    tracing::error!("Failed to read response from Vite: {}", e);
                    return Ok(Json(serde_json::json!({
                        "success": false,
                        "componentId": component_id,
                        "error": format!("Failed to read response: {}", e)
                    })));
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch from Vite dev server: {}", e);
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
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "componentId": component_id,
            "codeSize": transpiled_code.len()
        }))),
        Err(e) => {
            tracing::error!("Failed to reload component {}: {}", component_id, e);
            Ok(Json(serde_json::json!({
                "success": false,
                "componentId": component_id,
                "error": e.to_string()
            })))
        }
    }
}

fn handle_invalidate_api_route(state: &ServerState, file_path: &str) -> axum::Json<Value> {
    let Some(api_handler) = &state.api_route_handler else {
        return Json(serde_json::json!({
            "success": false,
            "filePath": file_path,
            "error": "API route handler not available"
        }));
    };

    api_handler.invalidate_handler(file_path);

    Json(serde_json::json!({
        "success": true,
        "filePath": file_path,
        "message": "API route handler cache invalidated"
    }))
}

async fn handle_reload_component(
    state: ServerState,
    component_id: String,
    bundle_path: String,
) -> Result<Json<Value>, StatusCode> {
    let project_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let bundle_full_path = match validate_safe_path(&project_root, &bundle_path).await {
        Ok(path) => path,
        Err(e) => {
            tracing::error!(
                bundle_path = %bundle_path,
                error = %e,
                "Bundle path validation failed"
            );
            return Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("Invalid bundle path: {}", e)
            })));
        }
    };

    let mut bundle_code = String::new();
    let mut last_error = None;
    for attempt in 0..3 {
        match fs::read_to_string(&bundle_full_path).await {
            Ok(code) => {
                bundle_code = code;
                last_error = None;
                break;
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < 2 {
                    time::sleep(time::Duration::from_millis(50)).await;
                }
            }
        }
    }

    if let Some(e) = last_error {
        tracing::error!("Failed to read bundle file {}: {}", bundle_full_path.display(), e);
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": format!("Failed to read bundle: {}", e)
        })));
    }

    if let Err(e) = {
        let renderer = state.renderer.lock().await;
        renderer.runtime.invalidate_component(&component_id).await
    } {
        tracing::error!("Failed to invalidate component {}: {}", component_id, e);
    }

    let load_result = {
        let renderer = state.renderer.lock().await;
        renderer.runtime.load_component_code(&component_id, &bundle_code).await
    };

    match load_result {
        Ok(()) => {
            {
                let renderer = state.renderer.lock().await;
                let mut registry = renderer.component_registry.lock();

                registry.remove_component(&component_id);

                let dependencies = extract_dependencies(&bundle_code);

                match registry.register_component(
                    &component_id,
                    &bundle_code,
                    bundle_code.clone(),
                    dependencies.into_iter().collect(),
                ) {
                    Ok(()) => {
                        registry.mark_component_loaded(&component_id);
                        registry.mark_component_initially_loaded(&component_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to register component {}: {}", component_id, e);
                        registry.remove_component(&component_id);
                        return Ok(Json(serde_json::json!({
                            "success": false,
                            "message": format!("Failed to register component: {}", e)
                        })));
                    }
                }
            }

            invalidate_component_cache(&state.response_cache, &component_id).await;

            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("Component {} reloaded successfully", component_id)
            })))
        }
        Err(e) => {
            tracing::error!("Failed to reload component {}: {}", component_id, e);
            Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to reload component: {}", e)
            })))
        }
    }
}
