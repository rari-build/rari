use std::{sync::Arc, time::Duration};

use deno_core::ModuleId;
use rari_error::RariError;
use serde_json::Value;
use tokio::{sync::mpsc::Sender, time};

use super::super::interface::{AsyncBatchResult, JsRuntimeInterface};
use crate::request_context::RequestContext;

macro_rules! forward_async_to_runtime_with_timeout {
    ($(
        pub async fn $name:ident(&self $(, $($arg:ident: $arg_ty:ty),*)?) -> Result<$ok:ty, RariError>;
    )*) => {
        $(
            pub async fn $name(&self $(, $($arg: $arg_ty),*)?) -> Result<$ok, RariError> {
                match time::timeout(
                    Duration::from_millis(self.timeout_ms),
                    self.runtime.$name($($($arg),*)?),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(RariError::timeout(format!(
                        concat!(stringify!($name), " timed out after {} ms"),
                        self.timeout_ms
                    ))),
                }
            }
        )*
    };
}

pub struct PooledRuntime {
    idx: usize,
    runtime: Arc<dyn JsRuntimeInterface>,
    timeout_ms: u64,
}

impl PooledRuntime {
    pub(super) fn new(idx: usize, runtime: Arc<dyn JsRuntimeInterface>, timeout_ms: u64) -> Self {
        Self { idx, runtime, timeout_ms }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn runtime(&self) -> &Arc<dyn JsRuntimeInterface> {
        &self.runtime
    }

    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    pub fn execute_script_batch(&self, scripts: Vec<(String, String)>) -> AsyncBatchResult {
        self.runtime.execute_script_batch(scripts)
    }

    /// Streaming holds the isolate for the response lifetime; no overall timeout
    /// (matches [`crate::runtime::JsExecutionRuntime::execute_script_for_streaming`]).
    pub async fn execute_script_for_streaming(
        &self,
        script_name: String,
        script_code: String,
        chunk_sender: Sender<Result<Vec<u8>, RariError>>,
    ) -> Result<(), RariError> {
        self.runtime.execute_script_for_streaming(script_name, script_code, chunk_sender).await
    }

    forward_async_to_runtime_with_timeout! {
        pub async fn execute_script(&self, script_name: String, script_code: String) -> Result<Value, RariError>;
        pub async fn execute_function(&self, function_name: &str, args: Vec<Value>) -> Result<Value, RariError>;
        pub async fn add_module_to_loader(&self, specifier: &str, code: String) -> Result<(), RariError>;
        pub async fn clear_module_loader_caches(&self, component_id: &str) -> Result<(), RariError>;
        pub async fn load_es_module(&self, specifier: &str) -> Result<ModuleId, RariError>;
        pub async fn evaluate_module(&self, module_id: ModuleId) -> Result<Value, RariError>;
        pub async fn get_module_namespace(&self, module_id: ModuleId) -> Result<Value, RariError>;
        pub async fn set_request_context(&self, request_context: Arc<RequestContext>) -> Result<(), RariError>;
        pub async fn clear_request_context(&self) -> Result<(), RariError>;
        pub async fn clear_request_context_if_matches(&self, expected_context: Arc<RequestContext>) -> Result<(), RariError>;
        pub async fn execute_script_with_request_context(&self, request_context: Arc<RequestContext>, script_name: String, script_code: String) -> Result<Value, RariError>;
    }
}
