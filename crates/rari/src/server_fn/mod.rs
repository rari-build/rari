use crate::error::RariError;
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub mod executor;
pub mod js_fn;
pub mod registry;

const DEFAULT_SERVER_FUNCTION_VERSION: &str = "1.0";
const SERVER_FUNCTION_PREFIX: &str = "server-function:";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerFunctionBindingType {
    None,
    Arguments(Vec<ServerFunctionValue>),
    This(Box<ServerFunctionValue>),
    Both { this: Box<ServerFunctionValue>, arguments: Vec<ServerFunctionValue> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub transport_used: ServerFunctionTransport,
    pub result: ServerFunctionValue,
    pub hydration_state: HydrationState,
    pub binding: Option<ServerFunctionBindingType>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServerFunctionTransport {
    WebSocket,
    Http,
    Hybrid,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServerFunctionType {
    Action,
    Query,
    Mutation,
    Subscription,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HydrationState {
    Idle,
    Loading,
    Ready,
    Error,
}

pub use executor::ServerFunctionExecutor;
pub use js_fn::{JsServerFunction, JsServerFunctionManager};
pub use registry::{
    RegisterFunctionsRequest, ServerFunction as RscServerFunction, ServerFunctionRegistration,
    ServerFunctionRegistry,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Option<ServerFunctionValue>,
    pub warnings: Option<ServerFunctionValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerFunctionValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ServerFunctionValue>),
    Object(FxHashMap<String, ServerFunctionValue>),
    Null,
    ServerReference {
        #[serde(rename = "$$typeof")]
        type_of: String,
        #[serde(rename = "$$id")]
        id: String,
        #[serde(rename = "$$async")]
        is_async: bool,
        #[serde(rename = "$$bound")]
        bound_args: Option<Vec<ServerFunctionValue>>,
    },
}

impl From<ServerFunctionValue> for serde_json::Value {
    fn from(value: ServerFunctionValue) -> Self {
        match value {
            ServerFunctionValue::String(s) => serde_json::Value::String(s),
            ServerFunctionValue::Number(n) => convert_number_to_json(n),
            ServerFunctionValue::Boolean(b) => serde_json::Value::Bool(b),
            ServerFunctionValue::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(serde_json::Value::from).collect())
            }
            ServerFunctionValue::Object(map) => {
                let mut result = serde_json::Map::new();
                for (k, v) in map {
                    result.insert(k, serde_json::Value::from(v));
                }
                serde_json::Value::Object(result)
            }
            ServerFunctionValue::Null => serde_json::Value::Null,
            ServerFunctionValue::ServerReference { type_of, id, is_async, bound_args } => {
                create_server_reference_json(type_of, id, is_async, bound_args)
            }
        }
    }
}

fn convert_number_to_json(n: f64) -> serde_json::Value {
    if n.fract() == 0.0 && n <= i64::MAX as f64 && n >= i64::MIN as f64 {
        serde_json::Value::Number(serde_json::Number::from(n as i64))
    } else {
        match serde_json::Number::from_f64(n) {
            Some(num) => serde_json::Value::Number(num),
            None => serde_json::Value::Null,
        }
    }
}

fn create_server_reference_json(
    type_of: String,
    id: String,
    is_async: bool,
    bound_args: Option<Vec<ServerFunctionValue>>,
) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("$$typeof".to_string(), serde_json::Value::String(type_of));
    map.insert("$$id".to_string(), serde_json::Value::String(id));
    map.insert("$$async".to_string(), serde_json::Value::Bool(is_async));
    if let Some(args) = bound_args {
        map.insert(
            "$$bound".to_string(),
            serde_json::Value::Array(args.into_iter().map(serde_json::Value::from).collect()),
        );
    }
    serde_json::Value::Object(map)
}

impl From<serde_json::Value> for ServerFunctionValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(s) => ServerFunctionValue::String(s),
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    ServerFunctionValue::Number(f)
                } else {
                    ServerFunctionValue::Null
                }
            }
            serde_json::Value::Bool(b) => ServerFunctionValue::Boolean(b),
            serde_json::Value::Array(arr) => {
                ServerFunctionValue::Array(arr.into_iter().map(ServerFunctionValue::from).collect())
            }
            serde_json::Value::Object(map) => {
                let mut result = FxHashMap::default();
                for (k, v) in map {
                    result.insert(k, ServerFunctionValue::from(v));
                }
                ServerFunctionValue::Object(result)
            }
            serde_json::Value::Null => ServerFunctionValue::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFunctionMetadata {
    pub version: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub properties: FxHashMap<String, String>,
}

