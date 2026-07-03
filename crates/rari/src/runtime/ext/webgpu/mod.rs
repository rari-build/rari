use deno_core::Extension;

use super::ExtensionTrait;

impl ExtensionTrait<()> for deno_webgpu::deno_webgpu {
    fn init((): ()) -> Extension {
        Self::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_webgpu::deno_webgpu::build((), is_snapshot)]
}
