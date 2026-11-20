// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_types)]
#![deny(clippy::unnecessary_wraps)]
#![deny(clippy::print_stderr)]
#![deny(clippy::print_stdout)]
#![expect(
    clippy::absolute_paths,
    clippy::cast_possible_truncation,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::missing_errors_doc,
    clippy::redundant_pub_crate,
    reason = "Ported code not yet updated to the workspace lint rules"
)]
#![cfg_attr(
    not(test),
    expect(
        clippy::wildcard_imports,
        reason = "Ported code not yet updated to the workspace lint rules"
    )
)]

#[cfg(feature = "cjs")]
mod cjs_parse;
pub mod diagnostics;
#[cfg(feature = "emit")]
mod emit;
mod exports;
mod parsed_source;
mod parsing;
mod source_range;
mod str_ext;
#[cfg(feature = "transpiling")]
mod transpiling;
mod types;

#[cfg(feature = "cjs")]
pub use cjs_parse::*;
pub use deno_media_type::*;
#[cfg(feature = "emit")]
pub use emit::*;
pub use exports::*;
pub use parsed_source::*;
pub use parsing::*;
pub use source_range::*;
pub use str_ext::*;
#[cfg(feature = "transpiling")]
pub use transpiling::*;
pub use types::*;

pub type ModuleSpecifier = url::Url;

/// Re-exports of oxc crates for consumers that need direct access.
pub mod oxc {
    #[cfg(feature = "visit")]
    pub use oxc::ast_visit;
    #[cfg(feature = "codegen")]
    pub use oxc::codegen;
    #[cfg(feature = "semantic")]
    pub use oxc::semantic;
    pub use oxc::{allocator, ast, diagnostics, parser};
    /// Re-export of `oxc::span` plus the string types (`Str`, `Ident`,
    /// `CompactStr`) that moved to `oxc::str` in oxc 0.134.
    pub mod span {
        pub use oxc::{
            span::*,
            str::{CompactStr, Ident, Str},
        };
    }
    #[cfg(feature = "transpiling")]
    pub use oxc::transformer;
    pub use oxc::{str, syntax};
}
