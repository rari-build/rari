//! Protocol-agnostic core server state shared across all backends.
//!
//! Backend-specific state holds an `Arc<CoreState>` plus any backend-only
//! fields.  Handlers that only need core infrastructure can extract
//! `Arc<CoreState>` from the backend state via axum's `FromRef` trait.

use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};

use dashmap::DashMap;

use crate::{
    cache::{
        handler::CacheHandlerRegistry,
        response::{PrebuiltResponse, ResponseCache},
    },
    config::Config,
    image::ImageOptimizer,
};

/// Shared server infrastructure that is independent of the rendering backend.
#[derive(Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "Shared across crate boundary; backend-specific state constructs via literal syntax"
)]
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
    /// Named cache handler registry — resolves logical cache layers to
    /// concrete [`CacheHandler`](crate::cache::handler::CacheHandler)
    /// implementations (memory, Redis, etc.).
    pub cache_registry: Arc<CacheHandlerRegistry>,
}
