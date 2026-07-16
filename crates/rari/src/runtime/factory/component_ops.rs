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

pub async fn load_component_code(
    runtime: &dyn JsRuntimeInterface,
    component_id: &str,
    component_code: &str,
) -> Result<(), RariError> {
    let is_esm = is_esm_code(component_code);

    if is_esm {
        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();

        let hmr_specifier = format!("file:///rari_hmr/server/{component_id}.js?v={timestamp}");

        if let Err(e) = runtime.clear_module_loader_caches(component_id).await {
            tracing::warn!("Failed to clear module loader caches for {}: {}", component_id, e);
        }

        runtime.add_module_to_loader(&hmr_specifier, component_code.to_string()).await.map_err(
            |e| {
                let error_msg =
                    format!("Failed to add component module to loader for {component_id}: {e}");
                tracing::error!("{}", error_msg);
                RariError::js_execution(error_msg)
            },
        )?;

        let module_id = runtime.load_es_module(component_id).await.map_err(|e| {
            let error_msg = format!("Failed to load ES module for {component_id}: {e}");
            tracing::error!("{}", error_msg);
            RariError::js_execution(error_msg)
        })?;

        runtime.evaluate_module(module_id).await.map_err(|e| {
            let error_msg = format!("Failed to evaluate ES module for {component_id}: {e}");
            tracing::error!("{}", error_msg);
            RariError::js_execution(error_msg)
        })?;

        let escaped_component_id = escape_js_string(component_id);
        let escaped_hmr_specifier = escape_js_string(&hmr_specifier);

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
                let error_msg =
                    format!("Failed to register component {component_id} to globalThis: {e}");
                tracing::error!("{}", error_msg);
                RariError::js_execution(error_msg)
            })?;

        let success = result.get("success").and_then(Value::as_bool).unwrap_or(false);

        if !success {
            let error_msg = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            tracing::error!("Component registration failed for {}: {}", component_id, error_msg);
            return Err(RariError::js_execution(format!(
                "Component registration failed for {component_id}: {error_msg}"
            )));
        }

        Ok(())
    } else {
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
}
