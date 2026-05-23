use super::ExtensionTrait;
use deno_core::Extension;

impl ExtensionTrait<()> for deno_webgpu::deno_webgpu {
    fn init((): ()) -> Extension {
        deno_webgpu::deno_webgpu::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_webgpu::deno_webgpu::build((), is_snapshot)]
}
