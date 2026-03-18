use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_rsc_renderer,
    deps = [rari, init_react, init_rsc_modules],
    esm_entry_point = "ext:init_rsc_renderer/init_rsc_renderer.js",
            esm = [
        dir "src/runtime/ext/rsc_renderer",
        "init_rsc_renderer.js",
        "rsc_traversal.js",
        "client_registry.js"
    ],
);

impl ExtensionTrait<()> for init_rsc_renderer {
    fn init((): ()) -> Extension {
        init_rsc_renderer::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![init_rsc_renderer::build((), is_snapshot)]
}
