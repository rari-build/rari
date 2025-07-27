use std::path::PathBuf;

use super::ServerFunctionInfo;
use crate::error::RariError;
use crate::runtime::JsExecutionRuntime;
use crate::server_fn::ServerFunction;
use serde_json::Value as JsonValue;

const DEFAULT_TIMEOUT_MS: u64 = 30000;
const DEFAULT_PROJECT_ROOT: &str = ".";
const SERVER_FUNCTION_ID_PREFIX: &str = "server-function:";

#[derive(Debug, Clone)]
pub struct JsServerFunction {
    pub info: ServerFunctionInfo,
    pub project_root: PathBuf,
    pub timeout_ms: u64,
}

impl JsServerFunction {
    pub fn new(info: ServerFunctionInfo, project_root: PathBuf) -> Self {
        Self { info, project_root, timeout_ms: DEFAULT_TIMEOUT_MS }
    }

    pub fn new_simple(id: String, file_path: String) -> Self {
        let info = ServerFunctionInfo {
            id: id.clone(),
            file_path,
            source_code: String::new(),
            function_name: id,
            line_number: 0,
        };

        Self {
            info,
            project_root: PathBuf::from(DEFAULT_PROJECT_ROOT),
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn id(&self) -> &str {
        &self.info.id
    }

    pub fn source_code(&self) -> &str {
        &self.info.source_code
    }

    pub fn function_name(&self) -> &str {
        &self.info.function_name
    }

    pub fn with_id(mut self, id: String) -> Self {
        self.info.id = id;
        self
    }
}

#[async_trait::async_trait]
impl ServerFunction for JsServerFunction {
    async fn execute(&self, args: Vec<serde_json::Value>) -> Result<serde_json::Value, RariError> {
        let function_name = self.info.function_name.clone();
        let timeout_ms = self.timeout_ms;

        tokio::task::spawn_blocking(move || {
            execute_js_function_sync(function_name, args, timeout_ms)
        })
        .await
        .map_err(|e| RariError::js_execution(format!("Task join error: {e}")))?
    }

    fn get_id(&self) -> String {
        self.info.id.clone()
    }
}

fn execute_js_function_sync(
    function_name: String,
    args: Vec<JsonValue>,
    timeout_ms: u64,
) -> Result<JsonValue, RariError> {
    let runtime_result = std::panic::catch_unwind(|| {
        let runtime = JsExecutionRuntime::new(None).with_timeout(timeout_ms);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| RariError::js_execution(format!("Failed to build Tokio runtime: {e}")))?;

        rt.block_on(async move { runtime.execute_function(&function_name, args).await })
    });

    match runtime_result {
        Ok(result) => result,
        Err(panic_error) => {
            let error_msg = extract_panic_message(panic_error);
            Err(RariError::js_execution(error_msg))
        }
    }
}

fn extract_panic_message(panic_error: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic_error.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = panic_error.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic occurred during JavaScript execution".to_string()
    }
}

pub struct JsServerFunctionManager {
    project_root: PathBuf,
    default_timeout_ms: u64,
}

impl JsServerFunctionManager {
    pub fn new(project_root: impl AsRef<std::path::Path>) -> Self {
        let project_root = project_root.as_ref().to_path_buf();

        Self { project_root, default_timeout_ms: DEFAULT_TIMEOUT_MS }
    }

    pub fn with_default_timeout(mut self, timeout_ms: u64) -> Self {
        self.default_timeout_ms = timeout_ms;
        self
    }

    pub async fn register_function(
        &self,
        function_manager: &mut crate::server_fn::ServerFunctionManager,
        mut info: ServerFunctionInfo,
    ) -> String {
        let id = self.generate_function_id(&info);
        let simple_name = info.function_name.clone();

        info.id = id.clone();

        let js_fn = self.create_js_function(info);
        let js_fn_simple = js_fn.clone().with_id(simple_name.clone());

        function_manager.register(id.clone(), js_fn).await;
        function_manager.register(simple_name, js_fn_simple).await;

        id
    }

    pub async fn register_functions(
        &self,
        function_manager: &mut crate::server_fn::ServerFunctionManager,
        functions_info: Vec<ServerFunctionInfo>,
    ) -> Vec<String> {
        let mut function_ids = Vec::new();

        for info in functions_info {
            let id = self.register_function(function_manager, info).await;
            function_ids.push(id);
        }

        function_ids
    }

    pub async fn register_from_websocket(
        &self,
        function_manager: &mut crate::server_fn::ServerFunctionManager,
        info: serde_json::Value,
    ) -> Result<String, crate::error::RariError> {
        let function_info = self.parse_websocket_function_info(info)?;
        let js_fn = self.create_js_function(function_info);
        let id = js_fn.get_id();

        function_manager.register(id.clone(), js_fn).await;

        Ok(id)
    }

    fn generate_function_id(&self, info: &ServerFunctionInfo) -> String {
        let mut id = info.id.clone();

        if !id.starts_with(SERVER_FUNCTION_ID_PREFIX) {
            id = if id.contains(':') {
                format!("{SERVER_FUNCTION_ID_PREFIX}{id}")
            } else {
                format!("{SERVER_FUNCTION_ID_PREFIX}{}:{}", info.file_path, info.function_name)
            };
        }

        id
    }

    fn create_js_function(&self, info: ServerFunctionInfo) -> JsServerFunction {
        JsServerFunction::new(info, self.project_root.clone()).with_timeout(self.default_timeout_ms)
    }

    fn parse_websocket_function_info(
        &self,
        info: serde_json::Value,
    ) -> Result<ServerFunctionInfo, crate::error::RariError> {
        let id = self.extract_required_string(&info, "id")?;
        let file_path = self.extract_required_string(&info, "file_path")?;
        let source_code = self.extract_required_string(&info, "source_code")?;
        let function_name = self.extract_required_string(&info, "function_name")?;
        let line_number = info.get("line_number").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        Ok(ServerFunctionInfo { id, file_path, source_code, function_name, line_number })
    }

    fn extract_required_string(
        &self,
        info: &serde_json::Value,
        field: &str,
    ) -> Result<String, crate::error::RariError> {
        info.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| crate::error::RariError::validation(format!("Missing {field}")))
    }
}
