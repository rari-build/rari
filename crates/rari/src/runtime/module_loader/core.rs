use std::{
    borrow::Cow,
    env,
    fmt::{self, Write},
    fs,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
    rc::Rc,
    string::ToString,
    sync::Arc,
};

use cow_utils::CowUtils;
use dashmap::DashMap;
use deno_ast::MediaType;
use deno_core::{
    FastString, ModuleLoadOptions, ModuleLoadReferrer, ModuleLoadResponse, ModuleLoader,
    ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType, ResolutionKind, resolve_import,
};
use deno_error::JsErrorBox;
use deno_node::NodeResolver;
use deno_resolver::npm::DenoInNpmPackageChecker;
use node_resolver::{
    NodeResolution, NodeResolutionKind, ResolutionMode, errors::NodeResolveErrorKind,
};
use sys_traits::impls::RealSys;

use super::{
    cache::{DEFAULT_TTL_SECS, ModuleCaching},
    config::RuntimeConfig,
    react_vendor,
    storage::ModuleStorage,
    stubs::{
        FALLBACK_MODULE_TEMPLATE, LOADER_STUB_TEMPLATE, RARI_CACHE_STUB, RARI_CALL_SERVER_STUB,
        RARI_CLIENT_STUB, RARI_DEFAULT_STUB, RARI_HEADERS_STUB, RARI_IMAGE_STUB, RARI_ROUTER_STUB,
        create_component_stub,
    },
    transpiler::{needs_jsx_transpilation, needs_typescript_transpilation},
};
use crate::{
    rsc::{DependencyList, extract_dependencies},
    runtime::{
        ext::{NodeCodeTranslator, NpmPackageFolderResolverImpl, Resolver},
        transpile,
    },
    server::{cache::handler::CacheHandlerRegistry, config::CacheLayerConfig},
    utils::path::path_to_file_url,
};

type ExtensionTranspilerResult = Result<(FastString, Option<Cow<'static, [u8]>>), JsErrorBox>;
type ExtensionTranspilerFn = dyn Fn(FastString, FastString) -> ExtensionTranspilerResult;
type RariNodeResolver =
    NodeResolver<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys>;

const NODE_MODULES_PATH: &str = "/node_modules/";
const RARI_COMPONENT_PATH: &str = "/rari_component/";
const RARI_STUB_PATH: &str = "/rari_stub/";
const FILE_PROTOCOL: &str = "file://";
const NODE_PREFIX: &str = "node:";
const FUNCTIONS_MODULE: &str = "functions";
const VERSION_QUERY_PARAM: &str = "?v=";
const DENO_GLOBAL_SCOPE_SHARED: &str = "ext:runtime/98_global_scope_shared.js";
const NODE_CONSOLE_SCOPE_SOURCE: &str = include_str!("../ext/runtime/node_console_scope.ts");
const RELATIVE_CURRENT_PATH: &str = "./";
const RELATIVE_UP_PATH: &str = "../";
const RARI_INTERNAL_PATH: &str = "/rari_internal/";
const LOADER_STUB_PREFIX: &str = "load_";
const RSC_REFERENCES_SPECIFIER: &str = "react-server-dom-rari/server";
const RARI_RSC_REFERENCES_EXPORT: &str = "rari/runtime/rsc-references";
const RARI_MDX_REGISTRY_SPECIFIER: &str = "rari/mdx/registry";
const RARI_MDX_REGISTRY_INTERNAL: &str = "file:///rari_internal/mdx-registry.js";
const RARI_MDX_REGISTRY_MANIFEST: &str = "dist/server/manifest.json";
const RARI_MDX_REGISTRY_EXPORT_PATH: &str = "dist/mdx/registry.mjs";

fn file_url_to_path(url: &str) -> Option<PathBuf> {
    if !url.starts_with(FILE_PROTOCOL) {
        return None;
    }

    ModuleSpecifier::parse(url).ok().and_then(|spec| spec.to_file_path().ok()).or_else(|| {
        url.strip_prefix(FILE_PROTOCOL).map(|path_str| {
            #[cfg(windows)]
            let path_str = path_str.strip_prefix('/').unwrap_or(path_str);

            PathBuf::from(path_str)
        })
    })
}

fn append_extension_only(path: &str) -> (String, &str) {
    let (base_path, suffix) = if let Some(query_pos) = path.find('?') {
        (&path[..query_pos], &path[query_pos..])
    } else if let Some(hash_pos) = path.find('#') {
        (&path[..hash_pos], &path[hash_pos..])
    } else {
        (path, "")
    };

    let base_with_ext = if base_path.ends_with(".ts")
        || base_path.ends_with(".js")
        || base_path.ends_with(".tsx")
        || base_path.ends_with(".jsx")
        || base_path.ends_with(".mjs")
        || base_path.ends_with(".cjs")
    {
        base_path.to_string()
    } else {
        format!("{base_path}.ts")
    };

    (base_with_ext, suffix)
}

fn component_id_aliases(component_id: &str) -> Vec<String> {
    let mut aliases = vec![component_id.to_string()];
    if let Some(stripped) = component_id.strip_prefix('/') {
        aliases.push(stripped.to_string());
    } else {
        aliases.push(format!("/{component_id}"));
    }
    aliases
}

fn is_virtual_referrer(referrer: &str) -> bool {
    referrer.is_empty()
        || referrer.contains(RARI_COMPONENT_PATH)
        || referrer.contains(RARI_INTERNAL_PATH)
        || referrer.contains(RARI_STUB_PATH)
        || referrer.contains("/react_vendor/")
        || referrer.starts_with(react_vendor::NODE_VENDOR_PREFIX)
        || referrer.contains("/rari_hmr/")
        || referrer.starts_with("ext:")
}

pub struct RariModuleLoader {
    storage: ModuleStorage,
    resolver: Arc<Resolver>,
    node_resolver: Arc<RariNodeResolver>,
    code_translator: Rc<NodeCodeTranslator>,
    pub module_caching: ModuleCaching,
    pub component_specifiers: DashMap<String, String>,
}

impl fmt::Debug for RariModuleLoader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RariModuleLoader")
            .field("storage", &self.storage)
            .field("module_caching", &self.module_caching)
            .field("component_specifiers", &self.component_specifiers)
            .finish_non_exhaustive()
    }
}

