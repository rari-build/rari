use std::path::PathBuf;

use deno_core::{Extension, extension};

use super::ExtensionTrait;

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
impl ExtensionTrait<Option<PathBuf>> for deno_webstorage::deno_webstorage {
    fn init(origin_storage_dir: Option<PathBuf>) -> Extension {
        Self::init(origin_storage_dir)
    }
}

pub fn extensions(origin_storage_dir: Option<PathBuf>, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_webstorage::deno_webstorage::build(origin_storage_dir, is_snapshot),
        init_webstorage::build((), is_snapshot),
    ]
}
