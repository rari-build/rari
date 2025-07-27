use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_server_functions,
    deps = [rari, init_promise_manager, init_rsc_modules],
    esm_entry_point = "ext:init_server_functions/init_server_functions.js",
    esm = [ dir "src/runtime/ext/server_functions", "init_server_functions.js" ],
);

impl ExtensionTrait<()> for init_server_functions {
    fn init((): ()) -> Extension {
        init_server_functions::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![init_server_functions::build((), is_snapshot)]
}
