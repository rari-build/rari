use crate::error::RariError;
use dashmap::DashMap;
use deno_core::{
    FastString, ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier,
    ModuleType, RequestedModuleType, ResolutionKind,
};
use deno_error::JsErrorBox;
use parking_lot::{Mutex, RwLock};
use regex;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use smallvec::{SmallVec, smallvec};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::time::{Duration, Instant};

const TYPESCRIPT_EXTENSION: &str = ".ts";
const TYPESCRIPT_JSX_EXTENSION: &str = ".tsx";
const JSX_EXTENSION: &str = ".jsx";
const JAVASCRIPT_EXTENSION: &str = ".js";

const NODE_MODULES_PATH: &str = "/node_modules/";
const RARI_COMPONENT_PATH: &str = "/rari_component/";
const NODE_BUILTIN_PATH: &str = "/node_builtin/";
const FILE_PROTOCOL: &str = "file://";
const NODE_PROTOCOL: &str = "node";
const NODE_PREFIX: &str = "node:";

const PATH_MODULE: &str = "path";
const PROCESS_MODULE: &str = "process";
const FS_MODULE: &str = "fs";
const REACT_MODULE: &str = "react";
const FUNCTIONS_MODULE: &str = "functions";

const VERSION_QUERY_PARAM: &str = "?v=";
const RELATIVE_CURRENT_PATH: &str = "./";
const RELATIVE_UP_PATH: &str = "../";
const RARI_INTERNAL_PATH: &str = "/rari_internal/";
const LOADER_STUB_PREFIX: &str = "load_";

const NODE_FS_STUB: &str = r#"
// ESM-compatible bridge for node:fs to Deno APIs

const readFileSync = (path, encoding) => {
  try {
    if (globalThis.Deno?.readTextFileSync) {
      const content = globalThis.Deno.readTextFileSync(path);
      if (encoding === 'utf-8' || encoding === 'utf8') {
        return content;
      }
      return new TextEncoder().encode(content);
    }
    return new Uint8Array(0);
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, open '${path}'`);
  }
};

const readFile = async (path, encoding) => {
  try {
    if (globalThis.Deno?.readTextFile) {
      const content = await globalThis.Deno.readTextFile(path);
      if (encoding === 'utf-8' || encoding === 'utf8') {
        return content;
      }
      return new TextEncoder().encode(content);
    }
    return new Uint8Array(0);
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, open '${path}'`);
  }
};

const existsSync = (path) => {
  try {
    if (globalThis.Deno?.statSync) {
      globalThis.Deno.statSync(path);
      return true;
    }
    return false;
  } catch (error) {
    return false;
  }
};

const readdirSync = (path) => {
  try {
    if (globalThis.Deno?.readDirSync) {
      const entries = [];
      for (const entry of globalThis.Deno.readDirSync(path)) {
        entries.push(entry.name);
      }
      return entries;
    }
    return [];
  } catch (error) {
    throw new Error(`ENOENT: no such file or directory, scandir '${path}'`);
  }
};

export default {
  readFileSync,
  readFile,
  existsSync,
  readdirSync,
  writeFileSync: () => {},
  writeFile: () => Promise.resolve(),
  exists: () => Promise.resolve(false),
  mkdirSync: () => {},
  mkdir: () => Promise.resolve(),
  readdir: () => Promise.resolve([]),
};

export { readFileSync, readFile, existsSync, readdirSync };
export const writeFileSync = () => {};
export const writeFile = () => Promise.resolve();
export const exists = () => Promise.resolve(false);
export const mkdirSync = () => {};
export const mkdir = () => Promise.resolve();
export const readdir = () => Promise.resolve([]);
export const __esModule = true;
"#;

const LOADER_STUB_TEMPLATE: &str = r#"
// Auto-generated loader stub for {component_id}

if (typeof globalThis.registerModule === 'function') {{
    globalThis.registerModule({{}}, '{component_id}');
}}

if (typeof globalThis.__rsc_functions === 'undefined') {{
    globalThis.__rsc_functions = {{}};
}}

if (typeof globalThis.__rsc_modules === 'undefined') {{
    globalThis.__rsc_modules = {{}};
}}

globalThis.__rsc_modules['{component_id}'] = {{
    __isLoaderStub: true,
    __awaitingRegistration: true
}};

export default {{
    __isLoaderStub: true,
    __componentId: "{component_id}",
    __timestamp: Date.now()
}};
"#;

const FALLBACK_MODULE_TEMPLATE: &str = r#"
// Dynamic fallback module for: {module_name}

if (typeof globalThis.__rsc_modules === 'undefined') {{
    globalThis.__rsc_modules = {{}};
}}

globalThis.__rsc_modules['{module_name}'] = {{
    __isFallback: true,
    __timestamp: Date.now()
}};

export default {{
    __isFallback: true,
    __module: "{module_name}",
    __timestamp: Date.now()
}};
"#;

const NODE_PATH_STUB: &str = r#"
// ESM-compatible stub for node:path

export function join(...parts) {
  return parts.join('/');
}
export function dirname(path) {
  return path.split('/').slice(0, -1).join('/');
}
export function basename(path) {
  return path.split('/').pop();
}
export function extname(path) {
  const parts = path.split('.');
  return parts.length > 1 ? `.${parts.pop()}` : '';
}
export function resolve(...parts) {
  return '/' + parts.join('/');
}
export function isAbsolute(path) {
  return path.startsWith('/');
}
export default {
  join,
  dirname,
  basename,
  extname,
  resolve,
  isAbsolute,
};
export const __esModule = true;
"#;

