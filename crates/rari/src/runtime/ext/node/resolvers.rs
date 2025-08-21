use deno_ast::{MediaType, ModuleSpecifier};
use deno_error::JsErrorBox;
use deno_fs::{FileSystem, RealFs};
use deno_node::{NodeExtInitServices, NodeRequireLoader, NodeResolver};
use deno_package_json::{PackageJsonCache, PackageJsonRc};
use deno_process::NpmProcessStateProvider;
use deno_resolver::npm::{
    ByonmInNpmPackageChecker, ByonmNpmResolver, ByonmNpmResolverCreateOptions,
    DenoInNpmPackageChecker,
};
use deno_semver::package::PackageReq;
use node_resolver::{
    DenoIsBuiltInNodeModuleChecker, InNpmPackageChecker, NodeConditionOptions, NodeResolutionCache,
    NpmPackageFolderResolver, PackageJsonResolver, UrlOrPath, UrlOrPathRef,
    cache::NodeResolutionSys,
    errors::{
        ClosestPkgJsonError, PackageFolderResolveError, PackageFolderResolveErrorKind,
        PackageNotFoundError,
    },
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, RwLock},
};
use sys_traits::impls::RealSys;

const NODE_MODULES_DIR: &str = "node_modules";
const TYPESCRIPT_VERSION: &str = "5.8.3";

#[derive(Debug)]
pub struct Resolver {
    in_pkg_checker: DenoInNpmPackageChecker,
    folder_resolver: NpmPackageFolderResolverImpl,
    fs: Arc<dyn FileSystem + Send + Sync>,

    require_loader: RequireLoader,
    known: RwLock<FxHashMap<ModuleSpecifier, bool>>,
}
impl Default for Resolver {
    fn default() -> Self {
        Self::new(None, Arc::new(RealFs))
    }
}
impl Resolver {
    pub fn new(base_dir: Option<PathBuf>, fs: Arc<dyn FileSystem + Send + Sync>) -> Self {
        let folder_resolver = NpmPackageFolderResolverImpl::new(base_dir);
        let in_pkg_checker = DenoInNpmPackageChecker::Byonm(ByonmInNpmPackageChecker);
        let require_loader = RequireLoader(fs.clone());

        Self {
            in_pkg_checker,
            folder_resolver,
            fs,

            require_loader,
            known: RwLock::new(FxHashMap::default()),
        }
    }

    pub fn node_resolver(
        self: &Arc<Self>,
    ) -> Arc<NodeResolver<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys>> {
        NodeResolver::new(
            self.in_pkg_checker.clone(),
            DenoIsBuiltInNodeModuleChecker,
            self.folder_resolver.clone(),
            self.folder_resolver.pjson_resolver(),
            NodeResolutionSys::new(RealSys, Some(self.folder_resolver.resolution_cache())),
            node_resolver::NodeResolverOptions {
                conditions: NodeConditionOptions::default(),
                is_browser_platform: false,
                bundle_mode: false,
                typescript_version: Some(
                    deno_semver::Version::parse_standard(TYPESCRIPT_VERSION)
                        .expect("failed to parse typescript version"),
                ),
            },
        )
        .into()
    }

    pub fn code_translator(
        self: &Arc<Self>,
        node_resolver: Arc<
            NodeResolver<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys>,
        >,
    ) -> super::cjs_translator::NodeCodeTranslator {
        use super::cjs_translator::CjsCodeAnalyzer;
        use node_resolver::analyze::CjsModuleExportAnalyzer;

        let cjs = CjsCodeAnalyzer::new(self.filesystem(), self.clone());

        let module_export_analyzer = CjsModuleExportAnalyzer::new(
            cjs,
            self.in_pkg_checker.clone(),
            node_resolver,
            self.folder_resolver.clone(),
            self.package_json_resolver(),
            RealSys,
        );

        super::cjs_translator::NodeCodeTranslator::new(
            module_export_analyzer.into(),
            node_resolver::analyze::NodeCodeTranslatorMode::ModuleLoader,
        )
    }

    pub fn package_json_resolver(&self) -> Arc<PackageJsonResolver<RealSys>> {
        self.folder_resolver.pjson_resolver()
    }