impl Default for ServerFunctionMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerFunctionMetadata {
    pub fn new() -> Self {
        let now = current_timestamp();

        Self {
            version: DEFAULT_SERVER_FUNCTION_VERSION.to_string(),
            created_at: now,
            updated_at: now,
            properties: FxHashMap::default(),
        }
    }

    pub fn update(&mut self) {
        self.updated_at = current_timestamp();
    }

    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.properties.get(key)
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before UNIX_EPOCH")
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFunctionConfig {
    pub id: String,
    pub function_type: ServerFunctionType,
    pub transport: ServerFunctionTransport,
    pub progressive_enhancement_url: Option<String>,
    pub metadata: Option<ServerFunctionMetadata>,
}

impl ServerFunctionConfig {
    pub async fn validate_with_transport(
        &self,
        args: Vec<String>,
        _transport: ServerFunctionTransport,
    ) -> Result<ValidationResult, RariError> {
        if args.is_empty() {
            return Ok(ValidationResult {
                is_valid: false,
                errors: Some(ServerFunctionValue::String("No arguments provided".to_string())),
                warnings: None,
            });
        }

        Ok(ValidationResult { is_valid: true, errors: None, warnings: None })
    }

    pub fn serialize_for_transport<T: serde::Serialize>(
        &self,
        data: &T,
        transport: ServerFunctionTransport,
    ) -> Result<Vec<u8>, RariError> {
        match transport {
            ServerFunctionTransport::Http | ServerFunctionTransport::Hybrid => {
                serde_json::to_vec(data).map_err(|e| {
                    RariError::serialization(format!("JSON serialization failed: {e}"))
                })
            }
            ServerFunctionTransport::WebSocket | ServerFunctionTransport::Custom => {
                serde_json::to_vec(data)
                    .map_err(|e| RariError::serialization(format!("Serialization failed: {e}")))
            }
        }
    }

    pub fn deserialize_from_transport<T: serde::de::DeserializeOwned>(
        &self,
        data: &[u8],
        transport: ServerFunctionTransport,
    ) -> Result<T, RariError> {
        match transport {
            ServerFunctionTransport::Http | ServerFunctionTransport::Hybrid => {
                let json_value: serde_json::Value = serde_json::from_slice(data).map_err(|e| {
                    RariError::deserialization(format!("HTTP deserialization failed: {e}"))
                })?;

                preserve_primitive_types(json_value)
            }
            ServerFunctionTransport::WebSocket | ServerFunctionTransport::Custom => {
                serde_json::from_slice(data)
                    .map_err(|e| RariError::deserialization(format!("Deserialization failed: {e}")))
            }
        }
    }

