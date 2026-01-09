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
    Js(Box<deno_core::error::JsError>),
    #[error("Module timed out: {0}")]
    Timeout(String),
    #[error("Heap exhausted")]
    HeapExhausted,
}

impl From<deno_core::error::CoreError> for Error {
    fn from(e: deno_core::error::CoreError) -> Self {
        Error::Runtime(e.to_string())
    }
}

impl From<deno_core::error::JsError> for Error {
    fn from(e: deno_core::error::JsError) -> Self {
        Error::Js(Box::new(e))
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
    Forbidden(String, Option<ErrorMetadata>),
    Serialization(String, Option<ErrorMetadata>),
    Deserialization(String, Option<ErrorMetadata>),
    State(String, Option<ErrorMetadata>),
    Network(String, Option<ErrorMetadata>),
    Timeout(String, Option<ErrorMetadata>),
    ServerError(String, Option<ErrorMetadata>),
    JsExecution(String, Option<ErrorMetadata>),
    JsRuntime(String, Option<ErrorMetadata>),
    IoError(String, Option<ErrorMetadata>),
    ModuleReload(Box<ModuleReloadError>, Option<ErrorMetadata>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModuleReloadError {
    SyntaxError {
        message: String,
        file_path: String,
        line: Option<u32>,
        column: Option<u32>,
    },
    RuntimeError {
        message: String,
        file_path: String,
        stack: Option<String>,
        error_name: Option<String>,
    },
    Timeout {
        message: String,
        file_path: String,
        timeout_ms: u64,
    },
    NotFound {
        message: String,
        file_path: String,
    },
    MaxRetriesExceeded {
        message: String,
        file_path: String,
        attempts: usize,
        last_error: Option<String>,
    },
    HelpersNotInitialized {
        message: String,
    },
    RuntimeNotAvailable {
        message: String,
    },
    Other {
        message: String,
        file_path: Option<String>,
    },
}

impl std::fmt::Display for RariError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg, _) => write!(f, "Not found: {msg}"),
            Self::Validation(msg, _) => write!(f, "Validation error: {msg}"),
            Self::Internal(msg, _) => write!(f, "{msg}"),
            Self::BadRequest(msg, _) => write!(f, "Bad request: {msg}"),
            Self::Forbidden(msg, _) => write!(f, "Forbidden: {msg}"),
            Self::Serialization(msg, _) => write!(f, "Serialization error: {msg}"),
            Self::Deserialization(msg, _) => write!(f, "Deserialization error: {msg}"),
            Self::State(msg, _) => write!(f, "State error: {msg}"),
            Self::Network(msg, _) => write!(f, "Network error: {msg}"),
            Self::Timeout(msg, _) => write!(f, "Timeout error: {msg}"),
            Self::ServerError(msg, _) => write!(f, "Server error: {msg}"),
            Self::JsExecution(msg, _) => write!(f, "JavaScript execution error: {msg}"),
            Self::JsRuntime(msg, _) => write!(f, "JavaScript runtime error: {msg}"),
            Self::IoError(msg, _) => write!(f, "I/O error: {msg}"),
            Self::ModuleReload(err, _) => write!(f, "Module reload error: {err}"),
        }
    }
}

impl std::fmt::Display for ModuleReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SyntaxError { message, file_path, line, column } => {
                write!(f, "Syntax error in {file_path}")?;
                if let Some(line) = line {
                    write!(f, " at line {line}")?;
                    if let Some(column) = column {
                        write!(f, ", column {column}")?;
                    }
                }
                write!(f, ": {message}")
            }
            Self::RuntimeError { message, file_path, error_name, .. } => {
                if let Some(name) = error_name {
                    write!(f, "{name} in {file_path}: {message}")
                } else {
                    write!(f, "Runtime error in {file_path}: {message}")
                }
            }
            Self::Timeout { file_path, timeout_ms, .. } => {
                write!(f, "Module reload timed out after {timeout_ms}ms for {file_path}")
            }
            Self::NotFound { file_path, .. } => {
                write!(f, "Module not found: {file_path}")
            }
            Self::MaxRetriesExceeded { file_path, attempts, last_error, .. } => {
                write!(f, "Module reload failed after {attempts} attempts for {file_path}")?;
                if let Some(err) = last_error { write!(f, ". Last error: {err}") } else { Ok(()) }
            }
            Self::HelpersNotInitialized { message } => {
                write!(f, "Module reload helpers not initialized: {message}")
            }
            Self::RuntimeNotAvailable { message } => {
                write!(f, "JavaScript runtime not available: {message}")
            }
            Self::Other { message, file_path } => {
                if let Some(path) = file_path {
                    write!(f, "Module reload error for {path}: {message}")
                } else {
                    write!(f, "Module reload error: {message}")
                }
            }
        }
    }
}

