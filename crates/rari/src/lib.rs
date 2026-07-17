pub use rari_core::metadata;
pub mod rendering;
pub mod rsc;
pub use rari_core::runtime;
pub mod server;
pub use rari_core::utils;
pub use ::async_trait;
pub use rendering::{
    base::{RscJsLoader, RscRenderer},
    r#static::RscHtmlRenderer,
};
pub use rsc::{ClientReferenceRegistry, ComponentRegistry, extract_dependencies};
