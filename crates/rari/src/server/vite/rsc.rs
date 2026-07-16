#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{path::Path, sync::Arc};

use axum::{extract::State, http::StatusCode, response::Json};
use cow_utils::CowUtils;
use rari_error::RariError;
use serde_json::Value;
use tokio::{fs, time};

use crate::{
    rendering::base::run_with_renderer_result,
    rsc::extract_dependencies,
    runtime::factory::component_ops::is_esm_code,
    server::{
        RegisterClientRequest, RegisterRequest, ServerState,
        core::utils::{
            component::{get_dist_path_for_component, wrap_server_action_module},
            path_validation::{normalize_component_path, validate_component_path},
        },
        error_response,
    },
};

#[axum::debug_handler]
pub async fn register_component(
    State(state): State<ServerState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    if let Some(cache_config) = &request.cache_config {
        let mut cache_configs = state.component_cache_configs.write().await;
        cache_configs.insert(request.component_id.clone(), cache_config.clone());
    }

    let result = {
        let renderer = Arc::clone(&state.renderer);
        let component_id = request.component_id.clone();
        let component_code = request.component_code.clone();
        run_with_renderer_result(renderer, move |renderer| async move {
            renderer.register_component(&component_id, &component_code).await?;

            let is_client = {
                let registry = renderer.component_registry.lock();
                registry.is_client_reference(&component_id)
            };

            if is_client {
                let mark_script = format!(
                    r#"(function() {{
                        const comp = globalThis["{component_id}"];
                        if (comp && typeof comp === 'function') {{
                            comp['~isClientComponent'] = true;
                            comp['~clientComponentId'] = "{component_id}";
                        }}
                    }})()"#
                );

                if let Err(e) = renderer
                    .runtime
                    .execute_script(
                        format!("mark_client_{}.js", component_id.cow_replace('/', "_")),
                        mark_script,
                    )
                    .await
                {
                    tracing::error!("Failed to mark {} as client component: {}", component_id, e);
                }
            }

            Ok(())
        })
        .await
    };

    match result {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "component_id": request.component_id
        }))),
        Err(e) => {
            tracing::error!("Failed to register component {}: {}", request.component_id, e);
            Err(error_response::status(&e))
        }
    }
}

#[axum::debug_handler]
pub async fn register_client_component(
    State(state): State<ServerState>,
    Json(request): Json<RegisterClientRequest>,
) -> Result<Json<Value>, StatusCode> {
    {
        let renderer = state.renderer.lock().await;
        renderer.register_client_component(
            &request.component_id,
            &request.file_path,
            &request.export_name,
        );
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "component_id": request.component_id
    })))
}

