use deno_core::{Extension, ExtensionArguments, extension};
use deno_websocket::deno_websocket;

use super::{ExtensionTrait, lazy, web::WebOptions};

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

impl ExtensionTrait<WebOptions> for deno_websocket {
    const LAZY_INIT: bool = true;

    fn init(_options: WebOptions) -> Extension {
        Self::init()
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(_options: WebOptions) -> ExtensionArguments {
        Self::args()
    }
}

pub fn extensions(
    options: WebOptions,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<WebOptions, deno_websocket>(
        options,
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    lazy::register::<(), init_websocket>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
