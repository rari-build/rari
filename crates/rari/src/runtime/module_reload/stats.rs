use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct ReloadStats {
    pub total_reloads: u64,
    pub successful_reloads: u64,
    pub failed_reloads: u64,
    pub average_reload_time_ms: f64,
    pub estimated_memory_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ReloadHistoryEntry {
    pub component_id: String,
    pub timestamp: Instant,
    pub success: bool,
    pub duration_ms: u64,
}

impl ReloadStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_reload(&mut self, success: bool, duration_ms: u64) {
        self.total_reloads += 1;
        if success {
            self.successful_reloads += 1;
        } else {
            self.failed_reloads += 1;
        }

        let total_time = self.average_reload_time_ms * (self.total_reloads - 1) as f64;
        self.average_reload_time_ms = (total_time + duration_ms as f64) / self.total_reloads as f64;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_reloads == 0 {
            0.0
        } else {
            (self.successful_reloads as f64 / self.total_reloads as f64) * 100.0
        }
    }

    pub fn failure_rate(&self) -> f64 {
        if self.total_reloads == 0 {
            0.0
        } else {
            (self.failed_reloads as f64 / self.total_reloads as f64) * 100.0
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update_memory_estimate(&mut self, bytes: u64) {
        self.estimated_memory_bytes = bytes;
    }
}

impl ReloadHistoryEntry {
    pub fn new(component_id: String, success: bool, duration_ms: u64) -> Self {
        Self { component_id, timestamp: Instant::now(), success, duration_ms }
    }

    pub fn is_older_than(&self, duration: std::time::Duration) -> bool {
        self.timestamp.elapsed() > duration
    }
}
