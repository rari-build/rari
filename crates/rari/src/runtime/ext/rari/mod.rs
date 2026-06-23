use deno_core::{Extension, extension};

use super::ExtensionTrait;

extension!(
    rari,
    esm_entry_point = "ext:rari/rari.ts",
    esm = [
        dir "src/runtime/ext/rari",
        "rari.ts",
        "cookies.ts",
        "api_handler.ts",
        "component_loader.ts",
        "metadata_collector.ts",
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
