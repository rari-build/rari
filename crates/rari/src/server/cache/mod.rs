pub mod handler;
pub mod response;
pub mod revalidate;
pub mod warmup;
use std::sync::Arc;

pub use handler::*;

use crate::server::ServerState;

pub(crate) async fn merge_page_cache_tags(
    state: &ServerState,
    base_tags: Vec<String>,
) -> Vec<String> {
    let page_cache_tags = {
        let renderer = state.renderer.lock().await;
        let runtime = Arc::clone(&renderer.runtime);
        drop(renderer);
        runtime.collect_page_cache_tags().await.unwrap_or_default()
    };
    response::RouteCachePolicy::merge_cache_tags(base_tags, &page_cache_tags)
}