const NODE_PROCESS_STUB: &str = r#"
// ESM-compatible bridge for node:process to Deno APIs

const cwd = () => {
  try {
    if (globalThis.Deno?.cwd) {
      return globalThis.Deno.cwd();
    }
    return '/';
  } catch (error) {
    return '/';
  }
};

const env = new Proxy({}, {
  get(target, prop) {
    try {
      if (globalThis.process?.env && prop in globalThis.process.env) {
        return globalThis.process.env[prop];
      }
      if (globalThis.Deno?.env?.get) {
        return globalThis.Deno.env.get(prop);
      }
      return undefined;
    } catch (error) {
      return undefined;
    }
  },
  has(target, prop) {
    try {
      if (globalThis.process?.env && prop in globalThis.process.env) {
        return true;
      }
      if (globalThis.Deno?.env?.get) {
        return globalThis.Deno.env.get(prop) !== undefined;
      }
      return false;
    } catch (error) {
      return false;
    }
  },
  ownKeys(target) {
    try {
      if (globalThis.process?.env) {
        return Object.keys(globalThis.process.env);
      }
      return [];
    } catch (error) {
      return [];
    }
  },
  getOwnPropertyDescriptor(target, prop) {
    try {
      if (globalThis.process?.env && prop in globalThis.process.env) {
        return {
          enumerable: true,
          configurable: true,
          value: globalThis.process.env[prop]
        };
      }
      return undefined;
    } catch (error) {
      return undefined;
    }
  }
});

const argv = ['node'];

const platform = (() => {
  try {
    if (globalThis.Deno?.build?.os) {
      const os = globalThis.Deno.build.os;
      if (os === 'darwin') return 'darwin';
      if (os === 'linux') return 'linux';
      if (os === 'windows') return 'win32';
    }
    return 'linux';
  } catch (error) {
    return 'linux';
  }
})();

export default {
  cwd,
  env,
  argv,
  platform,
  nextTick: (fn) => setTimeout(fn, 0),
  exit: (code = 0) => {
    if (globalThis.Deno?.exit) {
      globalThis.Deno.exit(code);
    }
  },
};

export { cwd, env, argv, platform };
export const nextTick = (fn) => setTimeout(fn, 0);
export const exit = (code = 0) => {
  if (globalThis.Deno?.exit) {
    globalThis.Deno.exit(code);
  }
};
export const __esModule = true;
"#;

const REACT_STUB: &str = r#"
// React stub for Deno environment

const createElement = (type, props, ...children) => {
  if (typeof type === 'string') {
    // HTML element
    return { type, props: props || {}, children: children.flat() };
  }
  // Component
  return { type, props: props || {}, children: children.flat() };
};

const Fragment = Symbol('react.fragment');
const Suspense = (props) => props.children;
const useState = (initial) => [initial, () => {}];
const useEffect = () => {};
const useContext = () => null;
const use = (promise) => {
  if (promise && typeof promise.then === 'function') {
    throw promise; // Suspense behavior
  }
  return promise;
};
const createContext = (defaultValue) => ({
  Provider: ({ children }) => children,
  Consumer: ({ children }) => children(defaultValue),
  _currentValue: defaultValue
});
const memo = (component) => component;
const forwardRef = (component) => component;
const useRef = (initial) => ({ current: initial });
const useCallback = (fn) => fn;
const useMemo = (fn) => fn();
const createRef = () => ({ current: null });
const lazy = (factory) => factory;
const StrictMode = ({ children }) => children;
const useTransition = () => [false, (fn) => fn()];
const useDeferredValue = (value) => value;
const useId = () => Math.random().toString(36);
const startTransition = (fn) => fn();
const flushSync = (fn) => fn();
const unstable_act = (fn) => fn();

export {
  createElement,
  Fragment,
  Suspense,
  useState,
  useEffect,
  useContext,
  use,
  createContext,
  memo,
  forwardRef,
  useRef,
  useCallback,
  useMemo,
  createRef,
  lazy,
  StrictMode,
  useTransition,
  useDeferredValue,
  useId,
  startTransition,
  flushSync,
  unstable_act
};

export default {
  createElement,
  Fragment,
  Suspense,
  useState,
  useEffect,
  useContext,
  use,
  createContext,
  memo,
  forwardRef,
  useRef,
  useCallback,
  useMemo,
  createRef,
  lazy,
  StrictMode,
  useTransition,
  useDeferredValue,
  useId,
  startTransition,
  flushSync,
  unstable_act
};
"#;

fn create_generic_module_stub(module_path: &str) -> String {
    format!(
        r#"
// Generic fallback stub for node module: {module_path}

export default {{
  name: '{module_path}',
  isStub: true
}};

export const useState = (initialState) => [initialState, () => {{}}];
export const useEffect = (fn, deps) => {{}};
export const createElement = (type, props, ...children) => ({{ type, props, children }});
export const render = () => {{}};
export const Fragment = Symbol('fragment');
"#
    )
}

fn create_component_stub(component_name: &str) -> String {
    format!(
        r#"
// Auto-generated stub for component: {component_name}

const moduleExports = {{
    __isStub: true,
    __componentName: "{component_name}",
    __awaitingRegistration: true
}};

export function __rari_register() {{
    if (typeof globalThis.registerModule === 'function') {{
        globalThis.registerModule(moduleExports, '{component_name}');
    }}

    if (typeof globalThis.__rsc_functions === 'undefined') {{
        globalThis.__rsc_functions = {{}};
    }}

    if (typeof globalThis.__rsc_modules === 'undefined') {{
        globalThis.__rsc_modules = {{}};
    }}

    globalThis.__rsc_modules['{component_name}'] = moduleExports;
}}

export default moduleExports;
"#
    )
}

