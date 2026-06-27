use deno_core::{Extension, extension};

use super::ExtensionTrait;

extension!(
    init_crypto,
    deps = [init_utilities],
    esm_entry_point = "ext:init_crypto/init_crypto.ts",
    esm = [ dir "src/runtime/ext/crypto", "init_crypto.ts" ],
);
impl ExtensionTrait<()> for init_crypto {
    fn init((): ()) -> Extension {
        init_crypto::init()
    }
}
impl ExtensionTrait<Option<u64>> for deno_crypto::deno_crypto {
    fn init(seed: Option<u64>) -> Extension {
        deno_crypto::deno_crypto::init(seed)
    }
}

pub fn extensions(seed: Option<u64>, is_snapshot: bool) -> Vec<Extension> {
    vec![deno_crypto::deno_crypto::build(seed, is_snapshot), init_crypto::build((), is_snapshot)]
}
