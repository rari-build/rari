use crate::error::{ModuleReloadError, RariError};
use crate::rsc::component::ComponentRegistry;
use crate::runtime::JsExecutionRuntime;
use crate::runtime::dist_path_resolver::DistPathResolver;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

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
}

#[derive(Debug, Clone)]
pub struct ModuleReloadRequest {
    pub component_id: String,
    pub file_path: PathBuf,
    pub timestamp: Instant,
}

impl ModuleReloadRequest {
    pub fn new(component_id: String, file_path: PathBuf) -> Self {
        Self { component_id, file_path, timestamp: Instant::now() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsReloadResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsModuleCacheInfo {
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    pub specifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsComponentVerification {
    pub success: bool,
    pub component_id: String,
    pub exists: bool,
    pub is_function: bool,
    pub is_valid_component: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    pub timestamp: f64,
}

type DebouncePendingMap = FxHashMap<String, (ModuleReloadRequest, tokio::task::JoinHandle<()>)>;

pub struct ModuleReloadManager {
    reload_queue: Arc<Mutex<VecDeque<ModuleReloadRequest>>>,
    reload_stats: Arc<RwLock<ReloadStats>>,
    config: ReloadConfig,
    runtime: Option<Arc<JsExecutionRuntime>>,
    component_registry: Option<Arc<parking_lot::Mutex<ComponentRegistry>>>,
    debounce_pending: Arc<Mutex<DebouncePendingMap>>,
    reload_history: Arc<Mutex<VecDeque<ReloadHistoryEntry>>>,
    dist_path_resolver: Option<Arc<DistPathResolver>>,
}

impl Clone for ModuleReloadManager {
    fn clone(&self) -> Self {
        Self {
            reload_queue: Arc::clone(&self.reload_queue),
            reload_stats: Arc::clone(&self.reload_stats),
            config: self.config.clone(),
            runtime: self.runtime.clone(),
            component_registry: self.component_registry.clone(),
            debounce_pending: Arc::clone(&self.debounce_pending),
            reload_history: Arc::clone(&self.reload_history),
            dist_path_resolver: self.dist_path_resolver.clone(),
        }
    }
}

impl ModuleReloadManager {
    pub fn new(config: ReloadConfig) -> Self {
        Self {
            reload_queue: Arc::new(Mutex::new(VecDeque::new())),
            reload_stats: Arc::new(RwLock::new(ReloadStats::default())),
            config,
            runtime: None,
            component_registry: None,
            debounce_pending: Arc::new(Mutex::new(FxHashMap::default())),
            reload_history: Arc::new(Mutex::new(VecDeque::new())),
            dist_path_resolver: None,
        }
    }

    pub fn with_runtime(config: ReloadConfig, runtime: Arc<JsExecutionRuntime>) -> Self {
        Self {
            reload_queue: Arc::new(Mutex::new(VecDeque::new())),
            reload_stats: Arc::new(RwLock::new(ReloadStats::default())),
            config,
            runtime: Some(runtime),
            component_registry: None,
            debounce_pending: Arc::new(Mutex::new(FxHashMap::default())),
            reload_history: Arc::new(Mutex::new(VecDeque::new())),
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

    pub async fn get_stats(&self) -> ReloadStats {
        self.reload_stats.read().await.clone()
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

    async fn record_reload_stats(&self, success: bool, duration_ms: u64) {
        let mut stats = self.reload_stats.write().await;
        stats.record_reload(success, duration_ms);
    }

    async fn add_to_history(&self, component_id: String, success: bool, duration_ms: u64) {
        let mut history = self.reload_history.lock().await;

        history.push_back(ReloadHistoryEntry {
            component_id,
            timestamp: Instant::now(),
            success,
            duration_ms,
        });

        while history.len() > self.config.max_history_size {
            history.pop_front();
        }
    }

    pub async fn get_reload_history(&self) -> Vec<ReloadHistoryEntry> {
        let history = self.reload_history.lock().await;
        history.iter().cloned().collect()
    }

    pub async fn clear_history(&self) {
        let mut history = self.reload_history.lock().await;
        history.clear();
        info!("Cleared reload history to free memory");
    }

    pub async fn get_memory_usage(&self) -> u64 {
        if !self.config.enable_memory_monitoring {
            return 0;
        }

        let mut total_bytes = 0u64;

        let queue = self.reload_queue.lock().await;
        total_bytes += queue.len() as u64 * 256;

        let history = self.reload_history.lock().await;
        total_bytes += history.len() as u64 * 128;

        let pending = self.debounce_pending.lock().await;
        total_bytes += pending.len() as u64 * 512;

        {
            let mut stats = self.reload_stats.write().await;
            stats.estimated_memory_bytes = total_bytes;
        }

        total_bytes
    }

    async fn log_memory_usage(&self) {
        if !self.config.enable_memory_monitoring {
            return;
        }

        let memory_bytes = self.get_memory_usage().await;
        let memory_kb = memory_bytes as f64 / 1024.0;
        let memory_mb = memory_kb / 1024.0;

        if memory_mb > 10.0 {
            warn!(
                memory_mb = format!("{:.2}", memory_mb),
                "Module reload manager memory usage is high"
            );
        } else {
            debug!(memory_kb = format!("{:.2}", memory_kb), "Module reload manager memory usage");
        }
    }

    async fn clear_old_module_versions(&self, module_specifier: &str) -> Result<(), RariError> {
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            RariError::module_reload(ModuleReloadError::RuntimeNotAvailable {
                message: "JavaScript runtime not initialized for module reload".to_string(),
            })
        })?;

        let script = format!(
            r#"
            (function() {{
                const moduleId = "{}";
                if (globalThis.__rari_module_cache) {{
                    delete globalThis.__rari_module_cache[moduleId];
                }}
                if (globalThis.gc) {{
                    globalThis.gc();
                }}
                return {{ cleared: true }};
            }})()
            "#,
            module_specifier.replace('"', r#"\""#)
        );

        runtime.execute_script("clear_old_modules".to_string(), script).await.map_err(|e| {
            RariError::module_reload(ModuleReloadError::Other {
                message: format!("Failed to clear old module versions: {}", e),
                file_path: Some(module_specifier.to_string()),
            })
        })?;

        debug!(module_specifier = module_specifier, "Cleared old module versions from memory");
        Ok(())
    }

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

        {
            let mut pending = self.debounce_pending.lock().await;
            if let Some((_, handle)) = pending.remove(&component_id) {
                handle.abort();
                debug!(
                    component_id = %component_id,
                    "Cancelled pending reload due to new file change"
                );
            }
        }

        let manager = self.clone_for_task();
        let component_id_clone = component_id.clone();
        let file_path_clone = file_path.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(debounce_delay).await;

            {
                let mut pending = manager.debounce_pending.lock().await;
                pending.remove(&component_id_clone);
            }

            let _ = manager.reload_module(&component_id_clone, &file_path_clone).await;
        });

        {
            let mut pending = self.debounce_pending.lock().await;
            let request = ModuleReloadRequest::new(component_id.clone(), file_path.clone());
            pending.insert(component_id.clone(), (request, handle));
        }

        debug!(
            component_id = %component_id,
            debounce_delay_ms = self.config.debounce_delay_ms,
            "Scheduled debounced module reload"
        );

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

        let start = Instant::now();

        info!(
            component_id = component_id,
            file_path = ?file_path,
            "Reloading module"
        );

        let result = self.reload_with_retry(component_id, file_path).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        self.record_reload_stats(result.is_ok(), duration_ms).await;

        self.add_to_history(component_id.to_string(), result.is_ok(), duration_ms).await;

        if result.is_ok() {
            let module_specifier = file_path.to_str().unwrap_or("unknown");
            if let Err(e) = self.clear_old_module_versions(module_specifier).await {
                debug!(
                    component_id = component_id,
                    error = %e,
                    "Failed to clear old module versions (non-fatal)"
                );
            }
        }

        self.log_memory_usage().await;

        match &result {
            Ok(_) => {
                debug!(
                    component_id = component_id,
                    duration_ms = duration_ms,
                    "Module reload completed successfully"
                );

                if duration_ms > 1000 {
                    warn!(
                        component_id = component_id,
                        duration_ms = duration_ms,
                        "Module reload was slow (> 1 second)"
                    );
                }
            }
            Err(e) => {
                error!(
                    component_id = component_id,
                    file_path = ?file_path,
                    error = %e,
                    error_code = e.code(),
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
        let max_attempts = self.config.max_retry_attempts;
        let timeout_duration = std::time::Duration::from_millis(self.config.reload_timeout_ms);

        let current_attempt = if let Some(registry) = &self.component_registry {
            let registry = registry.lock();
            registry.get_component(component_id).map(|c| c.reload_attempt_count).unwrap_or(0)
        } else {
            0
        };

        if current_attempt >= max_attempts {
            let file_path_str = file_path.to_str().unwrap_or("unknown");
            warn!(
                component_id = component_id,
                attempt_count = current_attempt,
                max_attempts = max_attempts,
                file_path = file_path_str,
                "Module reload max attempts reached, skipping reload until next file change. Continuing to serve last known good version."
            );

            return Err(RariError::module_reload(ModuleReloadError::MaxRetriesExceeded {
                message: format!(
                    "Module reload failed after {} attempts. Serving last known good version.",
                    max_attempts
                ),
                file_path: file_path_str.to_string(),
                attempts: max_attempts,
                last_error: None,
            }));
        }

        let mut last_error: Option<RariError> = None;
        let file_path_str = file_path.to_str().unwrap_or("unknown");

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay_ms = 100 * (1 << (attempt - 1));
                debug!(
                    component_id = component_id,
                    attempt = attempt + 1,
                    delay_ms = delay_ms,
                    "Retrying module reload after backoff"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }

            let reload_future = self.perform_reload(component_id, file_path);
            let timeout_result = tokio::time::timeout(timeout_duration, reload_future).await;

            match timeout_result {
                Ok(Ok(())) => {
                    debug!(
                        component_id = component_id,
                        attempt = attempt + 1,
                        "Module reload succeeded"
                    );
                    return Ok(());
                }
                Ok(Err(e)) => {
                    warn!(
                        component_id = component_id,
                        attempt = attempt + 1,
                        max_attempts = max_attempts,
                        error = %e,
                        error_code = e.code(),
                        "Module reload attempt failed, will retry on next request"
                    );
                    last_error = Some(e);
                }
                Err(_) => {
                    let timeout_error = RariError::module_reload(ModuleReloadError::Timeout {
                        message: format!(
                            "Module reload timed out after {}ms",
                            self.config.reload_timeout_ms
                        ),
                        file_path: file_path_str.to_string(),
                        timeout_ms: self.config.reload_timeout_ms,
                    });

                    if let Some(registry) = &self.component_registry {
                        let mut registry = registry.lock();
                        registry.mark_module_stale(component_id);
                        if let Some(component) = registry.get_component_mut(component_id) {
                            component.reload_attempt_count += 1;
                        }
                    }

                    warn!(
                        component_id = component_id,
                        attempt = attempt + 1,
                        max_attempts = max_attempts,
                        timeout_ms = self.config.reload_timeout_ms,
                        file_path = file_path_str,
                        "Module reload timed out, will retry on next request"
                    );
                    last_error = Some(timeout_error);
                }
            }
        }

        let final_error = last_error.unwrap_or_else(|| {
            RariError::module_reload(ModuleReloadError::Other {
                message: "Module reload failed for unknown reason".to_string(),
                file_path: Some(file_path_str.to_string()),
            })
        });

        let current_attempt = if let Some(registry) = &self.component_registry {
            let registry = registry.lock();
            registry.get_component(component_id).map(|c| c.reload_attempt_count).unwrap_or(0)
        } else {
            0
        };

        if current_attempt >= 3 {
            warn!(
                component_id = component_id,
                consecutive_failures = current_attempt,
                max_attempts = max_attempts,
                file_path = file_path_str,
                last_error = %final_error,
                "Module reload has failed {} consecutive times. Continuing to serve last known good version.",
                current_attempt
            );
        }

        Err(final_error)
    }

    pub async fn reload_modules_batch(
        &self,
        requests: Vec<ModuleReloadRequest>,
    ) -> Vec<Result<(), RariError>> {
        if !self.config.enabled {
            return vec![Ok(()); requests.len()];
        }

        let count = requests.len();
        info!(
            count = count,
            parallel = self.config.parallel_reloads,
            "Starting batch module reload"
        );

        let start = Instant::now();
        let results = if self.config.parallel_reloads {
            self.reload_modules_parallel(requests).await
        } else {
            self.reload_modules_sequential(requests).await
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        let successful = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.iter().filter(|r| r.is_err()).count();

        info!(
            count = count,
            successful = successful,
            failed = failed,
            duration_ms = duration_ms,
            "Batch module reload completed"
        );

        results
    }

    async fn reload_modules_parallel(
        &self,
        requests: Vec<ModuleReloadRequest>,
    ) -> Vec<Result<(), RariError>> {
        debug!(count = requests.len(), "Reloading modules in parallel");

        let mut handles = Vec::new();

        for request in requests {
            let component_id = request.component_id.clone();
            let file_path = request.file_path.clone();
            let manager = self.clone_for_task();

            let handle =
                tokio::spawn(async move { manager.reload_module(&component_id, &file_path).await });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!(error = %e, "Task join error during parallel reload");
                    results.push(Err(RariError::internal(format!("Task join error: {}", e))))
                }
            }
        }

        results
    }

    async fn reload_modules_sequential(
        &self,
        requests: Vec<ModuleReloadRequest>,
    ) -> Vec<Result<(), RariError>> {
        debug!(count = requests.len(), "Reloading modules sequentially");

        let mut results = Vec::new();

        for request in requests {
            let result = self.reload_module(&request.component_id, &request.file_path).await;
            results.push(result);
        }

        results
    }

    async fn invalidate_runtime_cache(&self, module_specifier: &str) -> Result<String, RariError> {
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            RariError::module_reload(ModuleReloadError::RuntimeNotAvailable {
                message: "JavaScript runtime not initialized for module reload".to_string(),
            })
        })?;

        let script = format!(
            r#"
            (function() {{
                if (globalThis.__rari_module_reload && globalThis.__rari_module_reload.invalidateModuleCache) {{
                    return globalThis.__rari_module_reload.invalidateModuleCache("{}");
                }} else {{
                    return "{}" + "?t=" + Date.now();
                }}
            }})()
            "#,
            module_specifier.replace('"', r#"\""#),
            module_specifier.replace('"', r#"\""#)
        );

        let result =
            runtime.execute_script("invalidate_cache".to_string(), script).await.map_err(|e| {
                RariError::module_reload(ModuleReloadError::Other {
                    message: format!("Failed to invalidate module cache: {}", e),
                    file_path: Some(module_specifier.to_string()),
                })
            })?;

        serde_json::from_value(result).map_err(|e| {
            RariError::module_reload(ModuleReloadError::Other {
                message: format!("Failed to parse cache invalidation result: {}", e),
                file_path: Some(module_specifier.to_string()),
            })
        })
    }

    async fn re_import_module(&self, module_specifier: &str) -> Result<JsReloadResult, RariError> {
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            RariError::module_reload(ModuleReloadError::RuntimeNotAvailable {
                message: "JavaScript runtime not initialized for module reload".to_string(),
            })
        })?;

        let script = format!(
            r#"
            (async function() {{
                if (globalThis.__rari_module_reload && globalThis.__rari_module_reload.reloadModule) {{
                    return await globalThis.__rari_module_reload.reloadModule("{}");
                }} else {{
                    return {{
                        success: true,
                        specifier: "{}",
                        timestamp: Date.now()
                    }};
                }}
            }})()
            "#,
            module_specifier.replace('"', r#"\""#),
            module_specifier.replace('"', r#"\""#)
        );

        let result =
            runtime.execute_script("reload_module".to_string(), script).await.map_err(|e| {
                RariError::module_reload(ModuleReloadError::Other {
                    message: format!("Failed to re-import module: {}", e),
                    file_path: Some(module_specifier.to_string()),
                })
            })?;

        debug!(
            module_specifier = module_specifier,
            result = ?result,
            "Module reload result from JavaScript"
        );

        let result_clone = result.clone();
        serde_json::from_value(result).map_err(|e| {
            RariError::module_reload(ModuleReloadError::Other {
                message: format!(
                    "Failed to parse module reload result: {} - raw result: {:?}",
                    e, result_clone
                ),
                file_path: Some(module_specifier.to_string()),
            })
        })
    }

    pub async fn get_module_from_cache(
        &self,
        module_specifier: &str,
    ) -> Result<JsModuleCacheInfo, RariError> {
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            RariError::module_reload(ModuleReloadError::RuntimeNotAvailable {
                message: "JavaScript runtime not initialized for module reload".to_string(),
            })
        })?;

        let script = format!(
            r#"
            (function() {{
                if (globalThis.__rari_module_reload && globalThis.__rari_module_reload.getModuleFromCache) {{
                    return globalThis.__rari_module_reload.getModuleFromCache("{}");
                }} else {{
                    throw new Error("Module reload helpers not initialized");
                }}
            }})()
            "#,
            module_specifier.replace('"', r#"\""#)
        );

        let result =
            runtime.execute_script("get_module_cache".to_string(), script).await.map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Module reload helpers not initialized") {
                    RariError::module_reload(ModuleReloadError::HelpersNotInitialized {
                        message: error_msg,
                    })
                } else {
                    RariError::module_reload(ModuleReloadError::Other {
                        message: format!("Failed to get module from cache: {}", error_msg),
                        file_path: Some(module_specifier.to_string()),
                    })
                }
            })?;

        serde_json::from_value(result).map_err(|e| {
            RariError::module_reload(ModuleReloadError::Other {
                message: format!("Failed to parse module cache info: {}", e),
                file_path: Some(module_specifier.to_string()),
            })
        })
    }

