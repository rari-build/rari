use dashmap::DashMap;
use parking_lot::Mutex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};
use tracing::{debug, error};

use crate::error::RariError;
use crate::rsc::components::ComponentRegistry;
use crate::rsc::rendering::core::loader::{RscJsLoader, RscModuleOperation};
use crate::rsc::rendering::streaming::{RscStream, StreamingRenderer};
use crate::rsc::utils::dependency_utils::{extract_dependencies, hash_string};
use crate::rsc::wire_format::serializer::RscSerializer;
use crate::runtime::JsExecutionRuntime;

use super::constants::*;
use super::types::{ResourceLimits, ResourceMetrics, ResourceTracker};
use super::utils::transform_imports_for_hmr;

pub struct RscRenderer {
    pub(crate) runtime: Arc<JsExecutionRuntime>,
    pub(crate) timeout_ms: u64,
    pub(crate) initialized: bool,
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

        let final_script = format!("{combined_script}\n\n{BATCH_ERROR_COLLECTION_SCRIPT}");

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

        let _extension_check_result = self
            .runtime
            .execute_script("extension-checks".to_string(), EXTENSION_CHECKS_SCRIPT.to_string())
            .await?;

        self.runtime
            .execute_script(
                "init_react_globals".to_string(),
                REACT_GLOBALS_SETUP_SCRIPT.to_string(),
            )
            .await?;

        let html_render_script = include_str!("../layout/js/html_render.js");
        self.runtime
            .execute_script("html_render".to_string(), html_render_script.to_string())
            .await?;

        let route_renderer_script = include_str!("../layout/js/route_renderer.js");
        self.runtime
            .execute_script("route_renderer".to_string(), route_renderer_script.to_string())
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
            let dep_owned = dep.clone();
            if let Err(e) = self.register_dependency_if_needed(dep_owned).await {
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
            registry.mark_component_not_loaded(component_id);
        }

        self.runtime.clear_module_loader_caches(component_id).await?;

        let force_v8_cache_clear_script =
            V8_CACHE_CLEAR_SCRIPT.replace("{component_id}", component_id);

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
        let has_client_directive =
            content.contains("'use client'") || content.contains("\"use client\"");
        let has_component_export = content.contains("export default function")
            || content.contains("export default async function");

