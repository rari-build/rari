use crate::error::{ModuleReloadError, RariError};
use crate::rsc::components::ComponentRegistry;
use crate::runtime::JsExecutionRuntime;
use crate::runtime::module_reload::{DebounceManager, ModuleReloadRequest, ReloadConfig};
use crate::runtime::utils::DistPathResolver;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::error;

pub struct ModuleReloadManager {
    reload_queue: Arc<Mutex<VecDeque<ModuleReloadRequest>>>,
    config: ReloadConfig,
    runtime: Option<Arc<JsExecutionRuntime>>,
    component_registry: Option<Arc<parking_lot::Mutex<ComponentRegistry>>>,
    debounce_manager: DebounceManager,
    dist_path_resolver: Option<Arc<DistPathResolver>>,
}

impl Clone for ModuleReloadManager {
    fn clone(&self) -> Self {
        Self {
            reload_queue: Arc::clone(&self.reload_queue),
            config: self.config.clone(),
            runtime: self.runtime.clone(),
            component_registry: self.component_registry.clone(),
            debounce_manager: self.debounce_manager.clone(),
            dist_path_resolver: self.dist_path_resolver.clone(),
        }
    }
}

impl ModuleReloadManager {
    pub fn new(config: ReloadConfig) -> Self {
        Self {
            reload_queue: Arc::new(Mutex::new(VecDeque::new())),
            config,
            runtime: None,
            component_registry: None,
            debounce_manager: DebounceManager::new(),
            dist_path_resolver: None,
        }
    }

    pub fn with_runtime(config: ReloadConfig, runtime: Arc<JsExecutionRuntime>) -> Self {
        Self {
            reload_queue: Arc::new(Mutex::new(VecDeque::new())),
            config,
            runtime: Some(runtime),
            component_registry: None,
            debounce_manager: DebounceManager::new(),
            dist_path_resolver: None,
        }
    }

    pub fn set_runtime(&mut self, runtime: Arc<JsExecutionRuntime>) {
        self.runtime = Some(runtime);
    }

    pub fn set_component_registry(&mut self, registry: Arc<parking_lot::Mutex<ComponentRegistry>>) {
        self.component_registry = Some(registry);
    }

    pub fn set_dist_path_resolver(&mut self, resolver: Arc<DistPathResolver>) {
        self.dist_path_resolver = Some(resolver);
    }

    pub fn dist_path_resolver(&self) -> Option<&Arc<DistPathResolver>> {
        self.dist_path_resolver.as_ref()
    }

    pub fn config(&self) -> &ReloadConfig {
        &self.config
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn enqueue_reload(&self, request: ModuleReloadRequest) {
        let mut queue = self.reload_queue.lock().await;
        queue.push_back(request);
    }

    pub async fn dequeue_reload(&self) -> Option<ModuleReloadRequest> {
        let mut queue = self.reload_queue.lock().await;
        queue.pop_front()
    }

    pub async fn queue_size(&self) -> usize {
        let queue = self.reload_queue.lock().await;
        queue.len()
    }

    pub async fn clear_queue(&self) {
        let mut queue = self.reload_queue.lock().await;
        queue.clear();
    }

    fn clone_for_task(&self) -> Self {
        self.clone()
    }
}

impl ModuleReloadManager {
    pub async fn reload_module_debounced(
        &self,
        component_id: &str,
        file_path: &Path,
    ) -> Result<(), RariError> {
        if !self.config.enabled {
            return Ok(());
        }

        let component_id = component_id.to_string();
        let file_path = file_path.to_path_buf();
        let debounce_delay = Duration::from_millis(self.config.debounce_delay_ms);

        let manager = self.clone_for_task();
        let component_id_clone = component_id.clone();
        let file_path_clone = file_path.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(debounce_delay).await;

            manager.debounce_manager.remove_pending(&component_id_clone).await;

            if let Err(e) = manager.reload_module(&component_id_clone, &file_path_clone).await {
                error!("Failed to reload module {}: {}", component_id_clone, e);
            }
        });

        let request = ModuleReloadRequest::new(component_id.clone(), file_path.clone());
        self.debounce_manager.add_pending(component_id.clone(), request, handle).await;

        Ok(())
    }

    pub async fn reload_module(
        &self,
        component_id: &str,
        file_path: &Path,
    ) -> Result<(), RariError> {
        if !self.config.enabled {
            return Ok(());
        }

        let result = self.reload_with_retry(component_id, file_path).await;

        match &result {
            Ok(_) => {}
            Err(e) => {
                error!(
                    component_id = component_id,
                    file_path = ?file_path,
                    error = %e,
                    "Module reload failed"
                );
            }
        }

        result
    }

    async fn reload_with_retry(
        &self,
        component_id: &str,
        file_path: &Path,
    ) -> Result<(), RariError> {
        let mut attempts = 0;
        let max_attempts = self.config.max_retry_attempts;

        loop {
            attempts += 1;

            match self.reload_module_internal(component_id, file_path).await {
                Ok(_) => return Ok(()),
                Err(e) if attempts >= max_attempts => {
                    return Err(RariError::module_reload(ModuleReloadError::MaxRetriesExceeded {
                        message: format!("Failed to reload module '{}'", component_id),
                        file_path: file_path.display().to_string(),
                        attempts,
                        last_error: Some(e.to_string()),
                    }));
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(100 * attempts as u64)).await;
                }
            }
        }
    }

    async fn reload_module_internal(
        &self,
        component_id: &str,
        _file_path: &Path,
    ) -> Result<(), RariError> {
        self.runtime.as_ref().ok_or_else(|| {
            RariError::module_reload(ModuleReloadError::RuntimeNotAvailable {
                message: "Runtime not available".to_string(),
            })
        })?;

        if let Some(registry) = &self.component_registry {
            let mut reg = registry.lock();
            reg.remove_component(component_id);
        }

        Ok(())
    }

    pub async fn reload_modules_batch(
        &self,
        requests: Vec<ModuleReloadRequest>,
    ) -> Result<(), RariError> {
        if !self.config.enabled {
            return Ok(());
        }

        if !self.config.parallel_reloads {
            for request in requests {
                self.reload_module(&request.component_id, &request.file_path).await?;
            }
        } else {
            let mut handles = Vec::new();

            for request in requests {
                let manager = self.clone_for_task();
                let handle = tokio::spawn(async move {
                    manager.reload_module(&request.component_id, &request.file_path).await
                });
                handles.push(handle);
            }

            for handle in handles {
                if let Err(e) = handle.await {
                    error!(error = %e, "Batch reload task failed");
                }
            }
        }

        Ok(())
    }
}
