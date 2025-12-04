use crate::error::RariError;
use deno_error::JsErrorBox as JsError;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub mod bridge;
pub mod ext;
pub mod module_loader;
pub mod module_reload;
pub mod ops;
pub mod runtime_factory;
pub mod transpile;
pub mod utils;

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
        tracing::debug!("Invalidating component: {}", component_id);

        let escaped_component_id = component_id
            .replace('\\', "\\\\")
            .replace('"', r#"\""#)
            .replace('\n', "\\n")
            .replace('\r', "\\r");

        let script = format!(
            r#"
            (function() {{
                const componentId = "{escaped_id}";
                let deleted = false;

                if (globalThis[componentId]) {{
                    delete globalThis[componentId];
                    deleted = true;
                }}

                if (globalThis.__rsc_functions && globalThis.__rsc_functions[componentId]) {{
                    delete globalThis.__rsc_functions[componentId];
                    deleted = true;
                }}

                if (globalThis.__server_functions && globalThis.__server_functions[componentId]) {{
                    delete globalThis.__server_functions[componentId];
                    deleted = true;
                }}

                if (globalThis.__rsc_modules && globalThis.__rsc_modules[componentId]) {{
                    delete globalThis.__rsc_modules[componentId];
                    deleted = true;
                }}

                if (globalThis.__rsc_components && globalThis.__rsc_components[componentId]) {{
                    delete globalThis.__rsc_components[componentId];
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
            .execute_script(format!("invalidate_{}", component_id.replace('/', "_")), script)
            .await
        {
            Ok(_) => {
                tracing::info!("Component invalidated successfully: {}", component_id);
                Ok(())
            }
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
        tracing::debug!("Loading component: {} from {:?}", component_id, bundle_path);

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

        let script_name = format!("load_component_{}", component_id.replace('/', "_"));
        match self.execute_script(script_name, component_code).await {
            Ok(_) => {
                tracing::info!("Component loaded successfully: {}", component_id);
                Ok(())
            }
            Err(e) => {
                let error_msg =
                    format!("Failed to execute component code for {}: {}", component_id, e);
                tracing::error!("{}", error_msg);
                Err(RariError::js_execution(error_msg))
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
