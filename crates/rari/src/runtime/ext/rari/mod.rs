use super::ExtensionTrait;
use deno_core::{Extension, extension};

pub mod redis_cache;

extension!(
    rari,
    esm_entry_point = "ext:rari/rari.js",
    esm = [ dir "src/runtime/ext/rari", "rari.js" ],
);

impl ExtensionTrait<()> for rari {
    fn init((): ()) -> Extension {
        rari::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![rari::build((), is_snapshot)]
}

pub fn redis_cache_extensions(is_snapshot: bool) -> Vec<Extension> {
    redis_cache::extensions(None, is_snapshot)
}
