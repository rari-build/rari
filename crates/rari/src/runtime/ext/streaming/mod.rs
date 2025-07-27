use super::ExtensionTrait;
use deno_core::{Extension, extension};

use crate::runtime::ops::StreamOpState;

extension!(
    rari_streaming,
    state = |state| {
        state.put(StreamOpState::default());
    },
);

impl ExtensionTrait<()> for rari_streaming {
    fn init((): ()) -> Extension {
        rari_streaming::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![rari_streaming::build((), is_snapshot)]
}
