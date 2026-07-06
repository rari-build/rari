pub mod rendering;
pub mod rsc;
pub mod runtime;
pub mod server;
mod utils;
pub use ::async_trait;
pub use rendering::{
    base::{RscJsLoader, RscRenderer},
    r#static::RscHtmlRenderer,
};
pub use rsc::{ComponentRegistry, extract_dependencies};
