use super::{ExtensionTrait, web::WebOptions};
use deno_core::{Extension, extension};

extension!(
    init_websocket,
    deps = [rari],
    esm_entry_point = "ext:init_websocket/init_websocket.js",
    esm = [ dir "src/runtime/ext/websocket", "init_websocket.js" ],
);
impl ExtensionTrait<()> for init_websocket {
    fn init((): ()) -> Extension {
        init_websocket::init()
    }
}
impl ExtensionTrait<WebOptions> for deno_websocket::deno_websocket {
    fn init(_options: WebOptions) -> Extension {
        deno_websocket::deno_websocket::init()
    }
}

pub fn extensions(options: WebOptions, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_websocket::deno_websocket::build(options, is_snapshot),
        init_websocket::build((), is_snapshot),
    ]
}
