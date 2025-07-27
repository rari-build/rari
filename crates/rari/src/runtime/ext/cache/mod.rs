use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_cache,
    deps = [rari],
    esm_entry_point = "ext:init_cache/init_cache.js",
    esm = [ dir "src/runtime/ext/cache", "init_cache.js" ],
);
impl ExtensionTrait<()> for init_cache {
    fn init((): ()) -> Extension {
        init_cache::init()
    }
}
impl ExtensionTrait<Option<deno_cache::CreateCache>> for deno_cache::deno_cache {
    fn init(options: Option<deno_cache::CreateCache>) -> Extension {
        deno_cache::deno_cache::init(options)
    }
}

pub fn extensions(options: Option<deno_cache::CreateCache>, is_snapshot: bool) -> Vec<Extension> {
    vec![deno_cache::deno_cache::build(options, is_snapshot), init_cache::build((), is_snapshot)]
}
