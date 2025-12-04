use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use super::constants::*;

#[derive(Debug, Clone)]
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
    pub(crate) total_renders: AtomicU64,
    pub(crate) total_render_time_ms: AtomicU64,
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
            total_renders: AtomicU64::new(0),
            total_render_time_ms: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            timeout_errors: AtomicU64::new(0),
            memory_pressure_events: AtomicU64::new(0),
        }
    }

    pub fn get_metrics(&self) -> ResourceMetrics {
        let total_renders = self.total_renders.load(Ordering::Relaxed);
        let total_time = self.total_render_time_ms.load(Ordering::Relaxed);

        ResourceMetrics {
            active_renders: self.active_renders.load(Ordering::Relaxed),
            total_renders,
            average_render_time_ms: if total_renders > 0 {
                total_time as f64 / total_renders as f64
            } else {
                0.0
            },
            cache_hit_rate: {
                let hits = self.cache_hits.load(Ordering::Relaxed);
                let misses = self.cache_misses.load(Ordering::Relaxed);
                let total = hits + misses;
                if total > 0 { hits as f64 / total as f64 } else { 0.0 }
            },
            timeout_errors: self.timeout_errors.load(Ordering::Relaxed),
            memory_pressure_events: self.memory_pressure_events.load(Ordering::Relaxed),
        }
    }

    pub fn increment_active_renders(&self) {
        self.active_renders.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_active_renders(&self) {
        self.active_renders.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_render_completion(&self, duration: Duration) {
        self.total_renders.fetch_add(1, Ordering::Relaxed);
        self.total_render_time_ms.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_timeout_error(&self) {
        self.timeout_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_memory_pressure(&self) {
        self.memory_pressure_events.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceMetrics {
    pub active_renders: usize,
    pub total_renders: u64,
    pub average_render_time_ms: f64,
    pub cache_hit_rate: f64,
    pub timeout_errors: u64,
    pub memory_pressure_events: u64,
}
