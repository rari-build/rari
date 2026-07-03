use ::deno_http::Options;
use deno_core::{Extension, extension};
use deno_http::deno_http;

use super::ExtensionTrait;

mod runtime;
use runtime::deno_http_runtime;

impl ExtensionTrait<()> for deno_http_runtime {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

extension!(
    init_http,
    deps = [rari],
    esm_entry_point = "ext:init_http/init_http.ts",
    esm = [ dir "src/runtime/ext/http", "init_http.ts" ],
);
impl ExtensionTrait<()> for init_http {
    fn init((): ()) -> Extension {
        Self::init()
    }
}
impl ExtensionTrait<()> for deno_http {
    fn init((): ()) -> Extension {
        Self::init(Options {
            http2_builder_hook: None,
            no_legacy_abort: false,
            automatic_compression: false,
        })
    }
}

pub fn extensions((): (), is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_http_runtime::build((), is_snapshot),
        deno_http::build((), is_snapshot),
        init_http::build((), is_snapshot),
    ]
}
