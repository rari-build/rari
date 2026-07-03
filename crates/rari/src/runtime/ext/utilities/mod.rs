use deno_core::{Extension, extension};

use super::ExtensionTrait;

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

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![init_utilities::build((), is_snapshot)]
}
