use deno_core::Extension;
use deno_napi::deno_napi;

use super::ExtensionTrait;

impl ExtensionTrait<()> for deno_napi {
    fn init((): ()) -> Extension {
        Self::init(None)
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_napi::build((), is_snapshot)]
}
