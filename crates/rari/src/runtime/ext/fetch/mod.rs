use std::sync::Arc;

use deno_core::{Extension, extension};

use super::ExtensionTrait;
use crate::{runtime::ops, server::middleware::request_context::RequestContext};

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
) -> Vec<Extension> {
    vec![rari_fetch::build(request_context, is_snapshot)]
}