impl RariModuleLoader {
    pub fn new(resolver: Arc<Resolver>) -> Self {
        Self::with_config(resolver, &RuntimeConfig::default())
    }

    pub fn with_config(resolver: Arc<Resolver>, config: &RuntimeConfig) -> Self {
        Self::with_config_and_registry(
            resolver,
            config,
            &CacheHandlerRegistry::default_with_memory(),
        )
    }

    pub fn with_config_and_registry(
        resolver: Arc<Resolver>,
        config: &RuntimeConfig,
        registry: &CacheHandlerRegistry,
    ) -> Self {
        let layer = CacheLayerConfig {
            handler: config.module_cache_handler.clone(),
            url: None,
            max_entries: config.cache_size_limit,
            default_ttl_secs: DEFAULT_TTL_SECS,
        };
        let module_caching = ModuleCaching::from_config(&layer, registry);
        let node_resolver = resolver.node_resolver();
        let code_translator = Rc::new(resolver.code_translator(Arc::clone(&node_resolver)));
        Self {
            storage: ModuleStorage::new(),
            resolver,
            node_resolver,
            code_translator,
            module_caching,
            component_specifiers: DashMap::new(),
        }
    }

    pub async fn add_module(&self, specifier: &str, original_path: &str, code: String) {
        self.add_module_internal(specifier, original_path, &code);

        if specifier.contains(RARI_INTERNAL_PATH) {
            if let Err(e) =
                self.module_caching.insert(original_path.to_string(), serde_json::Value::Null).await
            {
                tracing::warn!(path = %original_path, error = %e, "module cache insert failed");
            }
        }
    }

