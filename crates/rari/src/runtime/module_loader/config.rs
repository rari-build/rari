use super::cache::DEFAULT_TTL_SECS;
use crate::server::config::{CACHE_LAYER_MODULE, CacheLayerConfig, Config};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RuntimeConfig {
    pub cache_size_limit: usize,
    pub module_cache_handler: String,
    pub default_ttl_secs: u64,
    pub max_bytes: usize,
    pub url: Option<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            cache_size_limit: 5000,
            module_cache_handler: "memory".to_string(),
            default_ttl_secs: DEFAULT_TTL_SECS,
            max_bytes: 0,
            url: None,
        }
    }
}

impl RuntimeConfig {
    pub fn from_global() -> Self {
        Config::get()
            .map(|config| Self::from_layer(&config.cache.layer(CACHE_LAYER_MODULE)))
            .unwrap_or_default()
    }

    pub fn from_layer(layer: &CacheLayerConfig) -> Self {
        Self {
            cache_size_limit: layer.max_entries.max(1),
            module_cache_handler: if layer.handler.is_empty() {
                "memory".to_string()
            } else {
                layer.handler.clone()
            },
            default_ttl_secs: if layer.default_ttl_secs == 0 {
                DEFAULT_TTL_SECS
            } else {
                layer.default_ttl_secs
            },
            max_bytes: layer.max_bytes,
            url: layer.url.clone(),
        }
    }

    pub fn as_cache_layer(&self) -> CacheLayerConfig {
        CacheLayerConfig {
            handler: self.module_cache_handler.clone(),
            url: self.url.clone(),
            max_entries: self.cache_size_limit.max(1),
            default_ttl_secs: self.default_ttl_secs,
            max_bytes: self.max_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_from_layer() {
        let layer = CacheLayerConfig {
            handler: "memory".to_string(),
            url: None,
            max_entries: 42,
            default_ttl_secs: 120,
            max_bytes: 4096,
        };
        let config = RuntimeConfig::from_layer(&layer);
        assert_eq!(config.cache_size_limit, 42);
        assert_eq!(config.default_ttl_secs, 120);
        assert_eq!(config.max_bytes, 4096);
        let round_trip = config.as_cache_layer();
        assert_eq!(round_trip.max_entries, 42);
        assert_eq!(round_trip.max_bytes, 4096);
    }

    #[test]
    fn test_runtime_config_zero_ttl_falls_back() {
        let layer = CacheLayerConfig {
            handler: "memory".to_string(),
            url: None,
            max_entries: 0,
            default_ttl_secs: 0,
            max_bytes: 0,
        };
        let config = RuntimeConfig::from_layer(&layer);
        assert_eq!(config.cache_size_limit, 1);
        assert_eq!(config.default_ttl_secs, DEFAULT_TTL_SECS);
    }
}
