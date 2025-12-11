use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_cache,
    deps = [rari, deno_cache],
    esm_entry_point = "ext:init_cache/init_cache.js",
    esm = [ dir "src/runtime/ext/cache", "init_cache.js" ],
);
impl ExtensionTrait<()> for init_cache {
    fn init((): ()) -> Extension {
        init_cache::init()
    }
}

impl ExtensionTrait<()> for deno_cache::deno_cache {
    fn init((): ()) -> Extension {
        deno_cache::deno_cache::init(None)
    }
}

pub fn extensions(_options: Option<()>, is_snapshot: bool) -> Vec<Extension> {
    vec![deno_cache::deno_cache::build((), is_snapshot), init_cache::build((), is_snapshot)]
}
