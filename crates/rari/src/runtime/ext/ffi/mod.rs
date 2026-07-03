use deno_core::{Extension, extension};
use deno_ffi::deno_ffi;

use super::ExtensionTrait;

extension!(
    init_ffi,
    deps = [rari],
    esm_entry_point = "ext:init_ffi/init_ffi.ts",
    esm = [ dir "src/runtime/ext/ffi", "init_ffi.ts" ],
);
impl ExtensionTrait<()> for init_ffi {
    fn init((): ()) -> Extension {
        Self::init()
    }
}
impl ExtensionTrait<()> for deno_ffi {
    fn init((): ()) -> Extension {
        Self::init(None)
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_ffi::build((), is_snapshot), init_ffi::build((), is_snapshot)]
}
