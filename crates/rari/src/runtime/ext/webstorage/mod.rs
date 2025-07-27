use super::ExtensionTrait;
use deno_core::{Extension, extension};
use std::path::PathBuf;

extension!(
    init_webstorage,
    deps = [rari],
    esm_entry_point = "ext:init_webstorage/init_webstorage.js",
    esm = [ dir "src/runtime/ext/webstorage", "init_webstorage.js" ],
);
impl ExtensionTrait<()> for init_webstorage {
    fn init((): ()) -> Extension {
        init_webstorage::init()
    }
}
impl ExtensionTrait<Option<PathBuf>> for deno_webstorage::deno_webstorage {
    fn init(origin_storage_dir: Option<PathBuf>) -> Extension {
        deno_webstorage::deno_webstorage::init(origin_storage_dir)
    }
}

pub fn extensions(origin_storage_dir: Option<PathBuf>, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_webstorage::deno_webstorage::build(origin_storage_dir, is_snapshot),
        init_webstorage::build((), is_snapshot),
    ]
}
