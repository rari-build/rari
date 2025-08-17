use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Error {
    #[error("{0} has no entrypoint. Register one, or add a default to the runtime")]
    MissingEntrypoint(String),

    #[error("{0} could not be found in global, or module exports")]
    ValueNotFound(String),

    #[error("{0} is not a function")]
    ValueNotCallable(String),

    #[error("{0} could not be encoded as a v8 value")]
    V8Encoding(String),

    #[error("value could not be deserialized: {0}")]
    JsonDecode(String),

    #[error("{0}")]
    ModuleNotFound(String),

    #[error("This worker has been destroyed")]
    WorkerHasStopped,

    #[error("{0}")]
    Runtime(String),

    #[error("{0}")]
    JsError(Box<deno_core::error::JsError>),

    #[error("Module timed out: {0}")]
    Timeout(String),

    #[error("Heap exhausted")]
    HeapExhausted,
}

impl From<deno_core::error::CoreError> for Error {
    fn from(e: deno_core::error::CoreError) -> Self {
        use deno_core::error::CoreErrorKind;

        let error_string = e.to_string();
        match e.into_kind() {
            CoreErrorKind::Js(js_error) => Error::JsError(Box::new(js_error)),
            CoreErrorKind::Io(_) => Error::Runtime(error_string),
            CoreErrorKind::Parse(_) => {
                Error::Runtime(format!("Module parse error: {error_string}"))
            }
            CoreErrorKind::Execute(_) => {
                Error::Runtime(format!("Module execution error: {error_string}"))
            }
            CoreErrorKind::MissingFromModuleMap(module) => Error::ModuleNotFound(module),
            CoreErrorKind::UnusedModules(_) => {
                Error::Runtime(format!("Unused modules: {error_string}"))
            }
            CoreErrorKind::NonEvaluatedModules(_) => {
                Error::Runtime(format!("Non-evaluated modules: {error_string}"))
            }
            CoreErrorKind::Url(_) => Error::Runtime(format!("URL parse error: {error_string}")),
            CoreErrorKind::JsBox(_) => Error::Runtime(error_string),
            CoreErrorKind::ExtensionTranspiler(_) => {
                Error::Runtime(format!("Extension transpiler error: {error_string}"))
            }
            CoreErrorKind::ExecutionTerminated => {
                Error::Runtime("JavaScript execution terminated".to_string())
            }
            CoreErrorKind::PendingPromiseResolution => {
                Error::Runtime("Pending promise resolution error".to_string())
            }
            CoreErrorKind::EvaluateDynamicImportedModule => {
                Error::Runtime("Dynamic import evaluation error".to_string())
            }
            CoreErrorKind::TLA => Error::Runtime("Top-level await error".to_string()),
            CoreErrorKind::CouldNotExecute { .. } => {
                Error::Runtime(format!("Could not execute: {error_string}"))
            }
            CoreErrorKind::Module(_) => Error::Runtime(format!("Module error: {error_string}")),
            CoreErrorKind::Data(_) => Error::JsonDecode(error_string),
            CoreErrorKind::CreateCodeCache(_) => {
                Error::Runtime(format!("Code cache creation error: {error_string}"))
            }
            CoreErrorKind::ExtensionSnapshotMismatch(_) => {
                Error::Runtime(format!("Extension snapshot mismatch: {error_string}"))
            }
            CoreErrorKind::ExtensionLazyInitCountMismatch(_) => {
                Error::Runtime(format!("Extension lazy init count mismatch: {error_string}"))
            }
            CoreErrorKind::ExtensionLazyInitOrderMismatch(_) => {
                Error::Runtime(format!("Extension lazy init order mismatch: {error_string}"))
            }
        }
    }
}

impl From<deno_core::error::JsError> for Error {
    fn from(e: deno_core::error::JsError) -> Self {
        Error::JsError(Box::new(e))
    }
}

