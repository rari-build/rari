use super::ExtensionTrait;
use deno_core::{Extension, extension};
use std::sync::Arc;

extension!(
    rari_fetch,
    ops = [
        crate::runtime::ops::op_fetch_with_cache,
        crate::runtime::ops::op_cache_get,
        crate::runtime::ops::op_cache_set,
    ],
    options = {
        request_context: Option<Arc<crate::server::middleware::request_context::RequestContext>>,
    },
    state = |state, options| {
        if let Some(ctx) = options.request_context {
            state.put(ctx);
        }
    },
);

impl ExtensionTrait<Option<Arc<crate::server::middleware::request_context::RequestContext>>>
    for rari_fetch
{
    fn init(
        request_context: Option<Arc<crate::server::middleware::request_context::RequestContext>>,
    ) -> Extension {
        rari_fetch::init(request_context)
    }
}

pub fn extensions(
    is_snapshot: bool,
    request_context: Option<Arc<crate::server::middleware::request_context::RequestContext>>,
) -> Vec<Extension> {
    vec![rari_fetch::build(request_context, is_snapshot)]
}
