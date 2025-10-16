use crate::error::RariError;
use crate::runtime::JsExecutionRuntime;
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub enable_cache: bool,
    pub cache_static_only: bool,
    pub debug_timing: bool,
    pub max_cache_size: usize,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            cache_static_only: false,
            debug_timing: false,
            max_cache_size: 1000,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    pub evictions: u64,
}

impl CacheStatistics {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { (self.hits as f64 / total as f64) * 100.0 }
    }
}

struct HtmlCache {
    cache: FxHashMap<u64, (String, u64)>,
    max_size: usize,
    stats: CacheStatistics,
}

impl HtmlCache {
    fn new(max_size: usize) -> Self {
        Self { cache: FxHashMap::default(), max_size, stats: CacheStatistics::default() }
    }

    fn get(&mut self, component_tree: &JsonValue) -> Option<String> {
        let hash = self.hash_component_tree(component_tree);

        if let Some((html, access_count)) = self.cache.get_mut(&hash) {
            *access_count += 1;
            self.stats.hits += 1;
            Some(html.clone())
        } else {
            self.stats.misses += 1;
            None
        }
    }

    fn insert(&mut self, component_tree: &JsonValue, html: String) {
        if self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        let hash = self.hash_component_tree(component_tree);
        self.cache.insert(hash, (html, 1));
        self.stats.size = self.cache.len();
    }

    fn evict_lru(&mut self) {
        let mut entries: Vec<(u64, u64)> =
            self.cache.iter().map(|(hash, (_, access_count))| (*hash, *access_count)).collect();

        entries.sort_by_key(|(_, count)| *count);

        let to_remove = entries.len() / 2;
        for (hash, _) in entries.iter().take(to_remove) {
            self.cache.remove(hash);
            self.stats.evictions += 1;
        }

        self.stats.size = self.cache.len();
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.stats.size = 0;
    }

    fn get_stats(&self) -> CacheStatistics {
        self.stats.clone()
    }

    fn hash_component_tree(&self, component_tree: &JsonValue) -> u64 {
        let mut hasher = DefaultHasher::new();
        component_tree.to_string().hash(&mut hasher);
        hasher.finish()
    }
}

pub struct Renderer {
    runtime: Arc<JsExecutionRuntime>,
    cache: Arc<Mutex<HtmlCache>>,
}

impl Renderer {
    pub fn new(runtime: Arc<JsExecutionRuntime>) -> Self {
        Self::with_cache_size(runtime, 1000)
    }

    pub fn with_cache_size(runtime: Arc<JsExecutionRuntime>, cache_size: usize) -> Self {
        Self { runtime, cache: Arc::new(Mutex::new(HtmlCache::new(cache_size))) }
    }

    pub async fn render_to_html(
        &self,
        component_tree: JsonValue,
        config: &RenderConfig,
    ) -> Result<String, RariError> {
        let start_time = Instant::now();

        if config.enable_cache {
            let cached = {
                let mut cache = self.cache.lock();
                cache.get(&component_tree)
            };

            if let Some(html) = cached {
                if config.debug_timing {
                    tracing::debug!(
                        "HTML render: cache hit ({:.2}ms)",
                        start_time.elapsed().as_secs_f64() * 1000.0
                    );
                }
                return Ok(html);
            }
        }

        let render_start = Instant::now();
        let result =
            self.runtime.execute_function("renderToHtml", vec![component_tree.clone()]).await?;

        let html = result
            .as_str()
            .ok_or_else(|| RariError::internal("renderToHtml did not return string"))?
            .to_string();

        let render_time = render_start.elapsed().as_secs_f64() * 1000.0;

        if config.enable_cache {
            let mut cache = self.cache.lock();
            cache.insert(&component_tree, html.clone());
        }

        let total_time = start_time.elapsed().as_secs_f64() * 1000.0;

        if config.debug_timing {
            tracing::debug!("HTML render: {:.2}ms (render: {:.2}ms)", total_time, render_time);
        }

        Ok(html)
    }

    pub async fn render_with_fallback(
        &self,
        component_tree: JsonValue,
        config: &RenderConfig,
    ) -> Result<String, RariError> {
        match self.render_to_html(component_tree.clone(), config).await {
            Ok(html) => Ok(html),
            Err(e) => {
                tracing::warn!("HTML rendering failed, would fall back to RSC path: {}", e);
                Err(e)
            }
        }
    }

    pub fn get_cache_stats(&self) -> CacheStatistics {
        let cache = self.cache.lock();
        cache.get_stats()
    }

    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_statistics() {
        let mut cache = HtmlCache::new(10);

        assert_eq!(cache.get_stats().hits, 0);
        assert_eq!(cache.get_stats().misses, 0);
        assert_eq!(cache.get_stats().hit_rate(), 0.0);

        let tree = serde_json::json!({"type": "div", "props": {}});
        cache.insert(&tree, "<div></div>".to_string());

        assert!(cache.get(&tree).is_some());
        assert_eq!(cache.get_stats().hits, 1);
        assert_eq!(cache.get_stats().misses, 0);

        let other_tree = serde_json::json!({"type": "span", "props": {}});
        assert!(cache.get(&other_tree).is_none());
        assert_eq!(cache.get_stats().hits, 1);
        assert_eq!(cache.get_stats().misses, 1);
        assert_eq!(cache.get_stats().hit_rate(), 50.0);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = HtmlCache::new(5);

        for i in 0..5 {
            let tree = serde_json::json!({"id": i});
            cache.insert(&tree, format!("<div>{}</div>", i));
        }

        assert_eq!(cache.get_stats().size, 5);

        let tree = serde_json::json!({"id": 999});
        cache.insert(&tree, "<div>999</div>".to_string());

        assert!(cache.get_stats().size < 5);
        assert!(cache.get_stats().evictions > 0);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = HtmlCache::new(10);

        for i in 0..3 {
            let tree = serde_json::json!({"id": i});
            cache.insert(&tree, format!("<div>{}</div>", i));
        }

        assert_eq!(cache.get_stats().size, 3);

        cache.clear();

        assert_eq!(cache.get_stats().size, 0);
    }

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();

        assert!(config.enable_cache);
        assert!(!config.cache_static_only);
        assert!(!config.debug_timing);
        assert_eq!(config.max_cache_size, 1000);
    }
}
