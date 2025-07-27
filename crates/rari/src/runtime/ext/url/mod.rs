use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_url,
    deps = [rari],
    esm_entry_point = "ext:init_url/init_url.js",
    esm = [ dir "src/runtime/ext/url", "init_url.js" ],
);
impl ExtensionTrait<()> for init_url {
    fn init((): ()) -> Extension {
        init_url::init()
    }
}
impl ExtensionTrait<()> for deno_url::deno_url {
    fn init((): ()) -> Extension {
        deno_url::deno_url::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_url::deno_url::build((), is_snapshot), init_url::build((), is_snapshot)]
}
