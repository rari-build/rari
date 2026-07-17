use deno_core::{Extension, ExtensionArguments, extension};

use super::{ExtensionTrait, lazy};

extension!(
    init_utilities,
    esm_entry_point = "ext:init_utilities/utilities.ts",
    esm = [ dir "src/runtime/ext/utilities", "utilities.ts" ],
);

impl ExtensionTrait<()> for init_utilities {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), init_utilities>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
