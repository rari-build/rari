use crate::error::RariError;
use crate::rsc::utils::dependency_utils::DependencyList;
use crate::runtime::module_loader::{
    cache::ModuleCaching,
    config::{InternerStats, PerformanceStats, ResourceStats, RuntimeConfig, RuntimeMetrics},
    interner::get_string_interner,
    node_stubs::*,
    resolver::ModuleResolver,
    storage::OrderedStorage,
    transpiler::*,
};
use dashmap::DashMap;
use deno_core::{
    FastString, ModuleLoadOptions, ModuleLoadReferrer, ModuleLoadResponse, ModuleLoader,
    ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType, ResolutionKind,
};
use deno_error::JsErrorBox;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::time::Instant;

type ExtensionTranspilerResult = Result<(FastString, Option<Cow<'static, [u8]>>), JsErrorBox>;
type ExtensionTranspilerFn = dyn Fn(FastString, FastString) -> ExtensionTranspilerResult;

const NODE_MODULES_PATH: &str = "/node_modules/";
const RARI_COMPONENT_PATH: &str = "/rari_component/";
const NODE_BUILTIN_PATH: &str = "/node_builtin/";
const FILE_PROTOCOL: &str = "file://";
const NODE_PROTOCOL: &str = "node";
const NODE_PREFIX: &str = "node:";

const PATH_MODULE: &str = "path";
const PROCESS_MODULE: &str = "process";
const FS_MODULE: &str = "fs";
const URL_MODULE: &str = "url";
const REACT_MODULE: &str = "react";
const FUNCTIONS_MODULE: &str = "functions";

const VERSION_QUERY_PARAM: &str = "?v=";
const RELATIVE_CURRENT_PATH: &str = "./";
const RELATIVE_UP_PATH: &str = "../";
const RARI_INTERNAL_PATH: &str = "/rari_internal/";
const LOADER_STUB_PREFIX: &str = "load_";

#[derive(Debug)]
struct AsyncFileManager {
    file_cache: Arc<RwLock<FxHashMap<String, (String, Instant)>>>,
}

impl AsyncFileManager {
    fn new() -> Self {
        Self { file_cache: Arc::new(RwLock::new(FxHashMap::default())) }
    }
}

static IMPORT_REGEX: OnceLock<regex::Regex> = OnceLock::new();
static ASYNC_FILE_MANAGER: OnceLock<AsyncFileManager> = OnceLock::new();

fn get_import_regex() -> &'static regex::Regex {
    IMPORT_REGEX.get_or_init(|| {
        regex::Regex::new(r#"(?:import|from)\s+(['"])(.*?)(['"])"#).expect("Invalid import regex")
    })
}

fn get_async_file_manager() -> &'static AsyncFileManager {
    ASYNC_FILE_MANAGER.get_or_init(AsyncFileManager::new)
}

#[derive(Debug)]
pub struct RariModuleLoader {
    storage: OrderedStorage,
    module_resolver: ModuleResolver,
    pub module_caching: ModuleCaching,
    pub component_specifiers: DashMap<String, String>,
    total_modules_loaded: AtomicUsize,
    total_load_time_ms: AtomicU64,
    peak_load_time_ms: AtomicU64,
    operations_count: AtomicUsize,
    start_time: std::time::Instant,
}

impl RariModuleLoader {
    pub fn new() -> Self {
        Self::with_config(RuntimeConfig::default())
    }

