pub(crate) mod constants;
mod deno_runtime;
mod executor;
mod interface;
mod runtime_builder;
mod v8_utils;

pub use deno_runtime::DenoRuntime;
pub use interface::JsRuntimeInterface;

use rustc_hash::FxHashMap;
use std::sync::Arc;

pub fn create_runtime() -> Arc<DenoRuntime> {
    Arc::new(DenoRuntime::new(None))
}

pub fn create_runtime_with_env(env_vars: FxHashMap<String, String>) -> Arc<DenoRuntime> {
    Arc::new(DenoRuntime::new(Some(env_vars)))
}