pub async fn reload_component_from_dist(
    state: &ServerState,
    file_path: &str,
    component_id: &str,
) -> Result<(), RariError> {
    let normalized_path = normalize_component_path(file_path);

    if let Err(e) = validate_component_path(&normalized_path) {
        tracing::error!(
            component_id = component_id,
            file_path = file_path,
            normalized_path = %normalized_path,
            error = %e,
            "Component path validation failed"
        );
        return Err(RariError::validation(format!("Path validation error: {e}")));
    }

    let file_path = &normalized_path;

    let dist_path = match get_dist_path_for_component(file_path) {
        Ok(path) => path,
        Err(e) => {
            tracing::error!(
                component_id = component_id,
                file_path = file_path,
                error = %e,
                "Failed to resolve dist path for component"
            );
            return Err(RariError::internal(format!("Path resolution error: {e}")));
        }
    };

    if !fs::try_exists(&dist_path).await.unwrap_or(false) {
        return Err(RariError::not_found(format!(
            "Dist file not found: {}. Vite may not have finished building yet. Last known good version will be preserved.",
            dist_path.display()
        )));
    }

    let mut dist_code = match fs::read_to_string(&dist_path).await {
        Ok(code) => code,
        Err(e) => {
            tracing::error!(
                component_id = component_id,
                dist_path = %dist_path.display(),
                error = %e,
                error_kind = ?e.kind(),
                "Failed to read dist file. Last known good version will be preserved."
            );
            return Err(RariError::io(format!(
                "Failed to read dist file {}: {}. Last known good version will be preserved.",
                dist_path.display(),
                e
            )));
        }
    };

    let needs_retry = {
        let renderer = state.renderer.lock().await;
        let registry = renderer.component_registry.lock();
        if let Some(existing_component) = registry.get_component(component_id) {
            let existing_snippet =
                existing_component.transformed_source.chars().take(500).collect::<String>();
            let new_snippet = dist_code.chars().take(500).collect::<String>();

            existing_snippet == new_snippet
        } else {
            false
        }
    };

    if needs_retry {
        time::sleep(time::Duration::from_millis(100)).await;

        let new_dist_code = match fs::read_to_string(&dist_path).await {
            Ok(code) => code,
            Err(e) => {
                tracing::error!(
                    component_id = component_id,
                    dist_path = %dist_path.display(),
                    error = %e,
                    "Failed to re-read dist file after retry"
                );
                return Err(RariError::io(format!("Failed to re-read dist file: {e}")));
            }
        };

        let renderer = state.renderer.lock().await;
        let registry = renderer.component_registry.lock();
        if let Some(existing_component) = registry.get_component(component_id) {
            let existing_snippet =
                existing_component.transformed_source.chars().take(500).collect::<String>();
            let new_snippet = new_dist_code.chars().take(500).collect::<String>();

            if existing_snippet == new_snippet {
                return Err(RariError::state(
                    "Dist file not yet updated by Vite. Last known good version preserved.",
                ));
            }
        }
        drop(registry);
        drop(renderer);

        dist_code = new_dist_code;
    }

    let is_esm = is_esm_code(&dist_code);

    let dist_path_display = dist_path.display().to_string();
    let component_id = component_id.to_string();
    let renderer = Arc::clone(&state.renderer);

    run_with_renderer_result(renderer, move |renderer| async move {
        if is_esm {
            renderer.runtime.load_component_code(&component_id, &dist_code).await?;
        } else {
            let wrapped_code = wrap_server_action_module(&dist_code, &component_id);

            let execution_result = renderer
                .runtime
                .execute_script(
                    format!("hmr_reload_{}.js", component_id.cow_replace('/', "_")),
                    wrapped_code.clone(),
                )
                .await;

            if let Err(e) = execution_result {
                tracing::error!(
                    component_id = component_id.as_str(),
                    dist_path = %dist_path_display,
                    error = %e,
                    code_length = dist_code.len(),
                    "Failed to execute component code during reload. Last known good version will be preserved."
                );
                return Err(RariError::js_execution(format!(
                    "Failed to execute component code: {e}. Last known good version will be preserved."
                )));
            }
        }

        let verification_script = format!(
            r"(function() {{
            const expectedKey = '{component_id}';
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
        }})()"
        );

        let result_json = match renderer
            .runtime
            .execute_script(
                format!("verify_{}.js", component_id.cow_replace('/', "_")),
                verification_script,
            )
            .await
        {
            Ok(json) => json,
            Err(e) => {
                tracing::error!(
                    component_id = component_id.as_str(),
                    error = %e,
                    "Failed to execute verification script. Last known good version will be preserved."
                );
                return Err(RariError::js_execution(format!(
                    "Failed to verify component reload: {e}. Last known good version will be preserved."
                )));
            }
        };

        if let Some(success) = result_json.get("success").and_then(serde_json::Value::as_bool) {
            if !success {
                let actual_keys = result_json
                    .get("actualKeys")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                let expected_key = result_json
                    .get("expectedKey")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&component_id);

                tracing::error!(
                    component_id = component_id.as_str(),
                    expected_key = expected_key,
                    actual_keys = actual_keys,
                    verification_result = ?result_json,
                    "Component not found in globalThis after reload. Expected key '{}' not found. Available keys: [{}]. Last known good version will be preserved.",
                    expected_key,
                    actual_keys
                );
                return Err(RariError::js_runtime(format!(
                    "Component '{component_id}' not found in globalThis after reload. Expected key '{expected_key}' but found keys: [{actual_keys}]. Last known good version will be preserved."
                )));
            }
        } else {
            tracing::error!(
                component_id = component_id.as_str(),
                verification_result = ?result_json,
                "Invalid verification result format. Last known good version will be preserved."
            );
            return Err(RariError::internal(
                "Invalid verification result format. Last known good version will be preserved."
            ));
        }

        if is_esm {
            renderer.clear_component_cache(&component_id);
            renderer.clear_script_cache();

            let dependencies = extract_dependencies(&dist_code);

            {
                let mut registry = renderer.component_registry.lock();

                registry.remove_component(&component_id);

                match registry.register_component(
                    &component_id,
                    &dist_code,
                    dist_code.clone(),
                    dependencies.into_iter().collect(),
                ) {
                    Ok(()) => {
                        registry.mark_component_loaded(&component_id);
                        registry.mark_component_initially_loaded(&component_id);
                    }
                    Err(e) => {
                        tracing::error!(
                            component_id = component_id.as_str(),
                            error = %e,
                            "Failed to register component during HMR reload"
                        );
                        registry.remove_component(&component_id);
                        return Err(RariError::internal(format!("Failed to register component: {e}")));
                    }
                }
            }
        }

        Ok(())
    })
    .await
}