impl From<std::cell::BorrowMutError> for Error {
    fn from(e: std::cell::BorrowMutError) -> Self {
        Error::Runtime(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::JsonDecode(e.to_string())
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::Runtime(e)
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::Runtime(e.to_string())
    }
}

impl From<deno_broadcast_channel::BroadcastChannelError> for Error {
    fn from(e: deno_broadcast_channel::BroadcastChannelError) -> Self {
        Error::Runtime(e.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMetadata {
    pub code: String,
    pub details: Option<FxHashMap<String, String>>,
    pub source: Option<String>,
    #[serde(skip)]
    pub error_source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Clone for ErrorMetadata {
    fn clone(&self) -> Self {
        Self {
            code: self.code.clone(),
            details: self.details.clone(),
            source: self.source.clone(),
            error_source: None,
        }
    }
}

impl PartialEq for ErrorMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code && self.details == other.details && self.source == other.source
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RariError {
    NotFound(String, Option<ErrorMetadata>),
    Validation(String, Option<ErrorMetadata>),
    Internal(String, Option<ErrorMetadata>),
    BadRequest(String, Option<ErrorMetadata>),
    Serialization(String, Option<ErrorMetadata>),
    Deserialization(String, Option<ErrorMetadata>),
    State(String, Option<ErrorMetadata>),
    Network(String, Option<ErrorMetadata>),
    Timeout(String, Option<ErrorMetadata>),
    ServerError(String, Option<ErrorMetadata>),
    JsExecution(String, Option<ErrorMetadata>),
    JsRuntime(String, Option<ErrorMetadata>),
    IoError(String, Option<ErrorMetadata>),
}

impl std::fmt::Display for RariError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg, _) => write!(f, "Not found: {msg}"),
            Self::Validation(msg, _) => write!(f, "Validation error: {msg}"),
            Self::Internal(msg, _) => write!(f, "{msg}"),
            Self::BadRequest(msg, _) => write!(f, "Bad request: {msg}"),
            Self::Serialization(msg, _) => write!(f, "Serialization error: {msg}"),
            Self::Deserialization(msg, _) => write!(f, "Deserialization error: {msg}"),
            Self::State(msg, _) => write!(f, "State error: {msg}"),
            Self::Network(msg, _) => write!(f, "Network error: {msg}"),
            Self::Timeout(msg, _) => write!(f, "Timeout error: {msg}"),
            Self::ServerError(msg, _) => write!(f, "Server error: {msg}"),
            Self::JsExecution(msg, _) => write!(f, "JavaScript execution error: {msg}"),
            Self::JsRuntime(msg, _) => write!(f, "JavaScript runtime error: {msg}"),
            Self::IoError(msg, _) => write!(f, "I/O error: {msg}"),
        }
    }
}

impl std::error::Error for RariError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.metadata()
            .and_then(|meta| meta.error_source.as_ref())
            .map(|source| source.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl RariError {
    pub fn message(&self) -> String {
        match self {
            Self::NotFound(msg, _) => msg.clone(),
            Self::Validation(msg, _) => msg.clone(),
            Self::Internal(msg, _) => msg.clone(),
            Self::BadRequest(msg, _) => msg.clone(),
            Self::Serialization(msg, _) => msg.clone(),
            Self::Deserialization(msg, _) => msg.clone(),
            Self::State(msg, _) => msg.clone(),
            Self::Network(msg, _) => msg.clone(),
            Self::Timeout(msg, _) => msg.clone(),
            Self::ServerError(msg, _) => msg.clone(),
            Self::JsExecution(msg, _) => msg.clone(),
            Self::JsRuntime(msg, _) => msg.clone(),
            Self::IoError(msg, _) => msg.clone(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_, _) => "NOT_FOUND",
            Self::Validation(_, _) => "VALIDATION",
            Self::Internal(_, _) => "INTERNAL",
            Self::BadRequest(_, _) => "BAD_REQUEST",
            Self::Serialization(_, _) => "SERIALIZATION_ERROR",
            Self::Deserialization(_, _) => "DESERIALIZATION_ERROR",
            Self::State(_, _) => "STATE_ERROR",
            Self::Network(_, _) => "NETWORK",
            Self::Timeout(_, _) => "TIMEOUT_ERROR",
            Self::ServerError(_, _) => "SERVER_ERROR",
            Self::JsExecution(_, _) => "JS_EXECUTION_ERROR",
            Self::JsRuntime(_, _) => "JS_RUNTIME_ERROR",
            Self::IoError(_, _) => "IO_ERROR",
        }
    }

    fn metadata(&self) -> Option<&ErrorMetadata> {
        match self {
            Self::NotFound(_, meta) => meta.as_ref(),
            Self::Validation(_, meta) => meta.as_ref(),
            Self::Internal(_, meta) => meta.as_ref(),
            Self::BadRequest(_, meta) => meta.as_ref(),
            Self::Serialization(_, meta) => meta.as_ref(),
            Self::Deserialization(_, meta) => meta.as_ref(),
            Self::State(_, meta) => meta.as_ref(),
            Self::Network(_, meta) => meta.as_ref(),
            Self::Timeout(_, meta) => meta.as_ref(),
            Self::ServerError(_, meta) => meta.as_ref(),
            Self::JsExecution(_, meta) => meta.as_ref(),
            Self::JsRuntime(_, meta) => meta.as_ref(),
            Self::IoError(_, meta) => meta.as_ref(),
        }
    }

    fn metadata_mut(&mut self) -> &mut Option<ErrorMetadata> {
        match self {
            Self::NotFound(_, meta) => meta,
            Self::Validation(_, meta) => meta,
            Self::Internal(_, meta) => meta,
            Self::BadRequest(_, meta) => meta,
            Self::Serialization(_, meta) => meta,
            Self::Deserialization(_, meta) => meta,
            Self::State(_, meta) => meta,
            Self::Network(_, meta) => meta,
            Self::Timeout(_, meta) => meta,
            Self::ServerError(_, meta) => meta,
            Self::JsExecution(_, meta) => meta,
            Self::JsRuntime(_, meta) => meta,
            Self::IoError(_, meta) => meta,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into(), None)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into(), None)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into(), None)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into(), None)
    }

    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization(message.into(), None)
    }

    pub fn deserialization(message: impl Into<String>) -> Self {
        Self::Deserialization(message.into(), None)
    }

    pub fn state(message: impl Into<String>) -> Self {
        Self::State(message.into(), None)
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::Network(message.into(), None)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout(message.into(), None)
    }

    pub fn server_error(message: impl Into<String>) -> Self {
        Self::ServerError(message.into(), None)
    }

    pub fn js_execution(message: impl Into<String>) -> Self {
        Self::JsExecution(message.into(), None)
    }

    pub fn js_runtime(message: impl Into<String>) -> Self {
        Self::JsRuntime(message.into(), None)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::IoError(message.into(), None)
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Internal(message.into(), None)
    }

    pub fn parsing(message: impl Into<String>) -> Self {
        Self::Deserialization(message.into(), None)
    }

    pub fn initialization(message: impl Into<String>) -> Self {
        Self::Internal(message.into(), None)
    }

    pub fn server_function_error(message: impl Into<String>) -> Self {
        Self::ServerError(message.into(), None)
    }

    pub fn with_source(mut self, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        let code = self.code().to_string();
        let metadata = self.metadata_mut();
        let mut new_meta = metadata.clone().unwrap_or_else(|| ErrorMetadata {
            code,
            details: Some(FxHashMap::default()),
            source: None,
            error_source: None,
        });
        new_meta.source = Some(source.to_string());
        new_meta.error_source = Some(source);
        *metadata = Some(new_meta);
        self
    }

    pub fn with_property(mut self, key: &str, value: &str) -> Self {
        self.set_property(key, value);
        self
    }

    pub fn set_property(&mut self, key: &str, value: &str) {
        let code = self.code().to_string();
        let metadata = self.metadata_mut();
        if metadata.is_none() {
            *metadata = Some(ErrorMetadata {
                code,
                details: Some(FxHashMap::default()),
                source: None,
                error_source: None,
            });
        }

        if let Some(meta) = metadata {
            if meta.details.is_none() {
                meta.details = Some(FxHashMap::default());
            }
            if let Some(details) = &mut meta.details {
                details.insert(key.to_string(), value.to_string());
            }
        }
    }

    pub fn get_property(&self, key: &str) -> Option<&str> {
        self.metadata()
            .and_then(|meta| meta.details.as_ref())
            .and_then(|details| details.get(key))
            .map(String::as_str)
    }

    pub fn remove_property(&mut self, key: &str) {
        if let Some(meta) = self.metadata_mut()
            && let Some(details) = meta.details.as_mut()
        {
            details.remove(key);
        }
    }
}

