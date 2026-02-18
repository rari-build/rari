use super::ExtensionTrait;
use deno_core::Extension;

impl ExtensionTrait<()> for deno_node_crypto::deno_node_crypto {
    fn init((): ()) -> Extension {
        deno_node_crypto::deno_node_crypto::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_node_crypto::deno_node_crypto::build((), is_snapshot)]
}