    fn get_known_is_cjs(&self, specifier: &ModuleSpecifier) -> Option<bool> {
        self.known.read().ok().and_then(|k| k.get(specifier).copied())
    }

    fn set_is_cjs(&self, specifier: &ModuleSpecifier, value: bool) {
        if let Ok(mut known) = self.known.write() {
            known.insert(specifier.clone(), value);
        }
    }

    fn check_based_on_pkg_json(
        &self,
        specifier: &ModuleSpecifier,
    ) -> Result<bool, ClosestPkgJsonError> {
        let pjson = self.folder_resolver.pjson_resolver();

        let Ok(path) = specifier.to_file_path() else {
            return Ok(false);
        };

        if self.in_pkg_checker.in_npm_package(specifier) {
            if let Some(pkg_json) = pjson.get_closest_package_json(&path)? {
                let is_file_location_cjs = pkg_json.typ != "module";
                Ok(is_file_location_cjs)
            } else {
                Ok(true)
            }
        } else if let Some(pkg_json) = pjson.get_closest_package_json(&path)? {
            let is_cjs_type = pkg_json.typ == "commonjs";
            Ok(is_cjs_type)
        } else {
            Ok(false)
        }
    }

    pub fn is_cjs(
        &self,
        specifier: &ModuleSpecifier,
        media_type: MediaType,
        is_script: bool,
    ) -> bool {
        if specifier.scheme() != "file" {
            return false;
        }

        match media_type {
            MediaType::Wasm
            | MediaType::Json
            | MediaType::Mts
            | MediaType::Mjs
            | MediaType::Html
            | MediaType::Sql
            | MediaType::Dmts => false,

            MediaType::Cjs | MediaType::Cts | MediaType::Dcts => true,

            MediaType::Dts => {
                if let Some(value) = self.get_known_is_cjs(specifier) {
                    value
                } else {
                    let value = self.check_based_on_pkg_json(specifier).ok();
                    if let Some(value) = value {
                        self.set_is_cjs(specifier, value);
                    }
                    value.unwrap_or(false)
                }
            }

            MediaType::JavaScript
            | MediaType::Jsx
            | MediaType::TypeScript
            | MediaType::Tsx
            | MediaType::Css
            | MediaType::SourceMap
            | MediaType::Unknown => {
                if let Some(value) = self.get_known_is_cjs(specifier) {
                    if value && !is_script {
                        self.set_is_cjs(specifier, false);
                        false
                    } else {
                        value
                    }
                } else if !is_script {
                    self.set_is_cjs(specifier, false);
                    false
                } else {
                    let value = self.check_based_on_pkg_json(specifier).ok();
                    if let Some(value) = value {
                        self.set_is_cjs(specifier, value);
                    }
                    value.unwrap_or(false)
                }
            }
        }
    }

    pub fn has_node_modules_dir(&self) -> bool {
        self.folder_resolver.base_dir().as_ref().is_some_and(|d| {
            let checked_path = deno_permissions::CheckedPath::unsafe_new(Cow::Borrowed(d));
            self.fs.exists_sync(&checked_path) && self.fs.is_dir_sync(&checked_path)
        })
    }

    pub fn in_npm_package(&self, specifier: &ModuleSpecifier) -> bool {
        self.in_pkg_checker.in_npm_package(specifier)
    }

    pub fn filesystem(&self) -> Arc<dyn FileSystem + Send + Sync> {
        self.fs.clone()
    }

