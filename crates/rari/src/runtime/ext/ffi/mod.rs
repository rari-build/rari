use deno_core::{Extension, extension};

use super::ExtensionTrait;

extension!(
    init_ffi,
    deps = [rari],
    esm_entry_point = "ext:init_ffi/init_ffi.ts",
    esm = [ dir "src/runtime/ext/ffi", "init_ffi.ts" ],
);
impl ExtensionTrait<()> for init_ffi {
    fn init((): ()) -> Extension {
        init_ffi::init()
    }
}
impl ExtensionTrait<()> for deno_ffi::deno_ffi {
    fn init((): ()) -> Extension {
        deno_ffi::deno_ffi::init(None)
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_ffi::deno_ffi::build((), is_snapshot), init_ffi::build((), is_snapshot)]
}
