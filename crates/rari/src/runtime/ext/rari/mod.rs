use super::ExtensionTrait;
use deno_core::{Extension, extension};

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
