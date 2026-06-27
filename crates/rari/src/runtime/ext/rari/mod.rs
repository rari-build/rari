pub mod cache;

use cache::redis_cache;
use deno_core::{Extension, extension};

use super::ExtensionTrait;

extension!(
    rari,
    deps = [init_utilities, init_web],
    esm_entry_point = "ext:rari/core/rari.ts",
    esm = [
        dir "src/runtime/ext/rari",
        "core/rari.ts",
        "http/api_handler.ts",
        "rsc/client_registry.ts",
        "react/component_loader.ts",
        "http/cookies.ts",
        "react/metadata_collector.ts",
        "react/react_shims.ts",
        "rsc/rsc_modules.ts",
        "rsc/server_functions.ts"
    ],
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
