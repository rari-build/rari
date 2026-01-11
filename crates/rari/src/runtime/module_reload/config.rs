#[derive(Debug, Clone)]
pub struct ReloadConfig {
    pub enabled: bool,
    pub max_retry_attempts: usize,
    pub reload_timeout_ms: u64,
    pub parallel_reloads: bool,
    pub debounce_delay_ms: u64,
    pub max_history_size: usize,
    pub enable_memory_monitoring: bool,
}

impl Default for ReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retry_attempts: 3,
            reload_timeout_ms: 5000,
            parallel_reloads: true,
            debounce_delay_ms: 150,
            max_history_size: 100,
            enable_memory_monitoring: true,
        }
    }
}

impl ReloadConfig {
    pub fn production() -> Self {
        Self {
            enabled: true,
            max_retry_attempts: 2,
            reload_timeout_ms: 3000,
            parallel_reloads: true,
            debounce_delay_ms: 200,
            max_history_size: 50,
            enable_memory_monitoring: false,
        }
    }

    pub fn development() -> Self {
        Self {
            enabled: true,
            max_retry_attempts: 5,
            reload_timeout_ms: 10000,
            parallel_reloads: true,
            debounce_delay_ms: 100,
            max_history_size: 200,
            enable_memory_monitoring: true,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.max_retry_attempts == 0 {
            return Err("max_retry_attempts must be greater than 0".to_string());
        }

        if self.reload_timeout_ms < 100 {
            return Err("reload_timeout_ms should be at least 100ms".to_string());
        }

        if self.debounce_delay_ms > 10000 {
            return Err("debounce_delay_ms should not exceed 10 seconds".to_string());
        }

        Ok(())
    }
}
