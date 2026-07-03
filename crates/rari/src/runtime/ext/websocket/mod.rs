use deno_core::{Extension, extension};

use super::{ExtensionTrait, web::WebOptions};

extension!(
    init_websocket,
    deps = [init_utilities],
    esm_entry_point = "ext:init_websocket/init_websocket.ts",
    esm = [ dir "src/runtime/ext/websocket", "init_websocket.ts" ],
);
impl ExtensionTrait<()> for init_websocket {
    fn init((): ()) -> Extension {
        Self::init()
    }
}
impl ExtensionTrait<WebOptions> for deno_websocket::deno_websocket {
    fn init(_options: WebOptions) -> Extension {
        Self::init()
    }
}

pub fn extensions(options: WebOptions, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::build(options, is_snapshot),
        init_websocket::build((), is_snapshot),
    ]
}
