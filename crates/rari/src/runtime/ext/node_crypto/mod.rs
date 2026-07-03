use deno_core::Extension;
use deno_node_crypto::deno_node_crypto;

use super::ExtensionTrait;

impl ExtensionTrait<()> for deno_node_crypto {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_node_crypto::build((), is_snapshot)]
}
