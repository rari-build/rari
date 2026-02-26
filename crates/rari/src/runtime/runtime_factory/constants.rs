use crate::error::RariError;
use serde_json::Value as JsonValue;

pub const CHANNEL_CAPACITY: usize = 32;
pub const RUNTIME_RESTART_DELAY_MS: u64 = 1000;
pub const RUNTIME_QUICK_RESTART_DELAY_MS: u64 = 100;
pub const COMPONENT_PREFIX: &str = "component_";
pub const VERIFY_REGISTRATION_PREFIX: &str = "verify_registration_";
pub const RARI_REGISTER_FUNCTION: &str = "~rari_register";

pub const MODULE_ALREADY_EVALUATED_ERROR: &str = "Module already evaluated";
pub const JS_EXECUTOR_FAILED_ERROR: &str = "JS executor failed to respond";
pub const JS_EXECUTOR_CHANNEL_CLOSED_ERROR: &str = "JS executor channel closed";
pub const RUNTIME_RESTART_MESSAGE: &str =
    "Runtime is being restarted for stability. Please retry your request.";

pub const ENV_INJECTION_SCRIPT: &str = r#"
(() => {
    if (!globalThis.process.env) {
        globalThis.process.env = {};
    }

    const envVars = {};
    Object.assign(globalThis.process.env, envVars);

    return Object.keys(envVars).length;
})();
"#;

pub const MODULE_CHECK_SCRIPT: &str = r#"
(function() {
    if (!globalThis.RscModuleManager) {
        return { available: false, extension: 'rsc_modules' };
    }
    return { available: true, extension: 'rsc_modules' };
})()
"#;

pub const PROMISE_SETUP_SCRIPT: &str = r#"
(function() {
    try {
        if (!globalThis['~promises']) globalThis['~promises'] = {};
        const promise = globalThis['~promises'].currentObject;
        if (!promise || typeof promise.then !== 'function') {
            globalThis['~promises'].resolvedValue = {
                '~error': "Not a valid promise",
                received: typeof promise,
                promiseToString: String(promise)
            };
            globalThis['~promises'].resolutionComplete = true;
            return;
        }

        globalThis['~promises'].resolvedValue = null;
        globalThis['~promises'].resolutionComplete = false;

        promise.then(function(resolvedValue) {
            globalThis['~promises'].resolvedValue = resolvedValue;
            globalThis['~promises'].resolutionComplete = true;
        }).catch(function(error) {
            globalThis['~promises'].resolvedValue = {
                '~promiseError': true,
                error: String(error),
                stack: error.stack || "No stack trace"
            };
            globalThis['~promises'].resolutionComplete = true;
        });
    } catch (error) {
        globalThis['~promises'].resolvedValue = {
            '~promiseError': true,
            error: String(error),
            stack: error.stack || "No stack trace"
        };
        globalThis['~promises'].resolutionComplete = true;
    }
})()
"#;

pub const PROMISE_EXTRACT_SCRIPT: &str = r#"
(function() {
    if (!globalThis['~promises']) globalThis['~promises'] = {};
    if (globalThis['~promises'].resolutionComplete === true) {
        return globalThis['~promises'].resolvedValue;
    } else {
        return {
            '~timeoutError': "Promise did not resolve in time",
            '~debugInfo': {
                completion_flag: globalThis['~promises'].resolutionComplete,
                resolved_value: globalThis['~promises'].resolvedValue
            }
        };
    }
})()
"#;

pub fn is_critical_error(error: &RariError) -> bool {
    let error_str = error.to_string();
    error_str.contains("assertion") || error_str.contains("panicked")
}

pub fn is_runtime_restart_needed(error: &RariError) -> bool {
    let error_str = error.to_string();
    error_str.contains(MODULE_ALREADY_EVALUATED_ERROR)
        || error_str.contains(JS_EXECUTOR_FAILED_ERROR)
        || error_str.contains(JS_EXECUTOR_CHANNEL_CLOSED_ERROR)
}

pub fn create_graceful_error() -> RariError {
    RariError::js_runtime(RUNTIME_RESTART_MESSAGE.to_string())
}

#[allow(clippy::disallowed_methods)]
pub fn create_already_evaluated_response(component_name: &str) -> JsonValue {
    serde_json::json!({
        "status": "already_evaluated",
        "component": component_name
    })
}

#[allow(clippy::disallowed_methods)]
pub fn create_already_loaded_response(component_name: &str) -> JsonValue {
    serde_json::json!({
        "status": "already_loaded",
        "component": component_name
    })
}

pub fn create_registration_script(specifier_str: &str, script_name: &str) -> String {
    format!(
        r#"
        (async function() {{
            try {{
                const module = await import("{specifier_str}");
                if (typeof module.{RARI_REGISTER_FUNCTION} === 'function') {{
                    const result = module.{RARI_REGISTER_FUNCTION}.call(module);
                    return {{ success: true, result }};
                }} else {{
                    return {{ success: false, error: 'No {RARI_REGISTER_FUNCTION} function found' }};
                }}
            }} catch (e) {{
                console.error("[rari] Failed to call {RARI_REGISTER_FUNCTION} for '{script_name}': " + e.message);
                return {{ success: false, error: e.message }};
            }}
        }})()
        "#
    )
}