    pub fn init_services(
        self: &Arc<Self>,
    ) -> NodeExtInitServices<DenoInNpmPackageChecker, NpmPackageFolderResolverImpl, RealSys> {
        NodeExtInitServices {
            node_require_loader: Rc::new(self.require_loader.clone()),
            node_resolver: self.node_resolver(),
            pkg_json_resolver: self.package_json_resolver(),
            sys: RealSys,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NpmPackageFolderResolverImpl {
    byonm: ByonmNpmResolver<RealSys>,
    pjson: Arc<PackageJsonResolver<RealSys>>,
    resolution_cache: Arc<NodeResolutionCacheImpl>,
    base_dir: Option<PathBuf>,
}
impl NpmPackageFolderResolverImpl {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        let base = base_dir.or(std::env::current_dir().ok());
        let base_dir = base.map(|mut p| {
            p.push(NODE_MODULES_DIR);
            p
        });

        let resolution_cache = Arc::new(NodeResolutionCacheImpl::default());
        let pjson = Arc::new(PackageJsonResolver::new(
            RealSys,
            Some(Arc::new(PackageJsonCacheImpl::new())),
        ));

        let options = ByonmNpmResolverCreateOptions {
            sys: NodeResolutionSys::new(RealSys, Some(resolution_cache.clone())),
            root_node_modules_dir: base_dir.clone(),
            pkg_json_resolver: pjson.clone(),
        };

        let byonm = ByonmNpmResolver::new(options);

        Self { byonm, pjson, resolution_cache, base_dir }
    }

    pub fn npm_resolver(&self) -> ByonmNpmResolver<RealSys> {
        self.byonm.clone()
    }

    pub fn pjson_resolver(&self) -> Arc<PackageJsonResolver<RealSys>> {
        self.pjson.clone()
    }

    pub fn resolution_cache(&self) -> Arc<NodeResolutionCacheImpl> {
        self.resolution_cache.clone()
    }

    pub fn base_dir(&self) -> Option<&Path> {
        self.base_dir.as_deref()
    }
}
impl NpmPackageFolderResolver for NpmPackageFolderResolverImpl {
    fn resolve_package_folder_from_package(
        &self,
        specifier: &str,
        referrer: &UrlOrPathRef,
    ) -> Result<PathBuf, PackageFolderResolveError> {
        let referrer_url = match referrer.url() {
            Ok(url) => url,
            Err(e) => {
                let kind = PackageFolderResolveErrorKind::PathToUrl(e);
                return Err(PackageFolderResolveError(Box::new(kind)));
            }
        };

        let request = PackageReq::from_str(specifier).map_err(|_| {
            let e =
                Box::new(PackageFolderResolveErrorKind::PackageNotFound(PackageNotFoundError {
                    package_name: specifier.to_string(),
                    referrer: UrlOrPath::Url(referrer_url.clone()),
                    referrer_extra: None,
                }));
            PackageFolderResolveError(e)
        })?;

        let p = self.byonm.resolve_pkg_folder_from_deno_module_req(&request, referrer_url);
        match p {
            Ok(p) => Ok(p),
            Err(_) => self.byonm.resolve_package_folder_from_package(specifier, referrer),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PackageJsonCacheImpl(Arc<RwLock<PackageJsonCacheInner>>);
impl PackageJsonCacheImpl {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(PackageJsonCacheInner::default())))
    }
}
impl PackageJsonCache for PackageJsonCacheImpl {
    fn get(&self, path: &Path) -> Option<PackageJsonRc> {
        self.0.read().ok().and_then(|i| i.get(path))
    }

    fn set(&self, path: PathBuf, package_json: PackageJsonRc) {
        if let Ok(mut i) = self.0.write() {
            i.set(path, package_json);
        }
    }
}
#[derive(Debug, Default, Clone)]
pub struct PackageJsonCacheInner {
    cache: FxHashMap<PathBuf, PackageJsonRc>,
}
impl PackageJsonCacheInner {
    fn get(&self, path: &Path) -> Option<PackageJsonRc> {
        self.cache.get(path).cloned()
    }
    fn set(&mut self, path: PathBuf, package_json: PackageJsonRc) {
        self.cache.insert(path, package_json);
    }
}

#[derive(Debug, Clone)]
pub struct NodeResolutionCacheImpl {
    inner: Arc<RwLock<NodeResolutionCacheInner>>,
}
impl Default for NodeResolutionCacheImpl {
    fn default() -> Self {
        Self { inner: Arc::new(RwLock::new(NodeResolutionCacheInner::default())) }
    }
}
impl NodeResolutionCache for NodeResolutionCacheImpl {
    fn get_canonicalized(&self, path: &Path) -> Option<Result<PathBuf, std::io::Error>> {
        self.inner.read().ok().and_then(|i| i.get_canonicalized(path))
    }

    fn set_canonicalized(&self, from: PathBuf, to: &std::io::Result<PathBuf>) {
        if let Ok(mut i) = self.inner.write() {
            i.set_canonicalized(from, to);
        }
    }

    fn get_file_type(&self, path: &Path) -> Option<Option<sys_traits::FileType>> {
        self.inner.read().ok().and_then(|i| i.get_file_type(path))
    }

    fn set_file_type(&self, path: PathBuf, value: Option<sys_traits::FileType>) {
        if let Ok(mut i) = self.inner.write() {
            i.set_file_type(path, value);
        }
    }
}
#[derive(Debug, Default, Clone)]
pub struct NodeResolutionCacheInner {
    cache: FxHashMap<PathBuf, (Option<PathBuf>, Option<sys_traits::FileType>)>,
}
impl NodeResolutionCacheInner {
    fn get_canonicalized(&self, path: &Path) -> Option<Result<PathBuf, std::io::Error>> {
        self.cache.get(path).map(|(t, _)| {
            t.clone().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Not found."))
        })
    }

    fn set_canonicalized(&mut self, from: PathBuf, to: &std::io::Result<PathBuf>) {
        let canon = match to {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Ok(p) => Some(p.clone()),
            _ => return,
        };

        if let Some((t, _)) = self.cache.get_mut(&from) {
            *t = canon;
        } else {
            self.cache.insert(from, (canon, None));
        }
    }

    #[allow(clippy::option_option)]
    fn get_file_type(&self, path: &Path) -> Option<Option<sys_traits::FileType>> {
        self.cache.get(path).map(|(_, t)| *t)
    }

    fn set_file_type(&mut self, path: PathBuf, value: Option<sys_traits::FileType>) {
        if let Some((_, t)) = self.cache.get_mut(&path) {
            *t = value;
        } else {
            self.cache.insert(path, (None, value));
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NpmProcessState {
    pub kind: NpmProcessStateKind,
    pub local_node_modules_path: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NpmProcessStateKind {
    Byonm,
}
impl NpmProcessStateProvider for Resolver {
    fn get_npm_process_state(&self) -> String {
        let modules_path =
            self.folder_resolver.base_dir().as_ref().map(|p| p.to_string_lossy().to_string());
        let state = NpmProcessState {
            kind: NpmProcessStateKind::Byonm,
            local_node_modules_path: modules_path,
        };
        deno_core::serde_json::to_string(&state).unwrap_or_default()
    }
}

#[derive(Debug)]
struct RequireLoader(Arc<dyn FileSystem + Send + Sync>);
impl NodeRequireLoader for RequireLoader {
    fn load_text_file_lossy(&self, path: &Path) -> Result<deno_core::FastString, JsErrorBox> {
        let _media_type = MediaType::from_path(path);
        let path_checked = deno_permissions::CheckedPath::unsafe_new(Cow::Borrowed(path));
        let text = self.0.read_text_file_lossy_sync(&path_checked).map_err(JsErrorBox::from_err)?;
        Ok(deno_core::FastString::from(text.into_owned()))
    }

    fn ensure_read_permission<'a>(
        &self,
        permissions: &mut dyn deno_node::NodePermissions,
        path: Cow<'a, std::path::Path>,
    ) -> Result<std::borrow::Cow<'a, Path>, JsErrorBox> {
        let is_in_node_modules =
            path.components().all(|c| c.as_os_str().to_ascii_lowercase() != NODE_MODULES_DIR);
        if is_in_node_modules {
            let checked_path = permissions
                .check_open(path, deno_permissions::OpenAccessKind::Read, None)
                .map_err(JsErrorBox::from_err)?;
            Ok(Cow::Owned(checked_path.to_path_buf()))
        } else {
            Ok(path)
        }
    }

    fn is_maybe_cjs(&self, specifier: &reqwest::Url) -> Result<bool, ClosestPkgJsonError> {
        if specifier.scheme() != "file" {
            return Ok(false);
        }

        match MediaType::from_specifier(specifier) {
            MediaType::Wasm
            | MediaType::Json
            | MediaType::Mts
            | MediaType::Mjs
            | MediaType::Dmts => Ok(false),

            _ => Ok(true),
        }
    }
}
impl Clone for RequireLoader {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
