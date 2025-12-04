use crate::error::RariError;
use serde_json::Value as JsonValue;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

pub trait JsRuntimeInterface: Send + Sync {
    fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>>;

    fn execute_function(
        &self,
        function_name: &str,
        args: Vec<JsonValue>,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send + 'static>>;

    fn execute_script_for_streaming(
        &self,
        script_name: String,
        script_code: String,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn add_module_to_loader(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;

    fn load_es_module(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>>;

    fn evaluate_module(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>>;

    fn get_module_namespace(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>>;

    fn add_module_to_loader_only(
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
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>>;
}
