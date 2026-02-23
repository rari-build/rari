use crate::server::utils::component_utils::{
    get_dist_path_for_component, wrap_server_action_module,
};
use crate::server::utils::http_utils::merge_vary_with_accept;
use crate::server::{RegisterClientRequest, RegisterRequest, RenderRequest, ServerState};
use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{Json, Response},
};
use cow_utils::CowUtils;
use serde_json::Value;
use tracing::error;

const RSC_CONTENT_TYPE: &str = "text/x-component";

#[axum::debug_handler]
pub async fn stream_component(
    State(state): State<ServerState>,
    Json(request): Json<RenderRequest>,
) -> Result<Response, StatusCode> {
    {
        let renderer = state.renderer.lock().await;
        let registry = renderer.component_registry.lock();
        if !registry.is_component_registered(&request.component_id) {
            error!("Attempted to stream unregistered component: {}", request.component_id);
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let props_str = request.props.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let cache_key = if let Some(props) = &props_str {
        format!("/_rari/stream/{}?props={}", request.component_id, props)
    } else {
        format!("/_rari/stream/{}", request.component_id)
    };

    if let Some(cached) = state.response_cache.get(&cache_key).await {
        let merged_vary = merge_vary_with_accept(cached.headers.get("vary"));

        let mut response_builder = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", RSC_CONTENT_TYPE)
            .header("vary", merged_vary)
            .header("x-cache", "HIT");

        for (key, value) in cached.headers.iter() {
            if key.as_str() != "vary" {
                response_builder = response_builder.header(key, value);
            }
        }

        return Ok(response_builder
            .body(Body::from(cached.body))
            .expect("Valid cached RSC response"));
    }

    let stream_result = {
        let renderer = state.renderer.lock().await;
        renderer.render_with_streaming(&request.component_id, props_str.as_deref()).await
    };

    match stream_result {
        Ok(rsc_stream) => {
            let cache_control = state.config.get_cache_control_for_route("/_rari/stream");

            use futures::StreamExt;
            let byte_stream = rsc_stream
                .map(|result| result.map(bytes::Bytes::from).map_err(std::io::Error::other));

            Ok(Response::builder()
                .header("content-type", RSC_CONTENT_TYPE)
                .header("cache-control", cache_control)
                .header("vary", "Accept")
                .header("x-cache", "MISS")
                .body(Body::from_stream(byte_stream))
                .expect("Valid RSC response"))
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
    if let Some(cache_config) = &request.cache_config {
        let mut cache_configs = state.component_cache_configs.write().await;
        cache_configs.insert(request.component_id.clone(), cache_config.clone());
    }

    let result = {
        let mut renderer = state.renderer.lock().await;
        renderer.register_component(&request.component_id, &request.component_code).await
    };

    match result {
        Ok(_) => {
            let renderer = state.renderer.lock().await;
            let is_client =
                renderer.serializer.lock().is_client_component_registered(&request.component_id);

            if is_client {
                let mark_script = format!(
                    r#"(function() {{
                        const comp = globalThis["{}"];
                        if (comp && typeof comp === 'function') {{
                            comp['~isClientComponent'] = true;
                            comp['~clientComponentId'] = "{}";
                        }}
                    }})()"#,
                    request.component_id, request.component_id
                );

                if let Err(e) = renderer
                    .runtime
                    .execute_script(
                        format!("mark_client_{}.js", request.component_id.cow_replace('/', "_")),
                        mark_script,
                    )
                    .await
                {
                    error!("Failed to mark {} as client component: {}", request.component_id, e);
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
    {
        let renderer = state.renderer.lock().await;
        renderer.register_client_component(
            &request.component_id,
            &request.file_path,
            &request.export_name,
        );
    }

    #[allow(clippy::disallowed_methods)]
    Ok(Json(serde_json::json!({
        "success": true,
        "component_id": request.component_id
    })))
}

pub async fn reload_component_from_dist(
    state: &ServerState,
    file_path: &str,
    component_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::server::utils::path_validation::{
        normalize_component_path, validate_component_path,
    };

    let normalized_path = normalize_component_path(file_path);

    if let Err(e) = validate_component_path(&normalized_path) {
        error!(
            component_id = component_id,
            file_path = file_path,
            normalized_path = %normalized_path,
            error = %e,
            "Component path validation failed"
        );
        return Err(format!("Path validation error: {}", e).into());
    }

    let file_path = &normalized_path;

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

    if !dist_path.exists() {
        return Err(format!(
            "Dist file not found: {}. Vite may not have finished building yet. Last known good version will be preserved.",
            dist_path.display()
        )
        .into());
    }

    let mut dist_code = match tokio::fs::read_to_string(&dist_path).await {
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
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let new_dist_code = match tokio::fs::read_to_string(&dist_path).await {
            Ok(code) => code,
            Err(e) => {
                error!(
                    component_id = component_id,
                    dist_path = %dist_path.display(),
                    error = %e,
                    "Failed to re-read dist file after retry"
                );
                return Err(format!("Failed to re-read dist file: {}", e).into());
            }
        };

        let renderer = state.renderer.lock().await;
        let registry = renderer.component_registry.lock();
        if let Some(existing_component) = registry.get_component(component_id) {
            let existing_snippet =
                existing_component.transformed_source.chars().take(500).collect::<String>();
            let new_snippet = new_dist_code.chars().take(500).collect::<String>();

            if existing_snippet == new_snippet {
                return Err(
                    "Dist file not yet updated by Vite. Last known good version preserved.".into(),
                );
            }
        }
        drop(registry);
        drop(renderer);

        dist_code = new_dist_code;
    }

    let is_esm = dist_code.contains("export ")
        || dist_code.contains("export{")
        || dist_code.contains("export {")
        || dist_code.contains("export\n")
        || dist_code.contains("export\r");

    let renderer = state.renderer.lock().await;

    if is_esm {
        renderer.clear_component_cache(component_id);

        if let Err(e) = renderer.runtime.clear_module_loader_caches(component_id).await {
            error!("Failed to clear module loader caches for {}: {}", component_id, e);
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let hmr_specifier = format!("file:///rari_hmr/server/{}.js?v={}", component_id, timestamp);

        renderer
            .runtime
            .add_module_to_loader_only(&hmr_specifier, dist_code.clone())
            .await
            .map_err(|e| {
                error!(
                    component_id = component_id,
                    error = %e,
                    "Failed to add HMR module to loader"
                );
                format!("Failed to add HMR module to loader: {}", e)
            })?;

        let module_id = renderer.runtime.load_es_module(component_id).await.map_err(|e| {
            error!(
                component_id = component_id,
                error = %e,
                "Failed to load ES module during HMR"
            );
            format!("Failed to load ES module: {}", e)
        })?;

        renderer.runtime.evaluate_module(module_id).await.map_err(|e| {
            error!(
                component_id = component_id,
                module_id = module_id,
                error = %e,
                "Failed to evaluate module during HMR"
            );
            format!("Failed to evaluate module: {}", e)
        })?;

        let clear_script = format!(
            r#"(function() {{
                const componentId = "{}";

                delete globalThis[componentId];

                if (globalThis['~rsc'] && globalThis['~rsc'].modules) {{
                    delete globalThis['~rsc'].modules[componentId];
                }}

                return {{ success: true }};
            }})()"#,
            component_id
        );

        renderer
            .runtime
            .execute_script(
                format!("clear_old_{}.js", component_id.cow_replace('/', "_")),
                clear_script,
            )
            .await
            .map_err(|e| {
                error!(
                    component_id = component_id,
                    error = %e,
                    "Failed to clear old component"
                );
                format!("Failed to clear old component: {}", e)
            })?;

        let registration_script = format!(
            r#"(async function() {{
                try {{
                    const moduleNamespace = await import("{}");
                    const componentId = "{}";

                    if (moduleNamespace.default) {{
                        globalThis[componentId] = moduleNamespace.default;
                        if (typeof globalThis[componentId] === 'function') {{
                            globalThis[componentId].__hmr_timestamp = Date.now();
                            globalThis[componentId].__hmr_specifier = "{}";
                        }}
                    }} else {{
                        const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function');
                        if (exports.length > 0) {{
                            globalThis[componentId] = exports[0];
                            if (typeof globalThis[componentId] === 'function') {{
                                globalThis[componentId].__hmr_timestamp = Date.now();
                                globalThis[componentId].__hmr_specifier = "{}";
                            }}
                        }}
                    }}

                    for (const [key, value] of Object.entries(moduleNamespace)) {{
                        if (key !== 'default' && typeof value === 'function') {{
                            globalThis[key] = value;
                        }}
                    }}

                    if (!globalThis['~rsc']) globalThis['~rsc'] = {{}};
                    if (!globalThis['~rsc'].modules) globalThis['~rsc'].modules = {{}};
                    globalThis['~rsc'].modules[componentId] = moduleNamespace;

                    const component = globalThis[componentId];

                    return {{ success: true, hasDefault: !!moduleNamespace.default, timestamp: component?.__hmr_timestamp }};
                }} catch (error) {{
                    return {{ success: false, error: error.message }};
                }}
            }})()"#,
            hmr_specifier, component_id, hmr_specifier, hmr_specifier
        );

        renderer
            .runtime
            .execute_script(
                format!("register_esm_{}.js", component_id.cow_replace('/', "_")),
                registration_script,
            )
            .await
            .map_err(|e| {
                error!(
                    component_id = component_id,
                    error = %e,
                    "Failed to register ESM module exports to globalThis"
                );
                format!("Failed to register ESM module: {}", e)
            })?;

        renderer.clear_script_cache();

        let dependencies = crate::rsc::utils::dependency_utils::extract_dependencies(&dist_code);

        {
            let mut registry = renderer.component_registry.lock();

            registry.remove_component(component_id);

            let _ = registry.register_component(
                component_id,
                &dist_code,
                dist_code.clone(),
                dependencies.into_iter().collect(),
            );

            registry.mark_component_loaded(component_id);
            registry.mark_component_initially_loaded(component_id);
        }
    } else {
        let wrapped_code = wrap_server_action_module(&dist_code, component_id);

        let execution_result = renderer
            .runtime
            .execute_script(
                format!("hmr_reload_{}.js", component_id.cow_replace('/', "_")),
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
            format!("verify_{}.js", component_id.cow_replace('/', "_")),
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
    use crate::server::utils::path_validation::{
        normalize_component_path, validate_component_path,
    };

    let normalized_path = normalize_component_path(file_path);

    if let Err(e) = validate_component_path(&normalized_path) {
        error!(
            file_path = file_path,
            normalized_path = %normalized_path,
            error = %e,
            "Component path validation failed"
        );
        return Err(format!("Path validation error: {}", e).into());
    }

    let file_path = &normalized_path;

    let path = std::path::Path::new(file_path);
    let component_name =
        path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("UnknownComponent");

    {
        let mut renderer = state.renderer.lock().await;
        renderer.clear_script_cache();

        if let Err(e) = renderer.clear_component_module_cache(component_name).await {
            error!("Failed to clear component module cache for {}: {}", component_name, e);
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
                error!("Failed to clear component module cache for {}: {}", component_name, e);
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

            Ok(())
        }
    }
}

#[axum::debug_handler]
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    #[allow(clippy::disallowed_methods)]
    Ok(Json(serde_json::json!({
        "status": "ok",
        "service": "rari-rsc-server"
    })))
}
