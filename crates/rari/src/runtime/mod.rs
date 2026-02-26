use crate::error::RariError;
use cow_utils::CowUtils;
use deno_error::JsErrorBox as JsError;
use regex::Regex;
use serde_json::{Value, json};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::mpsc;

pub mod bridge;
pub mod ext;
pub mod module_loader;
pub mod module_reload;
pub mod ops;
pub mod runtime_factory;
pub mod utils;

mod metadata;

pub type TokioRuntime = tokio::runtime::Handle;

pub struct JsExecutionRuntime {
    runtime: Arc<Box<dyn runtime_factory::JsRuntimeInterface>>,
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
    let regex = ESM_REGEX
        .get_or_init(|| Regex::new(r"(?m)^\s*export[\s{]").expect("Valid ESM detection regex"));

    regex.is_match(code)
}

impl JsExecutionRuntime {
    pub fn new(env_vars: Option<rustc_hash::FxHashMap<String, String>>) -> Self {
        let runtime = if let Some(env_vars) = env_vars {
            runtime_factory::create_lazy_runtime_with_env(env_vars)
        } else {
            runtime_factory::create_lazy_runtime()
        };

        Self { runtime: Arc::new(runtime), timeout_ms: 30000 }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn tokio_runtime(&self) -> TokioRuntime {
        tokio::runtime::Handle::current()
    }

    pub async fn call_function_async<T>(
        &mut self,
        name: &str,
        args: &impl serde::ser::Serialize,
    ) -> Result<T, crate::error::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let args_json = serde_json::to_value(args)
            .map_err(|e| crate::error::Error::JsonDecode(e.to_string()))?;

        let args_vec = if let Value::Array(arr) = args_json { arr } else { vec![args_json] };

        let result = self
            .execute_function(name, args_vec)
            .await
            .map_err(|e| crate::error::Error::Runtime(e.to_string()))?;

        serde_json::from_value(result).map_err(|e| crate::error::Error::JsonDecode(e.to_string()))
    }

    pub async fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Result<Value, RariError> {
        let runtime = self.runtime.clone();
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

    pub async fn collect_metadata(
        &self,
        layout_paths: Vec<String>,
        page_path: String,
        params: rustc_hash::FxHashMap<String, crate::server::routing::types::ParamValue>,
        search_params: rustc_hash::FxHashMap<String, Vec<String>>,
    ) -> Result<Value, RariError> {
        const METADATA_COLLECTOR_TEMPLATE: &str = include_str!("js/metadata_collector.js");

        let layout_paths_json = serde_json::to_string(&layout_paths)
            .map_err(|e| RariError::serialization(e.to_string()))?;
        let params_json =
            serde_json::to_string(&params).map_err(|e| RariError::serialization(e.to_string()))?;

        let search_params_simple: rustc_hash::FxHashMap<String, String> = search_params
            .iter()
            .filter_map(|(k, v)| v.first().map(|val| (k.clone(), val.clone())))
            .collect();
        let search_params_json = serde_json::to_string(&search_params_simple)
            .map_err(|e| RariError::serialization(e.to_string()))?;

        let page_path_json = serde_json::to_string(&page_path)
            .map_err(|e| RariError::serialization(e.to_string()))?;

        let script = METADATA_COLLECTOR_TEMPLATE
            .cow_replace("LAYOUT_PATHS_PLACEHOLDER", &layout_paths_json)
            .cow_replace("'PAGE_PATH_PLACEHOLDER'", &page_path_json)
            .cow_replace("SEARCH_PARAMS_PLACEHOLDER", &search_params_json)
            .cow_replace("PARAMS_PLACEHOLDER", &params_json)
            .into_owned();

        let metadata_list = self.execute_script("collect_metadata".to_string(), script).await?;

        let metadata_array = metadata_list.as_array().ok_or_else(|| {
            RariError::serialization("Expected metadata list to be an array".to_string())
        })?;

        let mut merged_metadata = json!({});
        for metadata_item in metadata_array {
            merged_metadata = metadata::merge_metadata(&merged_metadata, metadata_item);
        }

        metadata::finalize_metadata(&mut merged_metadata);

        Ok(merged_metadata)
    }

    pub async fn execute_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RariError> {
        let runtime = self.runtime.clone();
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
        let runtime = self.runtime.clone();
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
        let runtime = self.runtime.clone();

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
        let runtime = self.runtime.clone();
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
        let runtime = self.runtime.clone();
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
        let runtime = self.runtime.clone();
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

    pub async fn load_and_evaluate_module(
        &self,
        module_specifier: &str,
    ) -> Result<Value, RariError> {
        let module_id = self.load_es_module(module_specifier).await?;

        self.evaluate_module(module_id).await?;
        self.get_module_namespace(module_id).await
    }

    pub async fn get_module_namespace(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Result<Value, RariError> {
        let runtime = self.runtime.clone();

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

    pub async fn execute_script_for_streaming(
        &self,
        script_name: String,
        script_code: String,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
    ) -> Result<(), RariError> {
        let callback_setup = r#"
            (function() {
                return { success: true };
            })();
        "#;

        let combined_script = format!("{callback_setup}\n\n{script_code}");

        let runtime = self.runtime.clone();
        let script_name_clone = script_name.clone();

        match tokio::time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.execute_script_for_streaming(script_name_clone, combined_script, chunk_sender),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Streaming script execution timed out after {} ms for {}",
                self.timeout_ms, script_name
            ))),
        }
    }

