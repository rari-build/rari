use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use cow_utils::CowUtils;
use rari_error::RariError;
use regex::Regex;
use serde_json::{Value, json};
use tracing::error;

pub mod ext;
pub mod factory;
pub mod module;
pub mod ops;
#[cfg(feature = "redis")]
pub mod redis_cache;
pub mod utils;

use factory::JsRuntimeInterface;

use crate::server::rendering::metadata::{finalize_metadata, merge_metadata};

pub struct JsExecutionRuntime {
    runtime: Arc<factory::RariRuntime>,
    timeout_ms: u64,
}

impl Default for JsExecutionRuntime {
    fn default() -> Self {
        Self::new(None)
    }
}

fn escape_js_string(s: &str) -> String {
    s.cow_replace('\\', "\\\\")
        .cow_replace('"', r#"\""#)
        .cow_replace('\n', "\\n")
        .cow_replace('\r', "\\r")
        .into_owned()
}

fn is_esm_code(code: &str) -> bool {
    static ESM_REGEX: OnceLock<Regex> = OnceLock::new();
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    let regex = ESM_REGEX
        .get_or_init(|| Regex::new(r"(?m)^\s*export[\s{]").expect("Valid ESM detection regex"));

    regex.is_match(code)
}

impl JsExecutionRuntime {
    pub fn new(env_vars: Option<rustc_hash::FxHashMap<String, String>>) -> Self {
        let runtime = if let Some(env_vars) = env_vars {
            factory::create_runtime_with_env(env_vars)
        } else {
            factory::create_runtime()
        };

        Self { runtime, timeout_ms: 30000 }
    }

    pub async fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Result<Value, RariError> {
        let runtime = Arc::clone(&self.runtime);
        let script_name_clone = script_name.clone();
        let script_code_clone = script_code.clone();

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.execute_script(script_name_clone, script_code_clone),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Script execution timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn execute_script_batch(
        &self,
        scripts: Vec<(String, String)>,
    ) -> tokio::sync::mpsc::UnboundedReceiver<(usize, Result<Value, RariError>)> {
        self.runtime.execute_script_batch(scripts).await
    }

    pub async fn collect_metadata(
        &self,
        layout_paths: Vec<String>,
        page_path: String,
        params: rustc_hash::FxHashMap<String, crate::server::routing::types::ParamValue>,
        search_params: rustc_hash::FxHashMap<String, Vec<String>>,
    ) -> Result<Value, RariError> {
        let data = json!({
            "layoutPaths": layout_paths,
            "pagePath": page_path,
            "params": params,
            "searchParams": search_params,
        });

        let data_json =
            serde_json::to_string(&data).map_err(|e| RariError::serialization(e.to_string()))?;

        let script = format!(
            r"(function() {{
                const data = {data_json};
                return globalThis['~rari'].metadataCollector.collect(
                    data.layoutPaths,
                    data.pagePath,
                    data.params,
                    data.searchParams
                );
            }})()"
        );

        let metadata_list = self.execute_script("collect_metadata".to_string(), script).await?;

        let metadata_array = metadata_list.as_array().ok_or_else(|| {
            RariError::serialization("Expected metadata list to be an array".to_string())
        })?;

        let mut merged_metadata = json!({});
        for metadata_item in metadata_array {
            merged_metadata = merge_metadata(&merged_metadata, metadata_item);
        }

        finalize_metadata(&mut merged_metadata);

        Ok(merged_metadata)
    }

    pub async fn execute_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RariError> {
        let runtime = Arc::clone(&self.runtime);
        let function_name = function_name.to_string();

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.execute_function(&function_name, args),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Function execution timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn load_es_module(&self, specifier: &str) -> Result<deno_core::ModuleId, RariError> {
        let runtime = Arc::clone(&self.runtime);
        let specifier = specifier.to_string();

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.load_es_module(&specifier),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Module loading timed out after {} ms for {}",
                self.timeout_ms, specifier
            ))),
        }
    }

    pub async fn evaluate_module(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Result<Value, RariError> {
        let runtime = Arc::clone(&self.runtime);

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.evaluate_module(module_id),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Module evaluation timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn add_module_to_loader(&self, specifier: &str) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);
        let specifier = specifier.to_string();

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.add_module_to_loader(&specifier),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Adding module to loader timed out after {} ms for {}",
                self.timeout_ms, specifier
            ))),
        }
    }

    pub async fn add_module_to_loader_only(
        &self,
        specifier: &str,
        code: String,
    ) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);
        let specifier = specifier.to_string();

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.add_module_to_loader_only(&specifier, code),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Adding module (only) to loader timed out after {} ms for {}",
                self.timeout_ms, specifier
            ))),
        }
    }

    pub async fn clear_module_loader_caches(&self, component_id: &str) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);
        let component_id = component_id.to_string();

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.clear_module_loader_caches(&component_id),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Clearing module loader caches timed out after {} ms for {}",
                self.timeout_ms, component_id
            ))),
        }
    }

    pub async fn get_module_namespace(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Result<Value, RariError> {
        let runtime = Arc::clone(&self.runtime);

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.get_module_namespace(module_id),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Getting module namespace timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn invalidate_component(&self, component_id: &str) -> Result<(), RariError> {
        let escaped_component_id = escape_js_string(component_id);

        let script = format!(
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
        );

        match self
            .execute_script(format!("invalidate_{}", component_id.cow_replace('/', "_")), script)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to invalidate component {}: {}", component_id, e);
                Err(RariError::js_runtime(format!(
                    "Failed to invalidate component {component_id}: {e}"
                )))
            }
        }
    }

    pub async fn load_component_code(
        &self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        let is_esm = is_esm_code(component_code);

        if is_esm {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();

            let hmr_specifier = format!("file:///rari_hmr/server/{component_id}.js?v={timestamp}");

            if let Err(e) = self.clear_module_loader_caches(component_id).await {
                tracing::warn!("Failed to clear module loader caches for {}: {}", component_id, e);
            }

            self.add_module_to_loader_only(&hmr_specifier, component_code.to_string())
                .await
                .map_err(|e| {
                    let error_msg =
                        format!("Failed to add component module to loader for {component_id}: {e}");
                    tracing::error!("{}", error_msg);
                    RariError::js_execution(error_msg)
                })?;

            let module_id = self.load_es_module(component_id).await.map_err(|e| {
                let error_msg = format!("Failed to load ES module for {component_id}: {e}");
                tracing::error!("{}", error_msg);
                RariError::js_execution(error_msg)
            })?;

            self.evaluate_module(module_id).await.map_err(|e| {
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

            let result = self
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

            let success =
                result.get("success").and_then(serde_json::Value::as_bool).unwrap_or(false);

            if !success {
                let error_msg =
                    result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                tracing::error!(
                    "Component registration failed for {}: {}",
                    component_id,
                    error_msg
                );
                return Err(RariError::js_execution(format!(
                    "Component registration failed for {component_id}: {error_msg}"
                )));
            }

            Ok(())
        } else {
            let script_name = format!("load_component_{}", component_id.cow_replace('/', "_"));
            match self.execute_script(script_name, component_code.to_string()).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    let error_msg =
                        format!("Failed to execute component code for {component_id}: {e}");
                    tracing::error!("{}", error_msg);
                    Err(RariError::js_execution(error_msg))
                }
            }
        }
    }

    pub async fn set_request_context(
        &self,
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
    ) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.set_request_context(request_context),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Setting request context timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn clear_request_context(&self) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.clear_request_context(),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Clearing request context timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn clear_request_context_if_matches(
        &self,
        expected_context: std::sync::Arc<
            crate::server::middleware::request_context::RequestContext,
        >,
    ) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.clear_request_context_if_matches(expected_context),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Clearing request context (if matches) timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn execute_with_request_context<F, T>(
        &self,
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        operation: F,
    ) -> Result<T, RariError>
    where
        F: std::future::Future<Output = Result<T, RariError>>,
    {
        self.set_request_context(request_context).await?;

        let result = operation.await;

        let clear_result = self.clear_request_context().await;

        match (result, clear_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(value), Err(clear_err)) => {
                error!("Failed to clear request context after successful operation: {}", clear_err);
                Ok(value)
            }
            (Err(op_err), Err(clear_err)) => {
                error!("Failed to clear request context after operation error: {}", clear_err);
                Err(op_err)
            }
            (Err(op_err), Ok(())) => Err(op_err),
        }
    }

    pub async fn execute_with_persistent_request_context<F, T>(
        &self,
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        operation: F,
    ) -> Result<T, RariError>
    where
        F: std::future::Future<Output = Result<T, RariError>>,
    {
        self.set_request_context(request_context).await?;
        operation.await
    }
}
