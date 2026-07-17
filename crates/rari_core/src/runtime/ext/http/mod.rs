use ::deno_http::Options;
use deno_core::{Extension, ExtensionArguments, extension};
use deno_http::deno_http;

use super::{ExtensionTrait, lazy};

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

fn http_options() -> Options {
    Options { http2_builder_hook: None, no_legacy_abort: false, automatic_compression: false }
}

impl ExtensionTrait<()> for deno_http {
    const LAZY_INIT: bool = true;

    fn init((): ()) -> Extension {
        Self::init(http_options())
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args((): ()) -> ExtensionArguments {
        Self::args(http_options())
    }
}

pub fn extensions((): (), is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), deno_http_runtime>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), deno_http>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_http>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
