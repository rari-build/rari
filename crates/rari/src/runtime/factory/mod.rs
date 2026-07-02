#![allow(clippy::implicit_hasher)]

mod create_params;
mod executor;
mod interface;
mod runtime;
mod runtime_builder;
pub(crate) mod utils;

use std::sync::Arc;

pub use interface::JsRuntimeInterface;
pub use runtime::RariRuntime;
use rustc_hash::FxHashMap;

pub fn create_runtime() -> Arc<RariRuntime> {
    Arc::new(RariRuntime::new(None))
}

pub fn create_runtime_with_env(env_vars: FxHashMap<String, String>) -> Arc<RariRuntime> {
    Arc::new(RariRuntime::new(Some(env_vars)))
}
