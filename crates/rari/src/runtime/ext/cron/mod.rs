use ::deno_cron::local::LocalCronHandler;
use deno_core::{Extension, ExtensionArguments, extension};
use deno_cron::deno_cron;

use super::{ExtensionTrait, lazy};

extension!(
    init_cron,
    deps = [rari],
    esm_entry_point = "ext:init_cron/init_cron.ts",
    esm = [ dir "src/runtime/ext/cron", "init_cron.ts" ],
);
impl ExtensionTrait<()> for init_cron {
    fn init((): ()) -> Extension {
        Self::init()
    }
}
impl ExtensionTrait<()> for deno_cron {
    fn init((): ()) -> Extension {
        Self::init(Box::new(LocalCronHandler::new()))
    }
}

pub fn extensions(is_snapshot: bool) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<(), deno_cron>((), is_snapshot, &mut extensions, &mut lazy_args);
    lazy::register::<(), init_cron>((), is_snapshot, &mut extensions, &mut lazy_args);
    (extensions, lazy_args)
}
