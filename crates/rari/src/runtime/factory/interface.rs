use std::{future::Future, pin::Pin, sync::Arc};

use rari_error::RariError;
use serde_json::Value;
use tokio::sync::{mpsc, mpsc::Sender};

use crate::server::middleware::request_context::RequestContext;

pub type BatchResultReceiver = mpsc::UnboundedReceiver<(usize, Result<Value, RariError>)>;
pub type AsyncBatchResult = Pin<Box<dyn Future<Output = BatchResultReceiver> + Send>>;
pub type StreamingCompletionFuture = Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;
pub type QueueStreamingScriptFuture =
    Pin<Box<dyn Future<Output = Result<StreamingCompletionFuture, RariError>> + Send>>;

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

    fn clear_request_context_if_matches(
        &self,
        expected_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn execute_script_for_streaming(
        &self,
        stream_id: String,
        script_name: String,
        script_code: String,
        chunk_sender: Sender<Result<Vec<u8>, RariError>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    /// Queue a streaming script on the isolate without waiting for it to finish.
    /// The returned future resolves when the stream completes (or errors).
    /// Optional `request_context` is registered in the same isolate turn.
    fn queue_script_for_streaming(
        &self,
        stream_id: String,
        script_name: String,
        script_code: String,
        chunk_sender: Sender<Result<Vec<u8>, RariError>>,
        request_context: Option<Arc<RequestContext>>,
    ) -> QueueStreamingScriptFuture;

    fn register_request_context(
        &self,
        request_context: Arc<RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn unregister_request_context(
        &self,
        request_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;
}
