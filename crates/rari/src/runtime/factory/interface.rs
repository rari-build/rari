use std::{future::Future, pin::Pin, sync::Arc};

use rari_error::RariError;
use serde_json::Value;
use tokio::sync::{mpsc, mpsc::Sender};

use crate::server::middleware::request_context::RequestContext;

pub type BatchResultReceiver = mpsc::UnboundedReceiver<(usize, Result<Value, RariError>)>;
pub type AsyncBatchResult = Pin<Box<dyn Future<Output = BatchResultReceiver> + Send>>;

pub trait JsRuntimeInterface: Send + Sync {
    fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>>;

    fn execute_script_batch(&self, scripts: Vec<(String, String)>) -> AsyncBatchResult;

    fn execute_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send + 'static>>;

    fn load_es_module(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>>;

    fn evaluate_module(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>>;

    fn get_module_namespace(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>>;

    fn add_module_to_loader(
        &self,
        specifier: &str,
        code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn clear_module_loader_caches(
        &self,
        component_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn set_request_context(
        &self,
        request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn clear_request_context(&self) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn clear_request_context_if_matches(
        &self,
        expected_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn execute_script_with_request_context(
        &self,
        request_context: Arc<RequestContext>,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<Value, RariError>> + Send>>;

    fn execute_script_for_streaming(
        &self,
        script_name: String,
        script_code: String,
        chunk_sender: Sender<Result<Vec<u8>, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;
}