impl From<std::io::Error> for RariError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(
            error.to_string(),
            Some(ErrorMetadata {
                code: "IO_ERROR".to_string(),
                details: None,
                source: Some("std::io::Error".to_string()),
                error_source: None,
            }),
        )
    }
}

impl From<tokio::time::error::Elapsed> for RariError {
    fn from(error: tokio::time::error::Elapsed) -> Self {
        Self::Timeout(error.to_string(), None)
    }
}

impl From<String> for RariError {
    fn from(error: String) -> Self {
        Self::Internal(error, None)
    }
}

impl From<&str> for RariError {
    fn from(error: &str) -> Self {
        Self::Internal(error.to_string(), None)
    }
}

impl From<serde_json::Error> for RariError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(
            error.to_string(),
            Some(ErrorMetadata {
                code: "JSON_ERROR".to_string(),
                details: None,
                source: Some("serde_json".to_string()),
                error_source: None,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deno_core::error::{CoreError, CoreErrorKind};

    #[test]
    fn test_core_error_io_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let core_error = CoreError::from(CoreErrorKind::Io(io_error));
        let error: Error = core_error.into();

        match error {
            Error::Runtime(msg) => {
                assert!(msg.contains("File not found"));
            }
            _ => panic!("Expected Runtime error variant"),
        }
    }

    #[test]
    fn test_core_error_module_not_found_conversion() {
        let module_name = "missing_module".to_string();
        let core_error = CoreError::from(CoreErrorKind::MissingFromModuleMap(module_name.clone()));
        let error: Error = core_error.into();

        match error {
            Error::ModuleNotFound(name) => {
                assert_eq!(name, module_name);
            }
            _ => panic!("Expected ModuleNotFound variant"),
        }
    }

    #[test]
    fn test_core_error_execution_terminated() {
        let core_error = CoreError::from(CoreErrorKind::ExecutionTerminated);
        let error: Error = core_error.into();

        match error {
            Error::Runtime(msg) => {
                assert_eq!(msg, "JavaScript execution terminated");
            }
            _ => panic!("Expected Runtime error variant"),
        }
    }

    #[test]
    fn test_core_error_tla_conversion() {
        let core_error = CoreError::from(CoreErrorKind::TLA);
        let error: Error = core_error.into();

        match error {
            Error::Runtime(msg) => {
                assert_eq!(msg, "Top-level await error");
            }
            _ => panic!("Expected Runtime error variant"),
        }
    }

    #[test]
    fn test_core_error_pending_promise_conversion() {
        let core_error = CoreError::from(CoreErrorKind::PendingPromiseResolution);
        let error: Error = core_error.into();

        match error {
            Error::Runtime(msg) => {
                assert_eq!(msg, "Pending promise resolution error");
            }
            _ => panic!("Expected Runtime error variant"),
        }
    }
}
