use std::path::PathBuf;

use deno_core::{Extension, ExtensionArguments, extension};
use deno_webstorage::deno_webstorage;

use super::{ExtensionTrait, lazy};

extension!(
    init_webstorage,
    deps = [init_utilities],
    esm_entry_point = "ext:init_webstorage/init_webstorage.ts",
    esm = [ dir "src/runtime/ext/webstorage", "init_webstorage.ts" ],
);

impl ExtensionTrait<()> for init_webstorage {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<Option<PathBuf>> for deno_webstorage {
    const LAZY_INIT: bool = true;

    fn init(origin_storage_dir: Option<PathBuf>) -> Extension {
        Self::init(origin_storage_dir)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(origin_storage_dir: Option<PathBuf>) -> ExtensionArguments {
        Self::args(origin_storage_dir)
    }
}

pub fn extensions(
    origin_storage_dir: Option<PathBuf>,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<Option<PathBuf>, deno_webstorage>(
        origin_storage_dir,
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), init_webstorage>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
