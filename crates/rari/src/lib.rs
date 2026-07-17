pub mod metadata;
pub mod rendering;
pub mod rsc;
pub mod runtime;
pub mod server;
pub use rari_core::utils;
pub use ::async_trait;
pub use rendering::{
    base::{RscJsLoader, RscRenderer},
    r#static::RscHtmlRenderer,
};
pub use rsc::{ComponentRegistry, extract_dependencies};