    async fn verify_component_after_reload(
        &self,
        component_id: &str,
    ) -> Result<JsComponentVerification, RariError> {
        let runtime = self.runtime.as_ref().ok_or_else(|| {
            RariError::module_reload(ModuleReloadError::RuntimeNotAvailable {
                message: "JavaScript runtime not initialized for module reload".to_string(),
            })
        })?;

        let script = format!(
            r#"
            (function() {{
                if (globalThis.__rari_module_reload && globalThis.__rari_module_reload.verifyComponentAfterReload) {{
                    return globalThis.__rari_module_reload.verifyComponentAfterReload("{}");
                }} else {{
                    throw new Error("Module reload helpers not initialized");
                }}
            }})()
            "#,
            component_id.replace('"', r#"\""#)
        );

        let result =
            runtime.execute_script("verify_component".to_string(), script).await.map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Module reload helpers not initialized") {
                    RariError::module_reload(ModuleReloadError::HelpersNotInitialized {
                        message: error_msg,
                    })
                } else {
                    RariError::module_reload(ModuleReloadError::Other {
                        message: format!("Failed to verify component: {}", error_msg),
                        file_path: None,
                    })
                }
            })?;

        serde_json::from_value(result).map_err(|e| {
            RariError::module_reload(ModuleReloadError::Other {
                message: format!("Failed to parse component verification result: {}", e),
                file_path: None,
            })
        })
    }

    async fn wait_for_vite_build(
        &self,
        component_id: &str,
        source_path: &Path,
    ) -> Result<(), RariError> {
        let resolver = self.dist_path_resolver.as_ref().ok_or_else(|| {
            RariError::module_reload(ModuleReloadError::Other {
                message: "Dist path resolver not initialized".to_string(),
                file_path: Some(source_path.to_str().unwrap_or("unknown").to_string()),
            })
        })?;

        let timeout_duration = Duration::from_millis(5000);
        let poll_interval = Duration::from_millis(50);
        let start = Instant::now();

        debug!(
            component_id = component_id,
            source_path = ?source_path,
            "Waiting for Vite build to complete"
        );

        loop {
            if resolver.is_dist_newer_than_source(component_id, source_path) {
                let elapsed = start.elapsed();
                debug!(
                    component_id = component_id,
                    elapsed_ms = elapsed.as_millis(),
                    "Vite build completed, dist file is up to date"
                );
                return Ok(());
            }

            if start.elapsed() >= timeout_duration {
                let error_msg = format!(
                    "Timeout waiting for Vite build to complete after {}ms. Dist file may be stale.",
                    timeout_duration.as_millis()
                );

                warn!(
                    component_id = component_id,
                    source_path = ?source_path,
                    timeout_ms = timeout_duration.as_millis(),
                    "Vite build timeout - proceeding with potentially stale dist file"
                );

                return Err(RariError::module_reload(ModuleReloadError::Timeout {
                    message: error_msg,
                    file_path: source_path.to_str().unwrap_or("unknown").to_string(),
                    timeout_ms: timeout_duration.as_millis() as u64,
                }));
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    async fn perform_reload(&self, component_id: &str, file_path: &Path) -> Result<(), RariError> {
        let module_specifier = file_path
            .to_str()
            .ok_or_else(|| RariError::internal("Invalid file path".to_string()))?;

        if let Err(e) = self.wait_for_vite_build(component_id, file_path).await {
            warn!(
                component_id = component_id,
                file_path = module_specifier,
                error = %e,
                "Failed to wait for Vite build, proceeding anyway"
            );
        }

        let _cache_busted_specifier = match self.invalidate_runtime_cache(module_specifier).await {
            Ok(specifier) => specifier,
            Err(e) => {
                if let Some(registry) = &self.component_registry {
                    let mut registry = registry.lock();
                    registry.mark_module_stale(component_id);
                    if let Some(component) = registry.get_component_mut(component_id) {
                        component.reload_attempt_count += 1;
                    }
                }

                error!(
                    component_id = component_id,
                    file_path = module_specifier,
                    error = %e,
                    "Failed to invalidate module cache"
                );

                return Err(e);
            }
        };

        let reload_result = match self.re_import_module(module_specifier).await {
            Ok(result) => result,
            Err(e) => {
                if let Some(registry) = &self.component_registry {
                    let mut registry = registry.lock();
                    registry.mark_module_stale(component_id);
                    if let Some(component) = registry.get_component_mut(component_id) {
                        component.reload_attempt_count += 1;
                    }
                }

                error!(
                    component_id = component_id,
                    file_path = module_specifier,
                    error = %e,
                    "Failed to re-import module"
                );

                return Err(e);
            }
        };

        if !reload_result.success {
            let error_msg = reload_result.error.unwrap_or_else(|| "Unknown error".to_string());
            let stack = reload_result.stack;
            let error_name = reload_result.name;

            let module_error = ModuleReloadError::from_js_error(
                error_msg.clone(),
                module_specifier.to_string(),
                stack.clone(),
                error_name.clone(),
            );

            if let Some(registry) = &self.component_registry {
                let mut registry = registry.lock();
                registry.mark_module_stale(component_id);
                if let Some(component) = registry.get_component_mut(component_id) {
                    component.reload_attempt_count += 1;
                }
            }

            error!(
                component_id = component_id,
                file_path = module_specifier,
                error_type = module_error.code(),
                error_message = %error_msg,
                error_name = ?error_name,
                stack = ?stack,
                "Module reload failed with JavaScript error"
            );

            return Err(RariError::module_reload(module_error));
        }

        let verification = match self.verify_component_after_reload(component_id).await {
            Ok(result) => result,
            Err(e) => {
                if let Some(registry) = &self.component_registry {
                    let mut registry = registry.lock();
                    registry.mark_module_stale(component_id);
                    if let Some(component) = registry.get_component_mut(component_id) {
                        component.reload_attempt_count += 1;
                    }
                }

                error!(
                    component_id = component_id,
                    file_path = module_specifier,
                    error = %e,
                    "Failed to verify component after reload"
                );

                return Err(e);
            }
        };

        if !verification.success || !verification.exists || !verification.is_valid_component {
            let error_msg = verification.error.unwrap_or_else(|| {
                if !verification.exists {
                    format!("Component '{}' not found in globalThis after reload", component_id)
                } else if !verification.is_function {
                    format!("Component '{}' is not a function", component_id)
                } else {
                    format!("Component '{}' is not a valid React component", component_id)
                }
            });

            if let Some(registry) = &self.component_registry {
                let mut registry = registry.lock();
                registry.mark_module_stale(component_id);
                if let Some(component) = registry.get_component_mut(component_id) {
                    component.reload_attempt_count += 1;
                }
            }

            error!(
                component_id = component_id,
                file_path = module_specifier,
                exists = verification.exists,
                is_function = verification.is_function,
                is_valid_component = verification.is_valid_component,
                error_message = %error_msg,
                "Component verification failed after reload"
            );

            return Err(RariError::module_reload(ModuleReloadError::Other {
                message: error_msg,
                file_path: Some(module_specifier.to_string()),
            }));
        }

        info!(
            component_id = component_id,
            file_path = module_specifier,
            component_name = ?verification.component_name,
            component_type = ?verification.component_type,
            exists = verification.exists,
            is_function = verification.is_function,
            is_valid_component = verification.is_valid_component,
            "Component verified successfully after reload"
        );

        if let Some(registry) = &self.component_registry {
            let mut registry = registry.lock();
            registry.update_module_reload_timestamp(component_id, Instant::now());
            if let Some(component) = registry.get_component_mut(component_id) {
                component.reload_attempt_count = 0;
            }
        }

        Ok(())
    }

    fn clone_for_task(&self) -> Self {
        Self {
            reload_queue: Arc::clone(&self.reload_queue),
            reload_stats: Arc::clone(&self.reload_stats),
            config: self.config.clone(),
            runtime: self.runtime.as_ref().map(Arc::clone),
            component_registry: self.component_registry.as_ref().map(Arc::clone),
            debounce_pending: Arc::clone(&self.debounce_pending),
            reload_history: Arc::clone(&self.reload_history),
            dist_path_resolver: self.dist_path_resolver.as_ref().map(Arc::clone),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reload_config_default() {
        let config = ReloadConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_retry_attempts, 3);
        assert_eq!(config.reload_timeout_ms, 5000);
        assert!(config.parallel_reloads);
    }

    #[tokio::test]
    async fn test_reload_stats_recording() {
        let mut stats = ReloadStats::default();

        stats.record_reload(true, 100);
        assert_eq!(stats.total_reloads, 1);
        assert_eq!(stats.successful_reloads, 1);
        assert_eq!(stats.failed_reloads, 0);
        assert_eq!(stats.average_reload_time_ms, 100.0);

        stats.record_reload(false, 200);
        assert_eq!(stats.total_reloads, 2);
        assert_eq!(stats.successful_reloads, 1);
        assert_eq!(stats.failed_reloads, 1);
        assert_eq!(stats.average_reload_time_ms, 150.0);
    }

    #[tokio::test]
    async fn test_module_reload_request_creation() {
        let request = ModuleReloadRequest::new(
            "test_component".to_string(),
            PathBuf::from("/path/to/component.tsx"),
        );

        assert_eq!(request.component_id, "test_component");
        assert_eq!(request.file_path, PathBuf::from("/path/to/component.tsx"));
    }

    #[tokio::test]
    async fn test_module_reload_manager_creation() {
        let config = ReloadConfig::default();
        let manager = ModuleReloadManager::new(config);

        assert!(manager.is_enabled());
        assert_eq!(manager.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_reload_queue_operations() {
        let manager = ModuleReloadManager::new(ReloadConfig::default());

        let request1 = ModuleReloadRequest::new(
            "component1".to_string(),
            PathBuf::from("/path/to/component1.tsx"),
        );
        let request2 = ModuleReloadRequest::new(
            "component2".to_string(),
            PathBuf::from("/path/to/component2.tsx"),
        );

        manager.enqueue_reload(request1).await;
        manager.enqueue_reload(request2).await;

        assert_eq!(manager.queue_size().await, 2);

        let dequeued = manager.dequeue_reload().await;
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().component_id, "component1");

        assert_eq!(manager.queue_size().await, 1);

        manager.clear_queue().await;
        assert_eq!(manager.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_reload_module_when_disabled() {
        let config = ReloadConfig { enabled: false, ..Default::default() };
        let manager = ModuleReloadManager::new(config);

        let result = manager.reload_module("test", Path::new("/test.tsx")).await;

        assert!(result.is_ok());
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_reloads, 0);
    }

    #[tokio::test]
    async fn test_reload_modules_batch_sequential() {
        let config = ReloadConfig { enabled: false, parallel_reloads: false, ..Default::default() };
        let manager = ModuleReloadManager::new(config);

        let requests = vec![
            ModuleReloadRequest::new("comp1".to_string(), PathBuf::from("/comp1.tsx")),
            ModuleReloadRequest::new("comp2".to_string(), PathBuf::from("/comp2.tsx")),
        ];

        let results = manager.reload_modules_batch(requests).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_reload_modules_batch_parallel() {
        let config = ReloadConfig { enabled: false, parallel_reloads: true, ..Default::default() };
        let manager = ModuleReloadManager::new(config);

        let requests = vec![
            ModuleReloadRequest::new("comp1".to_string(), PathBuf::from("/comp1.tsx")),
            ModuleReloadRequest::new("comp2".to_string(), PathBuf::from("/comp2.tsx")),
        ];

        let results = manager.reload_modules_batch(requests).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_reload_modules_batch_without_runtime() {
        let config = ReloadConfig { enabled: true, parallel_reloads: false, ..Default::default() };
        let manager = ModuleReloadManager::new(config);

        let requests =
            vec![ModuleReloadRequest::new("comp1".to_string(), PathBuf::from("/comp1.tsx"))];

        let results = manager.reload_modules_batch(requests).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());

        if let Err(e) = &results[0] {
            assert_eq!(e.code(), "MODULE_RELOAD_RUNTIME_NOT_AVAILABLE");
        }
    }
}
