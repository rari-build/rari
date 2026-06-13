#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub max_concurrent_operations: usize,
    pub operation_timeout_ms: u64,
    pub cache_size_limit: usize,
    pub enable_hot_reload: bool,
    pub batch_size_limit: usize,
    pub batch_time_limit_ms: u64,
    pub file_cache_duration_secs: u64,
    pub cleanup_interval_secs: u64,
    pub max_memory_per_component_mb: usize,
    pub string_interner_max_size: usize,
    pub enable_string_interning: bool,
    pub enable_batch_operations: bool,
    pub enable_async_file_cache: bool,
    pub enable_metrics_collection: bool,
    pub metrics_collection_interval_ms: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 100,
            operation_timeout_ms: 8000,
            cache_size_limit: 5000,
            enable_hot_reload: true,
            batch_size_limit: 50,
            batch_time_limit_ms: 100,
            file_cache_duration_secs: 300,
            cleanup_interval_secs: 300,
            max_memory_per_component_mb: 50,
            string_interner_max_size: 10000,
            enable_string_interning: true,
            enable_batch_operations: true,
            enable_async_file_cache: true,
            enable_metrics_collection: true,
            metrics_collection_interval_ms: 1000,
        }
    }
}

impl RuntimeConfig {
    pub fn production() -> Self {
        Self {
            cache_size_limit: 5000,
            batch_size_limit: 100,
            batch_time_limit_ms: 50,
            file_cache_duration_secs: 600,
            cleanup_interval_secs: 180,
            max_memory_per_component_mb: 100,
            string_interner_max_size: 50000,
            operation_timeout_ms: 3000,
            metrics_collection_interval_ms: 5000,
            ..Default::default()
        }
    }

    pub fn development() -> Self {
        Self {
            cache_size_limit: 100,
            batch_size_limit: 10,
            batch_time_limit_ms: 200,
            file_cache_duration_secs: 60,
            cleanup_interval_secs: 60,
            max_memory_per_component_mb: 25,
            string_interner_max_size: 1000,
            operation_timeout_ms: 10000,
            metrics_collection_interval_ms: 500,
            enable_hot_reload: true,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeMetrics {
    pub cache_stats: CacheStats,
    pub batch_stats: BatchStats,
    pub interner_stats: InternerStats,
    pub performance_stats: PerformanceStats,
    pub resource_stats: ResourceStats,
    pub collected_at: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct BatchStats {
    pub total_batches_processed: usize,
    pub total_operations_batched: usize,
    pub average_batch_size: f64,
    pub batch_flush_failures: usize,
    pub time_saved_by_batching_ms: u64,
}

#[derive(Debug, Clone)]
pub struct InternerStats {
    pub total_strings_interned: usize,
    pub memory_saved_bytes: usize,
    pub hit_rate_percentage: f64,
    pub cache_size: usize,
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub average_module_load_time_ms: f64,
    pub peak_module_load_time_ms: u64,
    pub total_modules_loaded: usize,
    pub cache_hit_rate_percentage: f64,
    pub operations_per_second: f64,
}

#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub memory_usage_mb: usize,
    pub active_threads: usize,
    pub pending_operations: usize,
    pub file_cache_size: usize,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub size: usize,
    pub memory_bytes: usize,
}
