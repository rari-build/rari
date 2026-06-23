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
    let mut exts = vec![rari::build((), is_snapshot)];

    let redis_enabled = crate::server::config::Config::get()
        .and_then(|c| c.use_cache.remote.as_ref())
        .filter(|layer| layer.handler == "redis")
        .and_then(|layer| layer.url.as_deref())
        .map(str::trim)
        .as_ref()
        .is_some_and(|url| !url.is_empty());

    if redis_enabled {
        exts.extend(redis_cache::extensions(None, is_snapshot));
    }

    exts
}
