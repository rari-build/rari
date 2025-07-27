use crate::error::RariError;
use crate::runtime::JsExecutionRuntime;
use crate::server_fn::registry::ServerFunctionRegistry;
use serde_json::Value as JsonValue;
use std::sync::Arc;

const DEFAULT_EMPTY_ARGS: &str = "[]";
const MODULE_EXPORTS_VAR: &str = "moduleExports";
const EXPORTS_VAR: &str = "exports";
const REQUIRE_VAR: &str = "require";
const MODULE_VAR: &str = "module";

pub struct ServerFunctionExecutor {
    registry: Arc<ServerFunctionRegistry>,
    runtime: Arc<JsExecutionRuntime>,
}

impl ServerFunctionExecutor {
    pub fn new(registry: Arc<ServerFunctionRegistry>, runtime: Arc<JsExecutionRuntime>) -> Self {
        Self { registry, runtime }
    }

    pub async fn execute(
        &self,
        function_id: &str,
        args: &[JsonValue],
    ) -> Result<JsonValue, RariError> {
        let function = self.get_function(function_id)?;
        let code = self.get_function_code(&function, function_id)?;
        let execute_script = self.create_execution_script(&code, &function.name, args)?;

        self.runtime
            .execute_script(format!("execute_server_function_{function_id}.js"), execute_script)
            .await
    }

    pub async fn load_module(&self, function_id: &str, code: &str) -> Result<(), RariError> {
        let mut function = self.get_function(function_id)?;
        function.code = Some(code.to_string());
        self.registry.register(function)?;

        let validate_script = self.create_validation_script(code)?;
        let exports = self
            .runtime
            .execute_script(format!("validate_server_function_{function_id}.js"), validate_script)
            .await?;

        self.validate_module_exports(exports)
    }

    fn get_function(
        &self,
        function_id: &str,
    ) -> Result<crate::server_fn::registry::ServerFunction, RariError> {
        self.registry.get(function_id)?.ok_or_else(|| {
            RariError::not_found(format!("Server function '{function_id}' not found"))
        })
    }

    fn get_function_code(
        &self,
        function: &crate::server_fn::registry::ServerFunction,
        function_id: &str,
    ) -> Result<String, RariError> {
        function.code.clone().ok_or_else(|| {
            RariError::internal(format!("Server function '{function_id}' has no code available"))
        })
    }

    fn create_execution_script(
        &self,
        code: &str,
        function_name: &str,
        args: &[JsonValue],
    ) -> Result<String, RariError> {
        let escaped_code = escape_javascript_string(code);
        let args_json = serialize_args(args);

        Ok(format!(
            r#"
            (function() {{
                try {{
                    const {MODULE_EXPORTS_VAR} = {{}};
                    const moduleFn = new Function("{EXPORTS_VAR}", "{REQUIRE_VAR}", "{MODULE_VAR}", "{escaped_code}");
                    const {MODULE_VAR} = {{ {EXPORTS_VAR}: {MODULE_EXPORTS_VAR} }};
                    moduleFn({MODULE_EXPORTS_VAR}, null, {MODULE_VAR}, {{}});

                    const fn = {MODULE_EXPORTS_VAR}["{function_name}"];
                    if (typeof fn !== "function") {{
                        throw new Error(`Function '{function_name}' is not exported or not a function`);
                    }}

                    const args = {args_json};
                    return fn(...args);
                }} catch (error) {{
                    throw new Error(`Server function execution error: ${{error.message}}`);
                }}
            }})()
            "#
        ))
    }

    fn create_validation_script(&self, code: &str) -> Result<String, RariError> {
        let escaped_code = escape_javascript_string(code);

        Ok(format!(
            r#"
            (function() {{
                try {{
                    const {MODULE_EXPORTS_VAR} = {{}};
                    const moduleFn = new Function("{EXPORTS_VAR}", "{REQUIRE_VAR}", "{MODULE_VAR}", "{escaped_code}");
                    const {MODULE_VAR} = {{ {EXPORTS_VAR}: {MODULE_EXPORTS_VAR} }};
                    moduleFn({MODULE_EXPORTS_VAR}, null, {MODULE_VAR}, {{}});

                    return Object.keys({MODULE_EXPORTS_VAR});
                }} catch (error) {{
                    throw new Error(`Module validation error: ${{error.message}}`);
                }}
            }})()
            "#
        ))
    }

    fn validate_module_exports(&self, exports: JsonValue) -> Result<(), RariError> {
        match exports {
            JsonValue::Array(_) => Ok(()),
            _ => Err(RariError::internal(format!(
                "Module validation returned unexpected result: {exports:?}"
            ))),
        }
    }
}

fn escape_javascript_string(input: &str) -> String {
    input
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}

fn serialize_args(args: &[JsonValue]) -> String {
    serde_json::to_string(args).unwrap_or_else(|_| DEFAULT_EMPTY_ARGS.to_string())
}
