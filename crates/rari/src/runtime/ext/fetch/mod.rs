use super::ExtensionTrait;
use deno_core::{Extension, extension};

use crate::runtime::ops::FetchOpState;

extension!(
    rari_fetch,
    ops = [crate::runtime::ops::op_fetch_with_cache],
    state = |state| {
        state.put(FetchOpState::default());
    },
);

impl ExtensionTrait<()> for rari_fetch {
    fn init((): ()) -> Extension {
        rari_fetch::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![rari_fetch::build((), is_snapshot)]
}
