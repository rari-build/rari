mod executor;
mod interface;
mod runtime;
mod runtime_builder;
pub(crate) mod utils;

pub use interface::JsRuntimeInterface;
pub use runtime::RariRuntime;

use rustc_hash::FxHashMap;
use std::sync::Arc;

pub fn create_runtime() -> Arc<RariRuntime> {
    Arc::new(RariRuntime::new(None))
}

pub fn create_runtime_with_env(env_vars: FxHashMap<String, String>) -> Arc<RariRuntime> {
    Arc::new(RariRuntime::new(Some(env_vars)))
}
