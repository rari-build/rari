#![allow(clippy::unused_async_trait_impl)]
#![expect(clippy::too_many_lines)]

use std::{
    env,
    fmt::Write,
    future,
    path::{Path, PathBuf},
    string::ToString,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cow_utils::CowUtils;
use dashmap::DashMap;
use parking_lot::Mutex;
use rari_error::RariError;
use rari_rsc::{
    components::ComponentRegistry,
    utils::{self, extract_dependencies, hash_string},
};
use rustc_hash::FxHashSet;
use serde_json::Value;
use tokio::{fs, time, time::timeout};
use tracing::error;

use super::{
    constants::{
        BATCH_ERROR_COLLECTION, CACHE_CLEANUP_INTERVAL, EXTENSION_CHECKS,
        SERVER_ACTION_INVOCATION_SCRIPT, SERVER_FUNCTION_RESOLVER, V8_CACHE_CLEAR_SCRIPT,
        module_registration_script, resolve_server_functions_for_component,
    },
    types::{ResourceLimits, ResourceMetrics, ResourceTracker},
    utils::transform_imports_for_hmr,
};
use crate::{
    rendering::base::loader::{RscJsLoader, RscModuleOperation},
    runtime::JsExecutionRuntime,
    server::middleware::request_context::RequestContext,
    utils::cast,
};

pub struct RscRenderer {
    pub(crate) runtime: Arc<JsExecutionRuntime>,
    pub(crate) timeout_ms: u64,
    pub(crate) initialized: bool,
    pub(crate) component_registry: Arc<Mutex<ComponentRegistry>>,
    pub(crate) script_cache: DashMap<String, String>,
    pub(crate) resource_limits: ResourceLimits,
    pub(crate) resource_tracker: Arc<ResourceTracker>,
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
            time::sleep(CACHE_CLEANUP_INTERVAL).await;
        }

        self.clear_script_cache();

        Ok(())
    }

    pub fn is_under_memory_pressure(&self) -> bool {
        let metrics = self.get_resource_metrics();
        let current_renders = metrics.active_renders;
        let max_renders = self.resource_limits.max_concurrent_renders;

        current_renders * 10 > max_renders * 8 || metrics.memory_pressure_events > 0
    }

    pub fn force_cleanup(&self) -> impl Future<Output = Result<(), RariError>> {
        self.clear_script_cache();
        self.resource_tracker.memory_pressure_events.store(0, Ordering::Relaxed);
        future::ready(Ok(()))
    }

    #[must_use]
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
    ) -> Result<Value, RariError> {
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

    fn create_batch_script_section(index: usize, name: &str, script: &str) -> String {
        format!(
            r#"
            // === Batch Script {}: {} ===
            try {{
                {}
            }} catch (batchError_{}) {{
                if (!globalThis['~errors']) globalThis['~errors'] = {{}};
                if (!globalThis['~errors'].batch) globalThis['~errors'].batch = [];
globalThis['~errors'].batch.push({{
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
    ) -> Result<Value, RariError> {
        if scripts.is_empty() {
            return Ok(serde_json::json!({}));
        }

        let batch_sections: Vec<String> = scripts
            .iter()
            .enumerate()
            .map(|(i, (name, script))| Self::create_batch_script_section(i, name, script))
            .collect();

        let combined_script = batch_sections.join("\n");

        let final_script = format!("{combined_script}\n\n{BATCH_ERROR_COLLECTION}");

        let batch_name = format!("batch_execution_{}", scripts.len());
        let result = self.execute_script_with_timeout(batch_name, final_script).await?;

        Self::handle_batch_script_result(result, scripts.len())
    }

    fn handle_batch_script_result(result: Value, _script_count: usize) -> Result<Value, RariError> {
        if let Some(success) = result.get("success").and_then(serde_json::Value::as_bool)
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

        self.runtime
            .execute_script(
                "init_rsc_namespace".to_string(),
                r"(function() {
                    if (!globalThis['~rsc']) globalThis['~rsc'] = {};
                    if (!globalThis['~rsc'].componentNamespaces) globalThis['~rsc'].componentNamespaces = new Map();
                    if (!globalThis['~rsc'].modules) globalThis['~rsc'].modules = {};
                    if (!globalThis['~rsc'].functions) globalThis['~rsc'].functions = {};
                })()".to_string(),
            )
            .await?;

        self.runtime
            .execute_script("extension-checks".to_string(), EXTENSION_CHECKS.to_string())
            .await?;

        let setup_fizz_script = r"
            (async function() {
                try {
                    const [react, reactDomServer, flightClient, flightServer] = await Promise.all([
                        import('file:///react_vendor/react.js'),
                        import('file:///react_vendor/react-dom-server.js'),
                        import('file:///react_vendor/react-server-dom-webpack-client.js'),
                        import('file:///react_vendor/react-server-dom-webpack-server.js'),
                    ]);
                    globalThis.React = react.default && react.default.createElement ? react.default : react;
                    globalThis['~reactServer'] = reactDomServer;
                    globalThis['~flightClient'] = flightClient;
                    globalThis['~reactServerRenderer'] = flightServer;
                    return {
                        success: !!(globalThis.React.createElement
                            && globalThis['~reactServer'].renderToReadableStream
                            && globalThis['~flightClient'].createFromReadableStream
                            && globalThis['~reactServerRenderer'].renderToReadableStream),
                    };
                } catch (e) {
                    console.warn('[rari] Could not load React server modules:', e?.message || e);
                    return { success: false, error: String(e?.message || e) };
                }
            })()
        ";

        match self
            .runtime
            .execute_script("setup_react_server".to_string(), setup_fizz_script.to_string())
            .await
        {
            Ok(result) => {
                let success =
                    result.get("success").and_then(serde_json::Value::as_bool).unwrap_or(false);
                if success {
                    let fizz_render_script = include_str!("../layout/js/fizz_render.ts");
                    if let Err(e) = self
                        .runtime
                        .execute_script(
                            "fizz_render.ts".to_string(),
                            fizz_render_script.to_string(),
                        )
                        .await
                    {
                        tracing::warn!("Failed to initialize Fizz renderer: {e}");
                    }

                    let flight_render_script = include_str!("../layout/js/flight_render.ts");
                    if let Err(e) = self
                        .runtime
                        .execute_script(
                            "flight_render.ts".to_string(),
                            flight_render_script.to_string(),
                        )
                        .await
                    {
                        tracing::warn!("Failed to initialize Flight renderer: {e}");
                    }
                } else {
                    let err = result.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
                    tracing::warn!("React Fizz module load returned failure: {err}");
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load React Fizz renderer, falling back to custom HTML renderer: {e}"
                );
            }
        }

        self.initialized = true;

        Ok(())
    }

    pub async fn register_component(
        &mut self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        let isolation_namespacing_script =
            RscJsLoader::create_isolation_namespacing_script(component_id);

        self.runtime
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
                    "[rari] RSC: Failed to register dependency '{dep}' for component '{component_id}': {e}"
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
            V8_CACHE_CLEAR_SCRIPT.cow_replace("{component_id}", component_id).into_owned();

        self.runtime
            .execute_script(
                format!("force_v8_cache_clear_{component_id}.ts"),
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

    async fn register_dependency_if_needed(&self, dep: String) -> Result<(), RariError> {
        let mut stack: Vec<String> = vec![dep];
        let mut visited: FxHashSet<String> = FxHashSet::default();

        let base_path = env::current_dir().unwrap_or_default();
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

            let mut resolved_path_candidates: Vec<PathBuf> = Vec::new();
            if current.starts_with("../") {
                let up_count = current.matches("../").count();
                let remaining_path = current.cow_replacen("../", "", up_count).into_owned();
                if up_count == 1 {
                    resolved_path_candidates.push(src_dir.join(&remaining_path));
                } else if up_count == 2 {
                    resolved_path_candidates.push(base_path.join(&remaining_path));
                }
            } else if current.starts_with("./") {
                resolved_path_candidates.push(src_dir.join("components").join(clean_dep));
                resolved_path_candidates.push(src_dir.join(clean_dep));
            }

            let mut potential_paths: Vec<PathBuf> = Vec::new();
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
                    if let Ok(content) = fs::read_to_string(potential_path).await {
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

                        if !already_registered && Self::is_react_component_file(&content) {
                            let sub_dependencies = extract_dependencies(&content);
                            for sub_dep in sub_dependencies {
                                stack.push(sub_dep);
                            }
                            self.register_component_without_loading(&unique_dep_id, &content)
                                .await?;
                        }
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    async fn register_component_without_loading(
        &self,
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

        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let module_specifier = format!("file:///rari_component/{component_id}.js?v={timestamp}");

        if let Err(e) = self
            .runtime
            .add_module_to_loader(&module_specifier, transformed_module_code.clone())
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

    async fn load_all_components(&self) -> Result<(), RariError> {
        let components_to_load = {
            let registry = self.component_registry.lock();
            registry.get_unloaded_components_in_order()
        };

        if components_to_load.is_empty() {
            return Ok(());
        }

        for component_id in &components_to_load {
            let isolation_script = RscJsLoader::create_isolation_init_script(component_id);

            self.runtime
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

            self.runtime.add_module_to_loader(&module_specifier_js, transformed_source).await?;

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
                    let verify_script = Self::create_component_verification_script(component_id);
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

    fn create_component_verification_script(component_id: &str) -> String {
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

        let success = result.get("success").and_then(Value::as_bool).unwrap_or(false);

        if success {
            return Ok(());
        }

        let error_details =
            result.get("details").and_then(|v| v.as_str()).map(ToString::to_string).unwrap_or_else(
                || {
                    result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string)
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

    pub fn is_client_reference(&self, component_id: &str) -> impl Future<Output = bool> {
        let registry = self.component_registry.lock();
        future::ready(registry.is_client_reference(component_id))
    }

    pub fn register_client_component(
        &self,
        component_id: &str,
        file_path: &str,
        export_name: &str,
    ) {
        let mut registry = self.component_registry.lock();
        registry.register_client_reference(component_id, file_path, export_name);
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
        request_context: Option<Arc<RequestContext>>,
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
        request_context: Option<Arc<RequestContext>>,
    ) -> Result<String, RariError> {
        self.resource_tracker.increment_active_renders();
        let result =
            self.internal_render_to_string_with_context(component_id, props, request_context).await;
        self.resource_tracker.decrement_active_renders();
        result
    }

    async fn internal_render_to_rsc(
        &self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<String, RariError> {
        let render_start = Instant::now();

        if !self.initialized {
            return Err(RariError::internal("RSC renderer not initialized"));
        }

        if self.is_client_reference(component_id).await {
            return Self::handle_client_reference(component_id, props).await;
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

        let setup_scripts = vec![("clear_environment", clear_environment_script)];

        self.execute_batched_scripts(setup_scripts).await?;

        let render_script =
            RscJsLoader::load_component_render_with_data(component_id, &component_hash, props_json)
                .map_err(|e| {
                    RariError::js_execution(format!("Failed to load component render script: {e}"))
                })?;

        self.execute_script_with_timeout(format!("render_{component_id}.ts"), render_script)
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

        Self::process_rsc_extraction_result(component_id, &extraction_result)
    }

    fn process_rsc_extraction_result(
        component_id: &str,
        extraction_result: &Value,
    ) -> Result<String, RariError> {
        let parsed_result: Value = if let Some(obj) = extraction_result.as_object() {
            Value::Object(obj.clone())
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

        if let Some(wire_format) = rsc_data.as_str() {
            return Ok(wire_format.to_string());
        }

        Err(RariError::js_execution(format!(
            "RSC data for component '{component_id}' is not a wire format string"
        )))
    }

    async fn internal_render_to_rsc_with_context(
        &self,
        component_id: &str,
        props: Option<&str>,
        _request_context: Option<Arc<RequestContext>>,
    ) -> Result<String, RariError> {
        self.internal_render_to_rsc(component_id, props).await
    }

    async fn internal_render_to_string_with_context(
        &self,
        component_id: &str,
        props: Option<&str>,
        _request_context: Option<Arc<RequestContext>>,
    ) -> Result<String, RariError> {
        self.internal_render_to_string(component_id, props).await
    }

    async fn internal_render_to_string(
        &self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<String, RariError> {
        let render_start = Instant::now();

        self.resource_tracker.total_renders.fetch_add(1, Ordering::Relaxed);

        if !self.initialized {
            return Err(RariError::internal("RSC renderer not initialized"));
        }

        if self.is_client_reference(component_id).await {
            return Self::handle_client_reference(component_id, props).await;
        }

        let is_app_router_component = component_id.starts_with("app/");

        if !is_app_router_component {
            let component_found = self.component_exists(component_id);
            if !component_found {
                return Err(RariError::not_found(format!("Component not found: {component_id}")));
            }
        }

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

        let server_function_resolver_script = {
            let cache_key = "server_function_resolver".to_string();
            if let Some(cached) = self.get_cached_script(&cache_key) {
                cached
            } else {
                let script = SERVER_FUNCTION_RESOLVER.to_string();
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
            ("server_function_resolver", server_function_resolver_script),
            ("isolation_init", isolation_init_script),
        ];

        self.execute_batched_scripts(setup_scripts).await?;

        let resolve_server_functions_script = resolve_server_functions_for_component(component_id);

        self.execute_script_with_timeout(
            format!("resolve_server_functions_{component_id}.js"),
            resolve_server_functions_script,
        )
        .await?;

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

                html = Self::sanitize_html_output(&html, component_id);

                let render_duration = render_start.elapsed();

                self.resource_tracker
                    .total_render_time_ms
                    .fetch_add(cast::duration_millis_u64(render_duration), Ordering::Relaxed);

                if html == "<div></div>" || html.trim() == "" || html == "<div/>" {
                    return Ok(format!(
                        r"<div data-component-id='{}' data-diagnostic='true'>
                            <h2>Component: {}</h2>
                            <p>This component rendered with no content.</p>
                            <p>This may indicate the component doesn't return JSX or has a rendering issue.</p>
                            <p>Server time: {}</p>
                        </div>",
                        component_id,
                        component_id,
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    ));
                }

                Ok(html)
            }
            Err(e) => Ok(format!(
                r"<div>
                        <h2>Error Rendering {}</h2>
                        <p>There was an error rendering this component: {}</p>
                        <p>Server time: {}</p>
                    </div>",
                component_id,
                e,
                SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
            )),
        }
    }

    fn handle_client_reference(
        component_id: &str,
        _props: Option<&str>,
    ) -> impl Future<Output = Result<String, RariError>> {
        future::ready(Ok(format!(
            r#"<div data-client-component="{component_id}" data-component-id="{component_id}"></div>"#,
        )))
    }

    fn sanitize_html_output(html: &str, component_id: &str) -> String {
        let mut sanitized_html = html.to_string();

        let sanitization_rules =
            [(r#"\{"id".*?\}"#, ""), (r"<pre>\{.*?\}</pre>", ""), (r#"\[(\{".*?},?)+\]"#, "[]")];

        for (pattern, replacement) in &sanitization_rules {
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

        for marker in &boundary_markers {
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

        for (pattern, _) in &leakage_indicators {
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

            if let Ok(regex) = regex::Regex::new(r"<pre>.*?\{.*?\}.*?</pre>") {
                sanitized_html = regex.replace_all(&sanitized_html, "").to_string();
            }
        }

        let calculation_patterns = [(
            r"([a-zA-Z ]+: [0-9]+ \+ [0-9]+ =)\s*(\d+)([^0-9])",
            |captures: &regex::Captures| {
                format!("{}{}{}", &captures[1], &captures[2], &captures[3])
            },
        )];

        for (pattern, replacement) in &calculation_patterns {
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
        args: &[Value],
    ) -> Result<Value, RariError> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| RariError::serialization(format!("Failed to serialize args: {e}")))?;

        let script = SERVER_ACTION_INVOCATION_SCRIPT
            .cow_replace("{function_name}", export_name)
            .cow_replace("{args_json}", &args_json)
            .into_owned();

        self.runtime
            .execute_script(
                format!("execute_action_{}_{}.ts", function_id.cow_replace('/', "_"), export_name),
                script,
            )
            .await
            .map_err(|e| RariError::js_execution(format!("Server function execution failed: {e}")))
    }

    pub async fn ensure_component_loaded(&self, component_id: &str) -> Result<(), RariError> {
        self.ensure_component_loaded_with_force(component_id, false).await
    }

    pub async fn ensure_component_loaded_with_force(
        &self,
        component_id: &str,
        force_reload: bool,
    ) -> Result<(), RariError> {
        let is_loaded = {
            let registry = self.component_registry.lock();
            registry.is_component_loaded(component_id)
        };
        if is_loaded && !force_reload {
            return Ok(());
        }

        let is_registered = {
            let registry = self.component_registry.lock();
            registry.is_component_registered(component_id)
        };

        if !is_registered {
            let component_path = component_id.strip_prefix("app/").unwrap_or(component_id);
            let dist_path = Path::new("dist/server").join(format!("{component_path}.js"));

            if dist_path.exists() {
                let component_code = fs::read_to_string(&dist_path).await.map_err(|e| {
                    RariError::io(format!(
                        "Failed to read component file {}: {}",
                        dist_path.display(),
                        e
                    ))
                })?;

                let dependencies = utils::extract_dependencies(&component_code);

                {
                    let mut registry = self.component_registry.lock();
                    registry
                        .register_component(
                            component_id,
                            &component_code,
                            component_code.clone(),
                            dependencies.into_iter().collect(),
                        )
                        .map_err(|e| {
                            RariError::internal(format!("Failed to register component: {e}"))
                        })?;
                }
            } else {
                tracing::error!("Component file not found: {}", dist_path.display());
                return Err(RariError::not_found(format!(
                    "Component not registered and file not found: {component_id}"
                )));
            }
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

        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();

        let module_specifier_js = if force_reload {
            format!("file:///rari_component/{component_id}.js")
        } else {
            format!("file:///rari_component/{component_id}.js?v={timestamp}")
        };

        self.runtime.add_module_to_loader(&module_specifier_js, transformed_source.clone()).await?;

        let needs_initial_load = !force_reload;

        if needs_initial_load {
            let module_id = self
                .runtime
                .load_es_module(component_id)
                .await
                .map_err(|e| {
                    RariError::js_execution(format!(
                        "Failed to load ES module for component '{component_id}' (specifier: '{module_specifier_js}'): {e}"
                    ))
                })?;
            self.runtime.evaluate_module(module_id).await.map_err(|e| {
                RariError::js_execution(format!(
                    "Failed to evaluate ES module '{module_specifier_js}': {e}"
                ))
            })?;
            let module_namespace =
                self.runtime
                    .get_module_namespace(module_id)
                    .await
                    .map_err(|e| {
                        RariError::js_execution(format!(
                            "Failed to get module namespace for component '{component_id}' (module_id: {module_id}): {e}"
                        ))
                    })?;

            let module_namespace_json =
                serde_json::to_string(&module_namespace).unwrap_or_else(|_| "null".to_string());
            let register_from_namespace_script =
                module_registration_script(&module_namespace_json, component_id);

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
            let mut transformed_source_safe = transformed_source.clone();

            if transformed_source_safe.contains("export default async function") {
                transformed_source_safe = transformed_source_safe
                    .cow_replace("export default async function", "async function")
                    .into_owned();
            } else if transformed_source_safe.contains("export default function") {
                transformed_source_safe = transformed_source_safe
                    .cow_replace("export default function", "function")
                    .into_owned();
            } else {
                transformed_source_safe =
                    transformed_source_safe.cow_replace("export default ", "").into_owned();
            }

            transformed_source_safe = transformed_source_safe
                .cow_replace(&format!("export const ~rari_main_export = {component_id};"), "")
                .cow_replace("export const metadata =", "const metadata =")
                .cow_replace("export const ", "const ")
                .cow_replace("export function ", "function ")
                .cow_replace("export async function ", "async function ")
                .cow_replace("export {", "// export {")
                .cow_replace("export *", "// export *")
                .into_owned();

            transformed_source_safe = transformed_source_safe
                .cow_replace("\"use module\";", "")
                .cow_replace("'use module';", "")
                .into_owned();

            let import_transformed_source = transform_imports_for_hmr(&transformed_source_safe);
            let mut eval_safe_source = import_transformed_source;

            let _ = write!(
                eval_safe_source,
                r"

globalThis.{component_id} = {component_id};
if (!globalThis['~rsc']) globalThis['~rsc'] = {{}};
globalThis['~rsc'].functions = globalThis['~rsc'].functions || {{}};
globalThis['~rsc'].functions['{component_id}'] = {component_id};
"
            );

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
        self.runtime
            .execute_script(format!("post_register_{component_id}.js"), post_register_script)
            .await?;

        let verify_script = Self::create_component_verification_script(component_id);
        self.execute_verification_script(component_id, verify_script).await?;
        {
            let mut registry = self.component_registry.lock();
            registry.mark_component_loaded(component_id);
        }
        Ok(())
    }
}
