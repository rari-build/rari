use std::sync::Arc;

use deno_core::ModuleId;
use rari_error::RariError;
use serde_json::Value as JsonValue;

use super::super::interface::{AsyncBatchResult, JsRuntimeInterface};

macro_rules! forward_async_to_runtime {
    ($(
        pub async fn $name:ident(&self $(, $($arg:ident: $arg_ty:ty),*)?) -> $ret:ty;
    )*) => {
        $(
            pub async fn $name(&self $(, $($arg: $arg_ty),*)?) -> $ret {
                self.runtime.$name($($($arg),*)?).await
            }
        )*
    };
}

pub struct PooledRuntime {
    idx: usize,
    runtime: Arc<dyn JsRuntimeInterface>,
}

impl PooledRuntime {
    pub(super) fn new(idx: usize, runtime: Arc<dyn JsRuntimeInterface>) -> Self {
        Self { idx, runtime }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn runtime(&self) -> &Arc<dyn JsRuntimeInterface> {
        &self.runtime
    }

    pub fn execute_script_batch(&self, scripts: Vec<(String, String)>) -> AsyncBatchResult {
        self.runtime.execute_script_batch(scripts)
    }

    forward_async_to_runtime! {
        pub async fn execute_script(&self, script_name: String, script_code: String) -> Result<JsonValue, RariError>;
        pub async fn execute_function(&self, function_name: &str, args: Vec<JsonValue>) -> Result<JsonValue, RariError>;
        pub async fn add_module_to_loader(&self, specifier: &str) -> Result<(), RariError>;
        pub async fn add_module_to_loader_only(&self, specifier: &str, code: String) -> Result<(), RariError>;
        pub async fn clear_module_loader_caches(&self, component_id: &str) -> Result<(), RariError>;
        pub async fn load_es_module(&self, specifier: &str) -> Result<ModuleId, RariError>;
        pub async fn evaluate_module(&self, module_id: ModuleId) -> Result<JsonValue, RariError>;
        pub async fn get_module_namespace(&self, module_id: ModuleId) -> Result<JsonValue, RariError>;
        pub async fn set_request_context(&self, request_context: Arc<crate::server::middleware::request_context::RequestContext>) -> Result<(), RariError>;
        pub async fn clear_request_context(&self) -> Result<(), RariError>;
        pub async fn clear_request_context_if_matches(&self, expected_context: Arc<crate::server::middleware::request_context::RequestContext>) -> Result<(), RariError>;
    }
}
