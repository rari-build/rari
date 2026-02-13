pub mod constants;
mod deno_runtime;
mod executor;
mod interface;
mod lazy_runtime;
mod runtime_builder;
mod v8_utils;

pub use deno_runtime::DenoRuntime;
pub use interface::JsRuntimeInterface;
pub use lazy_runtime::{create_lazy_runtime, create_lazy_runtime_with_env};

use crate::runtime::module_loader::RuntimeConfig;

pub fn create_runtime() -> Box<dyn JsRuntimeInterface> {
    create_runtime_with_config(RuntimeConfig::default())
}

pub fn create_runtime_with_config(_config: RuntimeConfig) -> Box<dyn JsRuntimeInterface> {
    Box::new(DenoRuntime::new(None))
}
