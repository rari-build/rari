use deno_core::{Extension, ExtensionArguments, extension};
use deno_ffi::deno_ffi;

use super::{ExtensionTrait, lazy};

extension!(
    init_ffi,
    deps = [rari],
    esm_entry_point = "ext:init_ffi/init_ffi.ts",
    esm = [ dir "src/runtime/ext/ffi", "init_ffi.ts" ],
);
impl ExtensionTrait<()> for init_ffi {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<()> for deno_ffi {
    const LAZY_INIT: bool = true;

    fn init((): ()) -> Extension {
        Self::init(None)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args((): ()) -> ExtensionArguments {
        Self::args(None)
    }
}

pub fn extensions(is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), deno_ffi>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_ffi>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