    pub fn with_config(config: RuntimeConfig) -> Self {
        Self {
            storage: OrderedStorage::new(),
            module_resolver: ModuleResolver::new(),
            module_caching: ModuleCaching::new(config.cache_size_limit),
            component_specifiers: DashMap::new(),
            total_modules_loaded: AtomicUsize::new(0),
            total_load_time_ms: AtomicU64::new(0),
            peak_load_time_ms: AtomicU64::new(0),
            operations_count: AtomicUsize::new(0),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn get_metrics(&self) -> RuntimeMetrics {
        let cache_stats = self.module_caching.get_cache_stats();
        let (hits, misses) = get_string_interner().stats();

        let total_modules = self.total_modules_loaded.load(Ordering::Relaxed);
        let total_time_ms = self.total_load_time_ms.load(Ordering::Relaxed);
        let peak_time_ms = self.peak_load_time_ms.load(Ordering::Relaxed);
        let ops_count = self.operations_count.load(Ordering::Relaxed);
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();

        RuntimeMetrics {
            cache_stats: cache_stats.clone(),
            batch_stats: self.storage.get_batch_stats(),
            interner_stats: InternerStats {
                total_strings_interned: hits + misses,
                memory_saved_bytes: Self::calculate_memory_savings(hits, misses),
                hit_rate_percentage: if hits + misses > 0 {
                    (hits as f64 / (hits + misses) as f64) * 100.0
                } else {
                    0.0
                },
                cache_size: hits + misses,
            },
            performance_stats: PerformanceStats {
                average_module_load_time_ms: if total_modules > 0 {
                    total_time_ms as f64 / total_modules as f64
                } else {
                    0.0
                },
                peak_module_load_time_ms: peak_time_ms,
                total_modules_loaded: total_modules,
                cache_hit_rate_percentage: if cache_stats.hits + cache_stats.misses > 0 {
                    (cache_stats.hits as f64 / (cache_stats.hits + cache_stats.misses) as f64)
                        * 100.0
                } else {
                    0.0
                },
                operations_per_second: if elapsed_secs > 0.0 {
                    ops_count as f64 / elapsed_secs
                } else {
                    0.0
                },
            },
            resource_stats: ResourceStats {
                memory_usage_mb: Self::estimate_memory_usage(&cache_stats, hits + misses),
                active_threads: Self::count_active_threads(),
                pending_operations: 0,
                file_cache_size: get_async_file_manager().file_cache.read().len(),
            },
            collected_at: std::time::Instant::now(),
        }
    }

    pub fn flush_all_batches(&self) -> Result<(), RariError> {
        self.storage.flush_pending_batch()
    }

    #[cfg(test)]
    pub fn with_test_config() -> Self {
        Self::with_config(RuntimeConfig::test())
    }

    fn calculate_memory_savings(hits: usize, _misses: usize) -> usize {
        hits * 24
    }

    fn estimate_memory_usage(
        cache_stats: &crate::runtime::module_loader::config::CacheStats,
        interner_size: usize,
    ) -> usize {
        let cache_mb = cache_stats.memory_bytes / (1024 * 1024);
        let interner_mb = (interner_size * 24) / (1024 * 1024);
        cache_mb + interner_mb
    }

    fn count_active_threads() -> usize {
        4
    }

    pub fn record_module_load(&self, duration_ms: u64) {
        self.total_modules_loaded.fetch_add(1, Ordering::Relaxed);
        self.total_load_time_ms.fetch_add(duration_ms, Ordering::Relaxed);

        let current_peak = self.peak_load_time_ms.load(Ordering::Relaxed);
        if duration_ms > current_peak {
            let _ = self.peak_load_time_ms.compare_exchange_weak(
                current_peak,
                duration_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
        }
    }

    pub fn record_operation(&self) {
        self.operations_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_module(&self, specifier: &str, original_path: &str, code: String) {
        if let Err(_batch_error) = self.storage.add_module_interned(specifier, &code) {
            self.add_module_internal(specifier, original_path, code.clone());
        }

        if specifier.contains(RARI_INTERNAL_PATH) {
            let cache_key = get_string_interner().intern(specifier);
            #[allow(clippy::disallowed_methods)]
            if let Err(_e) = self.module_caching.insert(
                cache_key.to_string(),
                serde_json::json!({
                    "status": "module_added",
                    "specifier": specifier,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                }),
            ) {}
        }
    }

    fn add_module_internal(&self, specifier: &str, original_path: &str, code: String) {
        let is_update = self.storage.contains_module_code(specifier);
        let specifier_owned = specifier.to_string();

        if is_update {
            let current_version = self.storage.get_version(specifier).unwrap_or(0) + 1;
            let versioned_specifier = format!("{specifier}{VERSION_QUERY_PARAM}{current_version}");

            self.storage.set_module_code(specifier_owned.clone(), code.clone());
            self.storage.set_module_code(versioned_specifier.clone(), code.clone());

            self.storage.set_module_meta(format!("registered_{specifier_owned}"), true);
            self.storage.set_module_meta(format!("registered_{versioned_specifier}"), true);
            self.storage.set_module_meta(format!("hmr_{specifier_owned}"), true);
            self.storage.set_version(specifier_owned, current_version);
        } else {
            self.storage.set_module_code(specifier_owned.clone(), code.clone());
            self.storage.set_module_meta(format!("registered_{specifier_owned}"), true);
            self.storage.set_version(specifier_owned, 1);
        }

        let dependencies = self.register_dependencies(original_path, &code);

        if !dependencies.is_empty() {
            for dep in &dependencies {
                let module_name =
                    if dep.contains('/') { dep.split('/').next_back().unwrap_or(dep) } else { dep };

                let simplified_name = if module_name.contains('.') {
                    module_name.split('.').next().unwrap_or(module_name)
                } else {
                    module_name
                };

                let stub_specifier = format!("file://{RARI_INTERNAL_PATH}{simplified_name}.js");

                if !self.storage.contains_module_code(&stub_specifier) {
                    let stub_code = format!(
                        r#"
// Stub module for {module_name} (dependency of {original_path})

export const __isStub = true;
export const __stubFor = "{module_name}";
export const __dependencyOf = "{original_path}";

export default {{}};
"#
                    );

                    self.storage.set_module_code(stub_specifier.clone(), stub_code);
                }
            }
        }
    }

    fn register_dependencies(&self, _original_path: &str, code: &str) -> DependencyList {
        let import_regex = get_import_regex();
        let mut dependencies = DependencyList::new();

        for captures in import_regex.captures_iter(code) {
            if captures.len() >= 4
                && let Some(import_path) = captures.get(2)
            {
                let import_path_str = import_path.as_str().to_string();
                if import_path_str.contains("/") || import_path_str.contains(".") {
                    dependencies.push(import_path_str);
                }
            }
        }

        dependencies
    }

    pub fn set_module_code(&self, specifier: String, code: String) {
        self.storage.set_module_code(specifier, code);
    }

    pub fn get_component_specifier(&self, component_id: &str) -> Option<String> {
        if let Some(spec) = self.component_specifiers.get(component_id) {
            return Some(spec.value().clone());
        }

        let component_stub = format!("file://{RARI_COMPONENT_PATH}component_{component_id}.js");
        let internal_stub = format!("file://{RARI_INTERNAL_PATH}{component_id}.js");

        if self.storage.contains_module_code(&component_stub) {
            Some(component_stub)
        } else if self.storage.contains_module_code(&internal_stub) {
            Some(internal_stub)
        } else {
            None
        }
    }

    pub fn is_already_evaluated(&self, module_id: &str) -> bool {
        self.storage.get_module_meta(&format!("registered_{module_id}")).unwrap_or(false)
    }

    pub fn mark_module_evaluated(&self, module_id: &str) {
        self.storage.set_module_meta(format!("registered_{module_id}"), true);
    }

    pub fn is_hmr_module(&self, specifier: &str) -> bool {
        self.storage.get_module_meta(&format!("hmr_{specifier}")).unwrap_or(false)
    }

    pub fn get_versioned_specifier(&self, component_id: &str) -> Option<String> {
        let base_specifier = self.get_component_specifier(component_id)?;

        if let Some(version) = self.storage.get_version(&format!("version_{component_id}")) {
            Some(format!("{}{VERSION_QUERY_PARAM}{}", base_specifier, version))
        } else {
            Some(base_specifier)
        }
    }

    pub fn clear_component_caches(&self, component_id: &str) {
        let component_specifier = format!("file://{RARI_COMPONENT_PATH}{component_id}.js");

        self.module_caching.clear_component(component_id);

        let should_remove_mapping = self
            .component_specifiers
            .get(component_id)
            .map(|entry| !entry.contains("/rari_hmr/"))
            .unwrap_or(true);

        if should_remove_mapping {
            self.component_specifiers.remove(component_id);
        }

        self.storage.set_module_meta(format!("hmr_{component_specifier}"), false);
        self.storage.set_module_meta(format!("registered_{component_id}"), false);
        self.storage.set_version(format!("version_{component_id}"), 0);

        let file_cache = get_async_file_manager().file_cache.clone();
        let mut cache = file_cache.write();
        let keys_to_remove: Vec<String> =
            cache.keys().filter(|key| key.contains(component_id)).cloned().collect();
        for key in keys_to_remove {
            cache.remove(&key);
        }
    }

    pub fn create_specifier(&self, name: &str, prefix: &str) -> String {
        let clean_name = name.replace(".js", "").replace("/", "_");
        format!("file:///{prefix}/{clean_name}.js")
    }

    pub fn transform_to_esmodule(&self, code: &str, _original_path: &str) -> String {
        code.replace("'use server'", "// 'use server' directive removed")
            .replace("\"use server\"", "// \"use server\" directive removed")
    }

    pub fn as_extension_transpiler(self: &Rc<Self>) -> Rc<ExtensionTranspilerFn> {
        Rc::new(move |specifier: FastString, code: FastString| {
            match ModuleSpecifier::parse(specifier.as_str()) {
                Ok(_) => crate::runtime::utils::transpile::maybe_transpile_source(specifier, code),
                Err(e) => Err(JsErrorBox::from_err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Failed to parse module specifier '{specifier}': {e}"),
                )))),
            }
        })
    }

    fn find_package_directory(&self, current_dir: &Path, package_name: &str) -> Option<PathBuf> {
        let node_modules_path = current_dir.join("node_modules").join(package_name);
        if node_modules_path.exists() {
            return Some(node_modules_path);
        }

        let mut search_dir = current_dir.to_path_buf();
        loop {
            let node_modules_path = search_dir.join("node_modules").join(package_name);
            if node_modules_path.exists() {
                return Some(node_modules_path);
            }

            if !search_dir.pop() {
                break;
            }
        }

        None
    }

    fn resolve_from_node_modules(
        &self,
        package_specifier: &str,
        referrer_path: &str,
    ) -> Option<String> {
        let start_dir = if !referrer_path.is_empty() {
            let clean_referrer = referrer_path.strip_prefix(FILE_PROTOCOL).unwrap_or(referrer_path);

            if clean_referrer.contains("/rari_component/")
                || clean_referrer.contains("/rari_internal/")
                || clean_referrer.contains("/node_builtin/")
            {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else if let Some(last_slash) = clean_referrer.rfind('/') {
                let dir_path = PathBuf::from(&clean_referrer[..last_slash]);
                dir_path.canonicalize().unwrap_or(dir_path)
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };

        if let Some(slash_pos) = package_specifier.find('/') {
            if package_specifier.starts_with('@') {
                if let Some(second_slash_pos) = package_specifier[slash_pos + 1..].find('/') {
                    let actual_slash_pos = slash_pos + 1 + second_slash_pos;
                    let package_name = &package_specifier[..actual_slash_pos];
                    let subpath = &package_specifier[actual_slash_pos..];

                    return self.resolve_subpath_export_from_dir(package_name, subpath, &start_dir);
                }
            } else {
                let package_name = &package_specifier[..slash_pos];
                let subpath = &package_specifier[slash_pos..];

                return self.resolve_subpath_export_from_dir(package_name, subpath, &start_dir);
            }
        }

        self.resolve_regular_package_from_dir(package_specifier, &start_dir)
    }

    fn is_npm_package_context(&self, referrer: &str) -> bool {
        referrer.contains("node_modules") || self.module_resolver.contains_path(referrer)
    }

    fn extract_package_base_from_referrer(&self, referrer: &str) -> Option<String> {
        let clean_referrer = if referrer.starts_with(FILE_PROTOCOL) {
            referrer.strip_prefix(FILE_PROTOCOL).unwrap_or(referrer)
        } else {
            referrer
        };

        if clean_referrer.contains("node_modules")
            && let Some(last_slash) = clean_referrer.rfind('/')
        {
            let dir_path = &clean_referrer[..last_slash];
            return Some(dir_path.to_string());
        }

        self.module_resolver.get_package_base(clean_referrer)
    }

    fn resolve_relative_up(&self, specifier: &str, package_base: &str) -> String {
        let remaining = specifier.strip_prefix("../").unwrap_or(specifier);
        let parent_dir = if package_base.contains('/') {
            package_base.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
        } else {
            ""
        };
        format!("file://{parent_dir}/{remaining}")
    }

    fn resolve_relative_current(&self, specifier: &str, package_base: &str) -> String {
        let remaining = specifier.strip_prefix("./").unwrap_or(specifier);
        format!("file://{package_base}/{remaining}")
    }

    fn handle_cached_module(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if let Some(code) = self.storage.get_module_code(specifier_str) {
            let (final_code, module_type) = if needs_typescript_transpilation(specifier_str) {
                match crate::runtime::utils::transpile::maybe_transpile_source(
                    specifier_str.to_string().into(),
                    code.to_string().into(),
                ) {
                    Ok((transpiled_code, _source_map)) => {
                        (transpiled_code.to_string(), ModuleType::JavaScript)
                    }
                    Err(err) => {
                        return Some(ModuleLoadResponse::Sync(Err(JsErrorBox::generic(format!(
                            "Failed to transpile TypeScript module '{specifier_str}': {err}"
                        )))));
                    }
                }
            } else if needs_jsx_transpilation(specifier_str) {
                match crate::runtime::utils::transpile::maybe_transpile_source(
                    specifier_str.to_string().into(),
                    code.to_string().into(),
                ) {
                    Ok((transpiled_code, _source_map)) => {
                        (transpiled_code.to_string(), ModuleType::JavaScript)
                    }
                    Err(err) => {
                        return Some(ModuleLoadResponse::Sync(Err(JsErrorBox::generic(format!(
                            "Failed to transpile JSX module '{specifier_str}': {err}"
                        )))));
                    }
                }
            } else {
                (code, ModuleType::JavaScript)
            };

            return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                module_type,
                ModuleSourceCode::String(final_code.into()),
                module_specifier,
                None,
            ))));
        }
        None
    }

    fn handle_dynamic_import_validation(
        &self,
        specifier_str: &str,
        maybe_referrer: Option<&ModuleSpecifier>,
        is_dyn_import: bool,
    ) -> Option<ModuleLoadResponse> {
        if is_dyn_import && let Some(referrer) = maybe_referrer {
            let referrer_str = referrer.to_string();

            if referrer_str.contains(NODE_MODULES_PATH) {
                let file_path = if specifier_str.starts_with(FILE_PROTOCOL) {
                    specifier_str.strip_prefix("file://").unwrap_or(specifier_str)
                } else if specifier_str.starts_with(RELATIVE_CURRENT_PATH)
                    || specifier_str.starts_with(RELATIVE_UP_PATH)
                {
                    if let Ok(referrer_path) = referrer.to_file_path()
                        && let Some(referrer_dir) = referrer_path.parent()
                    {
                        let resolved = referrer_dir.join(specifier_str);
                        if let Ok(canonical) = resolved.canonicalize() {
                            return if !canonical.exists() {
                                Some(ModuleLoadResponse::Sync(Err(JsErrorBox::generic(
                                    "Module not found",
                                ))))
                            } else {
                                None
                            };
                        }
                    }
                    return None;
                } else {
                    return None;
                };

                if !std::path::Path::new(file_path).exists() {
                    return Some(ModuleLoadResponse::Sync(Err(JsErrorBox::generic(
                        "Module not found",
                    ))));
                }
            }
        }
        None
    }

    fn handle_version_query(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if specifier_str.contains(VERSION_QUERY_PARAM) {
            let base_specifier =
                specifier_str.split('?').next().unwrap_or(specifier_str).to_string();

            if let Some(code) = self.storage.get_module_code(specifier_str) {
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(code.into()),
                    module_specifier,
                    None,
                ))));
            } else if let Some(code) = self.storage.get_module_code(&base_specifier) {
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(code.into()),
                    module_specifier,
                    None,
                ))));
            }
        }
        None
    }

    fn handle_node_builtin_modules(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if specifier_str.contains(NODE_BUILTIN_PATH) || module_specifier.scheme() == NODE_PROTOCOL {
            let module_name = if specifier_str.contains(NODE_BUILTIN_PATH) {
                specifier_str
                    .split(NODE_BUILTIN_PATH)
                    .nth(1)
                    .unwrap_or("unknown")
                    .replace(".js", "")
            } else {
                module_specifier.path().trim_start_matches('/').replace(".js", "")
            };

            let stub_code = match module_name.as_str() {
                PATH_MODULE => NODE_PATH_STUB.to_string(),
                PROCESS_MODULE => NODE_PROCESS_STUB.to_string(),
                FS_MODULE => NODE_FS_STUB.to_string(),
                URL_MODULE => NODE_URL_STUB.to_string(),
                REACT_MODULE => REACT_STUB.to_string(),
                _ => format!(
                    r#"
// ESM-compatible stub for node:{module_name}
export default {{
  name: '{module_name}',
  isStub: true
}};
export const __esModule = true;
"#
                ),
            };

            return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(stub_code.into()),
                module_specifier,
                None,
            ))));
        }
        None
    }

    fn handle_rari_internal_modules(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if specifier_str.contains(RARI_INTERNAL_PATH) {
            let module_name = specifier_str
                .split(RARI_INTERNAL_PATH)
                .nth(1)
                .unwrap_or("unknown")
                .replace(".js", "");

            if let Some(code) = self.storage.get_module_code(specifier_str) {
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(code.into()),
                    module_specifier,
                    None,
                ))));
            }

            if module_name.starts_with(LOADER_STUB_PREFIX) {
                let component_id = module_name.trim_start_matches(LOADER_STUB_PREFIX);
                let stub_code = LOADER_STUB_TEMPLATE.replace("{component_id}", component_id);

                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(stub_code.into()),
                    module_specifier,
                    None,
                ))));
            }

            let fallback_code = FALLBACK_MODULE_TEMPLATE.replace("{module_name}", &module_name);

            return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(fallback_code.into()),
                module_specifier,
                None,
            ))));
        }
        None
    }

    fn handle_file_protocol_modules(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if specifier_str.starts_with(FILE_PROTOCOL) && !specifier_str.contains(NODE_BUILTIN_PATH) {
            let file_path = specifier_str.strip_prefix(FILE_PROTOCOL).unwrap_or(specifier_str);

            let cache = get_async_file_manager().file_cache.read();
            if let Some((content, _)) = cache.get(file_path) {
                let final_code = content.clone();
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(final_code.into()),
                    module_specifier,
                    None,
                ))));
            }
            drop(cache);

            if let Ok(content) = fs::read_to_string(file_path) {
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(content.into()),
                    module_specifier,
                    None,
                ))));
            }
        }
        None
    }

    fn handle_node_modules(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if specifier_str.contains(NODE_MODULES_PATH) {
            let parts: Vec<&str> = specifier_str.split(NODE_MODULES_PATH).collect();
            let module_path = parts.get(1).unwrap_or(&"unknown");

            if module_path.starts_with(REACT_MODULE) {
                if *module_path == "react/jsx-runtime.js"
                    || module_path.ends_with("/jsx-runtime.js")
                {
                    return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                        ModuleType::JavaScript,
                        ModuleSourceCode::String(JSX_RUNTIME_STUB.to_string().into()),
                        module_specifier,
                        None,
                    ))));
                } else {
                    return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                        ModuleType::JavaScript,
                        ModuleSourceCode::String(REACT_STUB.to_string().into()),
                        module_specifier,
                        None,
                    ))));
                }
            }

            let package_name = module_path.split('/').next().unwrap_or(module_path);

            if let Some(resolved_path) = self.resolve_from_node_modules(package_name, "") {
                let file_path = resolved_path.strip_prefix(FILE_PROTOCOL).unwrap_or(&resolved_path);

                if let Ok(content) = fs::read_to_string(file_path) {
                    return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                        ModuleType::JavaScript,
                        ModuleSourceCode::String(content.into()),
                        module_specifier,
                        None,
                    ))));
                }
            }

            let generic_stub = create_generic_module_stub(module_path);

            return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(generic_stub.into()),
                module_specifier,
                None,
            ))));
        }
        None
    }

    fn handle_rari_component_modules(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if specifier_str.contains(RARI_COMPONENT_PATH) {
            let component_name = specifier_str
                .split(RARI_COMPONENT_PATH)
                .nth(1)
                .unwrap_or("unknown")
                .replace(".js", "");

            if let Some(code) = self.storage.get_module_code(specifier_str) {
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(code.into()),
                    module_specifier,
                    None,
                ))));
            }

            for entry in self.component_specifiers.iter() {
                let component_id = entry.key();
                let specifier = entry.value();
                if (component_id == &component_name || specifier.contains(&component_name))
                    && let Some(code) = self.storage.get_module_code(specifier)
                {
                    return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                        ModuleType::JavaScript,
                        ModuleSourceCode::String(code.into()),
                        module_specifier,
                        None,
                    ))));
                }
            }

            if component_name.contains(FUNCTIONS_MODULE) {
                for entry in self.component_specifiers.iter() {
                    let component_id = entry.key();
                    let specifier = entry.value();
                    if component_id == FUNCTIONS_MODULE
                        && let Some(code) = self.storage.get_module_code(specifier)
                    {
                        return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                            ModuleType::JavaScript,
                            ModuleSourceCode::String(code.into()),
                            module_specifier,
                            None,
                        ))));
                    }
                }

                return Some(ModuleLoadResponse::Sync(Err(JsErrorBox::generic(
                    "Module not found",
                ))));
            }

            let stub_code = create_component_stub(&component_name);

            return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(stub_code.into()),
                module_specifier,
                None,
            ))));
        }
        None
    }

    fn resolve_regular_package_from_dir(
        &self,
        package_name: &str,
        start_dir: &Path,
    ) -> Option<String> {
        if let Some(cached_path) = self.module_resolver.get_cached_package(package_name) {
            return Some(cached_path);
        }

        if let Some(package_dir) = self.find_package_directory(start_dir, package_name) {
            if let Some(entry_point) = self.resolve_package_entry_point(&package_dir) {
                self.cache_resolved_package(package_name, &entry_point);
                return Some(entry_point);
            }

            let fallback_url = format!("file://{}", package_dir.display());
            self.cache_resolved_package(package_name, &fallback_url);
            return Some(fallback_url);
        }

        None
    }

    fn resolve_subpath_export_from_dir(
        &self,
        package_name: &str,
        subpath: &str,
        start_dir: &Path,
    ) -> Option<String> {
        let package_dir = self.find_package_directory(start_dir, package_name)?;

        let package_json_path = package_dir.join("package.json");
        if !package_json_path.exists() {
            return None;
        }

        let package_json_content = fs::read_to_string(&package_json_path).ok()?;
        let package_info = self.parse_package_json(&package_json_content).ok()?;

        if let Some(exports) = &package_info.exports {
            return self.resolve_subpath_from_exports(exports, subpath, &package_dir);
        }

        None
    }

    fn resolve_subpath_from_exports(
        &self,
        exports: &serde_json::Value,
        subpath: &str,
        package_dir: &Path,
    ) -> Option<String> {
        if let Some(exports_obj) = exports.as_object() {
            let subpath_variants = vec![
                subpath.to_string(),
                format!(".{}", subpath),
                subpath[1..].to_string(),
                format!("./{}", &subpath[1..]),
            ];

            for variant in &subpath_variants {
                if let Some(export_value) = exports_obj.get(variant)
                    && let Some(result) = self.resolve_export_value(export_value, package_dir)
                {
                    return Some(result);
                }
            }
        }

        None
    }

    #[allow(clippy::only_used_in_recursion)]
    fn resolve_export_value(
        &self,
        export_value: &serde_json::Value,
        package_dir: &Path,
    ) -> Option<String> {
        match export_value {
            serde_json::Value::String(path_str) => {
                let full_path = package_dir.join(path_str.trim_start_matches("./"));

                if full_path.exists() {
                    let file_url = format!("file://{}", full_path.to_string_lossy());
                    return Some(file_url);
                }
            }
            serde_json::Value::Object(obj) => {
                let conditions = ["import", "module", "default"];
                for condition in &conditions {
                    if let Some(nested_value) = obj.get(*condition)
                        && let Some(result) = self.resolve_export_value(nested_value, package_dir)
                    {
                        return Some(result);
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn cache_resolved_package(&self, package_name: &str, resolved_path: &str) {
        self.module_resolver
            .cache_package_resolution(package_name.to_string(), resolved_path.to_string());
    }

    fn parse_package_json(&self, content: &str) -> Result<PackageInfo, serde_json::Error> {
        let json: serde_json::Value = serde_json::from_str(content)?;

        Ok(PackageInfo {
            module: json.get("module").and_then(|v| v.as_str()).map(|s| s.to_string()),
            exports: json.get("exports").cloned(),
        })
    }

    fn resolve_entry_from_package_info(
        &self,
        package_info: &PackageInfo,
        package_dir: &Path,
    ) -> Option<String> {
        if let Some(exports) = &package_info.exports
            && let Some(resolved) = self.resolve_from_exports(exports, package_dir)
        {
            return Some(resolved);
        }

        if let Some(module_path) = &package_info.module {
            let full_path = package_dir.join(module_path);
            if full_path.exists() {
                let file_url = format!("file://{}", full_path.to_string_lossy());
                return Some(file_url);
            }
        }

        let fallbacks = ["index.mjs", "index.ts", "index.js"];
        for fallback in &fallbacks {
            let fallback_path = package_dir.join(fallback);
            if fallback_path.exists() {
                let file_url = format!("file://{}", fallback_path.to_string_lossy());
                return Some(file_url);
            }
        }

        None
    }

    fn resolve_from_exports(
        &self,
        exports: &serde_json::Value,
        package_dir: &Path,
    ) -> Option<String> {
        if let Some(export_path) = exports.as_str() {
            let clean_path = export_path.trim_start_matches("./");
            let full_path = package_dir.join(clean_path);
            if full_path.exists() {
                let file_url = format!("file://{}", full_path.to_string_lossy());
                return Some(file_url);
            }
        }

        if let Some(exports_obj) = exports.as_object()
            && let Some(main_export) = exports_obj.get(".")
        {
            if let Some(path) = main_export.as_str() {
                let clean_path = path.trim_start_matches("./");
                let full_path = package_dir.join(clean_path);
                if full_path.exists() {
                    let file_url = format!("file://{}", full_path.to_string_lossy());
                    return Some(file_url);
                }
            }

            if let Some(conditional) = main_export.as_object() {
                let conditions = ["import", "module", "default"];

                for condition in &conditions {
                    if let Some(condition_value) = conditional.get(*condition) {
                        if let Some(path) = condition_value.as_str() {
                            let clean_path = path.trim_start_matches("./");
                            let full_path = package_dir.join(clean_path);
                            if full_path.exists() {
                                let file_url = format!("file://{}", full_path.to_string_lossy());
                                return Some(file_url);
                            }
                        } else if let Some(nested_conditional) = condition_value.as_object() {
                            let nested_conditions = ["default", "module", "main"];
                            for nested_condition in &nested_conditions {
                                if let Some(path) = nested_conditional
                                    .get(*nested_condition)
                                    .and_then(|v| v.as_str())
                                {
                                    let clean_path = path.trim_start_matches("./");
                                    let full_path = package_dir.join(clean_path);

                                    if full_path.exists() {
                                        let file_url =
                                            format!("file://{}", full_path.to_string_lossy());

                                        return Some(file_url);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn resolve_package_entry_point(&self, package_dir: &Path) -> Option<String> {
        let package_json_path = package_dir.join("package.json");
        if let Ok(content) = fs::read_to_string(&package_json_path)
            && let Ok(package_info) = self.parse_package_json(&content)
        {
            return self.resolve_entry_from_package_info(&package_info, package_dir);
        }

        let default_files = ["index.mjs", "index.ts", "index.js"];
        for file in &default_files {
            let entry_path = package_dir.join(file);
            if entry_path.exists() {
                return Some(format!("file://{}", entry_path.to_string_lossy()));
            }
        }

        None
    }
}

impl Default for RariModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleLoader for RariModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, JsErrorBox> {
        self.record_operation();

        if matches!(kind, ResolutionKind::DynamicImport)
            && referrer.contains("node_modules")
            && let Some(package_start) = referrer.rfind("node_modules/")
        {
            let after_node_modules = &referrer[package_start + 13..];
            if after_node_modules.find('/').is_some()
                && (specifier.starts_with("./") || specifier.starts_with("../"))
            {
                let referrer_dir = match std::path::Path::new(referrer).parent() {
                    Some(dir) => dir,
                    None => {
                        return Err(JsErrorBox::generic("Module not found"));
                    }
                };
                let resolved_path = referrer_dir.join(specifier);

                if let Ok(canonical) = resolved_path.canonicalize()
                    && let Ok(url) = ModuleSpecifier::from_file_path(canonical)
                {
                    return Ok(url);
                }
            }
        }

        if specifier.starts_with(FILE_PROTOCOL) {
            let url = ModuleSpecifier::parse(specifier)
                .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;
            return Ok(url);
        }

        if specifier.starts_with("./") || specifier.starts_with("../") {
            if (referrer.contains("node_modules") || self.is_npm_package_context(referrer))
                && let Some(package_base) = self.extract_package_base_from_referrer(referrer)
            {
                let resolved_path = if specifier.starts_with("../") {
                    self.resolve_relative_up(specifier, &package_base)
                } else {
                    self.resolve_relative_current(specifier, &package_base)
                };

                let url = ModuleSpecifier::parse(&resolved_path)
                    .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;
                return Ok(url);
            }

            if specifier == "../functions" {
                let possible_keys = [
                    "functions",
                    "index",
                    "serverFunctions",
                    "server_functions",
                    "rari_internal:///functions.js",
                    "functions.js",
                ];

                for key in &possible_keys {
                    if let Some(functions_specifier) = self.component_specifiers.get(*key) {
                        let url = ModuleSpecifier::parse(functions_specifier.value())
                            .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;
                        return Ok(url);
                    }
                }
                return Err(JsErrorBox::generic("Module not found"));
            }

            if referrer.contains(RARI_COMPONENT_PATH) {
                let source_path = self.module_caching.get_component_source_path(referrer);

                if let Some(source_path) = source_path {
                    let source_dir = if source_path.contains('/') {
                        source_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
                    } else {
                        ""
                    };

                    let resolved_path = if specifier.starts_with("../") {
                        let remaining = specifier.strip_prefix("../").unwrap_or(specifier);
                        let parent_dir = if source_dir.contains('/') {
                            source_dir.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
                        } else {
                            ""
                        };

                        let remaining_with_ext = if !remaining.contains('.') {
                            format!("{remaining}.ts")
                        } else {
                            remaining.to_string()
                        };

                        format!("file://{parent_dir}/{remaining_with_ext}")
                    } else {
                        let remaining = specifier.strip_prefix("./").unwrap_or(specifier);

                        let remaining_with_ext = if !remaining.contains('.') {
                            format!("{remaining}.ts")
                        } else {
                            remaining.to_string()
                        };

                        format!("file://{source_dir}/{remaining_with_ext}")
                    };

                    let url = ModuleSpecifier::parse(&resolved_path)
                        .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;
                    return Ok(url);
                }
            }

            let referrer_path = if referrer.starts_with(FILE_PROTOCOL) {
                referrer.strip_prefix(FILE_PROTOCOL).unwrap_or(referrer)
            } else {
                referrer
            };

            let base_dir = if referrer_path.contains('/') {
                referrer_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
            } else {
                ""
            };

            let resolved_path = if specifier.starts_with("../") {
                let remaining = specifier.strip_prefix("../").unwrap_or(specifier);
                let parent_dir = if base_dir.contains('/') {
                    base_dir.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
                } else {
                    ""
                };
                format!("file://{parent_dir}/{remaining}")
            } else {
                let remaining = specifier.strip_prefix("./").unwrap_or(specifier);
                format!("file://{base_dir}/{remaining}")
            };

            let url = ModuleSpecifier::parse(&resolved_path)
                .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;
            return Ok(url);
        }

        if specifier.starts_with(NODE_PREFIX) {
            let node_module_name = specifier.replace(NODE_PREFIX, "");

            let known_node_modules = [
                "path",
                "fs",
                "os",
                "util",
                "buffer",
                "events",
                "stream",
                "url",
                "http",
                "https",
                "net",
                "dns",
                "crypto",
                "querystring",
                "child_process",
                "readline",
                "zlib",
                "assert",
                "console",
                "process",
                "timers",
                "_http_common",
            ];

            if known_node_modules.contains(&node_module_name.as_str()) {
                let result = ModuleSpecifier::parse(&format!(
                    "file://{NODE_BUILTIN_PATH}{node_module_name}.js"
                ))
                .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;

                return Ok(result);
            }

            let result = ModuleSpecifier::parse(specifier)
                .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;

            return Ok(result);
        }

        if let Some(component_specifier) = self.get_component_specifier(specifier) {
            return self.resolve(&component_specifier, referrer, kind);
        }

        if !specifier.contains("://") && !specifier.starts_with("/") {
            if specifier == "react" || specifier.starts_with("react/") {
                if specifier == "react/jsx-runtime" {
                    let jsx_runtime_url = "file:///node_modules/react/jsx-runtime.js".to_string();
                    return self.resolve(&jsx_runtime_url, referrer, kind);
                } else {
                    let react_url =
                        "file:///node_modules/react/esm/react.development.js".to_string();
                    return self.resolve(&react_url, referrer, kind);
                }
            }

            if let Some(resolved_path) = self.resolve_from_node_modules(specifier, referrer) {
                return self.resolve(&resolved_path, referrer, kind);
            }
        }

        let url = ModuleSpecifier::parse(specifier)
            .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;

        Ok(url)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleLoadReferrer>,
        options: ModuleLoadOptions,
    ) -> ModuleLoadResponse {
        let load_start = std::time::Instant::now();
        let specifier_str = module_specifier.to_string();
        let is_dyn_import = options.is_dynamic_import;

        if let Some(response) = self.handle_cached_module(&specifier_str, module_specifier) {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        let maybe_referrer_spec = maybe_referrer.map(|r| r.specifier.clone());
        if let Some(response) = self.handle_dynamic_import_validation(
            &specifier_str,
            maybe_referrer_spec.as_ref(),
            is_dyn_import,
        ) {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        if let Some(response) = self.handle_version_query(&specifier_str, module_specifier) {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        if let Some(response) = self.handle_node_builtin_modules(&specifier_str, module_specifier) {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        if let Some(response) = self.handle_rari_internal_modules(&specifier_str, module_specifier)
        {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        if let Some(response) = self.handle_file_protocol_modules(&specifier_str, module_specifier)
        {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        if let Some(response) = self.handle_node_modules(&specifier_str, module_specifier) {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        if let Some(response) = self.handle_rari_component_modules(&specifier_str, module_specifier)
        {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        let load_duration = load_start.elapsed().as_millis() as u64;
        self.record_module_load(load_duration);
        self.record_operation();
        ModuleLoadResponse::Sync(Err(JsErrorBox::generic("Module not found")))
    }
}

#[derive(Debug, Clone)]
struct PackageInfo {
    module: Option<String>,
    exports: Option<serde_json::Value>,
}
