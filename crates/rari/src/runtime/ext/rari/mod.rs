pub mod cache;

use deno_core::{Extension, extension};

use super::ExtensionTrait;

extension!(
    rari,
    deps = [init_utilities, init_web],
    esm_entry_point = "ext:rari/core/rari.ts",
    esm = [
        dir "src/runtime/ext/rari",
        "core/rari.ts",
        "http/api_handler.ts",
        "rsc/client_registry.ts",
        "react/component_loader.ts",
        "http/cookies.ts",
        "react/metadata_collector.ts",
        "rsc/rsc_modules.ts",
        "rsc/server_functions.ts"
    ],
    lazy_loaded_esm = [
        dir "src/runtime/ext/rari",
        "react/vendor/react.js",
        "react/vendor/react-server.js",
        "react/vendor/react-jsx-runtime.js",
        "react/vendor/react-dom-server.js",
        "react/vendor/react-dom.js",
        "react/vendor/react-server-dom-webpack-client.js",
        "react/vendor/react-server-dom-webpack-server.js",
        "react/vendor/index.js",
    ],
);

impl ExtensionTrait<()> for rari {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![rari::build((), is_snapshot)]
}
