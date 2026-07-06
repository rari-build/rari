use ::deno_io::Stdio;
use deno_core::{Extension, ExtensionArguments, extension};
use deno_io::deno_io;
use tty::deno_tty;

use super::{ExtensionTrait, lazy};

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
    const LAZY_INIT: bool = true;

    fn init(pipes: Option<Stdio>) -> Extension {
        Self::init(pipes)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(pipes: Option<Stdio>) -> ExtensionArguments {
        Self::args(pipes)
    }
}

impl ExtensionTrait<()> for deno_tty {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(
    pipes: Option<Stdio>,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<Option<Stdio>, deno_io>(pipes, is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), deno_tty>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_io>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
