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

pub mod rsc;
pub mod runtime;
pub mod server;
pub use ::async_trait;
