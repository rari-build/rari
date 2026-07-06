use deno_core::{Extension, ExtensionArguments};
use deno_node_crypto::deno_node_crypto;

use super::{ExtensionTrait, lazy};

impl ExtensionTrait<()> for deno_node_crypto {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), deno_node_crypto>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
