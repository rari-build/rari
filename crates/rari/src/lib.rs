pub mod rendering;
pub mod runtime;
pub mod server;
mod utils;
pub use ::async_trait;
pub use rari_rsc::{ComponentRegistry, extract_dependencies};
pub use rendering::{
    base::{RscJsLoader, RscRenderer},
    r#static::RscHtmlRenderer,
};
