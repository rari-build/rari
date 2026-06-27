use deno_core::{Extension, extension};

use super::ExtensionTrait;

extension!(
    init_webidl,
    deps = [init_utilities],
    esm_entry_point = "ext:init_webidl/init_webidl.ts",
    esm = [ dir "src/runtime/ext/webidl", "init_webidl.ts" ],
);

impl ExtensionTrait<()> for init_webidl {
    fn init((): ()) -> Extension {
        init_webidl::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_webidl::deno_webidl::init(), init_webidl::build((), is_snapshot)]
}