type DependencyList = SmallVec<[String; 4]>;
type ModuleOperations = SmallVec<[ModuleOperation; 8]>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum StorageKey {
    ModuleCode(String),
    ModuleMeta(String),
    Version(String),
}

#[derive(Debug, Clone)]
enum StorageValue {
    Code(String),
    Meta(bool),
    Version(u64),
}

impl StorageValue {
    fn as_code(&self) -> Option<&String> {
        match self {
            StorageValue::Code(code) => Some(code),
            _ => None,
        }
    }

    fn as_meta(&self) -> Option<bool> {
        match self {
            StorageValue::Meta(meta) => Some(*meta),
            _ => None,
        }
    }

    fn as_version(&self) -> Option<u64> {
        match self {
            StorageValue::Version(version) => Some(*version),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct StringInterner {
    cache: Arc<RwLock<FxHashMap<String, Arc<str>>>>,
    hit_count: AtomicUsize,
    miss_count: AtomicUsize,
}

impl StringInterner {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(FxHashMap::default())),
            hit_count: AtomicUsize::new(0),
            miss_count: AtomicUsize::new(0),
        }
    }

    fn stats(&self) -> (usize, usize) {
        (self.hit_count.load(Ordering::Relaxed), self.miss_count.load(Ordering::Relaxed))
    }

    fn intern(&self, s: &str) -> Arc<str> {
        let cache = self.cache.read();
        if let Some(existing) = cache.get(s) {
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            return existing.clone();
        }
        drop(cache);

        self.miss_count.fetch_add(1, Ordering::Relaxed);
        let arc_str: Arc<str> = Arc::from(s);

        let mut cache = self.cache.write();
        cache.insert(s.to_string(), arc_str.clone());

        arc_str
    }
}

#[derive(Debug)]
struct BatchedOperation {
    operations: ModuleOperations,
    created_at: Instant,
}

#[derive(Debug)]
enum ModuleOperation {
    AddModule { specifier: Arc<str>, code: Arc<str> },
}

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
static STRING_INTERNER: OnceLock<StringInterner> = OnceLock::new();
static ASYNC_FILE_MANAGER: OnceLock<AsyncFileManager> = OnceLock::new();

fn get_import_regex() -> &'static regex::Regex {
    IMPORT_REGEX.get_or_init(|| {
        regex::Regex::new(r#"(?:import|from)\s+(['"])(.*?)(['"])"#).expect("Invalid import regex")
    })
}

fn get_string_interner() -> &'static StringInterner {
    STRING_INTERNER.get_or_init(StringInterner::new)
}

fn get_async_file_manager() -> &'static AsyncFileManager {
    ASYNC_FILE_MANAGER.get_or_init(AsyncFileManager::new)
}

type ExtensionTranspilerResult = Result<(FastString, Option<Cow<'static, [u8]>>), JsErrorBox>;
type ExtensionTranspilerFn = dyn Fn(FastString, FastString) -> ExtensionTranspilerResult;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub max_concurrent_operations: usize,
    pub operation_timeout_ms: u64,
    pub cache_size_limit: usize,
    pub enable_hot_reload: bool,
    pub batch_size_limit: usize,
    pub batch_time_limit_ms: u64,
    pub file_cache_duration_secs: u64,
    pub cleanup_interval_secs: u64,
    pub max_memory_per_component_mb: usize,
    pub string_interner_max_size: usize,
    pub enable_string_interning: bool,
    pub enable_batch_operations: bool,
    pub enable_async_file_cache: bool,
    pub enable_metrics_collection: bool,
    pub metrics_collection_interval_ms: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 1000,
            operation_timeout_ms: 30000,
            cache_size_limit: 50000,
            enable_hot_reload: true,
            batch_size_limit: 500,
            batch_time_limit_ms: 100,
            file_cache_duration_secs: 300,
            cleanup_interval_secs: 300,
            max_memory_per_component_mb: 200,
            string_interner_max_size: 50000,
            enable_string_interning: true,
            enable_batch_operations: true,
            enable_async_file_cache: true,
            enable_metrics_collection: true,
            metrics_collection_interval_ms: 1000,
        }
    }
}

impl RuntimeConfig {
    pub fn production() -> Self {
        Self {
            cache_size_limit: 5000,
            batch_size_limit: 100,
            batch_time_limit_ms: 50,
            file_cache_duration_secs: 600,
            cleanup_interval_secs: 180,
            max_memory_per_component_mb: 100,
            string_interner_max_size: 50000,
            operation_timeout_ms: 3000,
            metrics_collection_interval_ms: 5000,
            ..Default::default()
        }
    }

