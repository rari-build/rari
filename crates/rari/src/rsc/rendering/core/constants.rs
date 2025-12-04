use std::time::Duration;

pub const MEMORY_PRESSURE_THRESHOLD: f64 = 0.8;
pub const CACHE_CLEANUP_INTERVAL: Duration = Duration::from_millis(10);

pub const MAX_RETRIES: u64 = 3;
pub const RETRY_BASE_DELAY_MS: u64 = 150;
pub const COMPONENT_AVAILABILITY_CHECK_DELAY_MS: u64 = 20;

pub const DEFAULT_MAX_CONCURRENT_RENDERS: usize = 50;
pub const DEFAULT_MAX_RENDER_TIME_MS: u64 = 8000;
pub const DEFAULT_MAX_SCRIPT_EXECUTION_TIME_MS: u64 = 3000;
pub const DEFAULT_MAX_MEMORY_PER_COMPONENT_MB: usize = 50;
pub const DEFAULT_MAX_CACHE_SIZE: usize = 1000;

pub const BATCH_ERROR_COLLECTION_SCRIPT: &str = include_str!("js/batch_error_collection.js");
pub const EXTENSION_CHECKS_SCRIPT: &str = include_str!("js/extension_checks.js");
pub const REACT_GLOBALS_SETUP_SCRIPT: &str = include_str!("js/react_globals_setup.js");
pub const V8_CACHE_CLEAR_SCRIPT: &str = include_str!("js/v8_cache_clear.js");
pub const PROMISE_MANAGER_CHECK_SCRIPT: &str = include_str!("js/promise_manager_check.js");
pub const SERVER_FUNCTION_RESOLVER_SCRIPT: &str = include_str!("js/server_function_resolver.js");
pub const RESOLVE_SERVER_FUNCTIONS_SCRIPT: &str = include_str!("js/resolve_server_functions.js");
pub const SERVER_ACTION_INVOCATION_SCRIPT: &str = include_str!("js/server_action_invocation.js");
pub const SAFE_PROPERTY_ACCESS_SCRIPT: &str = include_str!("js/safe_property_access.js");
pub const MODULE_REGISTRATION_SCRIPT: &str = include_str!("js/module_registration.js");
pub const JSX_RUNTIME_SETUP_SCRIPT: &str = include_str!("js/jsx_runtime_setup.js");
pub const REGISTRY_PROXY_SETUP_SCRIPT: &str = include_str!("js/registry_proxy_setup.js");
pub const COMPONENT_EVAL_SETUP_SCRIPT: &str = include_str!("js/component_eval_setup.js");
