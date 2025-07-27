use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_promise_manager,
    deps = [rari],
    esm_entry_point = "ext:init_promise_manager/init_promise_manager.js",
    esm = [ dir "src/runtime/ext/promise_manager", "init_promise_manager.js" ],
);

impl ExtensionTrait<()> for init_promise_manager {
    fn init((): ()) -> Extension {
        init_promise_manager::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![init_promise_manager::build((), is_snapshot)]
}
