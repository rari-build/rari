use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use cow_utils::CowUtils;
use rari_error::RariError;
use regex::Regex;
use serde_json::Value;

use super::interface::JsRuntimeInterface;

fn escape_js_string(s: &str) -> String {
    s.cow_replace('\\', "\\\\")
        .cow_replace('"', r#"\""#)
        .cow_replace('\n', "\\n")
        .cow_replace('\r', "\\r")
        .into_owned()
}

pub fn is_esm_code(code: &str) -> bool {
    static ESM_REGEX: OnceLock<Regex> = OnceLock::new();
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    let regex = ESM_REGEX
        .get_or_init(|| Regex::new(r"(?m)^\s*export[\s{]").expect("Valid ESM detection regex"));

    regex.is_match(code)
}

pub fn invalidate_script_name(component_id: &str) -> String {
    format!("invalidate_{}", component_id.cow_replace('/', "_"))
}

pub fn pending_component_id(component_id: &str) -> String {
    format!("__rari_hmr_pending__:{component_id}")
}

pub fn build_invalidate_script(component_id: &str) -> String {
    let escaped_component_id = escape_js_string(component_id);
    format!(
        r#"
            (function() {{
                const componentId = "{escaped_component_id}";
                let deleted = false;

                if (globalThis[componentId]) {{
                    delete globalThis[componentId];
                    deleted = true;
                }}

                const moduleNamespace = globalThis['~rsc']?.modules?.[componentId];
                if (moduleNamespace) {{
                    for (const key in moduleNamespace) {{
                        if (key !== 'default' && typeof moduleNamespace[key] === 'function' && globalThis[key] === moduleNamespace[key]) {{
                            delete globalThis[key];
                            deleted = true;
                        }}
                    }}
                }}

                if (globalThis['~rsc']?.functions?.[componentId]) {{
                    delete globalThis['~rsc'].functions[componentId];
                    deleted = true;
                }}

                if (globalThis['~rari']?.ssrModules) {{
                    const colonPrefix = componentId + ':';
                    const hashPrefix = componentId + '#';
                    for (const key in globalThis['~rari'].ssrModules) {{
                        if (key === componentId || key.startsWith(colonPrefix) || key.startsWith(hashPrefix)) {{
                            delete globalThis['~rari'].ssrModules[key];
                            deleted = true;
                        }}
                    }}
                }}

                if (globalThis['~rari']?.serverManifest) {{
                    const colonPrefix = componentId + ':';
                    const hashPrefix = componentId + '#';
                    for (const key in globalThis['~rari'].serverManifest) {{
                        if (key === componentId || key.startsWith(colonPrefix) || key.startsWith(hashPrefix)) {{
                            delete globalThis['~rari'].serverManifest[key];
                            deleted = true;
                        }}
                    }}
                }}

                if (globalThis['~rari']?.registeredServerFunctions) {{
                    const colonPrefix = componentId + ':';
                    const hashPrefix = componentId + '#';
                    for (const key of globalThis['~rari'].registeredServerFunctions) {{
                        if (key === componentId || key.startsWith(colonPrefix) || key.startsWith(hashPrefix)) {{
                            globalThis['~rari'].registeredServerFunctions.delete(key);
                            deleted = true;
                        }}
                    }}
                }}

                if (globalThis['~rsc']?.modules?.[componentId]) {{
                    delete globalThis['~rsc'].modules[componentId];
                    deleted = true;
                }}

                if (globalThis.RscModuleManager && globalThis.RscModuleManager.unregister) {{
                    try {{
                        globalThis.RscModuleManager.unregister(componentId);
                        deleted = true;
                    }} catch (e) {{
                        console.warn('Failed to unregister from RscModuleManager:', e);
                    }}
                }}

                return {{ success: true, deleted: deleted }};
            }})()
            "#
    )
}

fn build_pending_registration_script(component_id: &str, hmr_specifier: &str) -> String {
    let escaped_component_id = escape_js_string(component_id);
    let escaped_hmr_specifier = escape_js_string(hmr_specifier);
    format!(
        r#"(async function() {{
                    const componentId = "{escaped_component_id}";
                    try {{
                        const moduleNamespace = await import("{escaped_hmr_specifier}");

                        if (!globalThis['~rsc']) globalThis['~rsc'] = {{}};
                        if (!globalThis['~rsc'].hmrPending) globalThis['~rsc'].hmrPending = {{}};

                        let defaultExport = null;
                        if (moduleNamespace.default) {{
                            defaultExport = moduleNamespace.default;
                        }} else {{
                            const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function');
                            if (exports.length > 0) {{
                                defaultExport = exports[0];
                            }}
                        }}

                        const namedExports = {{}};
                        for (const [key, value] of Object.entries(moduleNamespace)) {{
                            if (key !== 'default' && typeof value === 'function') {{
                                namedExports[key] = value;
                            }}
                        }}

                        globalThis['~rsc'].hmrPending[componentId] = {{
                            moduleNamespace,
                            defaultExport,
                            namedExports,
                        }};

                        return {{ success: true, hasDefault: defaultExport != null }};
                    }} catch (error) {{
                        console.error('[rari] Failed to stage pending component ' + componentId + ':', error);
                        return {{ success: false, error: error.message }};
                    }}
                }})()"#
    )
}