    pub async fn validate(
        &self,
        _args: Vec<String>,
        _transport: ServerFunctionTransport,
    ) -> Result<ValidationResult, RariError> {
        self.validate_with_transport(_args, _transport).await
    }
}

fn preserve_primitive_types<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, RariError> {
    let conversion_result = match value {
        serde_json::Value::String(s) => preserve_string_type(&s)
            .or_else(|_| serde_json::from_value(serde_json::Value::String(s))),
        other => serde_json::from_value(other),
    };

    conversion_result
        .map_err(|e| RariError::deserialization(format!("Type preservation failed: {e}")))
}

fn preserve_string_type<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(&format!("\"{s}\""))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFunctionContext {
    pub id: String,
    pub is_pending: bool,
    pub last_response: Option<ServerFunctionValue>,
    pub hydration_state: HydrationState,
    pub in_transition: bool,
    pub transport: ServerFunctionTransport,
    pub replay_id: Option<String>,
    pub metadata: Option<ServerFunctionMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFunctionResult {
    pub id: String,
    pub success: bool,
    pub data: Option<ServerFunctionValue>,
    pub error: Option<String>,
    pub metadata: Option<ServerFunctionMetadata>,
    pub transition_complete: bool,
    pub hydration_complete: bool,
}

#[derive(Debug, Clone)]
pub struct ServerFunctionInfo {
    pub id: String,
    pub file_path: String,
    pub source_code: String,
    pub function_name: String,
    pub line_number: usize,
}

pub struct ServerFunctionMessageHandler {
    pub manager: Arc<RwLock<ServerFunctionManager>>,
    pub js_fn_manager: Option<Arc<js_fn::JsServerFunctionManager>>,
}

type ServerFunctionMap = DashMap<String, Box<dyn ServerFunction>>;

impl ServerFunctionMessageHandler {
    pub async fn handle_http_request(
        &self,
        function_id: String,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, RariError> {
        let result = {
            let manager = self.manager.read().await;
            manager.execute_function(&function_id, &args).await
        };

        match result {
            Ok(value) => Ok(value),
            Err(err) => Err(err),
        }
    }
}

#[async_trait::async_trait]
pub trait ServerFunction: Send + Sync + std::fmt::Debug {
    async fn execute(&self, args: Vec<serde_json::Value>) -> Result<serde_json::Value, RariError>;
    fn get_id(&self) -> String;
}

pub struct ServerFunctionManager {
    pub functions: ServerFunctionMap,
}

impl ServerFunctionManager {
    pub fn new() -> Self {
        Self { functions: DashMap::new() }
    }

    pub async fn register(&mut self, id: String, function: impl ServerFunction + 'static) {
        self.functions.insert(id, Box::new(function));
    }

    pub async fn execute_function(
        &self,
        function_id: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, RariError> {
        let function = self.find_function(function_id)?;
        function.execute(args.to_vec()).await
    }

    fn find_function(
        &self,
        function_id: &str,
    ) -> Result<dashmap::mapref::one::Ref<String, Box<dyn ServerFunction>>, RariError> {
        if let Some(function) = self.functions.get(function_id) {
            return Ok(function);
        }

        let id_without_prefix =
            function_id.strip_prefix(SERVER_FUNCTION_PREFIX).unwrap_or(function_id);
        if let Some(function) = self.functions.get(id_without_prefix) {
            return Ok(function);
        }

        let normalized_id = function_id.replace('\\', "/");
        if normalized_id != function_id
            && let Some(function) = self.functions.get(&normalized_id)
        {
            return Ok(function);
        }

        let lowercase_id = function_id.to_lowercase();
        if lowercase_id != function_id {
            for entry in self.functions.iter() {
                if entry.key().to_lowercase() == lowercase_id {
                    return Ok(self
                        .functions
                        .get(entry.key())
                        .expect("Function key must exist since we just found it"));
                }
            }
        }

        Err(RariError::not_found(format!("Function '{function_id}' not found")))
    }

    pub fn create_message_handler(&self) -> ServerFunctionMessageHandler {
        ServerFunctionMessageHandler {
            manager: Arc::new(RwLock::new(self.clone())),
            js_fn_manager: None,
        }
    }

    pub fn create_message_handler_with_js_manager(
        &self,
        js_manager: Arc<js_fn::JsServerFunctionManager>,
    ) -> ServerFunctionMessageHandler {
        ServerFunctionMessageHandler {
            manager: Arc::new(RwLock::new(self.clone())),
            js_fn_manager: Some(js_manager),
        }
    }

    pub async fn register_from_manifest(&mut self, manifest_path: &str) -> Result<(), RariError> {
        use crate::server_fn::js_fn::JsServerFunction;

        let manifest = self.load_manifest(manifest_path)?;
        let functions = self.parse_manifest_functions(manifest)?;

        for (i, func) in functions.iter().enumerate() {
            let (id, path) = self.extract_function_info(func, i)?;
            let js_fn = JsServerFunction::new_simple(id.clone(), path.clone());
            self.register(id, js_fn).await;
        }

        Ok(())
    }

    fn load_manifest(&self, manifest_path: &str) -> Result<serde_json::Value, RariError> {
        use std::fs;
        use std::path::Path;

        if !Path::new(manifest_path).exists() {
            return Err(RariError::not_found(format!(
                "Server function manifest not found: {manifest_path}"
            )));
        }

        let manifest_content = fs::read_to_string(manifest_path)
            .map_err(|e| RariError::io(format!("Failed to read manifest: {e}")))?;

        serde_json::from_str(&manifest_content)
            .map_err(|e| RariError::serialization(format!("Failed to parse manifest: {e}")))
    }

    fn parse_manifest_functions(
        &self,
        manifest: serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, RariError> {
        manifest
            .as_array()
            .ok_or_else(|| RariError::validation("Manifest is not an array"))
            .cloned()
    }

    fn extract_function_info(
        &self,
        func: &serde_json::Value,
        index: usize,
    ) -> Result<(String, String), RariError> {
        let id = func
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RariError::validation(format!("Function {index} has invalid id")))?;

        let path = func
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RariError::validation(format!("Function {index} has invalid path")))?;

        Ok((id.to_string(), path.to_string()))
    }
}

impl Clone for ServerFunctionManager {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl Default for ServerFunctionManager {
    fn default() -> Self {
        Self::new()
    }
}
