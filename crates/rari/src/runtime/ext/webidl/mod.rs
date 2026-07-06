use deno_core::{Extension, ExtensionArguments, extension};
use deno_webidl::deno_webidl;

use super::{ExtensionTrait, lazy};

extension!(
    init_webidl,
    deps = [init_utilities],
    esm_entry_point = "ext:init_webidl/init_webidl.ts",
    esm = [ dir "src/runtime/ext/webidl", "init_webidl.ts" ],
);

impl ExtensionTrait<()> for init_webidl {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<()> for deno_webidl {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), deno_webidl>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_webidl>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