fn build_atomic_swap_script(component_id: &str) -> String {
    let escaped_component_id = escape_js_string(component_id);
    format!(
        r#"(function() {{
                    const componentId = "{escaped_component_id}";
                    const pending = globalThis['~rsc']?.hmrPending?.[componentId];
                    if (!pending) {{
                        return {{ success: false, error: 'No pending HMR payload for ' + componentId }};
                    }}

                    if (!globalThis['~rsc']) globalThis['~rsc'] = {{}};
                    if (!globalThis['~rsc'].modules) globalThis['~rsc'].modules = {{}};
                    if (!globalThis['~rsc'].functions) globalThis['~rsc'].functions = {{}};

                    globalThis['~rsc'].modules[componentId] = pending.moduleNamespace;
                    if (pending.defaultExport) {{
                        globalThis[componentId] = pending.defaultExport;
                    }}
                    if (pending.namedExports && Object.keys(pending.namedExports).length > 0) {{
                        globalThis['~rsc'].functions[componentId] = pending.namedExports;
                    }} else {{
                        delete globalThis['~rsc'].functions[componentId];
                    }}

                    delete globalThis['~rsc'].hmrPending[componentId];
                    return {{
                        success: typeof globalThis[componentId] !== 'undefined',
                        componentId,
                    }};
                }})()"#
    )
}

fn build_discard_pending_script(component_id: &str) -> String {
    let escaped_component_id = escape_js_string(component_id);
    format!(
        r#"(function() {{
                    const componentId = "{escaped_component_id}";
                    if (globalThis['~rsc']?.hmrPending?.[componentId]) {{
                        delete globalThis['~rsc'].hmrPending[componentId];
                    }}
                    return {{ success: true }};
                }})()"#
    )
}

async fn discard_pending_component(
    runtime: &dyn JsRuntimeInterface,
    component_id: &str,
) -> Result<(), RariError> {
    let _ = runtime
        .execute_script(
            format!("discard_pending_{}.js", component_id.cow_replace('/', "_")),
            build_discard_pending_script(component_id),
        )
        .await;
    Ok(())
}

async fn load_esm_component_code_atomic(
    runtime: &dyn JsRuntimeInterface,
    component_id: &str,
    component_code: &str,
) -> Result<(), RariError> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();

    let pending_specifier = format!("file:///rari_hmr/pending/{component_id}.js?v={timestamp}");
    let live_specifier = format!("file:///rari_hmr/server/{component_id}.js?v={timestamp}");
    let pending_id = pending_component_id(component_id);

    runtime.add_module_to_loader(&pending_specifier, component_code.to_string()).await.map_err(
        |e| {
            let error_msg =
                format!("Failed to add pending component module for {component_id}: {e}");
            tracing::error!("{}", error_msg);
            RariError::js_execution(error_msg)
        },
    )?;

    let module_id = runtime.load_es_module(&pending_id).await.map_err(|e| {
        let error_msg = format!("Failed to load pending ES module for {component_id}: {e}");
        tracing::error!("{}", error_msg);
        RariError::js_execution(error_msg)
    })?;

    if let Err(e) = runtime.evaluate_module(module_id).await {
        let _ = discard_pending_component(runtime, component_id).await;
        let error_msg = format!("Failed to evaluate pending ES module for {component_id}: {e}");
        tracing::error!("{}", error_msg);
        return Err(RariError::js_execution(error_msg));
    }

    let stage_result = match runtime
        .execute_script(
            format!("stage_pending_{}.js", component_id.cow_replace('/', "_")),
            build_pending_registration_script(component_id, &pending_specifier),
        )
        .await
    {
        Ok(json) => json,
        Err(e) => {
            let _ = discard_pending_component(runtime, component_id).await;
            let error_msg =
                format!("Failed to stage pending component {component_id} to globalThis: {e}");
            tracing::error!("{}", error_msg);
            return Err(RariError::js_execution(error_msg));
        }
    };

    if !stage_result.get("success").and_then(Value::as_bool).unwrap_or(false) {
        let _ = discard_pending_component(runtime, component_id).await;
        let error_msg =
            stage_result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        tracing::error!("Pending component staging failed for {}: {}", component_id, error_msg);
        return Err(RariError::js_execution(format!(
            "Pending component staging failed for {component_id}: {error_msg}"
        )));
    }

    let swap_result = match runtime
        .execute_script(
            format!("swap_component_{}.js", component_id.cow_replace('/', "_")),
            build_atomic_swap_script(component_id),
        )
        .await
    {
        Ok(json) => json,
        Err(e) => {
            let _ = discard_pending_component(runtime, component_id).await;
            let error_msg = format!("Failed to atomically swap component {component_id}: {e}");
            tracing::error!("{}", error_msg);
            return Err(RariError::js_execution(error_msg));
        }
    };

    if !swap_result.get("success").and_then(Value::as_bool).unwrap_or(false) {
        let _ = discard_pending_component(runtime, component_id).await;
        let error_msg =
            swap_result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        return Err(RariError::js_execution(format!(
            "Atomic component swap failed for {component_id}: {error_msg}"
        )));
    }

    if let Err(e) = runtime.clear_module_loader_caches(component_id).await {
        tracing::warn!("Failed to clear module loader caches for {}: {}", component_id, e);
    }
    runtime.add_module_to_loader(&live_specifier, component_code.to_string()).await.map_err(
        |e| {
            let error_msg =
                format!("Failed to promote live component module for {component_id}: {e}");
            tracing::error!("{}", error_msg);
            RariError::js_execution(error_msg)
        },
    )?;

    Ok(())
}

pub async fn load_component_code(
    runtime: &dyn JsRuntimeInterface,
    component_id: &str,
    component_code: &str,
) -> Result<(), RariError> {
    if is_esm_code(component_code) {
        return load_esm_component_code_atomic(runtime, component_id, component_code).await;
    }

    let script_name = format!("load_component_{}", component_id.cow_replace('/', "_"));
    match runtime.execute_script(script_name, component_code.to_string()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = format!("Failed to execute component code for {component_id}: {e}");
            tracing::error!("{}", error_msg);
            Err(RariError::js_execution(error_msg))
        }
    }
}
