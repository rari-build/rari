use deno_core::{Extension, ExtensionArguments, extension};
use deno_crypto::deno_crypto;

use super::{ExtensionTrait, lazy};

extension!(
    init_crypto,
    deps = [init_utilities],
    esm_entry_point = "ext:init_crypto/init_crypto.ts",
    esm = [ dir "src/runtime/ext/crypto", "init_crypto.ts" ],
);

impl ExtensionTrait<()> for init_crypto {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

impl ExtensionTrait<Option<u64>> for deno_crypto {
    const LAZY_INIT: bool = true;

    fn init(seed: Option<u64>) -> Extension {
        Self::init(seed)
    }

    fn lazy_init() -> Extension {
        Self::lazy_init()
    }

    fn lazy_args(seed: Option<u64>) -> ExtensionArguments {
        Self::args(seed)
    }
}

pub fn extensions(
    seed: Option<u64>,
    is_snapshot: bool,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<Option<u64>, deno_crypto>(seed, is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_crypto>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