pub async fn immediate_component_reregistration(
    state: &ServerState,
    file_path: &str,
) -> Result<(), RariError> {
    let normalized_path = normalize_component_path(file_path);

    if let Err(e) = validate_component_path(&normalized_path) {
        tracing::error!(
            file_path = file_path,
            normalized_path = %normalized_path,
            error = %e,
            "Component path validation failed"
        );
        return Err(RariError::validation(format!("Path validation error: {e}")));
    }

    let file_path = &normalized_path;

    let path = Path::new(file_path);
    let component_name =
        path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("UnknownComponent");

    {
        let renderer = Arc::clone(&state.renderer);
        let component_name = component_name.to_string();
        run_with_renderer_result(renderer, move |mut renderer| async move {
            renderer.clear_script_cache();

            if let Err(e) = renderer.clear_component_module_cache(&component_name).await {
                tracing::error!(
                    "Failed to clear component module cache for {}: {}",
                    component_name,
                    e
                );
            }
            Ok(())
        })
        .await?;
    }

    let content = match fs::read_to_string(file_path).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                component_name = component_name,
                file_path = file_path,
                error = %e,
                error_kind = ?e.kind(),
                "Failed to read source file for immediate re-registration"
            );
            return Err(RariError::io(format!("Failed to read source file: {e}")));
        }
    };

    let component_name = component_name.to_string();
    let renderer = Arc::clone(&state.renderer);
    let content_for_register = content.clone();
    let name_for_register = component_name.clone();

    if let Err(e) = run_with_renderer_result(renderer, move |renderer| async move {
        renderer.register_component(&name_for_register, &content_for_register).await
    })
    .await
    {
        tracing::error!(
            component_name = component_name.as_str(),
            error = %e,
            "Failed to register component directly, preserving last known good version"
        );
        return Err(RariError::internal(format!("Failed to register component: {e}")));
    }

    time::sleep(time::Duration::from_millis(100)).await;

    {
        let renderer = Arc::clone(&state.renderer);
        let name = component_name.clone();
        run_with_renderer_result(renderer, move |mut renderer| async move {
            if let Err(e) = renderer.clear_component_module_cache(&name).await {
                tracing::error!("Failed to clear component module cache for {}: {}", name, e);
            }
            Ok(())
        })
        .await?;
    }

    time::sleep(time::Duration::from_millis(200)).await;

    let renderer = Arc::clone(&state.renderer);
    let name_for_reregister = component_name.clone();
    if let Err(e) = run_with_renderer_result(renderer, move |renderer| async move {
        renderer.register_component(&name_for_reregister, &content).await
    })
    .await
    {
        tracing::error!(
            component_name = component_name.as_str(),
            error = %e,
            "Failed to re-register component after cache clear, preserving last known good version"
        );
        return Err(RariError::internal(format!(
            "Failed to re-register component after cache clear: {e}"
        )));
    }

    time::sleep(time::Duration::from_millis(200)).await;

    Ok(())
}

#[axum::debug_handler]
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "ok",
        "service": "rari-rsc-server"
    })))
}