    pub fn development() -> Self {
        Self {
            cache_size_limit: 100,
            batch_size_limit: 10,
            batch_time_limit_ms: 200,
            file_cache_duration_secs: 60,
            cleanup_interval_secs: 60,
            max_memory_per_component_mb: 25,
            string_interner_max_size: 1000,
            operation_timeout_ms: 10000,
            metrics_collection_interval_ms: 500,
            enable_hot_reload: true,
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub fn test() -> Self {
        Self {
            cache_size_limit: 10,
            batch_size_limit: 5,
            batch_time_limit_ms: 50,
            file_cache_duration_secs: 10,
            cleanup_interval_secs: 5,
            max_memory_per_component_mb: 10,
            string_interner_max_size: 100,
            operation_timeout_ms: 1000,
            metrics_collection_interval_ms: 100,
            enable_hot_reload: false,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeMetrics {
    pub cache_stats: CacheStats,
    pub batch_stats: BatchStats,
    pub interner_stats: InternerStats,
    pub performance_stats: PerformanceStats,
    pub resource_stats: ResourceStats,
    pub collected_at: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct BatchStats {
    pub total_batches_processed: usize,
    pub total_operations_batched: usize,
    pub average_batch_size: f64,
    pub batch_flush_failures: usize,
    pub time_saved_by_batching_ms: u64,
}

#[derive(Debug, Clone)]
pub struct InternerStats {
    pub total_strings_interned: usize,
    pub memory_saved_bytes: usize,
    pub hit_rate_percentage: f64,
    pub cache_size: usize,
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub average_module_load_time_ms: f64,
    pub peak_module_load_time_ms: u64,
    pub total_modules_loaded: usize,
    pub cache_hit_rate_percentage: f64,
    pub operations_per_second: f64,
}

#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub memory_usage_mb: usize,
    pub active_threads: usize,
    pub pending_operations: usize,
    pub file_cache_size: usize,
}

#[derive(Debug)]
pub struct ModuleResolver {
    resolved_packages: DashMap<String, String>,
}

impl ModuleResolver {
    fn new() -> Self {
        Self { resolved_packages: DashMap::new() }
    }
}

#[derive(Debug)]
pub struct ModuleCaching {
    cache: ThreadSafeCache,
    component_source_paths: DashMap<String, String>,
}

impl ModuleCaching {
    fn new(cache_size: usize) -> Self {
        Self { cache: ThreadSafeCache::new(cache_size), component_source_paths: DashMap::new() }
    }

    fn get_cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    pub fn get(&self, key: &str) -> Option<JsonValue> {
        self.cache.get(key)
    }
}

#[derive(Debug)]
struct OrderedStorage {
    storage: DashMap<StorageKey, StorageValue>,
    pending_batch: Arc<Mutex<Option<BatchedOperation>>>,
    batch_size_limit: usize,
    batch_time_limit: Duration,
    total_batches_processed: AtomicUsize,
    total_operations_batched: AtomicUsize,
    batch_flush_failures: AtomicUsize,
    total_batch_time_saved_ms: AtomicU64,
}

impl OrderedStorage {
    fn new() -> Self {
        Self {
            storage: DashMap::new(),
            pending_batch: Arc::new(Mutex::new(None)),
            batch_size_limit: 50,
            batch_time_limit: Duration::from_millis(100),
            total_batches_processed: AtomicUsize::new(0),
            total_operations_batched: AtomicUsize::new(0),
            batch_flush_failures: AtomicUsize::new(0),
            total_batch_time_saved_ms: AtomicU64::new(0),
        }
    }

    fn get_module_code(&self, specifier: &str) -> Option<String> {
        self.storage
            .get(&StorageKey::ModuleCode(specifier.to_string()))
            .and_then(|entry| entry.value().as_code().cloned())
    }

    fn get_module_meta(&self, specifier: &str) -> Option<bool> {
        self.storage
            .get(&StorageKey::ModuleMeta(specifier.to_string()))
            .and_then(|entry| entry.value().as_meta())
    }

    fn get_version(&self, specifier: &str) -> Option<u64> {
        self.storage
            .get(&StorageKey::Version(specifier.to_string()))
            .and_then(|entry| entry.value().as_version())
    }

    pub fn set_module_code(&self, specifier: String, code: String) {
        self.storage.insert(StorageKey::ModuleCode(specifier), StorageValue::Code(code));
    }

    fn set_module_meta(&self, specifier: String, meta: bool) {
        self.storage.insert(StorageKey::ModuleMeta(specifier), StorageValue::Meta(meta));
    }

    fn set_version(&self, specifier: String, version: u64) {
        self.storage.insert(StorageKey::Version(specifier), StorageValue::Version(version));
    }

    fn contains_module_code(&self, specifier: &str) -> bool {
        self.storage.contains_key(&StorageKey::ModuleCode(specifier.to_string()))
    }

    fn add_to_batch(&self, operation: ModuleOperation) -> Result<(), RariError> {
        let now = Instant::now();

        let mut pending = self.pending_batch.lock();

        let should_flush = if let Some(ref mut batch) = pending.as_mut() {
            batch.operations.push(operation);

            batch.operations.len() >= self.batch_size_limit
                || now.duration_since(batch.created_at) >= self.batch_time_limit
        } else {
            *pending = Some(BatchedOperation { operations: smallvec![operation], created_at: now });
            false
        };

        if should_flush {
            let batch = pending.take().expect("Batch should exist when should_flush is true");
            drop(pending);
            self.flush_batch(batch)?;
        }

        Ok(())
    }

    fn flush_pending_batch(&self) -> Result<(), RariError> {
        let mut pending = self.pending_batch.lock();

        if let Some(batch) = pending.take() {
            drop(pending);
            self.flush_batch(batch)?;
        }

        Ok(())
    }

    fn flush_batch(&self, batch: BatchedOperation) -> Result<(), RariError> {
        if batch.operations.is_empty() {
            return Ok(());
        }

        let operation_count = batch.operations.len();

        for operation in &batch.operations {
            match operation {
                ModuleOperation::AddModule { specifier, code } => {
                    self.set_module_code(specifier.to_string(), code.to_string());
                }
            }
        }

        self.total_batches_processed.fetch_add(1, Ordering::Relaxed);
        self.total_operations_batched.fetch_add(operation_count, Ordering::Relaxed);

        let time_saved_estimate = (operation_count.saturating_sub(1)) * 2;
        self.total_batch_time_saved_ms.fetch_add(time_saved_estimate as u64, Ordering::Relaxed);

        Ok(())
    }

    fn add_module_interned(&self, specifier: &str, code: &str) -> Result<(), RariError> {
        let interner = get_string_interner();

        let operation = ModuleOperation::AddModule {
            specifier: interner.intern(specifier),
            code: interner.intern(code),
        };

        self.add_to_batch(operation)
    }

    fn get_batch_stats(&self) -> BatchStats {
        let total_batches = self.total_batches_processed.load(Ordering::Relaxed);
        let total_ops = self.total_operations_batched.load(Ordering::Relaxed);

        BatchStats {
            total_batches_processed: total_batches,
            total_operations_batched: total_ops,
            average_batch_size: if total_batches > 0 {
                total_ops as f64 / total_batches as f64
            } else {
                0.0
            },
            batch_flush_failures: self.batch_flush_failures.load(Ordering::Relaxed),
            time_saved_by_batching_ms: self.total_batch_time_saved_ms.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug)]
struct ThreadSafeCache {
    entries: Arc<RwLock<FxHashMap<String, CacheEntry>>>,
    access_order: Arc<RwLock<VecDeque<String>>>,
    max_size: usize,
    hit_count: AtomicUsize,
    miss_count: AtomicUsize,
    eviction_count: AtomicUsize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: JsonValue,
    last_accessed: Instant,
    access_count: usize,
}

impl ThreadSafeCache {
    fn new(max_size: usize) -> Self {
        let cache = Self {
            entries: Arc::new(RwLock::new(FxHashMap::default())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            hit_count: AtomicUsize::new(0),
            miss_count: AtomicUsize::new(0),
            eviction_count: AtomicUsize::new(0),
        };

        cache.start_cleanup_task();
        cache
    }

    fn start_cleanup_task(&self) {
        let entries_clone = Arc::clone(&self.entries);
        let access_order_clone = Arc::clone(&self.access_order);

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(300));

                let max_age = std::time::Duration::from_secs(3600);
                let now = Instant::now();

                let mut entries = entries_clone.write();
                let mut order = access_order_clone.write();
                {
                    let mut to_remove = Vec::new();

                    for (key, entry) in entries.iter() {
                        if now.duration_since(entry.last_accessed) > max_age {
                            to_remove.push(key.clone());
                        }
                    }

                    for key in &to_remove {
                        entries.remove(key);
                        if let Some(pos) = order.iter().position(|x| x == key) {
                            order.remove(pos);
                        }
                    }
                }
            }
        });
    }

    pub fn get(&self, key: &str) -> Option<JsonValue> {
        let now = Instant::now();

        {
            let mut entries = self.entries.write();
            if let Some(entry) = entries.get_mut(key) {
                entry.last_accessed = now;
                entry.access_count += 1;
                self.hit_count.fetch_add(1, Ordering::Relaxed);

                let mut order = self.access_order.write();
                order.retain(|k| k != key);
                order.push_back(key.to_string());

                return Some(entry.value.clone());
            }
        }

        self.miss_count.fetch_add(1, Ordering::Relaxed);
        None
    }

    fn insert(&self, key: String, value: JsonValue) -> Result<(), RariError> {
        let now = Instant::now();

        let mut entries = self.entries.write();
        let mut order = self.access_order.write();

        if entries.len() >= self.max_size && !entries.contains_key(&key) {
            self.evict_lru(&mut entries, &mut order);
        }

        let entry = CacheEntry { value, last_accessed: now, access_count: 1 };

        entries.insert(key.clone(), entry);

        order.retain(|k| k != &key);
        order.push_back(key);

        Ok(())
    }

    fn evict_lru(&self, entries: &mut FxHashMap<String, CacheEntry>, order: &mut VecDeque<String>) {
        if let Some(lru_key) = order.pop_front() {
            entries.remove(&lru_key);
            self.eviction_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hit_count.load(Ordering::Relaxed),
            misses: self.miss_count.load(Ordering::Relaxed),
            evictions: self.eviction_count.load(Ordering::Relaxed),
            size: self.entries.read().len(),
            max_size: self.max_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub size: usize,
    pub max_size: usize,
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
        hits * 20
    }

    fn estimate_memory_usage(cache_stats: &CacheStats, interner_entries: usize) -> usize {
        let cache_mb = (cache_stats.size * 100) / (1024 * 1024);
        let interner_mb = (interner_entries * 50) / (1024 * 1024);
        let base_mb = 10;

        cache_mb + interner_mb + base_mb
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

    pub fn set_module_code(&self, specifier: String, code: String) {
        self.storage.set_module_code(specifier, code);
    }

    pub fn as_extension_transpiler(self: &Rc<Self>) -> Rc<ExtensionTranspilerFn> {
        Rc::new(move |specifier: FastString, code: FastString| {
            match ModuleSpecifier::parse(specifier.as_str()) {
                Ok(_) => crate::runtime::transpile::maybe_transpile_source(specifier, code),
                Err(e) => Err(JsErrorBox::from_err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Failed to parse module specifier '{specifier}': {e}"),
                )))),
            }
        })
    }

    pub fn add_module(&self, specifier: &str, original_path: &str, code: String) {
        if let Err(_batch_error) = self.storage.add_module_interned(specifier, &code) {
            self.add_module_internal(specifier, original_path, code.clone());
        }

        if specifier.contains("/rari_internal/") {
            let cache_key = get_string_interner().intern(specifier);
            #[allow(clippy::disallowed_methods)]
            if let Err(_e) = self.module_caching.cache.insert(
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

    pub fn clear_component_caches(&self, component_id: &str) {
        let component_specifier = format!("file:///rari_component/{component_id}.js");

        self.storage.storage.remove(&StorageKey::ModuleCode(component_specifier.clone()));
        self.storage
            .storage
            .remove(&StorageKey::ModuleMeta(format!("registered_{component_specifier}")));
        self.storage.storage.remove(&StorageKey::ModuleMeta(format!("hmr_{component_specifier}")));
        self.storage.storage.remove(&StorageKey::Version(component_specifier.clone()));

        self.component_specifiers.remove(component_id);

        let file_cache = get_async_file_manager().file_cache.clone();
        let mut cache = file_cache.write();
        let keys_to_remove: Vec<String> =
            cache.keys().filter(|key| key.contains(component_id)).cloned().collect();
        for key in keys_to_remove {
            cache.remove(&key);
        }
        drop(cache);

        let module_cache_keys: Vec<String> = self
            .module_caching
            .cache
            .entries
            .read()
            .keys()
            .filter(|key| key.contains(component_id))
            .cloned()
            .collect();

        for key in module_cache_keys {
            self.module_caching.cache.entries.write().remove(&key);
        }
    }

    fn add_module_internal(&self, specifier: &str, original_path: &str, code: String) {
        let is_update = self.storage.contains_module_code(specifier);
        let specifier_owned = specifier.to_string();

        if is_update {
            let current_version = self.storage.get_version(specifier).unwrap_or(0) + 1;
            let versioned_specifier = format!("{specifier}?v={current_version}");

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

                let stub_specifier = format!("file:///rari_internal/{simplified_name}.js");

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

    pub fn get_component_specifier(&self, component_id: &str) -> Option<String> {
        if let Some(spec) = self.component_specifiers.get(component_id) {
            return Some(spec.value().clone());
        }

        let component_stub = format!("file:///rari_component/component_{component_id}.js");
        let internal_stub = format!("file:///rari_internal/{component_id}.js");

        if self.storage.contains_module_code(&component_stub) {
            Some(component_stub)
        } else if self.storage.contains_module_code(&internal_stub) {
            Some(internal_stub)
        } else {
            None
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

    pub fn create_specifier(&self, name: &str, prefix: &str) -> String {
        let clean_name = name.replace(".js", "").replace("/", "_");
        format!("file:///{prefix}/{clean_name}.js")
    }

    pub fn transform_to_esmodule(&self, code: &str, original_path: &str) -> String {
        let code = code
            .replace("'use server'", "// 'use server' directive removed")
            .replace("\"use server\"", "// \"use server\" directive removed");

        let is_module = code.contains("\"use module\"") || code.contains("'use module'");

        let has_exports = code.contains("export ");

        let path_parts: Vec<&str> = original_path.split('/').collect();
        let file_name = if let Some(last) = path_parts.last() {
            last.split('.').next().unwrap_or("module")
        } else {
            "module"
        };

        let mut transformed = String::new();

        if !is_module {
            transformed.push_str("\"use module\";\n\n");
        }

        transformed.push_str(&code);

        if !has_exports {
            transformed.push_str(&format!(
                r#"

const {file_name}_exports = {{
    default: typeof {file_name} !== 'undefined' ? {file_name} : null
}};

export default {file_name}_exports.default;

export function __rari_register() {{
    try {{
        const exportedValue = {file_name}_exports.default;
        if (exportedValue) {{
            globalThis["{original_path}"] = exportedValue;
            return true;
        }}
        return false;
    }} catch (e) {{
        console.error("Failed to register module:", e);
        return false;
    }}
}}
"#
            ));
        } else if !code.contains("__rari_register") {
            transformed.push_str(&format!(
                r#"

export function __rari_register() {{
    try {{
        const hasDefaultExport = typeof this.default !== 'undefined';
        if (hasDefaultExport) {{
            globalThis["{original_path}"] = this.default;
        }}
        return true;
    }} catch (e) {{
        console.error("Failed to register module:", e);
        return false;
    }}
}}
"#
            ));
        }

        transformed
    }

    pub fn is_hmr_module(&self, specifier: &str) -> bool {
        self.storage.get_module_meta(&format!("hmr_{specifier}")).unwrap_or(false)
    }

    pub fn get_versioned_specifier(&self, component_id: &str) -> Option<String> {
        let base_specifier = self.get_component_specifier(component_id)?;

        if !self.is_hmr_module(&base_specifier) {
            return Some(base_specifier);
        }

        let current_version = self.storage.get_version(&base_specifier).unwrap_or(1);
        Some(format!("{base_specifier}?v={current_version}"))
    }

    pub fn is_already_evaluated(&self, module_id: &str) -> bool {
        self.storage.get_module_meta(&format!("registered_{module_id}")).unwrap_or(false)
    }

    pub fn mark_module_evaluated(&self, module_id: &str) {
        self.storage.set_module_meta(format!("registered_{module_id}"), true);
    }

    fn resolve_from_node_modules(
        &self,
        package_specifier: &str,
        _referrer_path: &str,
    ) -> Option<String> {
        if let Some(slash_pos) = package_specifier.find('/') {
            if package_specifier.starts_with('@') {
                if let Some(second_slash_pos) = package_specifier[slash_pos + 1..].find('/') {
                    let actual_slash_pos = slash_pos + 1 + second_slash_pos;
                    let package_name = &package_specifier[..actual_slash_pos];
                    let subpath = &package_specifier[actual_slash_pos..];

                    return self.resolve_subpath_export(package_name, subpath);
                }
            } else {
                let package_name = &package_specifier[..slash_pos];
                let subpath = &package_specifier[slash_pos..];

                return self.resolve_subpath_export(package_name, subpath);
            }
        }

        self.resolve_regular_package(package_specifier)
    }

    fn resolve_regular_package(&self, package_name: &str) -> Option<String> {
        if let Some(cached_path) = self.module_resolver.resolved_packages.get(package_name) {
            return Some(cached_path.value().clone());
        }

        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        if let Some(package_dir) = self.find_package_directory(&current_dir, package_name) {
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

    fn find_package_directory(&self, current_dir: &Path, package_name: &str) -> Option<PathBuf> {
        let node_modules_path = current_dir.join("node_modules").join(package_name);
        if node_modules_path.exists() {
            return Some(node_modules_path);
        }

        let pnpm_package_name = if package_name.starts_with('@') && package_name.contains('/') {
            package_name.replace('/', "+")
        } else {
            package_name.to_string()
        };

        let mut search_dir = current_dir.to_path_buf();
        loop {
            let node_modules_path = search_dir.join("node_modules").join(package_name);
            if node_modules_path.exists() {
                return Some(node_modules_path);
            }

            let pnpm_dir = search_dir.join("node_modules").join(".pnpm");
            if pnpm_dir.exists()
                && let Ok(entries) = std::fs::read_dir(&pnpm_dir)
            {
                for entry in entries.flatten() {
                    let dir_name = entry.file_name();
                    let dir_name_str = dir_name.to_string_lossy();

                    if dir_name_str.starts_with(&format!("{pnpm_package_name}@")) {
                        let package_path = entry.path().join("node_modules").join(package_name);
                        if package_path.exists() {
                            return Some(package_path);
                        }
                    }
                }
            }

            if let Some(parent) = search_dir.parent() {
                search_dir = parent.to_path_buf();
            } else {
                break;
            }
        }

        None
    }

    fn resolve_subpath_export(&self, package_name: &str, subpath: &str) -> Option<String> {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        let package_dir = self.find_package_directory(&current_dir, package_name)?;

        let package_json_path = package_dir.join("package.json");
        if !package_json_path.exists() {
            return None;
        }

        let package_json_content = std::fs::read_to_string(&package_json_path).ok()?;
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
            .resolved_packages
            .insert(package_name.to_string(), resolved_path.to_string());
    }

    fn parse_package_json(&self, content: &str) -> Result<PackageInfo, serde_json::Error> {
        let json: serde_json::Value = serde_json::from_str(content)?;

        Ok(PackageInfo {
            name: json.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
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

    fn is_npm_package_context(&self, referrer: &str) -> bool {
        referrer.contains("node_modules")
            || self
                .module_resolver
                .resolved_packages
                .iter()
                .any(|entry| referrer.contains(entry.value()))
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

        for entry in self.module_resolver.resolved_packages.iter() {
            let package_path = entry.value();
            if clean_referrer.contains(package_path) {
                if let Some(base_dir) = package_path.rsplit_once('/') {
                    return Some(base_dir.0.to_string());
                }
                return Some(package_path.clone());
            }
        }

        None
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

    fn get_component_source_path(&self, virtual_specifier: &str) -> Option<String> {
        self.module_caching
            .component_source_paths
            .get(virtual_specifier)
            .map(|entry| entry.value().clone())
    }

    fn try_infer_component_source_path(&self, virtual_specifier: &str) -> Option<String> {
        let _component_id = virtual_specifier
            .strip_prefix("file:///rari_component/")?
            .strip_suffix(JAVASCRIPT_EXTENSION)?;

        None
    }

    fn handle_cached_module(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if let Some(code) = self.storage.get_module_code(specifier_str) {
            let (final_code, module_type) = if specifier_str.ends_with(TYPESCRIPT_EXTENSION)
                || specifier_str.ends_with(TYPESCRIPT_JSX_EXTENSION)
            {
                match crate::runtime::transpile::maybe_transpile_source(
                    specifier_str.to_string().into(),
                    code.to_string().into(),
                ) {
                    Ok((transpiled_code, _source_map)) => {
                        (transpiled_code.to_string(), ModuleType::JavaScript)
                    }
                    Err(err) => {
                        return Some(ModuleLoadResponse::Sync(Err(
                            deno_error::JsErrorBox::generic(format!(
                                "Failed to transpile TypeScript module '{specifier_str}': {err}"
                            )),
                        )));
                    }
                }
            } else if specifier_str.ends_with(JSX_EXTENSION) {
                match crate::runtime::transpile::maybe_transpile_source(
                    specifier_str.to_string().into(),
                    code.to_string().into(),
                ) {
                    Ok((transpiled_code, _source_map)) => {
                        (transpiled_code.to_string(), ModuleType::JavaScript)
                    }
                    Err(err) => {
                        return Some(ModuleLoadResponse::Sync(Err(
                            deno_error::JsErrorBox::generic(format!(
                                "Failed to transpile JSX module '{specifier_str}': {err}"
                            )),
                        )));
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

            if referrer_str.contains(NODE_MODULES_PATH)
                && (specifier_str.starts_with(RELATIVE_CURRENT_PATH)
                    || specifier_str.starts_with(RELATIVE_UP_PATH))
                && specifier_str.starts_with(FILE_PROTOCOL)
            {
                let file_path = specifier_str.strip_prefix("file://").unwrap_or(specifier_str);

                if !std::path::Path::new(file_path).exists() {
                    return Some(ModuleLoadResponse::Sync(Err(deno_error::JsErrorBox::generic(
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
                    .replace(JAVASCRIPT_EXTENSION, "")
            } else {
                module_specifier.path().trim_start_matches('/').replace(JAVASCRIPT_EXTENSION, "")
            };

            let stub_code = match module_name.as_str() {
                PATH_MODULE => NODE_PATH_STUB.to_string(),
                PROCESS_MODULE => NODE_PROCESS_STUB.to_string(),
                FS_MODULE => NODE_FS_STUB.to_string(),
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
                .replace(JAVASCRIPT_EXTENSION, "");

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
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(REACT_STUB.to_string().into()),
                    module_specifier,
                    None,
                ))));
            }

            let package_name = module_path.split('/').next().unwrap_or(module_path);

            if let Some(resolved_path) = self.resolve_from_node_modules(package_name, "docs") {
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
                .replace(JAVASCRIPT_EXTENSION, "");

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

                return Some(ModuleLoadResponse::Sync(Err(deno_error::JsErrorBox::generic(
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
    ) -> Result<ModuleSpecifier, deno_error::JsErrorBox> {
        if matches!(kind, ResolutionKind::DynamicImport)
            && referrer.contains("node_modules")
            && let Some(package_start) = referrer.rfind("node_modules/")
        {
            let after_node_modules = &referrer[package_start + 13..];
            if let Some(package_end) = after_node_modules.find('/') {
                let _package_name = &after_node_modules[..package_end];

                if specifier.starts_with("./") || specifier.starts_with("../") {
                    let referrer_dir = match std::path::Path::new(referrer).parent() {
                        Some(dir) => dir,
                        None => {
                            return Err(deno_error::JsErrorBox::generic("Module not found"));
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
        }

        if specifier.starts_with(FILE_PROTOCOL) {
            let url = ModuleSpecifier::parse(specifier)
                .map_err(|err| deno_error::JsErrorBox::generic(format!("Invalid URL: {err}")))?;
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

                let url = ModuleSpecifier::parse(&resolved_path).map_err(|err| {
                    deno_error::JsErrorBox::generic(format!("Invalid URL: {err}"))
                })?;
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
                        let url =
                            ModuleSpecifier::parse(functions_specifier.value()).map_err(|err| {
                                deno_error::JsErrorBox::generic(format!("Invalid URL: {err}"))
                            })?;
                        return Ok(url);
                    }
                }

                return Err(deno_error::JsErrorBox::generic("Module not found"));
            }

            if referrer.contains(RARI_COMPONENT_PATH) {
                let source_path = self
                    .get_component_source_path(referrer)
                    .or_else(|| self.try_infer_component_source_path(referrer));

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

                    let url = ModuleSpecifier::parse(&resolved_path).map_err(|err| {
                        deno_error::JsErrorBox::generic(format!("Invalid URL: {err}"))
                    })?;
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
                .map_err(|err| deno_error::JsErrorBox::generic(format!("Invalid URL: {err}")))?;
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
                let result =
                    ModuleSpecifier::parse(&format!("file:///node_builtin/{node_module_name}.js"))
                        .map_err(|err| {
                            deno_error::JsErrorBox::generic(format!("Invalid URL: {err}"))
                        })?;

                return Ok(result);
            }

            let result = ModuleSpecifier::parse(specifier)
                .map_err(|err| deno_error::JsErrorBox::generic(format!("Invalid URL: {err}")))?;

            return Ok(result);
        }

        if let Some(component_specifier) = self.get_component_specifier(specifier) {
            return self.resolve(&component_specifier, referrer, kind);
        }

        if !specifier.contains("://") && !specifier.starts_with("/") {
            if specifier == "react" || specifier.starts_with("react/") {
                let react_url = "file:///node_modules/react/esm/react.development.js".to_string();
                return self.resolve(&react_url, referrer, kind);
            }

            if let Some(resolved_path) = self.resolve_from_node_modules(specifier, referrer) {
                return self.resolve(&resolved_path, referrer, kind);
            }
        }

        let url = ModuleSpecifier::parse(specifier)
            .map_err(|err| deno_error::JsErrorBox::generic(format!("Invalid URL: {err}")))?;

        Ok(url)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleSpecifier>,
        is_dyn_import: bool,
        _requested_module_type: RequestedModuleType,
    ) -> ModuleLoadResponse {
        let load_start = std::time::Instant::now();
        let specifier_str = module_specifier.to_string();

        if let Some(response) = self.handle_cached_module(&specifier_str, module_specifier) {
            let load_duration = load_start.elapsed().as_millis() as u64;
            self.record_module_load(load_duration);
            self.record_operation();
            return response;
        }

        if let Some(response) =
            self.handle_dynamic_import_validation(&specifier_str, maybe_referrer, is_dyn_import)
        {
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
        ModuleLoadResponse::Sync(Err(deno_error::JsErrorBox::generic("Module not found")))
    }
}

#[derive(Debug, Clone)]
struct PackageInfo {
    #[allow(unused)]
    name: String,
    module: Option<String>,
    exports: Option<serde_json::Value>,
}
