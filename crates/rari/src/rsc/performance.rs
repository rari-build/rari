#![allow(clippy::disallowed_methods)]

use deno_core::anyhow::Result;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub struct SuspensePerformanceManager {
    preload_cache: Arc<RwLock<FxHashMap<String, PreloadedResource>>>,
    render_metrics: Arc<RwLock<FxHashMap<String, RenderMetrics>>>,
    global_metrics: Arc<RwLock<GlobalPerformanceMetrics>>,
}

#[derive(Debug, Clone)]
pub struct PreloadedResource {
    pub cache_key: String,
    pub promise: Option<serde_json::Value>,
    pub preloaded_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub hit_count: u64,
    pub data: serde_json::Value,
    pub content_type: String,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderMetrics {
    pub boundary_id: String,
    pub promise_count: u32,
    pub resolution_time_ms: u64,
    pub cache_hit_rate: f64,
    pub re_render_count: u32,
    pub first_render_time_ms: u64,
    pub last_render_time_ms: u64,
    pub total_render_time_ms: u64,
    pub error_count: u32,
    pub preload_hits: u32,
    pub preload_misses: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPerformanceMetrics {
    pub total_boundaries_created: u64,
    pub total_promises_handled: u64,
    pub total_cache_hits: u64,
    pub total_cache_misses: u64,
    pub average_resolution_time_ms: f64,
    pub memory_usage_bytes: usize,
    pub cleanup_runs: u64,
    pub preload_success_rate: f64,
    pub error_rate: f64,
    pub peak_concurrent_boundaries: usize,
    pub total_render_time_ms: u64,
    pub started_at: SystemTime,
}

impl Default for GlobalPerformanceMetrics {
    fn default() -> Self {
        Self {
            total_boundaries_created: 0,
            total_promises_handled: 0,
            total_cache_hits: 0,
            total_cache_misses: 0,
            average_resolution_time_ms: 0.0,
            memory_usage_bytes: 0,
            cleanup_runs: 0,
            preload_success_rate: 0.0,
            error_rate: 0.0,
            peak_concurrent_boundaries: 0,
            total_render_time_ms: 0,
            started_at: SystemTime::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub max_preload_size: usize,
    pub preload_ttl_seconds: u64,
    pub metrics_retention_seconds: u64,
    pub enable_memory_monitoring: bool,
    pub cleanup_interval_seconds: u64,
    pub cache_size_limit_mb: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_preload_size: 100,
            preload_ttl_seconds: 300,        // 5 minutes
            metrics_retention_seconds: 3600, // 1 hour
            enable_memory_monitoring: true,
            cleanup_interval_seconds: 60, // 1 minute
            cache_size_limit_mb: 50,
        }
    }
}

impl SuspensePerformanceManager {
    pub fn new(config: PerformanceConfig) -> Self {
        let _ = config; // Use config for future enhancements
        Self {
            preload_cache: Arc::new(RwLock::new(FxHashMap::default())),
            render_metrics: Arc::new(RwLock::new(FxHashMap::default())),
            global_metrics: Arc::new(RwLock::new(GlobalPerformanceMetrics::default())),
        }
    }

    pub async fn preload_resource<F, Fut>(
        &self,
        cache_key: String,
        resource_factory: F,
        ttl_seconds: Option<u64>,
    ) -> Result<PreloadedResource>
    where
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<serde_json::Value>> + Send,
    {
        let start_time = SystemTime::now();

        {
            let cache = self.preload_cache.read().await;
            if let Some(existing) = cache.get(&cache_key)
                && let Some(expires_at) = existing.expires_at
                && SystemTime::now() < expires_at
            {
                let mut updated = existing.clone();
                updated.hit_count += 1;

                drop(cache);
                let mut cache_write = self.preload_cache.write().await;
                cache_write.insert(cache_key.clone(), updated.clone());

                return Ok(updated);
            }
        }

        let data = resource_factory().await?;

        let expires_at = ttl_seconds.map(|ttl| SystemTime::now() + Duration::from_secs(ttl));

        let data_size = serde_json::to_string(&data).unwrap_or_default().len();

        let preloaded = PreloadedResource {
            cache_key: cache_key.clone(),
            promise: None,
            preloaded_at: start_time,
            expires_at,
            hit_count: 0,
            data,
            content_type: "application/json".to_string(),
            size_bytes: data_size,
        };

        {
            let mut cache = self.preload_cache.write().await;
            cache.insert(cache_key, preloaded.clone());
        }

        {
            let mut metrics = self.global_metrics.write().await;
            metrics.memory_usage_bytes += data_size;
        }

        Ok(preloaded)
    }

    pub async fn get_preloaded_resource(&self, cache_key: &str) -> Option<PreloadedResource> {
        let cache = self.preload_cache.read().await;
        let resource = cache.get(cache_key)?;

        if let Some(expires_at) = resource.expires_at
            && SystemTime::now() > expires_at
        {
            return None;
        }

        let mut updated = resource.clone();
        updated.hit_count += 1;

        drop(cache);
        let mut cache_write = self.preload_cache.write().await;
        cache_write.insert(cache_key.to_string(), updated.clone());

        Some(updated)
    }

    pub async fn record_boundary_metrics(
        &self,
        boundary_id: String,
        metrics: RenderMetrics,
    ) -> Result<()> {
        let mut render_metrics = self.render_metrics.write().await;
        render_metrics.insert(boundary_id, metrics.clone());

        let mut global = self.global_metrics.write().await;
        global.total_boundaries_created += 1;
        global.total_render_time_ms += metrics.resolution_time_ms;

        if metrics.cache_hit_rate > 0.0 {
            global.total_cache_hits +=
                (metrics.promise_count as f64 * metrics.cache_hit_rate) as u64;
        }

        global.total_cache_misses += metrics.promise_count as u64
            - (metrics.promise_count as f64 * metrics.cache_hit_rate) as u64;

        if global.total_boundaries_created > 0 {
            global.average_resolution_time_ms =
                global.total_render_time_ms as f64 / global.total_boundaries_created as f64;
        }

        Ok(())
    }

    pub async fn get_boundary_metrics(&self, boundary_id: &str) -> Option<RenderMetrics> {
        let metrics = self.render_metrics.read().await;
        metrics.get(boundary_id).cloned()
    }

    pub async fn get_global_metrics(&self) -> GlobalPerformanceMetrics {
        let metrics = self.global_metrics.read().await;
        metrics.clone()
    }

    pub async fn cleanup_expired(&self, config: &PerformanceConfig) -> usize {
        let mut cleaned_count = 0;
        let now = SystemTime::now();

        {
            let mut cache = self.preload_cache.write().await;
            let keys_to_remove: Vec<String> = cache
                .iter()
                .filter_map(|(key, resource)| {
                    if let Some(expires_at) = resource.expires_at {
                        if now > expires_at { Some(key.clone()) } else { None }
                    } else {
                        let age = now.duration_since(resource.preloaded_at).unwrap_or_default();
                        if age.as_secs() > config.preload_ttl_seconds * 2 {
                            Some(key.clone())
                        } else {
                            None
                        }
                    }
                })
                .collect();

            for key in keys_to_remove {
                if let Some(resource) = cache.remove(&key) {
                    cleaned_count += 1;

                    let mut global = self.global_metrics.try_write();
                    if let Ok(ref mut metrics) = global {
                        metrics.memory_usage_bytes =
                            metrics.memory_usage_bytes.saturating_sub(resource.size_bytes);
                    }
                }
            }
        }

        {
            let mut metrics = self.render_metrics.write().await;
            let cutoff = now - Duration::from_secs(config.metrics_retention_seconds);

            let boundaries_to_remove: Vec<String> = metrics
                .iter()
                .filter_map(|(id, metric)| {
                    let metric_age = UNIX_EPOCH + Duration::from_millis(metric.last_render_time_ms);
                    if metric_age < cutoff { Some(id.clone()) } else { None }
                })
                .collect();

            for boundary_id in boundaries_to_remove {
                metrics.remove(&boundary_id);
                cleaned_count += 1;
            }
        }

        {
            let mut global = self.global_metrics.write().await;
            global.cleanup_runs += 1;
        }

        cleaned_count
    }

    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.preload_cache.read().await;
        let global = self.global_metrics.read().await;

        let total_size_bytes: usize = cache.values().map(|r| r.size_bytes).sum();
        let total_hits: u64 = cache.values().map(|r| r.hit_count).sum();
        let expired_count = cache
            .values()
            .filter(|r| r.expires_at.map(|exp| SystemTime::now() > exp).unwrap_or(false))
            .count();

        CacheStats {
            total_entries: cache.len(),
            total_size_bytes,
            total_hits,
            expired_entries: expired_count,
            hit_rate: if global.total_cache_hits + global.total_cache_misses > 0 {
                global.total_cache_hits as f64
                    / (global.total_cache_hits + global.total_cache_misses) as f64
            } else {
                0.0
            },
            memory_pressure: total_size_bytes > (50 * 1024 * 1024), // 50MB threshold
        }
    }

    pub async fn optimize_cache(&self, target_size_mb: usize) -> usize {
        let target_bytes = target_size_mb * 1024 * 1024;
        let mut removed_count = 0;

        let mut cache = self.preload_cache.write().await;
        let current_size: usize = cache.values().map(|r| r.size_bytes).sum();

        if current_size <= target_bytes {
            return 0;
        }

        let mut entries: Vec<(String, PreloadedResource)> =
            cache.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        entries.sort_by(|a, b| {
            let hit_cmp = a.1.hit_count.cmp(&b.1.hit_count);
            if hit_cmp == std::cmp::Ordering::Equal {
                a.1.preloaded_at.cmp(&b.1.preloaded_at)
            } else {
                hit_cmp
            }
        });

        let mut current_size_mut = current_size;
        for (key, resource) in entries {
            if current_size_mut <= target_bytes {
                break;
            }

            cache.remove(&key);
            current_size_mut -= resource.size_bytes;
            removed_count += 1;
        }

        if let Ok(mut global) = self.global_metrics.try_write() {
            global.memory_usage_bytes = current_size_mut;
        }

        removed_count
    }

    pub async fn start_render_timing(&self, boundary_id: String) -> RenderTimer {
        RenderTimer {
            boundary_id,
            start_time: SystemTime::now(),
            promise_count: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub async fn complete_render_timing(
        &self,
        timer: RenderTimer,
        error_occurred: bool,
    ) -> Result<()> {
        let duration = SystemTime::now().duration_since(timer.start_time).unwrap_or_default();

        let cache_hit_rate = if timer.promise_count > 0 {
            timer.cache_hits as f64 / timer.promise_count as f64
        } else {
            0.0
        };

        let render_metrics = RenderMetrics {
            boundary_id: timer.boundary_id.clone(),
            promise_count: timer.promise_count,
            resolution_time_ms: duration.as_millis() as u64,
            cache_hit_rate,
            re_render_count: 1,
            first_render_time_ms: duration.as_millis() as u64,
            last_render_time_ms: duration.as_millis() as u64,
            total_render_time_ms: duration.as_millis() as u64,
            error_count: if error_occurred { 1 } else { 0 },
            preload_hits: timer.cache_hits,
            preload_misses: timer.cache_misses,
        };

        self.record_boundary_metrics(timer.boundary_id, render_metrics).await?;

        Ok(())
    }

    pub async fn preload_critical_resources(
        &self,
        component_paths: Vec<String>,
    ) -> Result<Vec<String>> {
        let mut preloaded_keys = Vec::new();

        for path in component_paths {
            let cache_key = format!("critical:{path}");

            {
                let cache = self.preload_cache.read().await;
                if cache.contains_key(&cache_key) {
                    continue;
                }
            }

            let preloaded = PreloadedResource {
                cache_key: cache_key.clone(),
                promise: None,
                preloaded_at: SystemTime::now(),
                expires_at: Some(SystemTime::now() + Duration::from_secs(300)),
                hit_count: 0,
                data: serde_json::json!({
                    "component_path": path,
                    "preloaded": true,
                    "critical": true
                }),
                content_type: "application/json".to_string(),
                size_bytes: path.len() + 100,
            };

            {
                let mut cache = self.preload_cache.write().await;
                cache.insert(cache_key.clone(), preloaded);
            }

            preloaded_keys.push(cache_key);
        }

        Ok(preloaded_keys)
    }

    pub async fn get_performance_recommendations(&self) -> Vec<PerformanceRecommendation> {
        let mut recommendations = Vec::new();
        let global = self.global_metrics.read().await;
        let cache_stats = self.get_cache_stats().await;

        if cache_stats.hit_rate < 0.7 {
            recommendations.push(PerformanceRecommendation {
                category: "Cache".to_string(),
                severity: if cache_stats.hit_rate < 0.3 { "High" } else { "Medium" }.to_string(),
                message: format!(
                    "Low cache hit rate: {:.1}%. Consider preloading more resources.",
                    cache_stats.hit_rate * 100.0
                ),
                action: "Increase preloading or review caching strategy".to_string(),
            });
        }

        if cache_stats.memory_pressure {
            recommendations.push(PerformanceRecommendation {
                category: "Memory".to_string(),
                severity: "High".to_string(),
                message: "High memory usage detected in preload cache".to_string(),
                action: "Reduce cache size or increase cleanup frequency".to_string(),
            });
        }

        if global.error_rate > 0.05 {
            recommendations.push(PerformanceRecommendation {
                category: "Reliability".to_string(),
                severity: "High".to_string(),
                message: format!("High error rate: {:.1}%", global.error_rate * 100.0),
                action: "Review error handling and add more robust fallbacks".to_string(),
            });
        }

        if global.average_resolution_time_ms > 500.0 {
            recommendations.push(PerformanceRecommendation {
                category: "Performance".to_string(),
                severity: "Medium".to_string(),
                message: format!(
                    "Slow average resolution time: {:.0}ms",
                    global.average_resolution_time_ms
                ),
                action: "Optimize async operations or add more preloading".to_string(),
            });
        }

        recommendations
    }
}

impl Default for SuspensePerformanceManager {
    fn default() -> Self {
        Self::new(PerformanceConfig::default())
    }
}

#[derive(Debug)]
pub struct RenderTimer {
    pub boundary_id: String,
    pub start_time: SystemTime,
    pub promise_count: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
}

impl RenderTimer {
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
        self.promise_count += 1;
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
        self.promise_count += 1;
    }

    pub fn elapsed(&self) -> Duration {
        SystemTime::now().duration_since(self.start_time).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub total_hits: u64,
    pub expired_entries: usize,
    pub hit_rate: f64,
    pub memory_pressure: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceRecommendation {
    pub category: String,
    pub severity: String,
    pub message: String,
    pub action: String,
}

impl RenderMetrics {
    pub fn new(boundary_id: String) -> Self {
        Self {
            boundary_id,
            promise_count: 0,
            resolution_time_ms: 0,
            cache_hit_rate: 0.0,
            re_render_count: 0,
            first_render_time_ms: 0,
            last_render_time_ms: 0,
            total_render_time_ms: 0,
            error_count: 0,
            preload_hits: 0,
            preload_misses: 0,
        }
    }

    pub fn update_render(&mut self, duration_ms: u64, cache_hits: u32, cache_misses: u32) {
        self.re_render_count += 1;
        self.last_render_time_ms = duration_ms;
        self.resolution_time_ms = duration_ms;
        self.total_render_time_ms += duration_ms;

        if self.re_render_count == 1 {
            self.first_render_time_ms = duration_ms;
        }

        let total_promises = cache_hits + cache_misses;
        if total_promises > 0 {
            self.promise_count += total_promises;
            self.preload_hits += cache_hits;
            self.preload_misses += cache_misses;
            self.cache_hit_rate = self.preload_hits as f64 / self.promise_count as f64;
        }
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    pub fn get_average_render_time(&self) -> f64 {
        if self.re_render_count > 0 {
            self.total_render_time_ms as f64 / self.re_render_count as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_manager_creation() {
        let config = PerformanceConfig::default();
        let manager = SuspensePerformanceManager::new(config);

        let global_metrics = manager.get_global_metrics().await;
        assert_eq!(global_metrics.total_boundaries_created, 0);
        assert_eq!(global_metrics.total_promises_handled, 0);
    }

    #[tokio::test]
    async fn test_preload_resource() {
        let manager = SuspensePerformanceManager::default();

        let result = manager
            .preload_resource(
                "test-resource".to_string(),
                || async { Ok(serde_json::json!({"data": "test_value"})) },
                Some(60),
            )
            .await;

        assert!(result.is_ok());
        let preloaded = result.unwrap();
        assert_eq!(preloaded.cache_key, "test-resource");
        assert_eq!(preloaded.hit_count, 0);
    }

    #[tokio::test]
    async fn test_cache_hit_optimization() {
        let manager = SuspensePerformanceManager::default();

        manager
            .preload_resource(
                "test-cache".to_string(),
                || async { Ok(serde_json::json!({"cached": true})) },
                Some(60),
            )
            .await
            .unwrap();

        let cached = manager.get_preloaded_resource("test-cache").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().hit_count, 1);

        let cached_again = manager.get_preloaded_resource("test-cache").await;
        assert!(cached_again.is_some());
        assert_eq!(cached_again.unwrap().hit_count, 2);
    }

    #[tokio::test]
    async fn test_render_metrics() {
        let manager = SuspensePerformanceManager::default();

        let mut metrics = RenderMetrics::new("test-boundary".to_string());
        metrics.update_render(150, 3, 1); // 150ms, 3 hits, 1 miss

        manager.record_boundary_metrics("test-boundary".to_string(), metrics).await.unwrap();

        let recorded = manager.get_boundary_metrics("test-boundary").await;
        assert!(recorded.is_some());

        let recorded_metrics = recorded.unwrap();
        assert_eq!(recorded_metrics.boundary_id, "test-boundary");
        assert_eq!(recorded_metrics.resolution_time_ms, 150);
        assert_eq!(recorded_metrics.cache_hit_rate, 0.75); // 3/4
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let config = PerformanceConfig { preload_ttl_seconds: 1, ..Default::default() };
        let manager = SuspensePerformanceManager::new(config.clone());

        manager
            .preload_resource(
                "short-lived".to_string(),
                || async { Ok(serde_json::json!({"temporary": true})) },
                Some(1),
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(1100)).await;

        let cleaned = manager.cleanup_expired(&config).await;
        assert!(cleaned > 0);

        let cached = manager.get_preloaded_resource("short-lived").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_performance_recommendations() {
        let manager = SuspensePerformanceManager::default();

        {
            let mut global = manager.global_metrics.write().await;
            global.total_cache_hits = 10;
            global.total_cache_misses = 40;
            global.error_rate = 0.1;
            global.average_resolution_time_ms = 800.0;
        }

        let recommendations = manager.get_performance_recommendations().await;

        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.category == "Cache"));
        assert!(recommendations.iter().any(|r| r.category == "Reliability"));
        assert!(recommendations.iter().any(|r| r.category == "Performance"));
    }
}