impl ModuleReloadError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::SyntaxError { .. } => "MODULE_RELOAD_SYNTAX_ERROR",
            Self::RuntimeError { .. } => "MODULE_RELOAD_RUNTIME_ERROR",
            Self::Timeout { .. } => "MODULE_RELOAD_TIMEOUT",
            Self::NotFound { .. } => "MODULE_RELOAD_NOT_FOUND",
            Self::MaxRetriesExceeded { .. } => "MODULE_RELOAD_MAX_RETRIES",
            Self::HelpersNotInitialized { .. } => "MODULE_RELOAD_HELPERS_NOT_INITIALIZED",
            Self::RuntimeNotAvailable { .. } => "MODULE_RELOAD_RUNTIME_NOT_AVAILABLE",
            Self::Other { .. } => "MODULE_RELOAD_ERROR",
        }
    }

    pub fn file_path(&self) -> Option<&str> {
        match self {
            Self::SyntaxError { file_path, .. } => Some(file_path),
            Self::RuntimeError { file_path, .. } => Some(file_path),
            Self::Timeout { file_path, .. } => Some(file_path),
            Self::NotFound { file_path, .. } => Some(file_path),
            Self::MaxRetriesExceeded { file_path, .. } => Some(file_path),
            Self::Other { file_path, .. } => file_path.as_deref(),
            _ => None,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::SyntaxError { message, .. } => message,
            Self::RuntimeError { message, .. } => message,
            Self::Timeout { message, .. } => message,
            Self::NotFound { message, .. } => message,
            Self::MaxRetriesExceeded { message, .. } => message,
            Self::HelpersNotInitialized { message } => message,
            Self::RuntimeNotAvailable { message } => message,
            Self::Other { message, .. } => message,
        }
    }

    pub fn from_js_error(
        error_msg: String,
        file_path: String,
        stack: Option<String>,
        error_name: Option<String>,
    ) -> Self {
        if let Some(ref name) = error_name
            && name.contains("SyntaxError")
        {
            let (line, column) = Self::extract_line_column(&error_msg);
            return Self::SyntaxError { message: error_msg, file_path, line, column };
        }

        if error_msg.contains("Cannot find module")
            || error_msg.contains("Module not found")
            || error_msg.contains("not found")
        {
            return Self::NotFound { message: error_msg, file_path };
        }

        Self::RuntimeError { message: error_msg, file_path, stack, error_name }
    }

    fn extract_line_column(message: &str) -> (Option<u32>, Option<u32>) {
        if let Some(line_start) = message.find("line ") {
            let after_line = &message[line_start + 5..];
            if let Some(line_end) = after_line.find(|c: char| !c.is_numeric())
                && let Ok(line) = after_line[..line_end].parse::<u32>()
            {
                if let Some(col_start) = after_line.find("column ") {
                    let after_col = &after_line[col_start + 7..];
                    if let Some(col_end) = after_col.find(|c: char| !c.is_numeric())
                        && let Ok(column) = after_col[..col_end].parse::<u32>()
                    {
                        return (Some(line), Some(column));
                    }
                }
                return (Some(line), None);
            }
        }

        if let Some(colon_pos) = message.find(':') {
            let after_colon = &message[colon_pos + 1..];
            if let Some(next_colon) = after_colon.find(':')
                && let Ok(line) = after_colon[..next_colon].parse::<u32>()
            {
                let after_second = &after_colon[next_colon + 1..];
                if let Some(end) = after_second.find(|c: char| !c.is_numeric())
                    && let Ok(column) = after_second[..end].parse::<u32>()
                {
                    return (Some(line), Some(column));
                }
            }
        }

        (None, None)
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
            Self::Forbidden(msg, _) => msg.clone(),
            Self::Serialization(msg, _) => msg.clone(),
            Self::Deserialization(msg, _) => msg.clone(),
            Self::State(msg, _) => msg.clone(),
            Self::Network(msg, _) => msg.clone(),
            Self::Timeout(msg, _) => msg.clone(),
            Self::ServerError(msg, _) => msg.clone(),
            Self::JsExecution(msg, _) => msg.clone(),
            Self::JsRuntime(msg, _) => msg.clone(),
            Self::IoError(msg, _) => msg.clone(),
            Self::ModuleReload(err, _) => err.message().to_string(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_, _) => "NOT_FOUND",
            Self::Validation(_, _) => "VALIDATION",
            Self::Internal(_, _) => "INTERNAL",
            Self::BadRequest(_, _) => "BAD_REQUEST",
            Self::Forbidden(_, _) => "FORBIDDEN",
            Self::Serialization(_, _) => "SERIALIZATION_ERROR",
            Self::Deserialization(_, _) => "DESERIALIZATION_ERROR",
            Self::State(_, _) => "STATE_ERROR",
            Self::Network(_, _) => "NETWORK",
            Self::Timeout(_, _) => "TIMEOUT_ERROR",
            Self::ServerError(_, _) => "SERVER_ERROR",
            Self::JsExecution(_, _) => "JS_EXECUTION_ERROR",
            Self::JsRuntime(_, _) => "JS_RUNTIME_ERROR",
            Self::IoError(_, _) => "IO_ERROR",
            Self::ModuleReload(err, _) => err.code(),
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
            Self::ModuleReload(_, meta) => meta.as_ref(),
            Self::Forbidden(_, meta) => meta.as_ref(),
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
            Self::ModuleReload(_, meta) => meta,
            Self::Forbidden(_, meta) => meta,
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

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into(), None)
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

    pub fn module_reload(error: ModuleReloadError) -> Self {
        Self::ModuleReload(Box::new(error), None)
    }

    pub fn module_reload_syntax_error(
        message: impl Into<String>,
        file_path: impl Into<String>,
        line: Option<u32>,
        column: Option<u32>,
    ) -> Self {
        Self::ModuleReload(
            Box::new(ModuleReloadError::SyntaxError {
                message: message.into(),
                file_path: file_path.into(),
                line,
                column,
            }),
            None,
        )
    }

    pub fn module_reload_runtime_error(
        message: impl Into<String>,
        file_path: impl Into<String>,
        stack: Option<String>,
        error_name: Option<String>,
    ) -> Self {
        Self::ModuleReload(
            Box::new(ModuleReloadError::RuntimeError {
                message: message.into(),
                file_path: file_path.into(),
                stack,
                error_name,
            }),
            None,
        )
    }

    pub fn module_reload_timeout(
        message: impl Into<String>,
        file_path: impl Into<String>,
        timeout_ms: u64,
    ) -> Self {
        Self::ModuleReload(
            Box::new(ModuleReloadError::Timeout {
                message: message.into(),
                file_path: file_path.into(),
                timeout_ms,
            }),
            None,
        )
    }

    pub fn module_reload_not_found(
        message: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Self {
        Self::ModuleReload(
            Box::new(ModuleReloadError::NotFound {
                message: message.into(),
                file_path: file_path.into(),
            }),
            None,
        )
    }

    pub fn module_reload_max_retries(
        message: impl Into<String>,
        file_path: impl Into<String>,
        attempts: usize,
        last_error: Option<String>,
    ) -> Self {
        Self::ModuleReload(
            Box::new(ModuleReloadError::MaxRetriesExceeded {
                message: message.into(),
                file_path: file_path.into(),
                attempts,
                last_error,
            }),
            None,
        )
    }

    pub fn module_reload_helpers_not_initialized(message: impl Into<String>) -> Self {
        Self::ModuleReload(
            Box::new(ModuleReloadError::HelpersNotInitialized { message: message.into() }),
            None,
        )
    }

    pub fn module_reload_runtime_not_available(message: impl Into<String>) -> Self {
        Self::ModuleReload(
            Box::new(ModuleReloadError::RuntimeNotAvailable { message: message.into() }),
            None,
        )
    }

    pub fn module_reload_other(message: impl Into<String>, file_path: Option<String>) -> Self {
        Self::ModuleReload(
            Box::new(ModuleReloadError::Other { message: message.into(), file_path }),
            None,
        )
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StreamingError {
    StreamInitError { message: String, component_id: Option<String> },
    ChunkConversionError { message: String, chunk_type: Option<String> },
    BoundaryTimeout { message: String, boundary_id: String, timeout_ms: u64 },
    ClientDisconnected { message: String },
}

impl std::fmt::Display for StreamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StreamInitError { message, component_id } => {
                write!(f, "Failed to initialize streaming: {}", message)?;
                if let Some(id) = component_id {
                    write!(f, " (component: {})", id)?;
                }
                Ok(())
            }
            Self::ChunkConversionError { message, chunk_type } => {
                write!(f, "Error converting chunk to HTML: {}", message)?;
                if let Some(ct) = chunk_type {
                    write!(f, " (chunk type: {})", ct)?;
                }
                Ok(())
            }
            Self::BoundaryTimeout { message, boundary_id, timeout_ms } => {
                write!(
                    f,
                    "Suspense boundary '{}' timed out after {}ms: {}",
                    boundary_id, timeout_ms, message
                )
            }
            Self::ClientDisconnected { message } => {
                write!(f, "Client disconnected during streaming: {}", message)
            }
        }
    }
}

