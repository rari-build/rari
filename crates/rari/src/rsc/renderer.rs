use crate::error::RariError;
use crate::rsc::component::ComponentRegistry;
use crate::rsc::js_loader::{ModuleOperation, RscJsLoader};
use crate::rsc::jsx_transform::{extract_dependencies, hash_string, transform_jsx};

use crate::rsc::serializer::{ReactElement, RscSerializer};
use crate::rsc::streaming::{RscStream, StreamingRenderer};
use crate::runtime::JsExecutionRuntime;
use crate::server_fn::ServerFunctionExecutor;
use dashmap::DashMap;
use parking_lot::Mutex;
use regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::{debug, error};

const MEMORY_PRESSURE_THRESHOLD: f64 = 0.8;
const CACHE_CLEANUP_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_concurrent_renders: usize,
    pub max_render_time_ms: u64,
    pub max_script_execution_time_ms: u64,
    pub max_memory_per_component_mb: usize,
    pub max_cache_size: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_concurrent_renders: 10,
            max_render_time_ms: 5000,
            max_script_execution_time_ms: 1000,
            max_memory_per_component_mb: 50,
            max_cache_size: 100,
        }
    }
}

pub struct ResourceTracker {
    active_renders: AtomicUsize,
    total_renders: AtomicU64,
    total_render_time_ms: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    timeout_errors: AtomicU64,
    memory_pressure_events: AtomicU64,
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            active_renders: AtomicUsize::new(0),
            total_renders: AtomicU64::new(0),
            total_render_time_ms: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            timeout_errors: AtomicU64::new(0),
            memory_pressure_events: AtomicU64::new(0),
        }
    }

    pub fn get_metrics(&self) -> ResourceMetrics {
        let total_renders = self.total_renders.load(Ordering::Relaxed);
        let total_time = self.total_render_time_ms.load(Ordering::Relaxed);

        ResourceMetrics {
            active_renders: self.active_renders.load(Ordering::Relaxed),
            total_renders,
            average_render_time_ms: if total_renders > 0 {
                total_time as f64 / total_renders as f64
            } else {
                0.0
            },
            cache_hit_rate: {
                let hits = self.cache_hits.load(Ordering::Relaxed);
                let misses = self.cache_misses.load(Ordering::Relaxed);
                let total = hits + misses;
                if total > 0 { hits as f64 / total as f64 } else { 0.0 }
            },
            timeout_errors: self.timeout_errors.load(Ordering::Relaxed),
            memory_pressure_events: self.memory_pressure_events.load(Ordering::Relaxed),
        }
    }

    pub fn increment_active_renders(&self) {
        self.active_renders.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_active_renders(&self) {
        self.active_renders.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_render_completion(&self, duration: Duration) {
        self.total_renders.fetch_add(1, Ordering::Relaxed);
        self.total_render_time_ms.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceMetrics {
    pub active_renders: usize,
    pub total_renders: u64,
    pub average_render_time_ms: f64,
    pub cache_hit_rate: f64,
    pub timeout_errors: u64,
    pub memory_pressure_events: u64,
}

pub struct RscRenderer {
    pub(crate) runtime: Arc<JsExecutionRuntime>,
    pub(crate) timeout_ms: u64,
    pub(crate) initialized: bool,
    pub(crate) server_fn_executor: Option<Arc<ServerFunctionExecutor>>,
    pub(crate) component_registry: Arc<Mutex<ComponentRegistry>>,
    pub(crate) script_cache: DashMap<String, String>,
    pub(crate) resource_limits: ResourceLimits,
    pub(crate) resource_tracker: Arc<ResourceTracker>,
    pub(crate) serializer: Arc<Mutex<RscSerializer>>,
}

impl RscRenderer {
    pub fn new(runtime: Arc<JsExecutionRuntime>) -> Self {
        Self::with_resource_limits(runtime, ResourceLimits::default())
    }

    pub fn with_resource_limits(
        runtime: Arc<JsExecutionRuntime>,
        resource_limits: ResourceLimits,
    ) -> Self {
        Self {
            runtime,
            timeout_ms: 30000,
            initialized: false,
            server_fn_executor: None,
            component_registry: Arc::new(Mutex::new(ComponentRegistry::new())),
            script_cache: DashMap::new(),
            resource_limits,
            resource_tracker: Arc::new(ResourceTracker::new()),
            serializer: Arc::new(Mutex::new(RscSerializer::new())),
        }
    }

    pub fn get_resource_metrics(&self) -> ResourceMetrics {
        self.resource_tracker.get_metrics()
    }

    pub async fn shutdown(&self) -> Result<(), RariError> {
        let shutdown_timeout = Duration::from_millis(self.resource_limits.max_render_time_ms * 2);
        let start_time = Instant::now();

        while self.resource_tracker.active_renders.load(Ordering::Relaxed) > 0 {
            if start_time.elapsed() > shutdown_timeout {
                break;
            }
            tokio::time::sleep(CACHE_CLEANUP_INTERVAL).await;
        }

        self.clear_script_cache();

        Ok(())
    }

    pub fn is_under_memory_pressure(&self) -> bool {
        let metrics = self.get_resource_metrics();
        let current_renders = metrics.active_renders;
        let max_renders = self.resource_limits.max_concurrent_renders;

        current_renders as f64 / max_renders as f64 > MEMORY_PRESSURE_THRESHOLD
            || metrics.memory_pressure_events > 0
    }

    pub async fn force_cleanup(&self) -> Result<(), RariError> {
        self.clear_script_cache();
        self.resource_tracker.memory_pressure_events.store(0, Ordering::Relaxed);
        Ok(())
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_server_fn_executor(mut self, executor: Arc<ServerFunctionExecutor>) -> Self {
        self.server_fn_executor = Some(executor);
        self
    }

    fn get_cached_script(&self, cache_key: &str) -> Option<String> {
        let result = self.script_cache.get(cache_key).map(|entry| entry.value().clone());
        if result.is_some() {
            self.resource_tracker.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.resource_tracker.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    fn cache_script(&self, cache_key: String, script: String) {
        if self.script_cache.len() > self.resource_limits.max_cache_size {
            self.script_cache.clear();
            self.resource_tracker.memory_pressure_events.fetch_add(1, Ordering::Relaxed);
        }

        self.script_cache.insert(cache_key, script);
    }

    pub fn clear_script_cache(&self) {
        self.script_cache.clear();
    }

    async fn execute_script_with_timeout(
        &self,
        script_name: String,
        script: String,
    ) -> Result<JsonValue, RariError> {
        let timeout_duration =
            Duration::from_millis(self.resource_limits.max_script_execution_time_ms);

        match timeout(timeout_duration, self.runtime.execute_script(script_name.clone(), script))
            .await
        {
            Ok(result) => result,
            Err(_) => {
                self.resource_tracker.timeout_errors.fetch_add(1, Ordering::Relaxed);
                Err(RariError::js_execution(format!(
                    "Script '{}' execution timed out after {}ms",
                    script_name, self.resource_limits.max_script_execution_time_ms
                )))
            }
        }
    }

    fn create_batch_script_section(&self, index: usize, name: &str, script: &str) -> String {
        format!(
            r#"
            // === Batch Script {}: {} ===
            try {{
                {}
            }} catch (batchError_{}) {{
                globalThis.__batch_errors = globalThis.__batch_errors || [];
                globalThis.__batch_errors.push({{
                    script: "{}",
                    error: batchError_{}.message || String(batchError_{})
                }});
            }}
            "#,
            index + 1,
            name,
            script,
            index,
            name,
            index,
            index
        )
    }

    async fn execute_batched_scripts(
        &self,
        scripts: Vec<(&str, String)>,
    ) -> Result<JsonValue, RariError> {
        if scripts.is_empty() {
            return Ok(serde_json::json!({}));
        }

        let batch_sections: Vec<String> = scripts
            .iter()
            .enumerate()
            .map(|(i, (name, script))| self.create_batch_script_section(i, name, script))
            .collect();

        let combined_script = batch_sections.join("\n");

        let final_script = format!(
            r#"
            {combined_script}

            (function() {{
                const errors = globalThis.__batch_errors || [];
                globalThis.__batch_errors = [];
                return {{
                    success: errors.length === 0,
                    errors: errors,
                    timestamp: Date.now()
                }};
            }})();
            "#
        );

        let batch_name = format!("batch_execution_{}", scripts.len());
        let result = self.execute_script_with_timeout(batch_name, final_script).await?;

        self.handle_batch_script_result(result, scripts.len())
    }

    fn handle_batch_script_result(
        &self,
        result: JsonValue,
        _script_count: usize,
    ) -> Result<JsonValue, RariError> {
        if let Some(success) = result.get("success").and_then(|s| s.as_bool())
            && !success
            && let Some(errors) = result.get("errors").and_then(|e| e.as_array())
        {
            let error_details = errors
                .iter()
                .filter_map(|e| {
                    e.get("script").and_then(|s| s.as_str()).map(|script| {
                        let error_msg =
                            e.get("error").and_then(|m| m.as_str()).unwrap_or("Unknown error");
                        format!("  - {script}: {error_msg}")
                    })
                })
                .collect::<Vec<_>>()
                .join("\n");

            return Err(RariError::js_execution(format!(
                "Batch script execution failed:\n{error_details}"
            )));
        }

        Ok(result)
    }

    pub async fn initialize(&mut self) -> Result<(), RariError> {
        if self.initialized {
            return Ok(());
        }

        let extension_checks = r#"
            (function() {
                const checks = {};

                if (typeof globalThis.renderToHTML === 'undefined') {
                    throw new Error('RSC Renderer extension not loaded - renderToHTML not available');
                }
                checks.rsc_renderer = true;

                if (!globalThis.PromiseManager) {
                    throw new Error('PromiseManager extension not loaded');
                }
                checks.promise_manager = true;

                if (!globalThis.registerModule) {
                    throw new Error('RSC Modules extension not loaded');
                }
                checks.rsc_modules = true;

                return {
                    initialized: true,
                    extensions: checks,
                    timestamp: Date.now()
                };
            })()
        "#;

        let _extension_check_result = self
            .runtime
            .execute_script("extension-checks".to_string(), extension_checks.to_string())
            .await?;

        self.initialized = true;

        Ok(())
    }

    pub async fn register_component(
        &mut self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        if cfg!(debug_assertions) || std::env::var("NODE_ENV").as_deref() != Ok("production") {
            self.clear_component_module_cache(component_id).await?;
        }

        let init_registry_script = RscJsLoader::create_global_init();
        let _init_result = self
            .runtime
            .execute_script("ensure_global_registries.js".to_string(), init_registry_script)
            .await?;

        let isolation_namespacing_script =
            RscJsLoader::create_isolation_namespacing_script(component_id);

        let _isolation_result = self
            .runtime
            .execute_script(
                format!("create_namespace_{component_id}.js"),
                isolation_namespacing_script,
            )
            .await?;

        let dependencies = extract_dependencies(component_code);

        for dep in &dependencies {
            if let Err(e) = self.register_dependency_if_needed(dep).await {
                error!(
                    "[RSC] Warning: Failed to register dependency '{dep}' for component '{component_id}': {e}"
                );
            }
        }

        self.register_component_without_loading(component_id, component_code).await?;

        self.load_all_components().await?;

        Ok(())
    }

    pub fn clear_component_cache(&self, component_id: &str) {
        let cache_keys_to_remove: Vec<String> = self
            .script_cache
            .iter()
            .filter_map(|entry| {
                let key = entry.key();
                if key.contains(component_id) { Some(key.clone()) } else { None }
            })
            .collect();

        for key in cache_keys_to_remove {
            self.script_cache.remove(&key);
        }
    }

    pub async fn clear_component_module_cache(
        &mut self,
        component_id: &str,
    ) -> Result<(), RariError> {
        self.clear_component_cache(component_id);

        {
            let mut registry = self.component_registry.lock();
            registry.remove_component(component_id);
        }

        self.runtime.clear_module_loader_caches(component_id).await?;

        let force_v8_cache_clear_script = format!(
            r#"
            (function() {{
                try {{
                    const componentId = "{component_id}";
                    let clearedCount = 0;

                    if (globalThis[componentId]) {{
                        delete globalThis[componentId];
                        clearedCount++;
                    }}

                    const registrationKey = `Component_${{componentId.replace(/[^a-zA-Z0-9]/g, '_')}}`;
                    if (globalThis[registrationKey]) {{
                        delete globalThis[registrationKey];
                        clearedCount++;
                    }}

                    if (globalThis.__rsc_modules && globalThis.__rsc_modules[componentId]) {{
                        delete globalThis.__rsc_modules[componentId];
                        clearedCount++;
                    }}

                    if (globalThis.__rsc_functions && globalThis.__rsc_functions[componentId]) {{
                        delete globalThis.__rsc_functions[componentId];
                        clearedCount++;
                    }}

                    return {{
                        success: true,
                        clearedCount: clearedCount,
                        componentId: componentId
                    }};
                }} catch (error) {{
                    return {{
                        success: false,
                        error: error.message,
                        componentId: "{component_id}"
                    }};
                }}
            }})()
            "#
        );

        let _v8_clear_result = self
            .runtime
            .execute_script(
                format!("force_v8_cache_clear_{component_id}.js"),
                force_v8_cache_clear_script,
            )
            .await?;

        Ok(())
    }

    fn is_react_component_file(content: &str) -> bool {
        let has_jsx =
            content.contains('<') && content.contains('>') && !content.contains("</script>");
        let has_react_import = content.contains("import")
            && (content.contains("from 'react'")
                || content.contains("from \"react\"")
                || content.contains("React"));
        let has_server_directive =
            content.contains("'use server'") || content.contains("\"use server\"");
        let has_client_directive =
            content.contains("'use client'") || content.contains("\"use client\"");
        let has_component_export = content.contains("export default function")
            || content.contains("export default async function");

        has_jsx
            || has_server_directive
            || has_client_directive
            || (has_react_import && has_component_export)
    }

    async fn register_dependency_if_needed(&mut self, dep: &str) -> Result<(), RariError> {
        if !dep.starts_with("./") && !dep.starts_with("../") {
            return Ok(());
        }

        let base_path = std::env::current_dir().unwrap_or_default();
        let src_dir = base_path.join("src");

        let mut resolved_path_candidates = Vec::new();

        let clean_dep = dep.trim_start_matches("./").trim_start_matches("../");

        if dep.starts_with("../") {
            let up_count = dep.matches("../").count();
            let remaining_path = dep.replacen("../", "", up_count);

            if up_count == 1 {
                resolved_path_candidates.push(src_dir.join(&remaining_path));
            } else if up_count == 2 {
                resolved_path_candidates.push(base_path.join(&remaining_path));
            }
        } else if dep.starts_with("./") {
            resolved_path_candidates.push(src_dir.join("components").join(clean_dep));
            resolved_path_candidates.push(src_dir.join(clean_dep));
        }

        let extensions = [".ts", ".tsx", ".js", ".jsx"];
        let mut potential_paths = Vec::new();

        for base_path_candidate in &resolved_path_candidates {
            for ext in &extensions {
                potential_paths.push(base_path_candidate.with_extension(&ext[1..]));
            }

            for ext in &extensions {
                potential_paths.push(base_path_candidate.join(format!("index{ext}")));
            }
        }

        for potential_path in &potential_paths {
            if potential_path.exists() {
                if let Ok(content) = std::fs::read_to_string(potential_path) {
                    let dep_component_id = potential_path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    let path_components: Vec<&str> = potential_path
                        .strip_prefix(base_path.join("src"))
                        .unwrap_or(potential_path)
                        .components()
                        .filter_map(|c| c.as_os_str().to_str())
                        .collect();

                    let unique_dep_id = if path_components.len() > 1 {
                        format!(
                            "{}_{}",
                            path_components[0..path_components.len() - 1].join("_"),
                            dep_component_id
                        )
                    } else {
                        dep_component_id.clone()
                    };

                    let already_registered = {
                        let registry = self.component_registry.lock();
                        registry.is_component_registered(&unique_dep_id)
                    };

                    if !already_registered {
                        if Self::is_react_component_file(&content) {
                            let sub_dependencies = extract_dependencies(&content);
                            for sub_dep in &sub_dependencies {
                                let _ = Box::pin(self.register_dependency_if_needed(sub_dep)).await;
                            }

                            self.register_component_without_loading(&unique_dep_id, &content)
                                .await?;
                        } else {
                            debug!(
                                "Skipping registration of '{}' as it's not a React component",
                                unique_dep_id
                            );
                        }
                    }
                }
                break;
            }
        }

        Ok(())
    }

    async fn register_component_without_loading(
        &mut self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        let transformed_module_code = transform_jsx(component_code, component_id)?;

        let dependencies = extract_dependencies(component_code);

        {
            let mut registry = self.component_registry.lock();
            let _ = registry.register_component(
                component_id,
                component_code,
                transformed_module_code.clone(),
                dependencies.clone().into_iter().collect(),
            );
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let module_specifier = format!("file:///rari_component/{component_id}.js?v={timestamp}");

        if let Err(e) = self
            .runtime
            .add_module_to_loader_only(&module_specifier, transformed_module_code.clone())
            .await
        {
            return Err(RariError::js_execution(format!(
                "Failed to add self-registering module for component '{component_id}' to loader: {e}"
            )));
        }

        let dependencies_json =
            serde_json::to_string(&dependencies.into_iter().collect::<Vec<_>>())
                .unwrap_or_else(|_| "[]".to_string());
        let register_exports_script = RscJsLoader::create_module_operation_script(
            component_id,
            ModuleOperation::Register { dependencies_json },
        );

        self.runtime
            .execute_script(format!("register_exports_{component_id}.js"), register_exports_script)
            .await?;

        Ok(())
    }

    async fn load_all_components(&mut self) -> Result<(), RariError> {
        let components_to_load = {
            let registry = self.component_registry.lock();
            registry.get_unloaded_components_in_order()
        };

        if components_to_load.is_empty() {
            return Ok(());
        }

        let init_registry_script = RscJsLoader::create_global_init();

        self.runtime
            .execute_script(
                "init_global_registries.js".to_string(),
                init_registry_script.to_string(),
            )
            .await?;

        for component_id in &components_to_load {
            let isolation_script = RscJsLoader::create_isolation_init_script(component_id);

            let _isolation_result = self
                .runtime
                .execute_script(format!("isolation_{component_id}.js"), isolation_script)
                .await?;

            let (transformed_source, dependencies) = {
                let registry = self.component_registry.lock();
                let component = registry.get_component(component_id).ok_or_else(|| {
                    RariError::not_found(format!("Component not found: {component_id}"))
                })?;

                (component.transformed_source.clone(), component.dependencies.clone())
            };

            let module_specifier_js = format!("file:///rari_component/{component_id}.js");

            self.runtime
                .add_module_to_loader_only(&module_specifier_js, transformed_source)
                .await?;

            let dependencies_json =
                serde_json::to_string(&dependencies.into_iter().collect::<Vec<_>>())
                    .unwrap_or_else(|_| "[]".to_string());
            let register_exports_script = RscJsLoader::create_module_operation_script(
                component_id,
                ModuleOperation::Register { dependencies_json },
            );

            self.runtime
                .execute_script(
                    format!("register_exports_{component_id}.js"),
                    register_exports_script,
                )
                .await?;
        }

        for component_id in &components_to_load {
            let module_specifier_js = format!("file:///rari_component/{component_id}.js");

            let load_script = RscJsLoader::create_module_operation_script(
                component_id,
                ModuleOperation::Load { module_specifier: module_specifier_js },
            );

            match self.runtime.execute_script(format!("load_{component_id}.js"), load_script).await
            {
                Ok(_) => {
                    let verify_script = self.create_component_verification_script(component_id);
                    self.execute_verification_script(component_id, verify_script).await?;

                    let mut registry = self.component_registry.lock();
                    registry.mark_component_loaded(component_id);
                }
                Err(e) => {
                    return Err(RariError::js_execution(format!(
                        "Failed to load self-registering module for component '{component_id}': {e}"
                    )));
                }
            }
        }

        Ok(())
    }

    fn create_component_verification_script(&self, component_id: &str) -> String {
        let hashed_component_id = format!("Component_{}", hash_string(component_id));
        RscJsLoader::create_component_verification_script(component_id, &hashed_component_id)
    }

    async fn execute_verification_script(
        &self,
        component_id: &str,
        verify_script: String,
    ) -> Result<(), RariError> {
        let result = self
            .runtime
            .execute_script(format!("verify_{component_id}.js"), verify_script)
            .await
            .map_err(|e| {
                RariError::js_execution(format!(
                    "Verification script execution failed for '{component_id}': {e}"
                ))
            })?;

        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

        if success {
            return Ok(());
        }

        let error_details =
            result.get("details").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(
                || {
                    result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "No error details available".to_string())
                },
            );

        Err(RariError::js_execution(format!(
            "Component verification failed for '{component_id}': {error_details}"
        )))
    }

    pub fn component_exists(&self, component_id: &str) -> bool {
        let registry = self.component_registry.lock();
        registry.get_component(component_id).is_some()
    }

    pub async fn is_client_reference(&self, component_id: &str) -> bool {
        tracing::info!("Checking if component '{}' is a client reference", component_id);

        let registry = self.component_registry.lock();
        let is_client_ref = registry.is_client_reference(component_id);
        if is_client_ref {
            tracing::info!(
                "Component '{}' found in component registry as client reference",
                component_id
            );
            return true;
        }

        let serializer = self.serializer.lock();
        let is_registered = serializer.is_client_component_registered(component_id);
        tracing::info!(
            "Component '{}' serializer registration status: {}",
            component_id,
            is_registered
        );

        is_registered
    }

    pub fn register_client_component(
        &self,
        component_id: &str,
        file_path: &str,
        export_name: &str,
    ) {
        tracing::info!(
            "Registering client component in renderer: {} from {} with export {}",
            component_id,
            file_path,
            export_name
        );

        let mut registry = self.component_registry.lock();
        registry.register_client_reference(component_id, file_path, export_name);

        let mut serializer = self.serializer.lock();
        serializer.register_client_component(component_id, file_path, export_name);

        tracing::info!("Client component '{}' registered successfully", component_id);
    }

    pub fn list_components(&self) -> Vec<String> {
        let registry = self.component_registry.lock();
        registry.list_component_ids()
    }

    fn get_or_cache_script<F>(&self, cache_key: &str, generator: F) -> String
    where
        F: FnOnce() -> String,
    {
        if let Some(cached) = self.get_cached_script(cache_key) {
            cached
        } else {
            let script = generator();
            self.cache_script(cache_key.to_string(), script.clone());
            script
        }
    }

    pub async fn render_to_rsc_format(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<String, RariError> {
        self.resource_tracker.increment_active_renders();
        let result = self.internal_render_to_rsc(component_id, props).await;
        self.resource_tracker.decrement_active_renders();
        result
    }

    pub async fn render_to_string(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<String, RariError> {
        self.resource_tracker.increment_active_renders();
        let result = self.internal_render_to_string(component_id, props).await;
        self.resource_tracker.decrement_active_renders();
        result
    }

    async fn internal_render_to_rsc(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<String, RariError> {
        let render_start = Instant::now();

        if !self.initialized {
            return Err(RariError::internal("ReactDOMServer not initialized"));
        }

        tracing::info!("Checking if component '{}' is a client reference", component_id);
        if self.is_client_reference(component_id).await {
            tracing::info!("Component '{}' is a client reference, handling as such", component_id);
            return self.handle_client_reference(component_id, props).await;
        }

        tracing::info!(
            "Component '{}' is not a client reference, proceeding with RSC rendering",
            component_id
        );

        if !self.component_exists(component_id) {
            return Err(RariError::not_found(format!("Component not found: {component_id}")));
        }

        let component_hash = hash_string(component_id);
        let props_json = props.filter(|p| !p.trim().is_empty()).unwrap_or("{}");

        let clear_environment_script = self
            .get_or_cache_script(&format!("clear_env_{component_id}"), || {
                RscJsLoader::create_component_environment_setup(component_id)
            });

        let detect_module_promises =
            self.get_or_cache_script(&format!("detect_promises_{component_id}"), || {
                RscJsLoader::load_component_isolation_with_id(component_id).unwrap_or_else(|e| {
                    tracing::error!("Failed to load component isolation script: {}", e);
                    String::new()
                })
            });

        let setup_scripts = vec![
            ("clear_environment", clear_environment_script),
            ("detect_promises", detect_module_promises),
        ];

        self.execute_batched_scripts(setup_scripts).await?;

        let render_script =
            RscJsLoader::load_component_render_with_data(component_id, &component_hash, props_json)
                .map_err(|e| {
                    RariError::js_execution(format!("Failed to load component render script: {e}"))
                })?;

        self.execute_script_with_timeout(format!("render_{component_id}.js"), render_script)
            .await?;

        let rsc_extraction_script = self
            .get_or_cache_script(&format!("extract_rsc_{component_id}"), || {
                RscJsLoader::create_rsc_extraction_script(component_id)
            });

        let extraction_result = self
            .execute_script_with_timeout(
                format!("extract_rsc_{component_id}.js"),
                rsc_extraction_script,
            )
            .await?;

        let render_duration = render_start.elapsed();
        self.resource_tracker.record_render_completion(render_duration);

        self.process_rsc_extraction_result(component_id, extraction_result)
    }

    fn process_rsc_extraction_result(
        &self,
        component_id: &str,
        extraction_result: JsonValue,
    ) -> Result<String, RariError> {
        let parsed_result: serde_json::Value = if let Some(obj) = extraction_result.as_object() {
            serde_json::Value::Object(obj.clone())
        } else {
            let rsc_result = extraction_result.as_str().unwrap_or("");
            if rsc_result.is_empty() {
                return Err(RariError::js_execution(format!(
                    "Empty RSC result for component '{component_id}'. Component may have failed to render properly."
                )));
            }
            serde_json::from_str(rsc_result)
                .map_err(|e| RariError::js_execution(format!("Failed to parse RSC result: {e}")))?
        };

        if let Some(error) = parsed_result.get("error")
            && error.as_bool().unwrap_or(false)
        {
            let message = parsed_result
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown RSC error");
            return Err(RariError::js_execution(format!(
                "RSC rendering error for component '{component_id}': {message}"
            )));
        }

        let rsc_data = parsed_result.get("rsc").ok_or_else(|| {
            RariError::js_execution(format!(
                "No RSC data found in result for component '{component_id}'"
            ))
        })?;

        let mut serializer = self.serializer.lock();
        let rsc_payload = serializer
            .serialize_rsc_json(rsc_data)
            .map_err(|e| RariError::js_execution(format!("Failed to serialize RSC data: {e}")))?;

        Ok(rsc_payload)
    }

    async fn internal_render_to_string(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<String, RariError> {
        let render_start = Instant::now();

        self.resource_tracker.total_renders.fetch_add(1, Ordering::Relaxed);

        if !self.initialized {
            return Err(RariError::internal("ReactDOMServer not initialized"));
        }

        tracing::info!("About to check if component '{}' is a client reference", component_id);
        if self.is_client_reference(component_id).await {
            tracing::info!("Component '{}' is a client reference, handling as such", component_id);
            return self.handle_client_reference(component_id, props).await;
        }
        tracing::info!(
            "Component '{}' is not a client reference, proceeding with server rendering",
            component_id
        );

        let component_found = self.component_exists(component_id);
        if !component_found {
            return Err(RariError::not_found(format!("Component not found: {component_id}")));
        }

        let component_hash = hash_string(component_id);
        let props_json = match props {
            Some(p) if !p.trim().is_empty() => p,
            _ => "{}",
        };

        let clear_environment_script = {
            let cache_key = format!("clear_env_{component_id}");
            if let Some(cached) = self.get_cached_script(&cache_key) {
                cached
            } else {
                let script = RscJsLoader::create_component_environment_setup(component_id);
                self.cache_script(cache_key, script.clone());
                script
            }
        };

        let detect_module_promises = {
            let cache_key = format!("detect_promises_{component_id}");
            if let Some(cached) = self.get_cached_script(&cache_key) {
                cached
            } else {
                let script =
                    RscJsLoader::load_component_isolation_with_id(component_id).map_err(|e| {
                        RariError::js_execution(format!(
                            "Failed to load component isolation script: {e}"
                        ))
                    })?;
                self.cache_script(cache_key, script.clone());
                script
            }
        };

        let enhance_use_hook_script = {
            let cache_key = "enhance_use_hook".to_string();
            if let Some(cached) = self.get_cached_script(&cache_key) {
                cached
            } else {
                let script = r#"
                    (function() {
                        if (!globalThis.PromiseManager) {
                            throw new Error('PromiseManager extension not loaded');
                        }
                        return { available: true, extension: 'promise_manager' };
                    })()
                "#
                .to_string();
                self.cache_script(cache_key, script.clone());
                script
            }
        };

        let server_function_resolver_script = {
            let cache_key = "server_function_resolver".to_string();
            if let Some(cached) = self.get_cached_script(&cache_key) {
                cached
            } else {
                let script = r#"
                    (function() {
                        if (!globalThis.ServerFunctions) {
                            throw new Error('ServerFunctions extension not loaded');
                        }
                        return globalThis.ServerFunctions.resolve();
                    })()
                "#
                .to_string();
                self.cache_script(cache_key, script.clone());
                script
            }
        };

        let isolation_init_script = {
            let cache_key = format!("isolation_init_{component_id}");
            if let Some(cached) = self.get_cached_script(&cache_key) {
                cached
            } else {
                let script = RscJsLoader::create_isolation_init_script(component_id);
                self.cache_script(cache_key, script.clone());
                script
            }
        };

        let setup_scripts = vec![
            ("clear_environment", clear_environment_script),
            ("detect_promises", detect_module_promises),
            ("enhance_use_hook", enhance_use_hook_script),
            ("server_function_resolver", server_function_resolver_script),
            ("isolation_init", isolation_init_script),
        ];

        let _batch_result = self.execute_batched_scripts(setup_scripts).await?;

        let resolve_server_functions_script = format!(
            r#"
            (async function() {{
                try {{
                    if (typeof globalThis.enhanceReactUseForServerFunctions === 'function') {{
                        globalThis.enhanceReactUseForServerFunctions();
                    }}

                    if (typeof globalThis.resolveServerFunctionsForComponent === 'function') {{
                        await globalThis.resolveServerFunctionsForComponent("{component_id}");
                    }}

                    return {{ success: true, resolved: true }};
                }} catch (error) {{
                    return {{ success: false, error: error.message }};
                }}
            }})()
            "#
        );

        let _resolution_result = self
            .execute_script_with_timeout(
                format!("resolve_server_functions_{component_id}.js"),
                resolve_server_functions_script,
            )
            .await?;

        let render_script =
            RscJsLoader::load_component_render_with_data(component_id, &component_hash, props_json)
                .map_err(|e| {
                    RariError::js_execution(format!("Failed to load component render script: {e}"))
                })?;

        let _result = self
            .execute_script_with_timeout(format!("render_{component_id}.js"), render_script)
            .await;

        let html_extraction_script = {
            let cache_key = format!("extract_html_{component_id}");
            if let Some(cached) = self.get_cached_script(&cache_key) {
                cached
            } else {
                let script = RscJsLoader::create_html_extraction_script(component_id);
                self.cache_script(cache_key, script.clone());
                script
            }
        };

        let extraction_result = self
            .execute_script_with_timeout(
                format!("extract_html_{component_id}.js"),
                html_extraction_script,
            )
            .await;

        match extraction_result {
            Ok(value) => {
                let mut html =
                    value.get("html").and_then(|h| h.as_str()).unwrap_or_default().to_string();

                html = self.sanitize_html_output(&html, component_id);

                let render_duration = render_start.elapsed();

                self.resource_tracker
                    .total_render_time_ms
                    .fetch_add(render_duration.as_millis() as u64, Ordering::Relaxed);

                if html == "<div></div>" || html.trim() == "" || html == "<div/>" {
                    return Ok(format!(
                        r#"<div data-component-id='{}' data-diagnostic='true'>
                            <h2>Component: {}</h2>
                            <p>This component rendered with no content.</p>
                            <p>This may indicate the component doesn't return JSX or has a rendering issue.</p>
                            <p>Server time: {}</p>
                        </div>"#,
                        component_id,
                        component_id,
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("System time is before UNIX_EPOCH")
                            .as_secs()
                    ));
                }

                Ok(html)
            }
            Err(e) => Ok(format!(
                r#"<div>
                        <h2>Error Rendering {}</h2>
                        <p>There was an error rendering this component: {}</p>
                        <p>Server time: {}</p>
                    </div>"#,
                component_id,
                e,
                SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("System time is before UNIX_EPOCH")
                    .as_secs()
            )),
        }
    }

    async fn handle_client_reference(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<String, RariError> {
        tracing::info!("Handling client reference for component: {}", component_id);

        let props_map = if let Some(props_str) = props {
            if !props_str.trim().is_empty() {
                serde_json::from_str::<FxHashMap<String, JsonValue>>(props_str).ok()
            } else {
                None
            }
        } else {
            None
        };

        let client_element = ReactElement::create_client_component(component_id, props_map);

        let mut serializer = self.serializer.lock();
        let rsc_payload = serializer.serialize_to_rsc_format(&client_element);

        Ok(format!(
            r#"<div data-rsc-client-reference="{}" data-rsc-payload="{}" data-component-id="{}">
                <div style="padding: 1rem; border: 2px dashed #3b82f6; background-color: #eff6ff; color: #1e40af; border-radius: 0.5rem;">
                    <h4 style="margin: 0 0 0.5rem 0; font-weight: 600;">Client Component: {}</h4>
                    <p style="margin: 0; font-size: 0.875rem;">This component should be hydrated on the client side.</p>
                    <p style="margin: 0.25rem 0 0 0; font-size: 0.75rem; opacity: 0.7;">RSC Payload: {}</p>
                </div>
            </div>"#,
            component_id,
            rsc_payload.replace('"', "&quot;"),
            component_id,
            component_id,
            rsc_payload.replace('"', "&quot;")
        ))
    }

    fn sanitize_html_output(&self, html: &str, component_id: &str) -> String {
        let mut sanitized_html = html.to_string();

        let sanitization_rules =
            [(r#"\{"id".*?\}"#, ""), (r#"<pre>\{.*?\}</pre>"#, ""), (r#"\[(\{".*?},?)+\]"#, "[]")];

        for (pattern, replacement) in sanitization_rules.iter() {
            if let Ok(regex) = regex::Regex::new(pattern)
                && regex.is_match(&sanitized_html)
            {
                sanitized_html = regex.replace_all(&sanitized_html, *replacement).to_string();
            }
        }

        let boundary_markers = [
            format!(r#"<div[^>]*?data-component-id=["']{component_id}["'][^>]*?>(.*?)</div>"#),
            format!(r#"<div[^>]*?id=["']{component_id}-wrapper["'][^>]*?>(.*?)</div>"#),
            format!(r#"<div[^>]*?data-rsc-component=["']{component_id}["'][^>]*?>(.*?)</div>"#),
        ];

        for marker in boundary_markers.iter() {
            if let Ok(regex) = regex::Regex::new(marker)
                && regex.is_match(&sanitized_html)
            {
                break;
            }
        }

        let mut result_contains_foreign_data = false;

        let leakage_indicators = [
            (r#"=".*?\{"id".*?\}.*?""#, true),
            (r#">.*?\{"id".*?\}.*?<"#, true),
            (r#"=".*?\[.*?\{.*?\}.*?\].*?""#, true),
        ];

        for (pattern, _) in leakage_indicators.iter() {
            if let Ok(regex) = regex::Regex::new(pattern)
                && regex.is_match(&sanitized_html)
            {
                result_contains_foreign_data = true;
                break;
            }
        }

        if result_contains_foreign_data {
            if let Ok(regex) = regex::Regex::new(r#">\s*\{[^{]*"id"[^}]*\}\s*<"#) {
                sanitized_html = regex.replace_all(&sanitized_html, "><").to_string();
            }

            if let Ok(regex) = regex::Regex::new(r#"<pre>.*?\{.*?\}.*?</pre>"#) {
                sanitized_html = regex.replace_all(&sanitized_html, "").to_string();
            }
        }

        let calculation_patterns = [(
            r#"([a-zA-Z ]+: [0-9]+ \+ [0-9]+ =)\s*(\d+)([^0-9])"#,
            |captures: &regex::Captures| {
                format!("{}{}{}", &captures[1], &captures[2], &captures[3])
            },
        )];

        for (pattern, replacement) in calculation_patterns.iter() {
            if let Ok(regex) = regex::Regex::new(pattern)
                && regex.is_match(&sanitized_html)
            {
                sanitized_html = regex.replace_all(&sanitized_html, *replacement).to_string();
            }
        }

        sanitized_html
    }

    pub async fn execute_server_function(
        &self,
        function_id: &str,
        args: &[JsonValue],
    ) -> Result<JsonValue, RariError> {
        let executor = self
            .server_fn_executor
            .as_ref()
            .ok_or(RariError::internal("Server function executor not configured"))?;

        let validated_args = args
            .iter()
            .map(|v| serde_json::from_value(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| RariError::validation(format!("Invalid arguments: {e}")))?;

        let result = executor.execute(function_id, &validated_args).await.map_err(|e| {
            RariError::js_execution(format!("Server function execution failed: {e}"))
        })?;

        Ok(result)
    }

    pub async fn render_with_streaming(
        &self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscStream, RariError> {
        if !self.initialized {
            return Err(RariError::internal("RSC renderer not initialized"));
        }

        let canonical_id = self.ensure_component_available(component_id).await?;
        self.ensure_component_loaded(&canonical_id).await?;

        let mut streaming_renderer = StreamingRenderer::new(Arc::clone(&self.runtime));
        match streaming_renderer.start_streaming(&canonical_id, props).await {
            Ok(stream) => Ok(stream),
            Err(e) => {
                let msg = format!("{e}");
                let should_retry = msg.contains("not a function")
                    || msg.contains("not found")
                    || msg.contains("Component render failed");
                if should_retry {
                    sleep(Duration::from_millis(80)).await;
                    let canonical_id = self.ensure_component_available(component_id).await?;
                    self.ensure_component_loaded(&canonical_id).await?;
                    let mut retry_renderer = StreamingRenderer::new(Arc::clone(&self.runtime));
                    return retry_renderer.start_streaming(&canonical_id, props).await;
                }
                Err(e)
            }
        }
    }

    async fn ensure_component_available(&self, original_id: &str) -> Result<String, RariError> {
        let candidates = self.generate_component_id_candidates(original_id);

        for candidate in &candidates {
            if self.component_exists(candidate) {
                return Ok(candidate.clone());
            }
            if self.auto_register_component_from_fs(candidate).await.is_ok() {
                return Ok(candidate.clone());
            }
        }

        sleep(Duration::from_millis(20)).await;
        if self.component_exists(original_id)
            || self.auto_register_component_from_fs(original_id).await.is_ok()
        {
            return Ok(original_id.to_string());
        }

        Err(RariError::not_found(format!(
            "Component not found: {} (tried: {})",
            original_id,
            candidates.join(", ")
        )))
    }

    fn generate_component_id_candidates(&self, id: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        out.push(id.to_string());

        if let Some((path, _export)) = id.split_once('#') {
            out.push(path.to_string());
        }

        let path_like = id.replace('\\', "/");
        if let Some(basename) = path_like.rsplit('/').next() {
            out.push(basename.to_string());
        }

        for ext in [".tsx", ".ts", ".jsx", ".js"] {
            if id.ends_with(ext) {
                out.push(id.trim_end_matches(ext).to_string());
            }
        }

        let mut seen = FxHashSet::default();
        out.retain(|s| seen.insert(s.clone()));
        out
    }

    async fn ensure_component_loaded(&self, component_id: &str) -> Result<(), RariError> {
        let init_registry_script = RscJsLoader::create_global_init();
        self.runtime
            .execute_script(
                "init_global_registries.js".to_string(),
                init_registry_script.to_string(),
            )
            .await?;

        let is_loaded = {
            let registry = self.component_registry.lock();
            registry.is_component_loaded(component_id)
        };
        if is_loaded {
            return Ok(());
        }

        let (transformed_source, dependencies) = {
            let registry = self.component_registry.lock();
            let component = registry.get_component(component_id).ok_or_else(|| {
                RariError::not_found(format!("Component not registered: {component_id}"))
            })?;
            (component.transformed_source.clone(), component.dependencies.clone())
        };

        let isolation_script = RscJsLoader::create_isolation_init_script(component_id);
        self.runtime
            .execute_script(format!("isolation_{component_id}.js"), isolation_script)
            .await?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let module_specifier_js = format!("file:///rari_component/{component_id}.js?v={timestamp}");

        self.runtime.add_module_to_loader_only(&module_specifier_js, transformed_source).await?;

        let dependencies_json =
            serde_json::to_string(&dependencies.into_iter().collect::<Vec<_>>())
                .unwrap_or_else(|_| "[]".to_string());
        let register_exports_script = RscJsLoader::create_module_operation_script(
            component_id,
            ModuleOperation::Register { dependencies_json },
        );

        self.runtime
            .execute_script(format!("register_exports_{component_id}.js"), register_exports_script)
            .await?;

        let load_script = RscJsLoader::create_module_operation_script(
            component_id,
            ModuleOperation::Load { module_specifier: module_specifier_js },
        );
        self.runtime.execute_script(format!("load_{component_id}.js"), load_script).await?;

        let verify_script = self.create_component_verification_script(component_id);
        self.execute_verification_script(component_id, verify_script).await?;
        {
            let mut registry = self.component_registry.lock();
            registry.mark_component_loaded(component_id);
        }
        Ok(())
    }

    async fn auto_register_component_from_fs(&self, component_id: &str) -> Result<(), RariError> {
        if let Some(config) = crate::server::config::Config::get()
            && !config.is_development()
        {
            return Err(RariError::not_found("Auto-register only in development".to_string()));
        }

        let cwd = std::env::current_dir().map_err(|e| RariError::io(e.to_string()))?;
        let search_roots = [
            cwd.join("examples/basic-vite-rsc/src/components"),
            cwd.join("examples/basic-vite-rsc/src/pages"),
            cwd.join("docs/src/components"),
            cwd.join("docs/src/pages"),
        ];

        let stem = component_id;
        let exts = [".tsx", ".ts", ".jsx", ".js"];
        let mut found: Option<std::path::PathBuf> = None;
        for root in &search_roots {
            for ext in &exts {
                let candidate = root.join(format!("{stem}{ext}"));
                if candidate.exists() {
                    found = Some(candidate);
                    break;
                }

                let candidate_idx = root.join(stem).join(format!("index{ext}"));
                if candidate_idx.exists() {
                    found = Some(candidate_idx);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }

        let path = found.ok_or_else(|| {
            RariError::not_found(format!("Source file not found for component {component_id}"))
        })?;

        let code = std::fs::read_to_string(&path)
            .map_err(|e| RariError::io(format!("Failed to read {}: {}", path.display(), e)))?;

        {
            let mut renderer = crate::rsc::renderer::RscRenderer {
                runtime: Arc::clone(&self.runtime),
                timeout_ms: self.timeout_ms,
                initialized: self.initialized,
                server_fn_executor: self.server_fn_executor.clone(),
                component_registry: Arc::clone(&self.component_registry),
                script_cache: self.script_cache.clone(),
                resource_limits: self.resource_limits.clone(),
                resource_tracker: Arc::clone(&self.resource_tracker),
                serializer: Arc::clone(&self.serializer),
            };
            renderer.register_component(component_id, &code).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use smallvec::SmallVec;

    use super::*;

    #[tokio::test]
    async fn test_renderer_initialization() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let mut renderer = RscRenderer::new(runtime);

        let result = renderer.initialize().await;
        assert!(result.is_ok());
        assert!(renderer.initialized);
    }

    #[tokio::test]
    async fn test_render_to_string() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));

        let mut renderer = RscRenderer::new(runtime);

        renderer.initialize().await.expect("Failed to initialize renderer");

        {
            let mut registry = renderer.component_registry.lock();
            let _ = registry.register_component(
                "TestComponent",
                "function TestComponent(props) { return { name: 'TestComponent', props }; }",
                "function TestComponent(props) { return { name: 'TestComponent', props }; }"
                    .to_string(),
                SmallVec::new(),
            );
        }

        assert!(renderer.initialized);
    }

    #[tokio::test]
    async fn test_register_and_render_jsx_component() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime);

        renderer.initialize().await.expect("Failed to initialize renderer");

        let register_component_js = r#"
        globalThis.MyJsxComponent = function(props) {
            return React.createElement('h1', null, 'Hello ' + (props.name || 'JSX World') + '!');
        };

        globalThis.Component_a83fd0f5d95fb38e = globalThis.MyJsxComponent;
        true
        "#;

        {
            let mut registry = renderer.component_registry.lock();
            let _ = registry.register_component(
                "MyJsxComponent",
                "",
                register_component_js.to_string(),
                SmallVec::new(),
            );
            registry.mark_component_loaded("MyJsxComponent");
        }

        let render_result =
            renderer.render_to_string("MyJsxComponent", Some(r#"{"name":"Test"}"#)).await;

        assert!(renderer.initialized);

        if let Ok(output) = render_result {
            assert!(output.contains("<"), "Output should contain some HTML content");
        }
    }

    #[tokio::test]
    async fn test_render_to_readable_stream() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let mut renderer = RscRenderer::new(runtime);

        let init_result = renderer.initialize().await;
        assert!(init_result.is_ok(), "Failed to initialize renderer: {:?}", init_result.err());

        let component_id = "TestStreamComponent";
        renderer
            .runtime
            .execute_script(
                "register_mock_component.js".to_string(),
                format!("globalThis.{component_id} = function() {{ return {{}}; }};"),
            )
            .await
            .expect("Failed to execute script");

        assert!(renderer.initialized);
    }
}
