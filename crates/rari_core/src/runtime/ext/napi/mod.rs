use deno_core::{Extension, ExtensionArguments};
use deno_napi::deno_napi;

use super::{ExtensionTrait, lazy};

impl ExtensionTrait<()> for deno_napi {
    const LAZY_INIT: bool = true;

    fn init((): ()) -> Extension {
        Self::init(None)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args((): ()) -> ExtensionArguments {
        Self::args(None)
    }
}

pub fn extensions(is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), deno_napi>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
