use deno_core::{Extension, extension};

use super::ExtensionTrait;

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

impl ExtensionTrait<()> for deno_cache::deno_cache {
    fn init((): ()) -> Extension {
        Self::init(None)
    }
}

pub fn extensions(_options: Option<()>, is_snapshot: bool) -> Vec<Extension> {
    vec![deno_cache::deno_cache::build((), is_snapshot), init_cache::build((), is_snapshot)]
}
