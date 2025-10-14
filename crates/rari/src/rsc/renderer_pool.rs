use crate::error::RariError;
use crate::rsc::renderer::{ResourceLimits, RscRenderer};
use crate::runtime::JsExecutionRuntime;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, warn};

pub struct RendererGuard<'a> {
    renderer: tokio::sync::MutexGuard<'a, RscRenderer>,
}

impl<'a> Deref for RendererGuard<'a> {
    type Target = RscRenderer;

    fn deref(&self) -> &Self::Target {
        &self.renderer
    }
}

impl<'a> DerefMut for RendererGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.renderer
    }
}

pub struct RendererPool {
    renderers: Vec<Arc<tokio::sync::Mutex<RscRenderer>>>,
    pool_size: usize,
    next_renderer: AtomicUsize,
}

impl RendererPool {
    pub async fn new(pool_size: usize, resource_limits: ResourceLimits) -> Result<Self, RariError> {
        debug!("Creating renderer pool with {} renderers", pool_size);

        let mut renderers = Vec::with_capacity(pool_size);

        for i in 0..pool_size {
            debug!("Initializing renderer {} of {}", i + 1, pool_size);

            let env_vars: rustc_hash::FxHashMap<String, String> = std::env::vars().collect();
            let js_runtime = Arc::new(JsExecutionRuntime::new(Some(env_vars)));
            let mut renderer =
                RscRenderer::with_resource_limits(js_runtime, resource_limits.clone());

            renderer.initialize().await?;

            renderers.push(Arc::new(tokio::sync::Mutex::new(renderer)));
        }

        debug!("Renderer pool created successfully");

        Ok(Self { renderers, pool_size, next_renderer: AtomicUsize::new(0) })
    }

    pub async fn acquire(&self) -> Result<RendererGuard<'_>, RariError> {
        let idx = self.next_renderer.fetch_add(1, Ordering::Relaxed) % self.pool_size;
        let renderer = self.renderers[idx].lock().await;
        Ok(RendererGuard { renderer })
    }

    pub fn get(&self, index: usize) -> Option<Arc<tokio::sync::Mutex<RscRenderer>>> {
        self.renderers.get(index).cloned()
    }

    pub fn size(&self) -> usize {
        self.pool_size
    }

    pub async fn register_component_on_all(
        &self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        debug!(
            "Registering component {} on all {} renderers in parallel",
            component_id, self.pool_size
        );

        let mut tasks = Vec::new();

        for (i, renderer) in self.renderers.iter().enumerate() {
            let renderer = Arc::clone(renderer);
            let component_id = component_id.to_string();
            let component_code = component_code.to_string();

            let task = tokio::spawn(async move {
                let mut r = renderer.lock().await;
                r.register_component(&component_id, &component_code).await.map_err(|e| (i, e))
            });

            tasks.push(task);
        }

        let results = futures::future::join_all(tasks).await;

        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(Ok(())) => {}
                Ok(Err((i, e))) => {
                    warn!("Failed to register component {} on renderer {}: {}", component_id, i, e);
                    errors.push(e);
                }
                Err(e) => {
                    warn!("Task panicked while registering component {}: {}", component_id, e);
                    errors.push(RariError::internal(format!("Task panic: {}", e)));
                }
            }
        }

        if !errors.is_empty() {
            return Err(RariError::internal(format!(
                "Failed to register component on {} renderers",
                errors.len()
            )));
        }

        Ok(())
    }

    pub fn stats(&self) -> PoolStats {
        let mut busy = 0;
        for renderer in &self.renderers {
            if renderer.try_lock().is_err() {
                busy += 1;
            }
        }

        PoolStats {
            pool_size: self.pool_size,
            available_permits: self.pool_size - busy,
            busy_renderers: busy,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub pool_size: usize,
    pub available_permits: usize,
    pub busy_renderers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_acquisition() {
        let pool_size = 2;
        let limits = ResourceLimits::default();

        let pool = Arc::new(RendererPool::new(pool_size, limits).await.unwrap());

        let mut handles = vec![];
        for i in 0..10 {
            let pool_clone = Arc::clone(&pool);
            let handle = tokio::spawn(async move {
                let start = std::time::Instant::now();
                let _guard = pool_clone.acquire().await.unwrap();
                let acquire_time = start.elapsed();

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                (i, acquire_time)
            });
            handles.push(handle);
        }

        let mut total_acquire_time = std::time::Duration::ZERO;
        for handle in handles {
            let (task_id, acquire_time) = handle.await.unwrap();
            println!("Task {} acquired in {:?}", task_id, acquire_time);
            total_acquire_time += acquire_time;
        }

        let avg_acquire_time = total_acquire_time / 10;
        println!("Average acquire time: {:?}", avg_acquire_time);

        assert!(
            avg_acquire_time.as_millis() < 1000,
            "Average acquire time too high: {:?}",
            avg_acquire_time
        );
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let pool_size = 4;
        let limits = ResourceLimits::default();

        let pool = RendererPool::new(pool_size, limits).await.unwrap();

        let stats = pool.stats();
        assert_eq!(stats.pool_size, 4);
        assert_eq!(stats.available_permits, 4);
        assert_eq!(stats.busy_renderers, 0);

        let _guard = pool.acquire().await.unwrap();
        let stats = pool.stats();
        assert_eq!(stats.available_permits, 3);
        assert_eq!(stats.busy_renderers, 1);

        drop(_guard);
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let stats = pool.stats();
        assert_eq!(stats.available_permits, 4);
        assert_eq!(stats.busy_renderers, 0);
    }
}