impl std::error::Error for StreamingError {}

impl From<StreamingError> for RariError {
    fn from(error: StreamingError) -> Self {
        let message = error.to_string();
        let mut details = FxHashMap::default();

        match &error {
            StreamingError::StreamInitError { component_id, .. } => {
                details.insert("error_type".to_string(), "stream_init_error".to_string());
                if let Some(id) = component_id {
                    details.insert("component_id".to_string(), id.clone());
                }
            }
            StreamingError::ChunkConversionError { chunk_type, .. } => {
                details.insert("error_type".to_string(), "chunk_conversion_error".to_string());
                if let Some(ct) = chunk_type {
                    details.insert("chunk_type".to_string(), ct.clone());
                }
            }
            StreamingError::BoundaryTimeout { boundary_id, timeout_ms, .. } => {
                details.insert("error_type".to_string(), "boundary_timeout".to_string());
                details.insert("boundary_id".to_string(), boundary_id.clone());
                details.insert("timeout_ms".to_string(), timeout_ms.to_string());
            }
            StreamingError::ClientDisconnected { .. } => {
                details.insert("error_type".to_string(), "client_disconnected".to_string());
            }
        }

        RariError::Internal(
            message,
            Some(ErrorMetadata {
                code: "STREAMING_ERROR".to_string(),
                details: Some(details),
                source: Some("streaming_ssr".to_string()),
                error_source: None,
            }),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoadingStateError {
    LoadingNotFound { path: String, message: String },
    RenderError { path: String, message: String, source: Option<String> },
    SuspenseError { message: String, boundary_id: Option<String> },
    InvalidOutput { path: String, message: String, details: Option<String> },
}

impl std::fmt::Display for LoadingStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadingNotFound { path, message } => {
                write!(f, "Loading component not found at '{}': {}", path, message)
            }
            Self::RenderError { path, message, source } => {
                write!(f, "Failed to render loading component at '{}': {}", path, message)?;
                if let Some(src) = source {
                    write!(f, " (source: {})", src)?;
                }
                Ok(())
            }
            Self::SuspenseError { message, boundary_id } => {
                write!(f, "Suspense boundary error: {}", message)?;
                if let Some(id) = boundary_id {
                    write!(f, " (boundary ID: {})", id)?;
                }
                Ok(())
            }
            Self::InvalidOutput { path, message, details } => {
                write!(f, "Invalid loading component output from '{}': {}", path, message)?;
                if let Some(d) = details {
                    write!(f, " ({})", d)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for LoadingStateError {}

impl From<LoadingStateError> for RariError {
    fn from(error: LoadingStateError) -> Self {
        let message = error.to_string();
        let mut details = FxHashMap::default();

        match &error {
            LoadingStateError::LoadingNotFound { path, .. } => {
                details.insert("path".to_string(), path.clone());
                details.insert("error_type".to_string(), "loading_not_found".to_string());
            }
            LoadingStateError::RenderError { path, source, .. } => {
                details.insert("path".to_string(), path.clone());
                details.insert("error_type".to_string(), "render_error".to_string());
                if let Some(src) = source {
                    details.insert("source".to_string(), src.clone());
                }
            }
            LoadingStateError::SuspenseError { boundary_id, .. } => {
                details.insert("error_type".to_string(), "suspense_error".to_string());
                if let Some(id) = boundary_id {
                    details.insert("boundary_id".to_string(), id.clone());
                }
            }
            LoadingStateError::InvalidOutput { path, details: output_details, .. } => {
                details.insert("path".to_string(), path.clone());
                details.insert("error_type".to_string(), "invalid_output".to_string());
                if let Some(d) = output_details {
                    details.insert("output_details".to_string(), d.clone());
                }
            }
        }

        RariError::Internal(
            message,
            Some(ErrorMetadata {
                code: "LOADING_STATE_ERROR".to_string(),
                details: Some(details),
                source: Some("loading_state".to_string()),
                error_source: None,
            }),
        )
    }
}

impl RariError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound(_, _) => 404,
            Self::Validation(_, _) => 400,
            Self::BadRequest(_, _) => 400,
            Self::Forbidden(_, _) => 403,
            Self::Timeout(_, _) => 408,
            Self::Internal(_, _) => 500,
            Self::Serialization(_, _) => 500,
            Self::Deserialization(_, _) => 400,
            Self::State(_, _) => 500,
            Self::Network(_, _) => 502,
            Self::ServerError(_, _) => 500,
            Self::JsExecution(_, _) => 500,
            Self::JsRuntime(_, _) => 500,
            Self::IoError(_, _) => 500,
            Self::ModuleReload(_, _) => 500,
        }
    }

    pub fn safe_message(&self, is_development: bool) -> String {
        if is_development {
            self.to_string()
        } else {
            match self {
                Self::NotFound(_, _) => "Resource not found".to_string(),
                Self::Validation(_, _) => "Validation failed".to_string(),
                Self::BadRequest(_, _) => "Bad request".to_string(),
                Self::Forbidden(_, _) => "Access forbidden".to_string(),
                Self::Timeout(_, _) => "Request timeout".to_string(),
                Self::Internal(_, _) => "Internal server error".to_string(),
                Self::Serialization(_, _) => "Internal server error".to_string(),
                Self::Deserialization(_, _) => "Invalid request format".to_string(),
                Self::State(_, _) => "Internal server error".to_string(),
                Self::Network(_, _) => "Network error".to_string(),
                Self::ServerError(_, _) => "Server error".to_string(),
                Self::JsExecution(_, _) => "Server error".to_string(),
                Self::JsRuntime(_, _) => "Server error".to_string(),
                Self::IoError(_, _) => "Internal server error".to_string(),
                Self::ModuleReload(_, _) => "Internal server error".to_string(),
            }
        }
    }

    #[allow(clippy::disallowed_methods)]
    pub fn to_json_response(&self, is_development: bool) -> serde_json::Value {
        serde_json::json!({
            "error": self.safe_message(is_development),
            "code": self.code(),
            "status": self.status_code(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(RariError::not_found("test").status_code(), 404);
        assert_eq!(RariError::validation("test").status_code(), 400);
        assert_eq!(RariError::bad_request("test").status_code(), 400);
        assert_eq!(RariError::forbidden("test").status_code(), 403);
        assert_eq!(RariError::timeout("test").status_code(), 408);
        assert_eq!(RariError::internal("test").status_code(), 500);
        assert_eq!(RariError::server_error("test").status_code(), 500);
    }

    #[test]
    fn test_safe_message_development() {
        let error = RariError::internal("Detailed internal error with stack trace");
        let message = error.safe_message(true);

        assert!(message.contains("Detailed internal error"));
        assert!(message.contains("stack trace"));
    }

    #[test]
    fn test_safe_message_production_internal() {
        let error = RariError::internal("Detailed internal error with /path/to/file.rs:123");
        let message = error.safe_message(false);

        assert_eq!(message, "Internal server error");
        assert!(!message.contains("/path/to/file.rs"));
        assert!(!message.contains("Detailed"));
    }

    #[test]
    fn test_safe_message_production_validation() {
        let error = RariError::validation("Field 'password' must be at least 8 characters");
        let message = error.safe_message(false);

        assert_eq!(message, "Validation failed");
        assert!(!message.contains("password"));
        assert!(!message.contains("8 characters"));
    }

    #[test]
    fn test_safe_message_production_not_found() {
        let error = RariError::not_found("File /etc/passwd not found");
        let message = error.safe_message(false);

        assert_eq!(message, "Resource not found");
        assert!(!message.contains("/etc/passwd"));
    }

    #[test]
    fn test_safe_message_production_js_error() {
        let error = RariError::js_execution("ReferenceError: secretKey is not defined at line 42");
        let message = error.safe_message(false);

        assert_eq!(message, "Server error");
        assert!(!message.contains("secretKey"));
        assert!(!message.contains("line 42"));
    }

    #[test]
    fn test_safe_message_production_io_error() {
        let error = RariError::io("Failed to read /home/user/.env: Permission denied");
        let message = error.safe_message(false);

        assert_eq!(message, "Internal server error");
        assert!(!message.contains("/home/user/.env"));
        assert!(!message.contains("Permission denied"));
    }

    #[test]
    fn test_to_json_response_development() {
        let error = RariError::bad_request("Invalid JSON: expected '}' at line 5");
        let json = error.to_json_response(true);

        assert_eq!(json["code"], "BAD_REQUEST");
        assert_eq!(json["status"], 400);
        assert!(json["error"].as_str().unwrap().contains("Invalid JSON"));
        assert!(json["error"].as_str().unwrap().contains("line 5"));
    }

    #[test]
    fn test_to_json_response_production() {
        let error = RariError::bad_request("Invalid JSON: expected '}' at line 5");
        let json = error.to_json_response(false);

        assert_eq!(json["code"], "BAD_REQUEST");
        assert_eq!(json["status"], 400);
        assert_eq!(json["error"], "Bad request");
        assert!(!json["error"].as_str().unwrap().contains("Invalid JSON"));
        assert!(!json["error"].as_str().unwrap().contains("line 5"));
    }

    #[test]
    fn test_to_json_response_forbidden() {
        let error = RariError::forbidden("CSRF token validation failed: invalid signature");
        let json = error.to_json_response(false);

        assert_eq!(json["code"], "FORBIDDEN");
        assert_eq!(json["status"], 403);
        assert_eq!(json["error"], "Access forbidden");
        assert!(!json["error"].as_str().unwrap().contains("CSRF"));
        assert!(!json["error"].as_str().unwrap().contains("signature"));
    }

    #[test]
    fn test_module_reload_error_sanitization() {
        let error = RariError::module_reload_syntax_error(
            "Unexpected token at line 42",
            "/app/src/secret-component.tsx",
            Some(42),
            Some(10),
        );
        let message = error.safe_message(false);

        assert_eq!(message, "Internal server error");
        assert!(!message.contains("secret-component"));
        assert!(!message.contains("line 42"));
    }

    #[test]
    fn test_network_error_sanitization() {
        let error = RariError::network("Connection refused to internal-api.company.local:8080");
        let message = error.safe_message(false);

        assert_eq!(message, "Network error");
        assert!(!message.contains("internal-api"));
        assert!(!message.contains("company.local"));
        assert!(!message.contains("8080"));
    }

    #[test]
    fn test_deserialization_error_shows_bad_request() {
        let error = RariError::deserialization("Invalid JSON at position 123");
        let message = error.safe_message(false);

        assert_eq!(message, "Invalid request format");
        assert!(!message.contains("position 123"));
    }
}
