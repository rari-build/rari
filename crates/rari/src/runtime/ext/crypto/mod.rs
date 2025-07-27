use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_crypto,
    deps = [rari],
    esm_entry_point = "ext:init_crypto/init_crypto.js",
    esm = [ dir "src/runtime/ext/crypto", "init_crypto.js" ],
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
