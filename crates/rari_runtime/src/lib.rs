// Copyright 2018-2026 the Deno authors. MIT license.

#![expect(
    clippy::absolute_paths,
    clippy::cast_possible_truncation,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs
)]

mod bootstrap_options;
pub mod ops;

pub use bootstrap_options::BootstrapOptions;
pub use deno_terminal::colors;

// Keeps Deno's extension name: JS in other extensions imports from `ext:runtime/`.
deno_core::extension!(
    runtime,
    esm_entry_point = "ext:runtime/bootstrap.ts",
    esm = [
        dir "js",
        "bootstrap.ts",
        "navigator_user_agent_data.ts",
        "global_scope_window.ts",
        "errors.ts",
        "version.ts",
        "permissions.ts",
        "tty.ts",
        "prompt.ts",
    ],
);
