use std::sync::Arc;

use deno_core::{Extension, ExtensionArguments, extension};

use super::{ExtensionTrait, lazy};
use crate::{request_context::RequestContext, runtime::ops};

extension!(
    rari_fetch,
    ops = [
        ops::op_fetch_with_cache,
        ops::op_cache_get,
        ops::op_cache_set,
    ],
    options = {
        request_context: Option<Arc<RequestContext>>,
    },
    state = |state, options| {
        if let Some(ctx) = options.request_context {
            state.put(ctx);
        }
    },
);

impl ExtensionTrait<Option<Arc<RequestContext>>> for rari_fetch {
    fn init(request_context: Option<Arc<RequestContext>>) -> Extension {
        Self::init(request_context)
    }
}

pub fn extensions(
    is_snapshot: bool,
    request_context: Option<Arc<RequestContext>>,
) -> (Vec<Extension>, Vec<ExtensionArguments>) {
    let mut extensions = Vec::new();
    let mut lazy_args = Vec::new();
    lazy::register::<Option<Arc<RequestContext>>, rari_fetch>(
        request_context,
        is_snapshot,
        &mut extensions,
        &mut lazy_args,
    );
    (extensions, lazy_args)
}
