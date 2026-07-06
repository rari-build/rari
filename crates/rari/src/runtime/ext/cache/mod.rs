use deno_cache::deno_cache;
use deno_core::{Extension, ExtensionArguments, extension};

use super::{ExtensionTrait, lazy};

extension!(
    init_cache,
    deps = [init_utilities, deno_cache],
    esm_entry_point = "ext:init_cache/init_cache.ts",
    esm = [ dir "src/runtime/ext/cache", "init_cache.ts" ],
);
impl ExtensionTrait<()> for init_cache {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<()> for deno_cache {
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

pub fn extensions(
    _options: Option<()>,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), deno_cache>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_cache>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