    fn add_module_internal(&self, specifier: &str, original_path: &str, code: &str) {
        let is_update = self.storage.contains_module_code(specifier);
        let specifier_owned = specifier.to_string();

        let version_key = if specifier.contains(RARI_COMPONENT_PATH) {
            let component_id = specifier
                .strip_prefix(&format!("file://{RARI_COMPONENT_PATH}"))
                .and_then(|s| s.strip_suffix(".js"))
                .unwrap_or("");
            format!("version_{component_id}")
        } else {
            specifier_owned.clone()
        };

        if is_update {
            let current_version = self.storage.get_version(&version_key).unwrap_or(0) + 1;
            let versioned_specifier = format!("{specifier}{VERSION_QUERY_PARAM}{current_version}");

            self.storage.set_module_code(specifier_owned.clone(), code.to_string());
            self.storage.set_module_code(versioned_specifier.clone(), code.to_string());

            self.storage.set_module_meta(format!("registered_{specifier_owned}"), true);
            self.storage.set_module_meta(format!("registered_{versioned_specifier}"), true);
            self.storage.set_module_meta(format!("hmr_{specifier_owned}"), true);
            self.storage.set_version(version_key, current_version);
        } else {
            self.storage.set_module_code(specifier_owned.clone(), code.to_string());
            self.storage.set_module_meta(format!("registered_{specifier_owned}"), true);
            self.storage.set_version(version_key, 1);
        }

        let dependencies = Self::register_dependencies(original_path, code);

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

    fn register_dependencies(_original_path: &str, code: &str) -> DependencyList {
        extract_dependencies(code)
    }

    pub fn set_module_code(&self, specifier: String, code: String) {
        self.storage.set_module_code(specifier, code);
    }

    pub fn register_component_specifier(&self, component_id: &str, specifier: &str) {
        for alias in component_id_aliases(component_id) {
            self.component_specifiers.insert(alias, specifier.to_string());
        }
    }

    pub fn get_component_specifier(&self, component_id: &str) -> Option<String> {
        for alias in component_id_aliases(component_id) {
            if let Some(spec) = self.component_specifiers.get(&alias) {
                return Some(spec.value().clone());
            }
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
            Some(format!("{base_specifier}{VERSION_QUERY_PARAM}{version}"))
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
            for alias in component_id_aliases(component_id) {
                self.component_specifiers.remove(&alias);
            }
        }

        self.storage.set_module_meta(format!("hmr_{component_specifier}"), false);
        self.storage.set_module_meta(format!("registered_{component_id}"), false);
        self.storage.set_version(format!("version_{component_id}"), 0);
    }

    pub fn create_specifier(&self, name: &str, prefix: &str) -> String {
        let clean_name = name.cow_replace(".js", "").cow_replace("/", "_").into_owned();
        format!("file:///{prefix}/{clean_name}.js")
    }

    pub fn transform_to_esmodule(&self, code: &str, _original_path: &str) -> String {
        code.cow_replace("'use server'", "// 'use server' directive removed")
            .cow_replace("\"use server\"", "// \"use server\" directive removed")
            .into_owned()
    }

    pub fn as_extension_transpiler(self: &Rc<Self>) -> Rc<ExtensionTranspilerFn> {
        Rc::new(move |specifier: FastString, code: FastString| {
            match ModuleSpecifier::parse(specifier.as_str()) {
                Ok(_) => transpile::maybe_transpile_source(&specifier, code),
                Err(e) => Err(JsErrorBox::from_err(Box::new(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Failed to parse module specifier '{specifier}': {e}"),
                )))),
            }
        })
    }

    fn cwd_package_json_url() -> Result<ModuleSpecifier, JsErrorBox> {
        let cwd = env::current_dir().map_err(|err| {
            JsErrorBox::generic(format!("Failed to get current directory: {err}"))
        })?;
        ModuleSpecifier::from_file_path(cwd.join("package.json"))
            .map_err(|()| JsErrorBox::generic("Failed to convert cwd package.json to file URL"))
    }

    fn package_json_url_for_directory(dir: &Path) -> Result<ModuleSpecifier, JsErrorBox> {
        ModuleSpecifier::from_file_path(dir.join("package.json")).map_err(|()| {
            JsErrorBox::generic(format!(
                "Failed to convert package.json under {} to file URL",
                dir.display()
            ))
        })
    }

