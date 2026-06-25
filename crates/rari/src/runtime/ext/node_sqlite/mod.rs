use deno_core::Extension;

use super::ExtensionTrait;

impl ExtensionTrait<()> for deno_node_sqlite::deno_node_sqlite {
    fn init((): ()) -> Extension {
        deno_node_sqlite::deno_node_sqlite::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![deno_node_sqlite::deno_node_sqlite::build((), is_snapshot)]
}