    pub async fn invalidate_component(&self, component_id: &str) -> Result<(), RariError> {
        let escaped_component_id = escape_js_string(component_id);

        let script = format!(
            r#"
            (function() {{
                const componentId = "{escaped_id}";
                let deleted = false;

                if (globalThis[componentId]) {{
                    delete globalThis[componentId];
                    deleted = true;
                }}

                if (globalThis['~rsc'].functions && globalThis['~rsc'].functions[componentId]) {{
                    delete globalThis['~rsc'].functions[componentId];
                    deleted = true;
                }}

                if (globalThis['~serverFunctions']?.all && globalThis['~serverFunctions'].all[componentId]) {{
                    delete globalThis['~serverFunctions'].all[componentId];
                    deleted = true;
                }}

                if (globalThis['~rsc'].modules && globalThis['~rsc'].modules[componentId]) {{
                    delete globalThis['~rsc'].modules[componentId];
                    deleted = true;
                }}

                if (globalThis['~rsc'].components && globalThis['~rsc'].components[componentId]) {{
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
            "#,
            escaped_id = escaped_component_id
        );

        match self
            .execute_script(format!("invalidate_{}", component_id.cow_replace('/', "_")), script)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to invalidate component {}: {}", component_id, e);
                Err(RariError::js_runtime(format!(
                    "Failed to invalidate component {}: {}",
                    component_id, e
                )))
            }
        }
    }

    pub async fn load_component(
        &self,
        component_id: &str,
        bundle_path: &std::path::Path,
    ) -> Result<(), RariError> {
        if !bundle_path.exists() {
            let error_msg = format!("Component bundle file not found: {:?}", bundle_path);
            tracing::error!("{}", error_msg);
            return Err(RariError::not_found(error_msg));
        }

        let component_code = tokio::fs::read_to_string(bundle_path).await.map_err(|e| {
            let error_msg = format!("Failed to read component bundle file: {}", e);
            tracing::error!("{}", error_msg);
            RariError::io(error_msg)
        })?;

        self.load_component_code(component_id, &component_code).await
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

            let hmr_specifier =
                format!("file:///rari_hmr/server/{}.js?v={}", component_id, timestamp);

            if let Err(e) = self.clear_module_loader_caches(component_id).await {
                tracing::warn!("Failed to clear module loader caches for {}: {}", component_id, e);
            }

            self.add_module_to_loader_only(&hmr_specifier, component_code.to_string())
                .await
                .map_err(|e| {
                    let error_msg = format!(
                        "Failed to add component module to loader for {}: {}",
                        component_id, e
                    );
                    tracing::error!("{}", error_msg);
                    RariError::js_execution(error_msg)
                })?;

            let module_id = self.load_es_module(&hmr_specifier).await.map_err(|e| {
                let error_msg = format!("Failed to load ES module for {}: {}", component_id, e);
                tracing::error!("{}", error_msg);
                RariError::js_execution(error_msg)
            })?;

            self.evaluate_module(module_id).await.map_err(|e| {
                let error_msg = format!("Failed to evaluate ES module for {}: {}", component_id, e);
                tracing::error!("{}", error_msg);
                RariError::js_execution(error_msg)
            })?;

            let escaped_component_id = escape_js_string(component_id);
            let escaped_hmr_specifier = escape_js_string(&hmr_specifier);

            let registration_script = format!(
                r#"(async function() {{
                    try {{
                        const moduleNamespace = await import("{}");
                        const componentId = "{}";

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
                        console.error('[rari] Failed to register component {}:', error);
                        return {{ success: false, error: error.message }};
                    }}
                }})()"#,
                escaped_hmr_specifier, escaped_component_id, escaped_component_id
            );

            let result = self
                .execute_script(
                    format!("register_component_{}.js", component_id.cow_replace('/', "_")),
                    registration_script,
                )
                .await
                .map_err(|e| {
                    let error_msg = format!(
                        "Failed to register component {} to globalThis: {}",
                        component_id, e
                    );
                    tracing::error!("{}", error_msg);
                    RariError::js_execution(error_msg)
                })?;

            let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

            if !success {
                let error_msg =
                    result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                tracing::error!(
                    "Component registration failed for {}: {}",
                    component_id,
                    error_msg
                );
                return Err(RariError::js_execution(format!(
                    "Component registration failed for {}: {}",
                    component_id, error_msg
                )));
            }

            Ok(())
        } else {
            let script_name = format!("load_component_{}", component_id.cow_replace('/', "_"));
            match self.execute_script(script_name, component_code.to_string()).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    let error_msg =
                        format!("Failed to execute component code for {}: {}", component_id, e);
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
        let runtime = self.runtime.clone();

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
        let runtime = self.runtime.clone();

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
}

impl From<crate::error::RariError> for JsError {
    fn from(err: crate::error::RariError) -> Self {
        Self::generic(err.to_string())
    }
}

impl From<deno_core::v8::DataError> for crate::error::RariError {
    fn from(err: deno_core::v8::DataError) -> Self {
        crate::error::RariError::JsRuntime(format!("V8 Data Error: {err}"), None)
    }
}

pub trait RsFunction: 'static {
    fn call(
        &self,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, crate::error::RariError>;
}