    fn referrer_url_for_node(referrer: &str) -> Result<ModuleSpecifier, JsErrorBox> {
        if is_virtual_referrer(referrer) {
            return Self::cwd_package_json_url();
        }

        if let Ok(url) = ModuleSpecifier::parse(referrer) {
            if url.scheme() == "file" {
                if url.path().ends_with('/')
                    && let Ok(path) = url.to_file_path()
                {
                    return Self::package_json_url_for_directory(&path);
                }
                return Ok(url);
            }
            return Self::cwd_package_json_url();
        }

        let path = PathBuf::from(referrer);
        if path.is_absolute() {
            if path.is_dir() {
                Self::package_json_url_for_directory(&path)
                    .or_else(|_| Self::cwd_package_json_url())
            } else {
                ModuleSpecifier::from_file_path(&path).or_else(|()| Self::cwd_package_json_url())
            }
        } else {
            Self::cwd_package_json_url()
        }
    }

    fn resolve_via_node_resolver(
        &self,
        specifier: &str,
        referrer: &str,
    ) -> Result<ModuleSpecifier, JsErrorBox> {
        let referrer_url = Self::referrer_url_for_node(referrer)?;

        match self.node_resolver.resolve(
            specifier,
            &referrer_url,
            ResolutionMode::Import,
            NodeResolutionKind::Execution,
        ) {
            Ok(NodeResolution::BuiltIn(builtin_name)) => {
                match self.node_resolver.resolve_package(
                    specifier,
                    &referrer_url,
                    ResolutionMode::Import,
                    NodeResolutionKind::Execution,
                ) {
                    Ok(resolution) => {
                        resolution.into_url().map_err(|err| JsErrorBox::generic(err.to_string()))
                    }
                    Err(err) => {
                        if matches!(err.as_kind(), NodeResolveErrorKind::PackageResolve(_)) {
                            NodeResolution::BuiltIn(builtin_name)
                                .into_url()
                                .map_err(|err| JsErrorBox::generic(err.to_string()))
                        } else {
                            Err(JsErrorBox::generic(err.to_string()))
                        }
                    }
                }
            }
            Ok(resolution) => {
                resolution.into_url().map_err(|err| JsErrorBox::generic(err.to_string()))
            }
            Err(err) => Err(JsErrorBox::generic(err.to_string())),
        }
    }

    fn is_rari_mdx_registry_stub(path: &Path) -> bool {
        path.to_string_lossy().replace('\\', "/").contains(RARI_MDX_REGISTRY_EXPORT_PATH)
    }

