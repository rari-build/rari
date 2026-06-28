#![expect(clippy::missing_errors_doc)]
#![expect(clippy::cast_precision_loss)]
#![expect(clippy::cast_possible_truncation)]
#![expect(clippy::cast_sign_loss)]
#![expect(clippy::cast_possible_wrap)]
#![expect(clippy::unused_self)]
#![expect(clippy::manual_let_else)]
#![expect(clippy::too_many_lines)]
#![expect(clippy::needless_pass_by_value)]
#![expect(clippy::items_after_statements)]
#![expect(clippy::unused_async)]
#![expect(clippy::needless_pass_by_ref_mut)]

pub mod rendering;
pub mod runtime;
pub mod server;
pub use ::async_trait;
pub use rari_rsc::{
    ElementType, RSCRenderDebug, RSCRenderResult, RSCTree, ReactElement, RscElement,
    RscFlightParser, RscFlightTag, RscSerializer, SerializedReactElement, ServerComponentExecutor,
    SuspenseBoundary, escape_rsc_value,
};
pub use rendering::{
    base::{RscJsLoader, RscRenderer},
    r#static::RscHtmlRenderer,
    streaming::{RscStream, RscStreamChunk},
};
