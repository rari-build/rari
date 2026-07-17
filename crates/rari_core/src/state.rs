//! Protocol-agnostic core server state shared across all backends.
//!
//! Backend-specific state (e.g. `ServerState` in the `rari` crate) holds an
//! `Arc<CoreState>` plus any backend-only fields.  Handlers that only need
//! core infrastructure take `State(Arc<CoreState>)` directly; handlers that
//! also need backend fields take the backend state and access `core` through it.

use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};

use dashmap::DashMap;

use crate::{
    cache::{handler::CacheHandler, response::{PrebuiltResponse, ResponseCache}},
    config::Config,
    image::ImageOptimizer,
};

/// Shared server infrastructure that is independent of the rendering backend.
///
/// Every backend (RSC, TanStack, etc.) wraps this in its own state and merges
/// routes via `Router<()>`.
#[derive(Clone)]
#[expect(clippy::exhaustive_structs, reason = "Shared across crate boundary; backend-specific state constructs via literal syntax")]
pub struct CoreState {
    /// Global configuration.
    pub config: Arc<Config>,
    /// Monotonically increasing request counter (monitoring / observability).
    pub request_count: Arc<AtomicU64>,
    /// Wall-clock instant when the server started.
    pub start_time: Instant,
    /// Byte-level response cache with TTL, tag-based invalidation, and
    /// compression support.
    pub response_cache: Arc<ResponseCache>,
    /// Cache for fully-built static responses (e.g. pre-rendered HTML pages).
    pub static_fast_cache: Arc<DashMap<String, Arc<PrebuiltResponse>>>,
    /// Project root directory on disk.
    pub project_root: PathBuf,
    /// Optional image optimizer (resize, format conversion, caching).
    pub image_optimizer: Option<Arc<ImageOptimizer>>,
    /// Shared byte-level cache handler used by the image pipeline.
    pub image_handler: Arc<dyn CacheHandler>,
}
