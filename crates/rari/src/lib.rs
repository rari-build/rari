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
#![cfg_attr(test, allow(clippy::allow_attributes))]
#![cfg_attr(
    test,
    allow(
        clippy::unreadable_literal,
        clippy::needless_raw_string_hashes,
        clippy::panic,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::print_stdout,
        clippy::float_cmp,
        clippy::bool_assert_comparison,
        clippy::redundant_clone,
        clippy::redundant_closure_for_method_calls,
        clippy::single_char_pattern,
        clippy::approx_constant,
        clippy::uninlined_format_args,
        clippy::module_inception,
        clippy::return_self_not_must_use,
        clippy::disallowed_methods,
        clippy::clone_on_ref_ptr,
        clippy::get_unwrap,
    )
)]

pub mod rsc;
pub mod runtime;
pub mod server;
pub use ::async_trait;
