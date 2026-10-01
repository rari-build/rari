use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::constants::{
    DEFAULT_MAX_CACHE_SIZE, DEFAULT_MAX_CONCURRENT_RENDERS, DEFAULT_MAX_MEMORY_PER_COMPONENT_MB,
    DEFAULT_MAX_RENDER_TIME_MS, DEFAULT_MAX_SCRIPT_EXECUTION_TIME_MS,
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ResourceLimits {
    pub max_concurrent_renders: usize,
    pub max_render_time_ms: u64,
    pub max_script_execution_time_ms: u64,
    pub max_memory_per_component_mb: usize,
    pub max_cache_size: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_concurrent_renders: DEFAULT_MAX_CONCURRENT_RENDERS,
            max_render_time_ms: DEFAULT_MAX_RENDER_TIME_MS,
            max_script_execution_time_ms: DEFAULT_MAX_SCRIPT_EXECUTION_TIME_MS,
            max_memory_per_component_mb: DEFAULT_MAX_MEMORY_PER_COMPONENT_MB,
            max_cache_size: DEFAULT_MAX_CACHE_SIZE,
        }
    }
}

pub struct ResourceTracker {
    pub(crate) active_renders: AtomicUsize,
    pub(crate) cache_hits: AtomicU64,
    pub(crate) cache_misses: AtomicU64,
    pub(crate) timeout_errors: AtomicU64,
    pub(crate) memory_pressure_events: AtomicU64,
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            active_renders: AtomicUsize::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            timeout_errors: AtomicU64::new(0),
            memory_pressure_events: AtomicU64::new(0),
        }
    }

    pub fn increment_active_renders(&self) {
        self.active_renders.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_active_renders(&self) {
        self.active_renders.fetch_sub(1, Ordering::Relaxed);
    }
}
