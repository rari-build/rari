use super::ExtensionTrait;
use deno_core::{Extension, extension};
use deno_cron::local::LocalCronHandler;

extension!(
    init_cron,
    deps = [rari],
    esm_entry_point = "ext:init_cron/init_cron.js",
    esm = [ dir "src/runtime/ext/cron", "init_cron.js" ],
);
impl ExtensionTrait<()> for init_cron {
    fn init((): ()) -> Extension {
        init_cron::init()
    }
}
impl ExtensionTrait<()> for deno_cron::deno_cron {
    fn init((): ()) -> Extension {
        deno_cron::deno_cron::init(LocalCronHandler::new())
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_cron::deno_cron::build((), is_snapshot), init_cron::build((), is_snapshot)]
}
