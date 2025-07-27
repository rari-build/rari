use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_rsc_modules,
    deps = [rari, init_promise_manager],
    esm_entry_point = "ext:init_rsc_modules/init_rsc_modules.js",
    esm = [ dir "src/runtime/ext/rsc_modules", "init_rsc_modules.js" ],
);

impl ExtensionTrait<()> for init_rsc_modules {
    fn init((): ()) -> Extension {
        init_rsc_modules::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![init_rsc_modules::build((), is_snapshot)]
}
