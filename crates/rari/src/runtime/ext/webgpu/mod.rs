use deno_core::{Extension, ExtensionArguments};
use deno_webgpu::deno_webgpu;

use super::{ExtensionTrait, lazy};

impl ExtensionTrait<()> for deno_webgpu {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), deno_webgpu>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