    fn synthesize_mdx_registry_module(&self) -> String {
        if let Some(code) = self.storage.get_module_code(RARI_MDX_REGISTRY_INTERNAL) {
            return code;
        }

        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let manifest_path = cwd.join(RARI_MDX_REGISTRY_MANIFEST);

        let entries = match fs::read_to_string(&manifest_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(manifest) => match manifest.get("mdxRegistry") {
                    Some(value) => match value.as_array() {
                        Some(array) => array.clone(),
                        None => {
                            tracing::warn!(
                                path = %manifest_path.display(),
                                "manifest.json mdxRegistry is not an array; using empty registry"
                            );
                            Vec::new()
                        }
                    },
                    None => {
                        tracing::warn!(
                            path = %manifest_path.display(),
                            "manifest.json missing mdxRegistry; using empty registry"
                        );
                        Vec::new()
                    }
                },
                Err(err) => {
                    tracing::warn!(
                        path = %manifest_path.display(),
                        error = %err,
                        "failed to parse manifest.json; using empty mdxRegistry"
                    );
                    Vec::new()
                }
            },
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    tracing::warn!(
                        path = %manifest_path.display(),
                        "manifest.json not found; using empty mdxRegistry"
                    );
                } else {
                    tracing::warn!(
                        path = %manifest_path.display(),
                        error = %err,
                        "failed to read manifest.json; using empty mdxRegistry"
                    );
                }
                Vec::new()
            }
        };

        let mut registry_items = String::new();
        for entry in &entries {
            let Some(name) = entry.get("name").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(id) = entry.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let client = entry.get("client").and_then(serde_json::Value::as_bool).unwrap_or(true);

            let _ = writeln!(
                registry_items,
                "  {{ name: {name:?}, component: null, id: {id:?}, client: {client} }},"
            );
        }

        let code = format!(
            "import {{ defineMdxComponents }} from 'rari/mdx/define'\n\n\
             export const getMDXComponents = defineMdxComponents([\n\
             {registry_items}\
             ])\n"
        );
        self.storage.set_module_code(RARI_MDX_REGISTRY_INTERNAL.to_string(), code.clone());
        code
    }

    fn handle_cached_module(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if let Some(code) = self.storage.get_module_code(specifier_str) {
            let (final_code, module_type) = if needs_typescript_transpilation(specifier_str) {
                let module_name: FastString = specifier_str.to_string().into();
                match transpile::maybe_transpile_source(&module_name, code.into()) {
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
                let module_name: FastString = specifier_str.to_string().into();
                match transpile::maybe_transpile_source(&module_name, code.into()) {
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
        specifier_str: &str,
        maybe_referrer: Option<&ModuleSpecifier>,
        is_dyn_import: bool,
    ) -> Option<ModuleLoadResponse> {
        if is_dyn_import && let Some(referrer) = maybe_referrer {
            let referrer_str = referrer.to_string();

            if referrer_str.contains(NODE_MODULES_PATH) {
                let file_path = if specifier_str.starts_with(FILE_PROTOCOL) {
                    file_url_to_path(specifier_str)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| specifier_str.to_string())
                } else if specifier_str.starts_with(RELATIVE_CURRENT_PATH)
                    || specifier_str.starts_with(RELATIVE_UP_PATH)
                {
                    if let Ok(referrer_path) = referrer.to_file_path()
                        && let Some(referrer_dir) = referrer_path.parent()
                    {
                        let resolved = referrer_dir.join(specifier_str);
                        return if resolved.canonicalize().is_ok() {
                            None
                        } else {
                            Some(ModuleLoadResponse::Sync(Err(JsErrorBox::generic(
                                "Module not found",
                            ))))
                        };
                    }
                    return None;
                } else {
                    return None;
                };

                if !Path::new(&file_path).exists() {
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
                .cow_replace(".js", "");

            if let Some(code) = self.storage.get_module_code(specifier_str) {
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(code.into()),
                    module_specifier,
                    None,
                ))));
            }

            if module_name == "mdx-registry" {
                let stub_code = self.synthesize_mdx_registry_module();

                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(stub_code.into()),
                    module_specifier,
                    None,
                ))));
            }

            if module_name.starts_with(LOADER_STUB_PREFIX) {
                let component_id = module_name.trim_start_matches(LOADER_STUB_PREFIX);
                let stub_code = LOADER_STUB_TEMPLATE.cow_replace("{component_id}", component_id);

                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(stub_code.into_owned().into()),
                    module_specifier,
                    None,
                ))));
            }

            let fallback_code = FALLBACK_MODULE_TEMPLATE.cow_replace("{module_name}", &module_name);

            return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(fallback_code.into_owned().into()),
                module_specifier,
                None,
            ))));
        }
        None
    }

    fn handle_react_vendor_shim(
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        let module_name = specifier_str.strip_prefix(react_vendor::NODE_VENDOR_PREFIX)?;
        let source = react_vendor::reexport_shim_source(module_name)?;
        Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
            ModuleType::JavaScript,
            ModuleSourceCode::String(source.into()),
            module_specifier,
            None,
        ))))
    }

    fn handle_file_protocol_modules(
        &self,
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if !specifier_str.starts_with(FILE_PROTOCOL) {
            return None;
        }

        let Ok(file_path) = module_specifier.to_file_path() else {
            return None;
        };

        if Self::is_rari_mdx_registry_stub(&file_path) {
            let code = self.synthesize_mdx_registry_module();
            return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(code.into()),
                module_specifier,
                None,
            ))));
        }

        let file_path_str = file_path.to_string_lossy();
        let Ok(content) = fs::read_to_string(&file_path) else {
            return None;
        };

        let media_type = MediaType::from_specifier(module_specifier);
        let is_node_modules = file_path_str.contains("node_modules");
        let is_cjs = is_node_modules && self.resolver.is_cjs(module_specifier, media_type, true);

        if is_cjs {
            let code_translator = Rc::clone(&self.code_translator);
            let module_specifier = module_specifier.clone();
            return Some(ModuleLoadResponse::Async(Box::pin(async move {
                let translated = code_translator
                    .translate_cjs_to_esm(&module_specifier, Some(Cow::Owned(content)))
                    .await
                    .map_err(|err| JsErrorBox::generic(err.to_string()))?;

                Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(translated.into_owned().into()),
                    &module_specifier,
                    None,
                ))
            })));
        }

        Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
            ModuleType::JavaScript,
            ModuleSourceCode::String(content.into()),
            module_specifier,
            None,
        ))))
    }

    fn handle_rari_stub_modules(
        specifier_str: &str,
        module_specifier: &ModuleSpecifier,
    ) -> Option<ModuleLoadResponse> {
        if !specifier_str.contains(RARI_STUB_PATH) {
            return None;
        }

        let module_name =
            specifier_str.rsplit(RARI_STUB_PATH).next().unwrap_or("").trim_end_matches(".js");
        let stub_content = match module_name {
            "router" => RARI_ROUTER_STUB.to_string(),
            "headers" => RARI_HEADERS_STUB.to_string(),
            "cache" => RARI_CACHE_STUB.to_string(),
            "image" => RARI_IMAGE_STUB.to_string(),
            "client" => RARI_CLIENT_STUB.to_string(),
            "runtime/call-server" => RARI_CALL_SERVER_STUB.to_string(),
            _ => RARI_DEFAULT_STUB.to_string(),
        };

        Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
            ModuleType::JavaScript,
            ModuleSourceCode::String(stub_content.into()),
            module_specifier,
            None,
        ))))
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
                .cow_replace(".js", "");

            if let Some(code) = self.storage.get_module_code(specifier_str) {
                return Some(ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                    ModuleType::JavaScript,
                    ModuleSourceCode::String(code.into()),
                    module_specifier,
                    None,
                ))));
            }

            for entry in &self.component_specifiers {
                let component_id = entry.key();
                let specifier = entry.value();
                if (component_id == component_name.as_ref()
                    || specifier.contains(component_name.as_ref()))
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
                for entry in &self.component_specifiers {
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

    fn resolve_rari_component_relative(
        &self,
        specifier: &str,
        referrer: &str,
    ) -> Option<Result<ModuleSpecifier, JsErrorBox>> {
        if !referrer.contains(RARI_COMPONENT_PATH) {
            return None;
        }

        let source_path = self.module_caching.get_component_source_path(referrer)?;
        let source_dir = if source_path.contains('/') {
            source_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
        } else {
            ""
        };

        let resolved_path = if specifier.starts_with("../") {
            let remaining = specifier.strip_prefix("../").unwrap_or(specifier);
            let source_path = Path::new(source_dir);
            let parent_dir = source_path.parent().unwrap_or_else(|| Path::new(""));

            let (base_with_ext, suffix) = append_extension_only(remaining);
            let mut file_url = path_to_file_url(&parent_dir.join(base_with_ext));
            file_url.push_str(suffix);
            file_url
        } else {
            let remaining = specifier.strip_prefix("./").unwrap_or(specifier);

            let (base_with_ext, suffix) = append_extension_only(remaining);
            let mut file_url = path_to_file_url(&Path::new(source_dir).join(base_with_ext));
            file_url.push_str(suffix);
            file_url
        };

        Some(
            ModuleSpecifier::parse(&resolved_path)
                .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}"))),
        )
    }

    fn resolve_functions_special(&self) -> Result<ModuleSpecifier, JsErrorBox> {
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
                return ModuleSpecifier::parse(functions_specifier.value())
                    .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")));
            }
        }
        Err(JsErrorBox::generic("Module not found"))
    }
}

