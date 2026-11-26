// Copyright 2018-2026 the Deno authors. MIT license.

use deno_core::{OpState, op2};
use deno_terminal::colors::ColorLevel;

use crate::BootstrapOptions;

deno_core::extension!(
    deno_bootstrap,
    ops = [
        op_bootstrap_args,
        op_bootstrap_pid,
        op_bootstrap_numcpus,
        op_bootstrap_user_agent,
        op_bootstrap_language,
        op_bootstrap_color_depth,
        op_bootstrap_no_color,
        op_bootstrap_unstable_args,
        op_snapshot_options,
    ],
);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotOptions {
    pub ts_version: String,
    pub v8_version: &'static str,
    pub target: String,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        let arch = std::env::consts::ARCH;
        let platform = std::env::consts::OS;
        let target = match platform {
            "macos" => format!("{arch}-apple-darwin"),
            "linux" => format!("{arch}-unknown-linux-gnu"),
            "windows" => format!("{arch}-pc-windows-msvc"),
            rest => format!("{arch}-{rest}"),
        };

        Self { ts_version: "n/a".to_owned(), v8_version: deno_core::v8::VERSION_STRING, target }
    }
}

#[op2]
#[serde]
pub fn op_snapshot_options() -> SnapshotOptions {
    SnapshotOptions::default()
}

#[op2]
pub fn op_bootstrap_args(state: &OpState) -> Vec<String> {
    state.borrow::<BootstrapOptions>().args.clone()
}

#[op2(fast)]
#[smi]
pub fn op_bootstrap_pid() -> u32 {
    std::process::id()
}

#[op2(fast)]
#[smi]
pub fn op_bootstrap_numcpus(state: &OpState) -> u32 {
    state.borrow::<BootstrapOptions>().cpu_count as u32
}

#[op2]
#[string]
pub fn op_bootstrap_user_agent(state: &OpState) -> String {
    state.borrow::<BootstrapOptions>().user_agent.clone()
}

#[op2]
pub fn op_bootstrap_unstable_args() -> Vec<String> {
    Vec::new()
}

#[op2]
#[string]
pub fn op_bootstrap_language(state: &OpState) -> String {
    state.borrow::<BootstrapOptions>().locale.clone()
}

#[op2(fast)]
pub fn op_bootstrap_color_depth(state: &OpState) -> i32 {
    let options = state.borrow::<BootstrapOptions>();
    match options.color_level {
        ColorLevel::None => 1,
        ColorLevel::Ansi => 4,
        ColorLevel::Ansi256 => 8,
        ColorLevel::TrueColor => 24,
    }
}

#[op2(fast)]
pub fn op_bootstrap_no_color() -> bool {
    !deno_terminal::colors::use_color()
}
