use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    module_reload,
    esm_entry_point = "ext:module_reload/init_module_reload.js",
    esm = [ dir "src/runtime/ext/module_reload", "init_module_reload.js" ],
);

impl ExtensionTrait<()> for module_reload {
    #[allow(unused_variables)]
    fn init(options: ()) -> Extension {
        module_reload::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![module_reload::build((), is_snapshot)]
}