impl Default for RariModuleLoader {
    fn default() -> Self {
        Self::new(Arc::new(Resolver::default()))
    }
}

impl ModuleLoader for RariModuleLoader {
    #[expect(clippy::too_many_lines)]
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, JsErrorBox> {
        if referrer.starts_with("ext:")
            && (specifier.starts_with("./") || specifier.starts_with("../"))
        {
            if let Ok(referrer_url) = ModuleSpecifier::parse(referrer)
                && let Ok(resolved_url) = referrer_url.join(specifier)
            {
                return Ok(resolved_url);
            }
        }

        // App `file://` modules must not resolve to `ext:` (deno_core 0.408+).
        // Map to `node:rari/react-vendor/*` shims that `export *` from the real
        // `ext:rari/react/vendor/*` modules (`node:` may import `ext:`).
        if specifier.contains("/react_vendor/")
            || specifier.starts_with(react_vendor::NODE_VENDOR_PREFIX)
        {
            let raw_name = specifier
                .strip_prefix(react_vendor::NODE_VENDOR_PREFIX)
                .or_else(|| specifier.rsplit("/react_vendor/").next())
                .unwrap_or("");
            let Some(module_name) = react_vendor::normalize_vendor_module_name(raw_name) else {
                return Err(JsErrorBox::generic(format!(
                    "Unknown React vendor module: {raw_name}"
                )));
            };
            let url = ModuleSpecifier::parse(&react_vendor::node_vendor_specifier(&module_name))
                .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;
            return Ok(url);
        }

        if matches!(kind, ResolutionKind::DynamicImport)
            && referrer.contains("node_modules")
            && let Some(package_start) = referrer.rfind("node_modules/")
        {
            let after_node_modules = &referrer[package_start + 13..];
            if after_node_modules.find('/').is_some()
                && (specifier.starts_with("./") || specifier.starts_with("../"))
            {
                let Some(referrer_dir) = Path::new(referrer).parent() else {
                    return Err(JsErrorBox::generic("Module not found"));
                };
                let resolved_path = referrer_dir.join(specifier);

                if let Ok(canonical) = resolved_path.canonicalize() {
                    if Self::is_rari_mdx_registry_stub(&canonical) {
                        return ModuleSpecifier::parse(RARI_MDX_REGISTRY_INTERNAL)
                            .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")));
                    }

                    if let Ok(url) = ModuleSpecifier::from_file_path(canonical) {
                        return Ok(url);
                    }
                }
            }
        }

        if specifier.starts_with(FILE_PROTOCOL) {
            let url = ModuleSpecifier::parse(specifier)
                .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))?;
            return Ok(url);
        }

        if specifier == "../functions" {
            return self.resolve_functions_special();
        }

        if (specifier.starts_with("./") || specifier.starts_with("../"))
            && let Some(result) = self.resolve_rari_component_relative(specifier, referrer)
        {
            return result;
        }

        if specifier.starts_with(NODE_PREFIX) {
            return ModuleSpecifier::parse(specifier)
                .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")));
        }

        if let Some(component_specifier) = self.get_component_specifier(specifier) {
            return self.resolve(&component_specifier, referrer, kind);
        }

        if !specifier.contains("://") && !specifier.starts_with('/') {
            if specifier == "react" || specifier.starts_with("react/") {
                let react_url = if matches!(
                    specifier,
                    "react/jsx-runtime"
                        | "react/jsx-runtime.js"
                        | "react/jsx-dev-runtime"
                        | "react/jsx-dev-runtime.js"
                ) {
                    react_vendor::node_vendor_specifier("react-jsx-runtime.js")
                } else {
                    react_vendor::node_vendor_specifier("react.js")
                };
                return self.resolve(&react_url, referrer, kind);
            }

            if matches!(
                specifier,
                "react-dom/server" | "react-dom/server.browser" | "react-dom/server.node"
            ) || (specifier.starts_with("react-dom/")
                && !matches!(specifier, "react-dom/client" | "react-dom/client.js" | "react-dom"))
            {
                return self.resolve(
                    &react_vendor::node_vendor_specifier("react-dom-server.js"),
                    referrer,
                    kind,
                );
            }

            if matches!(specifier, "react-dom") {
                return self.resolve(
                    &react_vendor::node_vendor_specifier("react-dom.js"),
                    referrer,
                    kind,
                );
            }

            if matches!(
                specifier,
                "react-server-dom-webpack/server"
                    | "react-server-dom-webpack/server.browser"
                    | "react-server-dom-webpack/server.node"
                    | "react-server-dom-webpack/server.edge"
            ) {
                return self.resolve(
                    &react_vendor::node_vendor_specifier("react-server-dom-webpack-server.js"),
                    referrer,
                    kind,
                );
            }

            if matches!(
                specifier,
                "react-server-dom-webpack/client"
                    | "react-server-dom-webpack/client.browser"
                    | "react-server-dom-webpack/client.node"
                    | "react-server-dom-webpack/client.edge"
            ) {
                return self.resolve(
                    &react_vendor::node_vendor_specifier("react-server-dom-webpack-client.js"),
                    referrer,
                    kind,
                );
            }

            if specifier == "rari" || specifier.starts_with("rari/") {
                let is_ssr_context = referrer.contains("/ssr/");
                if is_ssr_context {
                    let subpath = specifier.strip_prefix("rari").unwrap_or("");
                    let rari_url = format!(
                        "file:///rari_stub{}.js",
                        if subpath.is_empty() { "/index" } else { subpath }
                    );
                    return self.resolve(&rari_url, referrer, kind);
                }
            }

            if specifier == RSC_REFERENCES_SPECIFIER {
                return self.resolve_via_node_resolver(RARI_RSC_REFERENCES_EXPORT, referrer);
            }

            if specifier == RARI_MDX_REGISTRY_SPECIFIER {
                return ModuleSpecifier::parse(RARI_MDX_REGISTRY_INTERNAL)
                    .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")));
            }
        }

        if (specifier.starts_with("./") || specifier.starts_with("../"))
            && referrer.starts_with(FILE_PROTOCOL)
            && !is_virtual_referrer(referrer)
        {
            if referrer.contains("node_modules") {
                return self.resolve_via_node_resolver(specifier, referrer);
            }
            match resolve_import(specifier, referrer) {
                Ok(url) => return Ok(url),
                Err(_) => {}
            }
        }

        // Bare package names / subpaths — Deno NodeResolver / BYONM.
        // Scheme URLs (`ext:`, `file:`, `data:`, …) must NOT go through Node:
        // that produces ERR_UNSUPPORTED_ESM_URL_SCHEME and breaks snapshot bootstrap.
        if !specifier.contains(':') && !specifier.starts_with('.') && !specifier.starts_with('/') {
            return self.resolve_via_node_resolver(specifier, referrer);
        }

        ModuleSpecifier::parse(specifier)
            .map_err(|err| JsErrorBox::generic(format!("Invalid URL: {err}")))
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleLoadReferrer>,
        options: ModuleLoadOptions,
    ) -> ModuleLoadResponse {
        let specifier_str = module_specifier.to_string();
        let is_dyn_import = options.is_dynamic_import;

        if let Some(response) = self.handle_cached_module(&specifier_str, module_specifier) {
            return response;
        }

        let maybe_referrer_spec = maybe_referrer.map(|r| r.specifier.clone());
        if let Some(response) = Self::handle_dynamic_import_validation(
            &specifier_str,
            maybe_referrer_spec.as_ref(),
            is_dyn_import,
        ) {
            return response;
        }

        if let Some(response) = self.handle_version_query(&specifier_str, module_specifier) {
            return response;
        }

        if specifier_str == DENO_GLOBAL_SCOPE_SHARED {
            return ModuleLoadResponse::Sync(Ok(ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(NODE_CONSOLE_SCOPE_SOURCE.to_owned().into()),
                module_specifier,
                None,
            )));
        }

        if let Some(response) = self.handle_rari_internal_modules(&specifier_str, module_specifier)
        {
            return response;
        }

        if let Some(response) = Self::handle_rari_stub_modules(&specifier_str, module_specifier) {
            return response;
        }

        if let Some(response) = Self::handle_react_vendor_shim(&specifier_str, module_specifier) {
            return response;
        }

        if let Some(response) = self.handle_file_protocol_modules(&specifier_str, module_specifier)
        {
            return response;
        }

        if let Some(response) = self.handle_rari_component_modules(&specifier_str, module_specifier)
        {
            return response;
        }

        ModuleLoadResponse::Sync(Err(JsErrorBox::generic(format!(
            "Module not found: {specifier_str}"
        ))))
    }
}
