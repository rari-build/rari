use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use cow_utils::CowUtils;
use rari_error::RariError;
use serde_json::Value as JsonValue;

use super::super::interface::JsRuntimeInterface;

pub(super) fn sanitize(s: &str) -> String {
    s.replace(['/', '\\', ':', '?', '#'], "_")
}

pub(super) fn build_invalidate_script(component_id: &str) -> String {
    let escaped = component_id.cow_replace('\\', "\\\\").cow_replace('"', r#"\""#).into_owned();
    format!(
        r#"
        (function() {{
            const componentId = "{escaped}";
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

            if (globalThis['~serverFunctions']?.all) {{
                const prefix = componentId + ':';
                for (const key in globalThis['~serverFunctions'].all) {{
                    if (key === componentId || key.startsWith(prefix)) {{
                        delete globalThis['~serverFunctions'].all[key];
                        deleted = true;
                    }}
                }}
            }}

            if (globalThis['~serverFunctions']?.exported) {{
                const prefix = componentId + ':';
                for (const key in globalThis['~serverFunctions'].exported) {{
                    if (key === componentId || key.startsWith(prefix)) {{
                        delete globalThis['~serverFunctions'].exported[key];
                        deleted = true;
                    }}
                }}
            }}

            if (globalThis['~rsc']?.modules?.[componentId]) {{
                delete globalThis['~rsc'].modules[componentId];
                deleted = true;
            }}

            if (globalThis.PromiseManager && globalThis.PromiseManager.clear) {{
                try {{
                    globalThis.PromiseManager.clear(componentId);
                    deleted = true;
                }} catch (e) {{
                    console.warn('Failed to clear PromiseManager for component:', componentId, e);
                }}
            }}

            if (globalThis['~rsc']?.components?.[componentId]) {{
                delete globalThis['~rsc'].components[componentId];
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

pub(super) async fn dispatch_load_component(
    runtime: &dyn JsRuntimeInterface,
    component_id: &str,
    code: &str,
) -> Result<(), RariError> {
    static ESM_REGEX: OnceLock<regex::Regex> = OnceLock::new();
    let esm_regex = ESM_REGEX.get_or_init(|| {
        #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
        regex::Regex::new(r"(?m)^\s*export[\s{]").expect("Valid ESM detection regex")
    });
    let is_esm = esm_regex.is_match(code);

    if is_esm {
        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let hmr_specifier = format!("file:///rari_hmr/server/{component_id}.js?v={timestamp}");

        if let Err(e) = runtime.clear_module_loader_caches(component_id).await {
            tracing::warn!("Failed to clear module loader caches for {}: {}", component_id, e);
        }

        runtime.add_module_to_loader_only(&hmr_specifier, code.to_string()).await.map_err(|e| {
            RariError::js_execution(format!(
                "Failed to add component module to loader for {component_id}: {e}"
            ))
        })?;

        let module_id = runtime.load_es_module(component_id).await.map_err(|e| {
            RariError::js_execution(format!("Failed to load ES module for {component_id}: {e}"))
        })?;

        runtime.evaluate_module(module_id).await.map_err(|e| {
            RariError::js_execution(format!("Failed to evaluate ES module for {component_id}: {e}"))
        })?;

        let escaped_component_id = component_id.replace('\\', "\\\\").replace('"', r#"\""#);
        let escaped_hmr_specifier = hmr_specifier.replace('\\', "\\\\").replace('"', r#"\""#);

        let registration_script = format!(
            r#"(async function() {{
                try {{
                    const moduleNamespace = await import("{escaped_hmr_specifier}");
                    const componentId = "{escaped_component_id}";

                    if (!globalThis['~rsc']) globalThis['~rsc'] = {{}};
                    if (!globalThis['~rsc'].modules) globalThis['~rsc'].modules = {{}};
                    if (!globalThis['~rsc'].functions) globalThis['~rsc'].functions = {{}};

                    globalThis['~rsc'].modules[componentId] = moduleNamespace;

                    if (moduleNamespace.default) {{
                        globalThis[componentId] = moduleNamespace.default;
                    }} else {{
                        const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function');
                        if (exports.length > 0) {{
                            globalThis[componentId] = exports[0];
                        }}
                    }}

                    const namedExports = {{}};
                    for (const [key, value] of Object.entries(moduleNamespace)) {{
                        if (key !== 'default' && typeof value === 'function') {{
                            namedExports[key] = value;
                        }}
                    }}

                    if (Object.keys(namedExports).length > 0) {{
                        globalThis['~rsc'].functions[componentId] = namedExports;
                    }}

                    return {{ success: true }};
                }} catch (error) {{
                    console.error('[rari] Failed to register component {escaped_component_id}:', error);
                    return {{ success: false, error: error.message }};
                }}
            }})()"#
        );

        let result = runtime
            .execute_script(
                format!("register_component_{}.js", component_id.cow_replace('/', "_")),
                registration_script,
            )
            .await
            .map_err(|e| {
                RariError::js_execution(format!(
                    "Failed to register component {component_id} to globalThis: {e}"
                ))
            })?;

        let success = result.get("success").and_then(JsonValue::as_bool).unwrap_or(false);
        if !success {
            let error_msg = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            return Err(RariError::js_execution(format!(
                "Component registration failed for {component_id}: {error_msg}"
            )));
        }

        Ok(())
    } else {
        let script_name = format!("load_component_{}", component_id.cow_replace('/', "_"));
        runtime.execute_script(script_name, code.to_string()).await.map(|_| ()).map_err(|e| {
            RariError::js_execution(format!(
                "Failed to execute component code for {component_id}: {e}"
            ))
        })
    }
}
