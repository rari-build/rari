use super::ExtensionTrait;
use deno_core::Extension;

impl ExtensionTrait<()> for deno_napi::deno_napi {
    fn init((): ()) -> Extension {
        deno_napi::deno_napi::init(None)
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_napi::deno_napi::build((), is_snapshot)]
}
