#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub cache_size_limit: usize,
    pub module_cache_handler: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            cache_size_limit: 5000,
            module_cache_handler: "memory".to_string(),
        }
    }
}