        has_jsx || has_client_directive || (has_react_import && has_component_export)
    }

    async fn register_dependency_if_needed(&mut self, dep: String) -> Result<(), RariError> {
        let mut stack: Vec<String> = vec![dep];
        let mut visited: FxHashSet<String> = FxHashSet::default();

        let base_path = std::env::current_dir().unwrap_or_default();
        let src_dir = base_path.join("src");
        let extensions = [".ts", ".tsx", ".js", ".jsx"];

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if !current.starts_with("./") && !current.starts_with("../") {
                continue;
            }

            let clean_dep = current.trim_start_matches("./").trim_start_matches("../");

            let mut resolved_path_candidates: Vec<std::path::PathBuf> = Vec::new();
            if current.starts_with("../") {
                let up_count = current.matches("../").count();
                let remaining_path = current.replacen("../", "", up_count);
                if up_count == 1 {
                    resolved_path_candidates.push(src_dir.join(&remaining_path));
                } else if up_count == 2 {
                    resolved_path_candidates.push(base_path.join(&remaining_path));
                }
            } else if current.starts_with("./") {
                resolved_path_candidates.push(src_dir.join("components").join(clean_dep));
                resolved_path_candidates.push(src_dir.join(clean_dep));
            }

            let mut potential_paths: Vec<std::path::PathBuf> = Vec::new();
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
                                for sub_dep in sub_dependencies {
                                    stack.push(sub_dep);
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
        }

        Ok(())
    }

    async fn register_component_without_loading(
        &mut self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        let transformed_module_code = component_code.to_string();

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
            RscModuleOperation::Register { dependencies_json },
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
                RscModuleOperation::Register { dependencies_json },
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
                RscModuleOperation::Load { module_specifier: module_specifier_js },
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
        let registry = self.component_registry.lock();
        let is_client_ref = registry.is_client_reference(component_id);
        if is_client_ref {
            return true;
        }

        let serializer = self.serializer.lock();

        serializer.is_client_component_registered(component_id)
    }

    pub fn register_client_component(
        &self,
        component_id: &str,
        file_path: &str,
        export_name: &str,
    ) {
        let mut registry = self.component_registry.lock();
        registry.register_client_reference(component_id, file_path, export_name);

        let mut serializer = self.serializer.lock();
        serializer.register_client_component(component_id, file_path, export_name);
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
        self.render_to_rsc_format_with_context(component_id, props, None).await
    }

    pub async fn render_to_rsc_format_with_context(
        &mut self,
        component_id: &str,
        props: Option<&str>,
        request_context: Option<Arc<crate::server::middleware::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        self.resource_tracker.increment_active_renders();
        let result =
            self.internal_render_to_rsc_with_context(component_id, props, request_context).await;
        self.resource_tracker.decrement_active_renders();
        result
    }

    pub async fn render_to_string(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<String, RariError> {
        self.render_to_string_with_context(component_id, props, None).await
    }

    pub async fn render_to_string_with_context(
        &mut self,
        component_id: &str,
        props: Option<&str>,
        request_context: Option<Arc<crate::server::middleware::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        self.resource_tracker.increment_active_renders();
        let result =
            self.internal_render_to_string_with_context(component_id, props, request_context).await;
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

        if self.is_client_reference(component_id).await {
            return self.handle_client_reference(component_id, props).await;
        }

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

        let _render_result = self
            .execute_script_with_timeout(format!("render_{component_id}.js"), render_script)
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

    async fn internal_render_to_rsc_with_context(
        &mut self,
        component_id: &str,
        props: Option<&str>,
        _request_context: Option<Arc<crate::server::middleware::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        self.internal_render_to_rsc(component_id, props).await
    }

    async fn internal_render_to_string_with_context(
        &mut self,
        component_id: &str,
        props: Option<&str>,
        _request_context: Option<Arc<crate::server::middleware::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        self.internal_render_to_string(component_id, props).await
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

        if self.is_client_reference(component_id).await {
            return self.handle_client_reference(component_id, props).await;
        }

        let is_app_router_component = component_id.starts_with("app/");

        if !is_app_router_component {
            let component_found = self.component_exists(component_id);
            if !component_found {
                return Err(RariError::not_found(format!("Component not found: {component_id}")));
            }
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
                let script = PROMISE_MANAGER_CHECK_SCRIPT.to_string();
                self.cache_script(cache_key, script.clone());
                script
            }
        };

        let server_function_resolver_script = {
            let cache_key = "server_function_resolver".to_string();
            if let Some(cached) = self.get_cached_script(&cache_key) {
                cached
            } else {
                let script = SERVER_FUNCTION_RESOLVER_SCRIPT.to_string();
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

        let resolve_server_functions_script =
            RESOLVE_SERVER_FUNCTIONS_SCRIPT.replace("{component_id}", component_id);

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
        let props_map = if let Some(props_str) = props {
            if !props_str.trim().is_empty() {
                serde_json::from_str::<FxHashMap<String, JsonValue>>(props_str).ok()
            } else {
                None
            }
        } else {
            None
        };

        let client_element =
            crate::rsc::wire_format::serializer::SerializedReactElement::create_client_component(
                component_id,
                props_map,
            );

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
        export_name: &str,
        args: &[JsonValue],
    ) -> Result<JsonValue, RariError> {
        debug!("Executing server function: {}::{}", function_id, export_name);

        let args_json = serde_json::to_string(args)
            .map_err(|e| RariError::serialization(format!("Failed to serialize args: {}", e)))?;

        let script = SERVER_ACTION_INVOCATION_SCRIPT
            .replace("{function_name}", export_name)
            .replace("{args_json}", &args_json);

        self.runtime
            .execute_script(
                format!("execute_action_{}_{}.js", function_id.replace('/', "_"), export_name),
                script,
            )
            .await
            .map_err(|e| {
                RariError::js_execution(format!("Server function execution failed: {}", e))
            })
    }

    pub async fn render_with_streaming(
        &self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscStream, RariError> {
        self.render_with_streaming_and_context(component_id, props, None).await
    }

    pub async fn render_with_streaming_and_context(
        &self,
        component_id: &str,
        props: Option<&str>,
        _request_context: Option<Arc<crate::server::middleware::request_context::RequestContext>>,
    ) -> Result<RscStream, RariError> {
        if !self.initialized {
            return Err(RariError::internal("RSC renderer not initialized"));
        }

        let max_retries = MAX_RETRIES;
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < max_retries {
            attempt += 1;

            let canonical_id = match self.ensure_component_available(component_id).await {
                Ok(id) => id,
                Err(e) => {
                    if attempt >= max_retries {
                        return Err(e);
                    }
                    sleep(Duration::from_millis(RETRY_BASE_DELAY_MS * attempt)).await;
                    continue;
                }
            };

            if let Err(e) = self.ensure_component_loaded(&canonical_id).await {
                if attempt >= max_retries {
                    return Err(e);
                }
                sleep(Duration::from_millis(RETRY_BASE_DELAY_MS * attempt)).await;
                continue;
            }

            let mut streaming_renderer = StreamingRenderer::new(Arc::clone(&self.runtime));
            match streaming_renderer.start_streaming(&canonical_id, props).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    let msg = format!("{e}");
                    let should_retry = msg.contains("not a function")
                        || msg.contains("not found")
                        || msg.contains("Component render failed");

                    if should_retry && attempt < max_retries {
                        last_error = Some(e);
                        sleep(Duration::from_millis(RETRY_BASE_DELAY_MS * attempt)).await;
                        continue;
                    }

                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            RariError::internal(format!(
                "Failed to render component {} after {} attempts with unknown error",
                component_id, max_retries
            ))
        }))
    }

    async fn ensure_component_available(&self, original_id: &str) -> Result<String, RariError> {
        let candidates = self.generate_component_id_candidates(original_id);

        for candidate in &candidates {
            if self.component_exists(candidate) {
                return Ok(candidate.clone());
            }
        }

        sleep(Duration::from_millis(COMPONENT_AVAILABILITY_CHECK_DELAY_MS)).await;
        if self.component_exists(original_id) {
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

    pub async fn ensure_component_loaded(&self, component_id: &str) -> Result<(), RariError> {
        self.ensure_component_loaded_with_force(component_id, false).await
    }

    pub async fn ensure_component_loaded_with_force(
        &self,
        component_id: &str,
        force_reload: bool,
    ) -> Result<(), RariError> {
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
        if is_loaded && !force_reload {
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

        let module_specifier_js = if force_reload {
            format!("file:///rari_component/{component_id}.js")
        } else {
            format!("file:///rari_component/{component_id}.js?v={timestamp}")
        };

        self.runtime
            .add_module_to_loader_only(&module_specifier_js, transformed_source.clone())
            .await?;

        let needs_initial_load = !force_reload;

        if needs_initial_load {
            let module_id = self.runtime.load_es_module(component_id).await.map_err(|e| {
                RariError::js_execution(format!(
                    "Failed to load ES module for component '{}' (specifier: '{}'): {}",
                    component_id, module_specifier_js, e
                ))
            })?;
            self.runtime.evaluate_module(module_id).await.map_err(|e| {
                RariError::js_execution(format!(
                    "Failed to evaluate ES module '{}': {}",
                    module_specifier_js, e
                ))
            })?;
            let module_namespace =
                self.runtime.get_module_namespace(module_id).await.map_err(|e| {
                    RariError::js_execution(format!(
                        "Failed to get module namespace for component '{}' (module_id: {}): {}",
                        component_id, module_id, e
                    ))
                })?;

            let module_namespace_json =
                serde_json::to_string(&module_namespace).unwrap_or_else(|_| "null".to_string());
            let register_from_namespace_script = MODULE_REGISTRATION_SCRIPT
                .replace("{module_namespace}", &module_namespace_json)
                .replace("{component_id}", component_id);

            self.runtime
                .execute_script(
                    format!("load_from_namespace_{component_id}.js"),
                    register_from_namespace_script,
                )
                .await?;

            self.component_registry.lock().mark_component_initially_loaded(component_id);
        } else {
            // HMR reload: Skip V8 ES module system entirely to avoid "Module already evaluated" crashes
        }

        let dependencies_json =
            serde_json::to_string(&dependencies.into_iter().collect::<Vec<_>>())
                .unwrap_or_else(|_| "[]".to_string());
        let register_exports_script = RscJsLoader::create_module_operation_script(
            component_id,
            RscModuleOperation::Register { dependencies_json },
        );

        self.runtime
            .execute_script(format!("register_exports_{component_id}.js"), register_exports_script)
            .await?;

        if force_reload {
            self.runtime
                .execute_script(
                    format!("setup_jsx_{component_id}.js"),
                    JSX_RUNTIME_SETUP_SCRIPT.to_string(),
                )
                .await?;

            let mut transformed_source_safe = transformed_source.clone();

            if transformed_source_safe.contains("export default async function") {
                transformed_source_safe = transformed_source_safe
                    .replace("export default async function", "async function");
            } else if transformed_source_safe.contains("export default function") {
                transformed_source_safe =
                    transformed_source_safe.replace("export default function", "function");
            } else {
                transformed_source_safe = transformed_source_safe.replace("export default ", "");
            }

            transformed_source_safe = transformed_source_safe
                .replace(&format!("export const ~rari_main_export = {component_id};"), "")
                .replace(
                    "export function ~rari_register() { /* Compatibility stub */ return true; }",
                    "",
                )
                .replace("export const __registry_proxy =", "const __registry_proxy =")
                .replace("const __registry_proxy =", "")
                .replace("export const metadata =", "const metadata =")
                .replace("export const ", "const ")
                .replace("export function ", "function ")
                .replace("export async function ", "async function ")
                .replace("export {", "// export {")
                .replace("export *", "// export *");

            transformed_source_safe.push_str(REGISTRY_PROXY_SETUP_SCRIPT);

            transformed_source_safe =
                transformed_source_safe.replace("\"use module\";", "").replace("'use module';", "");

            let mut eval_safe_source = COMPONENT_EVAL_SETUP_SCRIPT.to_string();

            let import_transformed_source = transform_imports_for_hmr(&transformed_source_safe);
            eval_safe_source.push_str(&import_transformed_source);

            eval_safe_source.push_str(&format!(
                r#"

globalThis.{component_id} = {component_id};
if (!globalThis['~rsc']) globalThis['~rsc'] = {{}};
globalThis['~rsc'].functions = globalThis['~rsc'].functions || {{}};
globalThis['~rsc'].functions['{component_id}'] = {component_id};
"#,
                component_id = component_id
            ));

            let execution_result = self
                .runtime
                .execute_script(format!("direct_execution_{component_id}.js"), eval_safe_source)
                .await;

            if let Err(e) = execution_result {
                error!(
                    "HMR wrapper script execution failed for component '{}': {:?}",
                    component_id, e
                );
                return Err(e);
            }
        }

        let post_register_script = RscJsLoader::create_module_operation_script(
            component_id,
            RscModuleOperation::PostRegister,
        );
        let _post_register_result = self
            .runtime
            .execute_script(format!("post_register_{component_id}.js"), post_register_script)
            .await?;

        let verify_script = self.create_component_verification_script(component_id);
        self.execute_verification_script(component_id, verify_script).await?;
        {
            let mut registry = self.component_registry.lock();
            registry.mark_component_loaded(component_id);
        }
        Ok(())
    }
}
