use deno_core::{Extension, extension};

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
impl ExtensionTrait<Option<deno_io::Stdio>> for deno_io::deno_io {
    fn init(pipes: Option<deno_io::Stdio>) -> Extension {
        Self::init(pipes)
    }
}
impl ExtensionTrait<()> for tty::deno_tty {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(pipes: Option<deno_io::Stdio>, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_io::deno_io::build(pipes, is_snapshot),
        tty::deno_tty::build((), is_snapshot),
        init_io::build((), is_snapshot),
    ]
}
