use super::ExtensionTrait;
use deno_core::{Extension, extension};

extension!(
    init_react,
    deps = [rari],
    esm_entry_point = "ext:init_react/init_react.js",
    esm = [ dir "src/runtime/ext/react", "init_react.js" ],
);

impl ExtensionTrait<()> for init_react {
    fn init((): ()) -> Extension {
        init_react::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![init_react::build((), is_snapshot)]
}
