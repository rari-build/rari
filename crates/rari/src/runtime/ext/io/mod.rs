use ::deno_io::Stdio;
use deno_core::{Extension, extension};
use deno_io::deno_io;
use tty::deno_tty;

use super::ExtensionTrait;

#[cfg(windows)]
mod tty_windows;
#[cfg(windows)]
use tty_windows as tty;

#[cfg(unix)]
mod tty_unix;
#[cfg(unix)]
use tty_unix as tty;

extension!(
    init_io,
    deps = [rari],
    esm_entry_point = "ext:init_io/init_io.ts",
    esm = [ dir "src/runtime/ext/io", "init_io.ts" ],
);
impl ExtensionTrait<()> for init_io {
    fn init((): ()) -> Extension {
        Self::init()
    }
}
impl ExtensionTrait<Option<Stdio>> for deno_io {
    fn init(pipes: Option<Stdio>) -> Extension {
        Self::init(pipes)
    }
}
impl ExtensionTrait<()> for deno_tty {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(pipes: Option<Stdio>, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_io::build(pipes, is_snapshot),
        deno_tty::build((), is_snapshot),
        init_io::build((), is_snapshot),
    ]
}
