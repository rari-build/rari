use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_webidl,
    deps = [rari],
    esm_entry_point = "ext:init_webidl/init_webidl.js",
    esm = [ dir "src/runtime/ext/webidl", "init_webidl.js" ],
);

impl ExtensionTrait<()> for init_webidl {
    fn init((): ()) -> Extension {
        Extension::default()
    }
}

#[allow(unused)]
pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_webidl::deno_webidl::init(), <init_webidl as ExtensionTrait<()>>::init(())]
}
