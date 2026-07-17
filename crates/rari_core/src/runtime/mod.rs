use std::{future::Future, sync::Arc, time::Duration};

use deno_core::ModuleId;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde_json::Value;
use tokio::{
    sync::mpsc::{Sender, UnboundedReceiver},
    time,
};

pub mod ext;
pub mod factory;
pub mod module_loader;
pub mod ops;
pub mod transpile;

use factory::JsRuntimeInterface;

use crate::{
    metadata,
    request_context::RequestContext,
    routing::types::ParamValue,
    runtime::factory::{
        RariRuntime,
        component_ops::{
            build_invalidate_script, invalidate_script_name, load_component_code as load_component,
        },
    },
};

pub struct JsExecutionRuntime {
    runtime: Arc<RariRuntime>,
    timeout_ms: u64,
}

impl Default for JsExecutionRuntime {
    fn default() -> Self {
        Self::new(None)
    }
}

fn parse_string_array_value(value: &Value) -> Vec<String> {
    if let Some(items) = value.as_array() {
        return items.iter().filter_map(|item| item.as_str().map(ToString::to_string)).collect();
    }

    if let Some(text) = value.as_str() {
        return serde_json::from_str(text).unwrap_or_default();
    }

    Vec::new()
}

#[expect(clippy::missing_errors_doc)]
impl JsExecutionRuntime {
    pub fn new(env_vars: Option<FxHashMap<String, String>>) -> Self {
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

        match time::timeout(
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
    ) -> UnboundedReceiver<(usize, Result<Value, RariError>)> {
        self.runtime.execute_script_batch(scripts).await
    }

    pub async fn execute_script_for_streaming(
        &self,
        script_name: String,
        script_code: String,
        chunk_sender: Sender<Result<Vec<u8>, RariError>>,
    ) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);
        runtime.execute_script_for_streaming(script_name, script_code, chunk_sender).await
    }

    pub async fn collect_metadata(
        &self,
        layout_paths: Vec<String>,
        page_path: String,
        params: FxHashMap<String, ParamValue>,
        search_params: FxHashMap<String, Vec<String>>,
    ) -> Result<Value, RariError> {
        let data = serde_json::json!({
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

        let mut merged_metadata = serde_json::json!({});
        for metadata_item in metadata_array {
            merged_metadata = metadata::merge_metadata(&merged_metadata, metadata_item);
        }

        metadata::finalize_metadata(&mut merged_metadata);

        Ok(merged_metadata)
    }

    pub async fn collect_page_cache_tags(&self) -> Result<Vec<String>, RariError> {
        const SCRIPT: &str = r"(() => {
            const tags = new Set(
                globalThis['~rari']?.pageCacheTags ? [...globalThis['~rari'].pageCacheTags] : [],
            );
            const fromRegistry = globalThis.__rariGetActiveUseCacheTags?.() ?? [];
            for (const tag of fromRegistry)
                tags.add(tag);
            return [...tags];
        })()";

        let result =
            self.execute_script("collect_page_cache_tags".to_string(), SCRIPT.to_string()).await?;

        Ok(parse_string_array_value(&result))
    }

    pub async fn is_dynamic_render(&self) -> Result<bool, RariError> {
        const SCRIPT: &str = "((globalThis['~rari']?.useCacheDynamicDepth ?? 0) > 0)";

        let result =
            self.execute_script("is_dynamic_render".to_string(), SCRIPT.to_string()).await?;

        Ok(result.as_bool().unwrap_or(false))
    }

    pub async fn execute_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RariError> {
        let runtime = Arc::clone(&self.runtime);
        let function_name = function_name.to_string();

        match time::timeout(
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

    pub async fn load_es_module(&self, specifier: &str) -> Result<ModuleId, RariError> {
        let runtime = Arc::clone(&self.runtime);
        let specifier = specifier.to_string();

        match time::timeout(
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

    pub async fn evaluate_module(&self, module_id: ModuleId) -> Result<Value, RariError> {
        let runtime = Arc::clone(&self.runtime);

        match time::timeout(
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

    pub async fn add_module_to_loader(
        &self,
        specifier: &str,
        code: String,
    ) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);
        let specifier = specifier.to_string();

        match time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.add_module_to_loader(&specifier, code),
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

    pub async fn clear_module_loader_caches(&self, component_id: &str) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);
        let component_id = component_id.to_string();

        match time::timeout(
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

    pub async fn get_module_namespace(&self, module_id: ModuleId) -> Result<Value, RariError> {
        let runtime = Arc::clone(&self.runtime);

        match time::timeout(
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
        let script = build_invalidate_script(component_id);

        match self.execute_script(invalidate_script_name(component_id), script).await {
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
        let runtime = Arc::clone(&self.runtime);
        let component_id = component_id.to_string();
        let component_code = component_code.to_string();

        match time::timeout(
            Duration::from_millis(self.timeout_ms),
            load_component(runtime.as_ref(), &component_id, &component_code),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Loading component {component_id} timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn execute_script_with_request_context(
        &self,
        request_context: Arc<RequestContext>,
        script_name: String,
        script_code: String,
    ) -> Result<Value, RariError> {
        let runtime = Arc::clone(&self.runtime);

        match time::timeout(
            Duration::from_millis(self.timeout_ms),
            runtime.execute_script_with_request_context(request_context, script_name, script_code),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(RariError::timeout(format!(
                "Script execution with request context timed out after {} ms",
                self.timeout_ms
            ))),
        }
    }

    pub async fn set_request_context(
        &self,
        request_context: Arc<RequestContext>,
    ) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);

        match time::timeout(
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

        match time::timeout(Duration::from_millis(self.timeout_ms), runtime.clear_request_context())
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
        expected_context: Arc<RequestContext>,
    ) -> Result<(), RariError> {
        let runtime = Arc::clone(&self.runtime);

        match time::timeout(
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
        request_context: Arc<RequestContext>,
        operation: F,
    ) -> Result<T, RariError>
    where
        F: Future<Output = Result<T, RariError>>,
    {
        self.set_request_context(request_context).await?;

        let result = operation.await;

        let clear_result = self.clear_request_context().await;

        match (result, clear_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(value), Err(clear_err)) => {
                tracing::error!(
                    "Failed to clear request context after successful operation: {}",
                    clear_err
                );
                Ok(value)
            }
            (Err(op_err), Err(clear_err)) => {
                tracing::error!(
                    "Failed to clear request context after operation error: {}",
                    clear_err
                );
                Err(op_err)
            }
            (Err(op_err), Ok(())) => Err(op_err),
        }
    }

    pub async fn execute_with_persistent_request_context<F, T>(
        &self,
        request_context: Arc<RequestContext>,
        operation: F,
    ) -> Result<T, RariError>
    where
        F: Future<Output = Result<T, RariError>>,
    {
        self.set_request_context(request_context).await?;
        operation.await
    }
}
