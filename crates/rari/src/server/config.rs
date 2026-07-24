#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]

use std::{
    cmp::Ordering,
    env,
    fmt::{self, Display, Formatter},
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use cow_utils::CowUtils;
use http::HeaderValue;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::server::{
    cache::handler::MemoryConfig, image::ImageConfig,
    rendering::html_bots::compile_html_limited_bots_pattern,
};

pub static GLOBAL_CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum Mode {
    #[default]
    Development,
    Production,
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub origin: Option<String>,
    pub enable_logging: bool,
    pub timeout_seconds: u64,
    #[serde(default = "default_js_pool_size")]
    pub js_pool_size: usize,
}

fn default_js_pool_size() -> usize {
    1
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            origin: None,
            enable_logging: true,
            timeout_seconds: 30,
            js_pool_size: default_js_pool_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: u32,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self { allowed_origins: vec![], allow_credentials: true, max_age: 86400 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RedirectConfig {
    pub allowed_hosts: Vec<String>,
    pub allow_relative: bool,
    pub allow_subdomains: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[non_exhaustive]
pub struct CacheLayerConfig {
    pub handler: String,
    pub url: Option<String>,
    pub max_entries: usize,
    pub default_ttl_secs: u64,
    /// Total payload byte budget for memory layers. `0` = unlimited.
    pub max_bytes: usize,
}

impl Default for CacheLayerConfig {
    fn default() -> Self {
        Self {
            handler: "memory".to_string(),
            url: None,
            max_entries: 1000,
            default_ttl_secs: 60,
            max_bytes: 0,
        }
    }
}

impl CacheLayerConfig {
    pub fn memory_config(&self) -> MemoryConfig {
        MemoryConfig {
            max_entries: self.max_entries.max(1),
            default_ttl: self.default_ttl_secs,
            max_bytes: self.max_bytes,
        }
    }
}

pub const CACHE_LAYER_RESPONSE: &str = "response";
pub const CACHE_LAYER_IMAGE: &str = "image";
pub const CACHE_LAYER_OG: &str = "og";
pub const CACHE_LAYER_LAYOUT: &str = "layout";
pub const CACHE_LAYER_MODULE: &str = "module";
pub const CACHE_LAYER_FETCH: &str = "fetch";

pub fn default_cache_layers() -> FxHashMap<String, CacheLayerConfig> {
    let mut layers = FxHashMap::default();
    layers.insert(CACHE_LAYER_RESPONSE.to_string(), CacheLayerConfig::default());
    layers.insert(CACHE_LAYER_IMAGE.to_string(), CacheLayerConfig::default());
    layers.insert(CACHE_LAYER_OG.to_string(), CacheLayerConfig::default());
    layers.insert(CACHE_LAYER_LAYOUT.to_string(), CacheLayerConfig::default());
    layers.insert(CACHE_LAYER_MODULE.to_string(), CacheLayerConfig::default());
    layers.insert(CACHE_LAYER_FETCH.to_string(), CacheLayerConfig::default());
    layers
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CacheConfig {
    #[serde(default = "default_cache_layers")]
    pub layers: FxHashMap<String, CacheLayerConfig>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self { layers: default_cache_layers() }
    }
}

impl CacheConfig {
    pub fn layer(&self, name: &str) -> CacheLayerConfig {
        self.layers.get(name).cloned().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UseCacheConfig {
    #[serde(default)]
    pub remote: Option<CacheLayerConfig>,
    #[serde(default, rename = "buildId")]
    pub build_id: Option<String>,
}

impl Default for RedirectConfig {
    fn default() -> Self {
        Self { allowed_hosts: vec![], allow_relative: true, allow_subdomains: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct ActionConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CspConfig {
    pub script_src: Vec<String>,
    pub style_src: Vec<String>,
    pub img_src: Vec<String>,
    pub font_src: Vec<String>,
    pub connect_src: Vec<String>,
    pub default_src: Vec<String>,
    pub worker_src: Vec<String>,
    #[serde(default = "csp_default_frame_ancestors")]
    pub frame_ancestors: Vec<String>,
    #[serde(default = "csp_default_base_uri")]
    pub base_uri: Vec<String>,
    #[serde(default = "csp_default_form_action")]
    pub form_action: Vec<String>,
    #[serde(default)]
    pub use_nonces: bool,
}

fn csp_default_frame_ancestors() -> Vec<String> {
    vec!["'self'".to_string()]
}

fn csp_default_base_uri() -> Vec<String> {
    vec!["'self'".to_string()]
}

fn csp_default_form_action() -> Vec<String> {
    vec!["'self'".to_string()]
}

impl Default for CspConfig {
    fn default() -> Self {
        Self {
            default_src: vec!["'self'".to_string()],
            script_src: vec!["'self'".to_string()],
            style_src: vec!["'self'".to_string()],
            img_src: vec!["'self'".to_string(), "data:".to_string(), "https:".to_string()],
            font_src: vec!["'self'".to_string(), "data:".to_string()],
            connect_src: vec!["'self'".to_string(), "ws:".to_string(), "wss:".to_string()],
            worker_src: vec!["'self'".to_string()],
            frame_ancestors: csp_default_frame_ancestors(),
            base_uri: csp_default_base_uri(),
            form_action: csp_default_form_action(),
            use_nonces: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ViteConfig {
    pub host: String,
    pub port: u16,
    pub enable_hmr_proxy: bool,
    pub ws_protocol: String,
}

impl Default for ViteConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5173,
            enable_hmr_proxy: true,
            ws_protocol: "vite-hmr".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StaticConfig {
    pub dev_public_dir: PathBuf,
    pub prod_public_dir: PathBuf,
    pub enable_directory_listing: bool,
    pub cache_control: String,
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            dev_public_dir: PathBuf::from("public"),
            prod_public_dir: PathBuf::from("dist"),
            enable_directory_listing: false,
            cache_control: "public, max-age=31536000".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
enum RoutePattern {
    Exact(String),
    Prefix(String),
    Regex(regex::Regex),
}

impl RoutePattern {
    fn from_pattern(pattern: &str) -> Self {
        if let Some(prefix) = pattern.strip_suffix("/*") {
            Self::Prefix(prefix.to_string())
        } else if pattern.contains('*') {
            let escaped = regex::escape(pattern);
            let regex_pattern = escaped.cow_replace(r"\*", ".*");
            match regex::Regex::new(&format!("^{regex_pattern}$")) {
                Ok(regex) => Self::Regex(regex),
                Err(_) => Self::Exact(pattern.to_string()),
            }
        } else {
            Self::Exact(pattern.to_string())
        }
    }

    fn matches(&self, path: &str) -> bool {
        match self {
            Self::Exact(pattern) => pattern == path,
            Self::Prefix(prefix) => path == prefix || path.starts_with(&format!("{prefix}/")),
            Self::Regex(regex) => regex.is_match(path),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CacheControlConfig {
    pub routes: FxHashMap<String, String>,
    pub static_files: String,
    pub server_components: String,
}

#[derive(Debug, Clone)]
struct CompiledCacheControlConfig {
    routes: Vec<(RoutePattern, String)>,
}

impl From<&CacheControlConfig> for CompiledCacheControlConfig {
    fn from(config: &CacheControlConfig) -> Self {
        let mut routes: Vec<(RoutePattern, String)> = config
            .routes
            .iter()
            .map(|(pattern, cache_control)| {
                (RoutePattern::from_pattern(pattern), cache_control.clone())
            })
            .collect();

        routes.sort_by(|(a, _), (b, _)| {
            #[expect(
                clippy::match_same_arms,
                reason = "Explicit ordering is clearer than merged patterns for documentation"
            )]
            match (a, b) {
                (RoutePattern::Exact(_), RoutePattern::Exact(_)) => Ordering::Equal,
                (RoutePattern::Exact(_), _) => Ordering::Less,
                (_, RoutePattern::Exact(_)) => Ordering::Greater,
                (RoutePattern::Prefix(a_prefix), RoutePattern::Prefix(b_prefix)) => {
                    b_prefix.len().cmp(&a_prefix.len())
                }
                (RoutePattern::Prefix(_), RoutePattern::Regex(_)) => Ordering::Less,
                (RoutePattern::Regex(_), RoutePattern::Prefix(_)) => Ordering::Greater,
                (RoutePattern::Regex(_), RoutePattern::Regex(_)) => Ordering::Equal,
            }
        });

        Self { routes }
    }
}

impl Default for CacheControlConfig {
    fn default() -> Self {
        Self {
            routes: FxHashMap::default(),
            static_files: "public, max-age=31536000, immutable".to_string(),
            server_components: "public, max-age=31536000, stale-while-revalidate=86400".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RscHtmlConfig {
    pub enabled: bool,
    pub timeout_ms: u64,
    pub cache_template: bool,
}

impl Default for RscHtmlConfig {
    fn default() -> Self {
        Self { enabled: true, timeout_ms: 5000, cache_template: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LoadingConfig {
    pub enabled: bool,
    pub min_display_time_ms: u64,
    pub cache_loading_components: bool,
}

impl Default for LoadingConfig {
    fn default() -> Self {
        Self { enabled: true, min_display_time_ms: 200, cache_loading_components: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RscConfig {
    pub script_execution_timeout_ms: u64,
}

impl Default for RscConfig {
    fn default() -> Self {
        Self { script_execution_timeout_ms: 3000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct Config {
    pub mode: Mode,
    pub server: ServerConfig,
    pub vite: ViteConfig,
    pub static_files: StaticConfig,
    pub rsc: RscConfig,
    pub rsc_html: RscHtmlConfig,
    pub caching: CacheControlConfig,
    pub loading: LoadingConfig,
    #[serde(default)]
    pub cors: CorsConfig,
    #[serde(default)]
    pub redirect: RedirectConfig,
    #[serde(default)]
    pub action: ActionConfig,
    #[serde(default)]
    pub csp: CspConfig,
    #[serde(default)]
    pub images: ImageConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub use_cache: UseCacheConfig,
    #[serde(default, rename = "htmlLimitedBots")]
    pub html_limited_bots: Option<String>,
    /// Precompiled override from `html_limited_bots`; `None` uses the default list.
    #[serde(skip)]
    pub html_limited_bots_regex: Option<regex::Regex>,
}

impl Config {
    pub fn server_components_cache_control_for_mode(mode: Mode) -> String {
        if mode == Mode::Production {
            "public, max-age=31536000, stale-while-revalidate=86400".to_string()
        } else {
            "no-cache, no-store, must-revalidate".to_string()
        }
    }

    pub fn apply_mode_cache_control(&mut self) {
        self.caching.server_components = Self::server_components_cache_control_for_mode(self.mode);
    }

    pub fn load_from_env_for_mode(mode: Mode) -> Result<Self, ConfigError> {
        Self::load_from_env_with_base_for_mode(mode, None)
    }

    pub fn load_from_env_with_base_for_mode(
        mode: Mode,
        base: Option<&Path>,
    ) -> Result<Self, ConfigError> {
        let mut config = match Self::from_env_with_base(base) {
            Ok(config) => config,
            Err(_) => Self::new(mode),
        };
        config.mode = mode;
        config.apply_mode_cache_control();
        config.sanitize_use_cache_for_mode();
        Ok(config)
    }

    fn sanitize_use_cache_for_mode(&mut self) {
        if self.mode != Mode::Production {
            return;
        }

        let Some(remote) = self.use_cache.remote.as_ref() else {
            return;
        };

        if remote.handler == "test" {
            tracing::warn!(
                "Invalid useCache.remote: handler='test' is for e2e tests only and is not allowed in production. Ignoring remote cache config."
            );
            self.use_cache.remote = None;
        }
    }

    pub fn new(mode: Mode) -> Self {
        let default_config = Self::default();
        Self {
            mode,
            vite: ViteConfig {
                port: default_config.vite.port,
                host: default_config.server.host.clone(),
                ..default_config.vite
            },
            rsc: default_config.rsc,
            rsc_html: default_config.rsc_html,
            caching: CacheControlConfig {
                server_components: Self::server_components_cache_control_for_mode(mode),
                ..default_config.caching
            },
            ..default_config
        }
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_with_base(None)
    }

    pub fn from_env_with_base(base: Option<&Path>) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        if let Ok(mode_str) = env::var("RARI_MODE") {
            config.mode = match mode_str.cow_to_lowercase().as_ref() {
                "development" | "dev" => Mode::Development,
                "production" | "prod" => Mode::Production,
                _ => return Err(ConfigError::Mode(mode_str)),
            };
        }

        if let Ok(host) = env::var("RARI_HOST") {
            config.server.host = host;
        }

        if let Ok(port_str) = env::var("RARI_PORT") {
            config.server.port = port_str.parse().map_err(|_| ConfigError::Port(port_str))?;
        }

        if let Ok(origin) = env::var("RARI_ORIGIN") {
            config.server.origin = Some(origin);
        }

        if let Ok(vite_host) = env::var("RARI_VITE_HOST") {
            config.vite.host = vite_host;
        }

        if let Ok(vite_port_str) = env::var("RARI_VITE_PORT") {
            config.vite.port =
                vite_port_str.parse().map_err(|_| ConfigError::VitePort(vite_port_str))?;
        }

        if let Ok(public_dir) = env::var("RARI_PUBLIC_DIR") {
            config.static_files.dev_public_dir = PathBuf::from(public_dir);
        }

        if let Ok(dist_dir) = env::var("RARI_DIST_DIR") {
            config.static_files.prod_public_dir = PathBuf::from(dist_dir);
        }

        if let Ok(timeout_str) = env::var("RARI_SCRIPT_EXECUTION_TIMEOUT_MS") {
            config.rsc.script_execution_timeout_ms =
                timeout_str.parse().map_err(|_| ConfigError::Timeout(timeout_str.clone()))?;
        }

        if let Ok(rsc_html_enabled_str) = env::var("RARI_RSC_HTML_ENABLED") {
            config.rsc_html.enabled = rsc_html_enabled_str.cow_to_lowercase() == "true"
                || rsc_html_enabled_str == "1"
                || rsc_html_enabled_str.cow_to_lowercase() == "yes";
        }

        if let Ok(rsc_html_timeout_str) = env::var("RARI_RSC_HTML_TIMEOUT_MS") {
            config.rsc_html.timeout_ms = rsc_html_timeout_str
                .parse()
                .map_err(|_| ConfigError::Config("RARI_RSC_HTML_TIMEOUT_MS".to_string()))?;
        }

        if let Ok(rsc_html_cache_template_str) = env::var("RARI_RSC_HTML_CACHE_TEMPLATE") {
            config.rsc_html.cache_template = rsc_html_cache_template_str.cow_to_lowercase()
                == "true"
                || rsc_html_cache_template_str == "1"
                || rsc_html_cache_template_str.cow_to_lowercase() == "yes";
        }

        if let Ok(loading_enabled_str) = env::var("RARI_LOADING_ENABLED") {
            config.loading.enabled = loading_enabled_str.cow_to_lowercase() == "true"
                || loading_enabled_str == "1"
                || loading_enabled_str.cow_to_lowercase() == "yes";
        }

        if let Ok(min_display_time_str) = env::var("RARI_LOADING_MIN_DISPLAY_TIME_MS") {
            config.loading.min_display_time_ms = min_display_time_str
                .parse()
                .map_err(|_| ConfigError::Config("RARI_LOADING_MIN_DISPLAY_TIME_MS".to_string()))?;
        }

        if let Ok(cache_loading_str) = env::var("RARI_LOADING_CACHE_COMPONENTS") {
            config.loading.cache_loading_components = cache_loading_str.cow_to_lowercase()
                == "true"
                || cache_loading_str == "1"
                || cache_loading_str.cow_to_lowercase() == "yes";
        }

        let config_path = match base {
            Some(b) => b.join("dist/server/config.json"),
            None => PathBuf::from("dist/server/config.json"),
        };

        if let Ok(server_config_json) = fs::read_to_string(&config_path) {
            let config_data = match serde_json::from_str::<serde_json::Value>(&server_config_json) {
                Ok(data) => Some(data),
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse dist/server/config.json: {}. Using defaults.",
                        e
                    );
                    None
                }
            };
            if let Some(config_data) = config_data {
                if let Some(csp_data) = config_data.get("csp") {
                    if let Some(script_src) = csp_data.get("scriptSrc").and_then(|v| v.as_array()) {
                        config.csp.script_src = script_src
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(style_src) = csp_data.get("styleSrc").and_then(|v| v.as_array()) {
                        config.csp.style_src = style_src
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(img_src) = csp_data.get("imgSrc").and_then(|v| v.as_array()) {
                        config.csp.img_src = img_src
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(font_src) = csp_data.get("fontSrc").and_then(|v| v.as_array()) {
                        config.csp.font_src = font_src
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(connect_src) = csp_data.get("connectSrc").and_then(|v| v.as_array())
                    {
                        config.csp.connect_src = connect_src
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(default_src) = csp_data.get("defaultSrc").and_then(|v| v.as_array())
                    {
                        config.csp.default_src = default_src
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(worker_src) = csp_data.get("workerSrc").and_then(|v| v.as_array()) {
                        config.csp.worker_src = worker_src
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(frame_ancestors) =
                        csp_data.get("frameAncestors").and_then(|v| v.as_array())
                    {
                        config.csp.frame_ancestors = frame_ancestors
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(base_uri) = csp_data.get("baseUri").and_then(|v| v.as_array()) {
                        config.csp.base_uri = base_uri
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(form_action) = csp_data.get("formAction").and_then(|v| v.as_array())
                    {
                        config.csp.form_action = form_action
                            .iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect();
                    }
                    if let Some(use_nonces) = csp_data.get("useNonces").and_then(Value::as_bool) {
                        config.csp.use_nonces = use_nonces;
                    }
                }

                if let Some(action_data) = config_data.get("action")
                    && let Some(allowed_origins) =
                        action_data.get("allowedOrigins").and_then(|v| v.as_array())
                {
                    config.action.allowed_origins = allowed_origins
                        .iter()
                        .filter_map(|v| v.as_str().map(ToString::to_string))
                        .collect();
                }

                if let Some(pool_size) =
                    config_data.get("jsPoolSize").and_then(serde_json::Value::as_u64)
                {
                    match usize::try_from(pool_size) {
                        Ok(0) | Err(_) => {
                            tracing::warn!(
                                "jsPoolSize must be >= 1 and fit in usize; ignoring value from config.json"
                            );
                        }
                        Ok(size) => {
                            config.server.js_pool_size = size;
                        }
                    }
                }

                if let Some(pattern) =
                    config_data.get("htmlLimitedBots").and_then(serde_json::Value::as_str)
                {
                    match compile_html_limited_bots_pattern(pattern) {
                        Ok(re) => {
                            config.html_limited_bots = Some(pattern.to_string());
                            config.html_limited_bots_regex = Some(re);
                        }
                        Err(err) => {
                            tracing::warn!(
                                "Invalid htmlLimitedBots regex in config.json ({pattern:?}): {err}. Using default list."
                            );
                        }
                    }
                }

                if let Some(cache_control_data) = config_data.get("cacheControl")
                    && let Some(routes) =
                        cache_control_data.get("routes").and_then(|v| v.as_object())
                {
                    for (route, cache_value) in routes {
                        if let Some(cache_str) = cache_value.as_str() {
                            if HeaderValue::from_str(cache_str).is_ok() {
                                config.caching.routes.insert(route.clone(), cache_str.to_string());
                            } else {
                                tracing::warn!(
                                    "Invalid cache-control header value for route '{}': '{}' (contains invalid characters)",
                                    route,
                                    cache_str
                                );
                            }
                        } else {
                            tracing::warn!(
                                "Invalid cache-control value type for route '{}': expected string, got {:?}",
                                route,
                                cache_value
                            );
                        }
                    }
                }

                if let Some(cache_data) = config_data.get("cache")
                    && let Some(layers_data) = cache_data.get("layers").and_then(|v| v.as_object())
                {
                    for (layer_name, layer_value) in layers_data {
                        if !layer_value.is_object() {
                            tracing::warn!(
                                "Invalid cache.layers.{} value type: expected object, got {:?}",
                                layer_name,
                                layer_value
                            );
                            continue;
                        }
                        match serde_json::from_value::<CacheLayerConfig>(layer_value.clone()) {
                            Ok(layer) => {
                                config.cache.layers.insert(layer_name.clone(), layer);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to parse cache.layers.{}: {}. Using default.",
                                    layer_name,
                                    e
                                );
                            }
                        }
                    }
                }

                if let Some(use_cache_data) = config_data.get("useCache") {
                    if let Some(build_id) =
                        use_cache_data.get("buildId").and_then(|value| value.as_str())
                    {
                        config.use_cache.build_id = Some(build_id.to_string());
                    }

                    if let Some(remote_value) = use_cache_data.get("remote") {
                        match serde_json::from_value::<CacheLayerConfig>(remote_value.clone()) {
                            Ok(mut layer) => {
                                let trimmed_url = layer.url.as_deref().map(str::trim);
                                let missing_url = trimmed_url.is_none_or(str::is_empty);

                                match layer.handler.as_str() {
                                    "test" if config.mode == Mode::Production => {
                                        tracing::warn!(
                                            "Invalid useCache.remote: handler='test' is for e2e tests only and is not allowed in production. Ignoring remote cache config."
                                        );
                                    }
                                    "redis" | "redb" if missing_url => {
                                        tracing::warn!(
                                            "Invalid useCache.remote: handler={} requires a non-empty url. Ignoring remote cache config.",
                                            layer.handler
                                        );
                                    }
                                    "redis" | "redb" | "test" => {
                                        layer.url = trimmed_url.map(String::from);
                                        config.use_cache.remote = Some(layer);
                                    }
                                    _ => {
                                        tracing::warn!(
                                            "Invalid useCache.remote: handler='{}' is not supported (allowed: test, redis, redb). Ignoring remote cache config.",
                                            layer.handler
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to parse useCache.remote: {}. Using default.",
                                    e
                                );
                            }
                        }
                    }
                }
            } // if let Some(config_data)
        } else {
            tracing::debug!("No dist/server/config.json found, using defaults");
        }

        // Env wins over config.json for deploy-time overrides.
        if let Ok(pool_size_str) = env::var("RARI_JS_POOL_SIZE") {
            let pool_size: usize = pool_size_str
                .parse()
                .map_err(|_| ConfigError::Config("RARI_JS_POOL_SIZE".to_string()))?;
            if pool_size == 0 {
                return Err(ConfigError::Config("RARI_JS_POOL_SIZE must be >= 1".to_string()));
            }
            config.server.js_pool_size = pool_size;
        }

        if let Ok(pattern) = env::var("RARI_HTML_LIMITED_BOTS") {
            match compile_html_limited_bots_pattern(&pattern) {
                Ok(re) => {
                    config.html_limited_bots = Some(pattern);
                    config.html_limited_bots_regex = Some(re);
                }
                Err(err) => {
                    tracing::warn!(
                        "Invalid RARI_HTML_LIMITED_BOTS regex ({pattern:?}): {err}. Ignoring override."
                    );
                }
            }
        }

        if config.mode == Mode::Development {
            config.apply_mode_cache_control();
        }

        Ok(config)
    }

    pub fn get() -> Option<&'static Self> {
        GLOBAL_CONFIG.get()
    }

    pub fn set_global(config: Self) -> Result<(), Box<Self>> {
        GLOBAL_CONFIG.set(config).map_err(Box::new)
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn vite_address(&self) -> String {
        format!("{}:{}", self.vite.host, self.vite.port)
    }

    pub fn public_dir(&self) -> &PathBuf {
        match self.mode {
            Mode::Development => &self.static_files.dev_public_dir,
            Mode::Production => &self.static_files.prod_public_dir,
        }
    }

    pub fn is_development(&self) -> bool {
        self.mode == Mode::Development
    }

    pub fn is_production(&self) -> bool {
        self.mode == Mode::Production
    }

    pub fn cors_config(&self) -> CorsConfig {
        if !self.cors.allowed_origins.is_empty() {
            return self.cors.clone();
        }

        if self.is_development() {
            CorsConfig {
                allowed_origins: vec![
                    format!("http://{}:{}", self.server.host, self.server.port),
                    format!("http://localhost:{}", self.server.port),
                    format!("http://127.0.0.1:{}", self.server.port),
                    format!("http://{}:{}", self.vite.host, self.vite.port),
                    format!("http://localhost:{}", self.vite.port),
                ],
                allow_credentials: true,
                max_age: 86400,
            }
        } else {
            let allowed_origins = if let Some(origin) = &self.server.origin {
                vec![origin.clone()]
            } else {
                vec![format!("http://{}:{}", self.server.host, self.server.port)]
            };

            CorsConfig { allowed_origins, allow_credentials: true, max_age: 86400 }
        }
    }

    pub fn action_origins(&self) -> Vec<String> {
        if !self.action.allowed_origins.is_empty() {
            return self.action.allowed_origins.clone();
        }

        if self.is_development() {
            vec![
                format!("http://{}:{}", self.server.host, self.server.port),
                format!("http://localhost:{}", self.server.port),
                format!("http://127.0.0.1:{}", self.server.port),
                format!("http://{}:{}", self.vite.host, self.vite.port),
                format!("http://localhost:{}", self.vite.port),
            ]
        } else if let Some(origin) = &self.server.origin {
            vec![origin.clone()]
        } else {
            tracing::warn!(
                "No origin configured for server actions in production. \
                 Using same-origin validation (comparing origin/referer with host header). \
                 Set RARI_ORIGIN environment variable for explicit origin validation."
            );
            vec![]
        }
    }

    pub fn redirect_config(&self) -> RedirectConfig {
        if !self.redirect.allowed_hosts.is_empty() {
            return self.redirect.clone();
        }

        if self.is_development() {
            let mut allowed_hosts =
                vec!["localhost".to_string(), "127.0.0.1".to_string(), self.server.host.clone()];

            if let Some(origin) = &self.server.origin
                && let Ok(url) = url::Url::parse(origin)
                && let Some(host) = url.host_str()
            {
                allowed_hosts.push(host.to_string());
            }

            RedirectConfig { allowed_hosts, allow_relative: true, allow_subdomains: false }
        } else {
            let allowed_hosts = if let Some(origin) = &self.server.origin {
                if let Ok(url) = url::Url::parse(origin) {
                    if let Some(host) = url.host_str() { vec![host.to_string()] } else { vec![] }
                } else {
                    vec![]
                }
            } else {
                vec![self.server.host.clone()]
            };

            RedirectConfig { allowed_hosts, allow_relative: true, allow_subdomains: false }
        }
    }

    pub fn get_cache_control_for_route(&self, path: &str) -> &str {
        if let Some(cache_control) = self.caching.routes.get(path) {
            return cache_control;
        }

        let compiled = CompiledCacheControlConfig::from(&self.caching);

        for (pattern, cache_control) in &compiled.routes {
            if pattern.matches(path) {
                for orig_cache_control in self.caching.routes.values() {
                    if orig_cache_control == cache_control {
                        return orig_cache_control;
                    }
                }
            }
        }

        &self.caching.server_components
    }

    pub fn csp_config(&self) -> CspConfig {
        let mut config = self.csp.clone();

        if config.use_nonces {
            config.script_src.retain(|s| s != "'unsafe-inline'");
            config.script_src.push("'nonce-{{NONCE}}'".to_string());
            if !config.script_src.contains(&"'strict-dynamic'".to_string()) {
                config.script_src.push("'strict-dynamic'".to_string());
            }
        } else if !config.script_src.contains(&"'unsafe-inline'".to_string()) {
            config.script_src.push("'unsafe-inline'".to_string());
        }

        if !config.style_src.contains(&"'unsafe-inline'".to_string()) {
            config.style_src.push("'unsafe-inline'".to_string());
        }

        if self.is_development() && !config.script_src.contains(&"'unsafe-eval'".to_string()) {
            config.script_src.push("'unsafe-eval'".to_string());
        }

        config
    }

    pub fn build_csp_policy(&self) -> String {
        let config = self.csp_config();
        let mut directives = Vec::new();

        if !config.default_src.is_empty() {
            directives.push(format!("default-src {}", config.default_src.join(" ")));
        }

        if !config.script_src.is_empty() {
            directives.push(format!("script-src {}", config.script_src.join(" ")));
        }

        if !config.style_src.is_empty() {
            directives.push(format!("style-src {}", config.style_src.join(" ")));
        }

        if !config.img_src.is_empty() {
            directives.push(format!("img-src {}", config.img_src.join(" ")));
        }

        if !config.font_src.is_empty() {
            directives.push(format!("font-src {}", config.font_src.join(" ")));
        }

        if !config.connect_src.is_empty() {
            directives.push(format!("connect-src {}", config.connect_src.join(" ")));
        }

        if !config.worker_src.is_empty() {
            directives.push(format!("worker-src {}", config.worker_src.join(" ")));
        }

        if !config.frame_ancestors.is_empty() {
            directives.push(format!("frame-ancestors {}", config.frame_ancestors.join(" ")));
        }

        if !config.base_uri.is_empty() {
            directives.push(format!("base-uri {}", config.base_uri.join(" ")));
        }

        if !config.form_action.is_empty() {
            directives.push(format!("form-action {}", config.form_action.join(" ")));
        }

        directives.join("; ")
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("Invalid mode: {0}")]
    Mode(String),
    #[error("Invalid port: {0}")]
    Port(String),
    #[error("Invalid Vite port: {0}")]
    VitePort(String),
    #[error("Invalid timeout: {0}")]
    Timeout(String),
    #[error("Invalid config value for {0}")]
    Config(String),
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use std::{fs::File, io::Write, process};

    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.mode, Mode::Development);
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.vite.port, 5173);
    }

    #[test]
    fn test_server_address() {
        let config = Config::default();
        assert_eq!(config.server_address(), "127.0.0.1:3000");
    }

    #[test]
    fn test_vite_address() {
        let config = Config::default();
        assert_eq!(config.vite_address(), "localhost:5173");
    }

    #[test]
    fn test_public_dir() {
        let dev_config = Config::new(Mode::Development);
        assert_eq!(dev_config.public_dir(), &PathBuf::from("public"));

        let prod_config = Config::new(Mode::Production);
        assert_eq!(prod_config.public_dir(), &PathBuf::from("dist"));
    }

    #[test]
    fn test_mode_display() {
        assert_eq!(Mode::Development.to_string(), "development");
        assert_eq!(Mode::Production.to_string(), "production");
    }

    #[test]
    fn test_cache_control_exact_match() {
        let mut config = Config::default();
        config.caching.routes.insert("/api/users".to_string(), "no-cache".to_string());

        let cache_control = config.get_cache_control_for_route("/api/users");
        assert_eq!(cache_control, "no-cache");
    }

    #[test]
    fn test_cache_control_glob_pattern() {
        let mut config = Config::default();
        config.caching.routes.insert("/api/*".to_string(), "no-cache".to_string());

        let cache_control = config.get_cache_control_for_route("/api/users");
        assert_eq!(cache_control, "no-cache");

        let cache_control = config.get_cache_control_for_route("/api/products/123");
        assert_eq!(cache_control, "no-cache");
    }

    #[test]
    fn test_cache_control_default_fallback() {
        let config = Config::default();

        let cache_control = config.get_cache_control_for_route("/some/random/path");
        assert_eq!(cache_control, "public, max-age=31536000, stale-while-revalidate=86400");
    }

    #[test]
    fn test_cache_control_pattern_priority() {
        let mut config = Config::default();
        config.caching.routes.insert("/api/*".to_string(), "no-cache".to_string());
        config.caching.routes.insert("/api/public".to_string(), "public, max-age=3600".to_string());

        let cache_control = config.get_cache_control_for_route("/api/public");
        assert_eq!(cache_control, "public, max-age=3600");

        let cache_control = config.get_cache_control_for_route("/api/private");
        assert_eq!(cache_control, "no-cache");
    }

    #[test]
    fn test_pattern_matching() {
        let pattern = RoutePattern::from_pattern("/api/*");
        assert!(pattern.matches("/api/users"));
        assert!(pattern.matches("/api/products/123"));
        assert!(!pattern.matches("/blog/posts"));
        assert!(!pattern.matches("/apiv2/users"));
        assert!(pattern.matches("/api"));

        let pattern = RoutePattern::from_pattern("/blog/*/comments");
        assert!(pattern.matches("/blog/post-1/comments"));
        assert!(!pattern.matches("/blog/post-1"));

        let pattern = RoutePattern::from_pattern("*");
        assert!(pattern.matches("/any/path"));

        let pattern = RoutePattern::from_pattern("*.js");
        assert!(pattern.matches("/static/app.js"));
        assert!(!pattern.matches("/static/app.css"));

        let pattern = RoutePattern::from_pattern("/v1.0/*");
        assert!(pattern.matches("/v1.0/users"));
        assert!(!pattern.matches("/v1X0/users"));
    }

    #[test]
    fn test_cache_config_serialization() {
        let mut routes = FxHashMap::default();
        routes.insert("/api/*".to_string(), "no-cache".to_string());
        routes.insert("/blog/*".to_string(), "public, max-age=3600".to_string());

        let cache_config = CacheControlConfig {
            routes,
            static_files: "public, max-age=31536000, immutable".to_string(),
            server_components: "no-cache".to_string(),
        };

        let serialized = serde_json::to_string(&cache_config).unwrap();
        let deserialized: CacheControlConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(cache_config.static_files, deserialized.static_files);
        assert_eq!(cache_config.server_components, deserialized.server_components);
        assert_eq!(cache_config.routes.len(), deserialized.routes.len());
    }

    #[test]
    fn test_config_from_env_cache_control_validation() {
        let temp_dir = env::temp_dir().join(format!("rari_test_{}", process::id()));
        let dist_server_dir = temp_dir.join("dist").join("server");
        fs::create_dir_all(&dist_server_dir).unwrap();

        let config_json = serde_json::json!({
            "cacheControl": {
                "routes": {
                    "/valid": "public, max-age=3600",
                    "/invalid-newline": "public\nmax-age=3600",
                    "/invalid-type": 12345
                }
            }
        });

        let config_path = dist_server_dir.join("config.json");
        let mut file = File::create(&config_path).unwrap();
        file.write_all(config_json.to_string().as_bytes()).unwrap();

        let result = Config::from_env_with_base(Some(&temp_dir));

        let _ = fs::remove_dir_all(&temp_dir);

        let config = result.unwrap();

        assert!(
            config.caching.routes.contains_key("/valid"),
            "Valid cache-control route should be accepted"
        );
        assert_eq!(&config.caching.routes["/valid"], "public, max-age=3600");

        assert!(
            !config.caching.routes.contains_key("/invalid-newline"),
            "Cache-control with newline should be rejected by HeaderValue::from_str"
        );
        assert!(
            !config.caching.routes.contains_key("/invalid-type"),
            "Non-string cache-control value should be rejected"
        );
    }

    #[test]
    fn test_js_pool_size_config_json_and_env_precedence() {
        let temp_dir = env::temp_dir().join(format!("rari_test_pool_size_{}", process::id()));
        let dist_server_dir = temp_dir.join("dist").join("server");
        fs::create_dir_all(&dist_server_dir).unwrap();

        let prev = env::var("RARI_JS_POOL_SIZE").ok();
        // SAFETY: test-only env mutation; restored below. Keep both cases in one
        // test so parallel suite workers cannot race on this process-global var.
        unsafe { env::remove_var("RARI_JS_POOL_SIZE") };

        fs::write(dist_server_dir.join("config.json"), r#"{"jsPoolSize":3}"#).unwrap();
        let from_json = Config::from_env_with_base(Some(&temp_dir)).unwrap();
        assert_eq!(from_json.server.js_pool_size, 3);

        unsafe { env::set_var("RARI_JS_POOL_SIZE", "4") };
        let from_env = Config::from_env_with_base(Some(&temp_dir)).unwrap();
        assert_eq!(from_env.server.js_pool_size, 4);

        let _ = fs::remove_dir_all(&temp_dir);
        match prev {
            Some(v) => unsafe { env::set_var("RARI_JS_POOL_SIZE", v) },
            None => unsafe { env::remove_var("RARI_JS_POOL_SIZE") },
        }
    }

    #[test]
    fn test_html_limited_bots_config_validates_regex() {
        let temp_dir =
            env::temp_dir().join(format!("rari_test_html_limited_bots_{}", process::id()));
        let dist_server_dir = temp_dir.join("dist").join("server");
        fs::create_dir_all(&dist_server_dir).unwrap();

        fs::write(dist_server_dir.join("config.json"), r#"{"htmlLimitedBots":"OnlyMyBot"}"#)
            .unwrap();
        let valid = Config::from_env_with_base(Some(&temp_dir)).unwrap();
        assert_eq!(valid.html_limited_bots.as_deref(), Some("OnlyMyBot"));
        assert!(valid.html_limited_bots_regex.is_some());

        fs::write(dist_server_dir.join("config.json"), r#"{"htmlLimitedBots":"(OnlyMyBot"}"#)
            .unwrap();
        let invalid = Config::from_env_with_base(Some(&temp_dir)).unwrap();
        assert!(
            invalid.html_limited_bots.is_none(),
            "invalid regex must fall back to default (None override)"
        );
        assert!(invalid.html_limited_bots_regex.is_none());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_from_env_for_mode_production() {
        let temp_dir = env::temp_dir().join(format!("rari_test_load_mode_{}", process::id()));
        let dist_server_dir = temp_dir.join("dist").join("server");
        fs::create_dir_all(&dist_server_dir).unwrap();
        fs::write(dist_server_dir.join("config.json"), "{}").unwrap();

        let config =
            Config::load_from_env_with_base_for_mode(Mode::Production, Some(&temp_dir)).unwrap();

        assert_eq!(config.mode, Mode::Production);
        assert_eq!(
            config.get_cache_control_for_route("/"),
            "public, max-age=31536000, stale-while-revalidate=86400"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_from_env_test_fixture_app_production_cache_control() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/app");
        if !base.join("dist/server/config.json").exists() {
            return;
        }

        let config =
            Config::load_from_env_with_base_for_mode(Mode::Production, Some(&base)).unwrap();

        assert_eq!(
            config.get_cache_control_for_route("/"),
            "public, max-age=31536000, stale-while-revalidate=86400",
            "e2e fixture app must use production cache-control after CLI mode is applied"
        );
    }

    #[test]
    fn test_from_env_then_production_mode_uses_production_cache_control() {
        let temp_dir = env::temp_dir().join(format!("rari_test_prod_mode_{}", process::id()));
        let dist_server_dir = temp_dir.join("dist").join("server");
        fs::create_dir_all(&dist_server_dir).unwrap();
        fs::write(dist_server_dir.join("config.json"), "{}").unwrap();

        let config =
            Config::load_from_env_with_base_for_mode(Mode::Production, Some(&temp_dir)).unwrap();

        assert_eq!(
            config.get_cache_control_for_route("/"),
            "public, max-age=31536000, stale-while-revalidate=86400",
            "CLI production mode must override dev cache-control defaults from from_env"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dev_mode_cache_control() {
        let dev_config = Config::new(Mode::Development);
        assert_eq!(
            dev_config.caching.server_components, "no-cache, no-store, must-revalidate",
            "Development mode should use no-cache for server components"
        );

        let prod_config = Config::new(Mode::Production);
        assert_eq!(
            prod_config.caching.server_components,
            "public, max-age=31536000, stale-while-revalidate=86400",
            "Production mode should use long cache for server components"
        );
    }

    #[test]
    fn test_cache_control_for_route_production_configured_and_fallback() {
        // All page-response builders route Cache-Control through this single
        // resolver, so its behavior is the contract they share. Under
        // production, a configured route wins; an unconfigured route falls back
        // to the production server-components default.
        let mut config = Config::new(Mode::Production);
        config.caching.routes.insert("/products".to_string(), "public, max-age=600".to_string());

        assert_eq!(config.get_cache_control_for_route("/products"), "public, max-age=600");
        assert_eq!(
            config.get_cache_control_for_route("/unconfigured"),
            "public, max-age=31536000, stale-while-revalidate=86400"
        );
    }

    #[test]
    fn test_cache_layer_config_default() {
        let layer = CacheLayerConfig::default();
        assert_eq!(layer.handler, "memory");
        assert_eq!(layer.max_entries, 1000);
        assert_eq!(layer.default_ttl_secs, 60);
        assert_eq!(layer.max_bytes, 0);
        let memory = layer.memory_config();
        assert_eq!(memory.max_entries, 1000);
        assert_eq!(memory.default_ttl, 60);
        assert_eq!(memory.max_bytes, 0);
    }

    #[test]
    fn test_cache_config_defaults_all_six_layers() {
        let cache = CacheConfig::default();
        for name in [
            CACHE_LAYER_RESPONSE,
            CACHE_LAYER_IMAGE,
            CACHE_LAYER_OG,
            CACHE_LAYER_LAYOUT,
            CACHE_LAYER_MODULE,
            CACHE_LAYER_FETCH,
        ] {
            assert!(cache.layers.contains_key(name), "missing default layer {name}");
        }
        assert_eq!(cache.layers.len(), 6);
    }

    #[test]
    fn test_cache_config_layer_fallback() {
        let cache = CacheConfig::default();
        let layer = cache.layer("nonexistent");
        assert_eq!(layer.handler, "memory");
        assert_eq!(layer.max_entries, 1000);
        assert_eq!(layer.default_ttl_secs, 60);
    }

    #[test]
    fn test_config_includes_default_cache() {
        let config = Config::default();
        assert_eq!(config.cache.layers.len(), 6);
        assert_eq!(config.cache.layer(CACHE_LAYER_RESPONSE).handler, "memory");
    }

    #[test]
    fn test_config_from_env_parses_cache_layers() {
        let temp_dir = env::temp_dir().join(format!("rari_test_cache_layers_{}", process::id()));
        let dist_server_dir = temp_dir.join("dist").join("server");
        fs::create_dir_all(&dist_server_dir).unwrap();

        let config_json = serde_json::json!({
            "cache": {
                "layers": {
                    "response": { "handler": "memory", "maxEntries": 2000, "defaultTtlSecs": 120 },
                    "image":    { "handler": "memory", "maxEntries": 500,  "defaultTtlSecs": 3600 },
                    "og":       { "handler": "memory", "maxEntries": 200,  "defaultTtlSecs": 86400 },
                    "bogus":    { "handler": "noop",  "maxEntries": 1,     "defaultTtlSecs": 0 },
                    "malformed": "not-an-object"
                }
            }
        });

        let config_path = dist_server_dir.join("config.json");
        let mut file = File::create(&config_path).unwrap();
        file.write_all(config_json.to_string().as_bytes()).unwrap();

        let result = Config::from_env_with_base(Some(&temp_dir));
        let _ = fs::remove_dir_all(&temp_dir);

        let config = result.unwrap();

        let response = config.cache.layer(CACHE_LAYER_RESPONSE);
        assert_eq!(response.handler, "memory");
        assert_eq!(response.max_entries, 2000);
        assert_eq!(response.default_ttl_secs, 120);

        let image = config.cache.layer(CACHE_LAYER_IMAGE);
        assert_eq!(image.max_entries, 500);
        assert_eq!(image.default_ttl_secs, 3600);

        let og = config.cache.layer(CACHE_LAYER_OG);
        assert_eq!(og.max_entries, 200);
        assert_eq!(og.default_ttl_secs, 86400);

        assert!(config.cache.layers.contains_key("bogus"));

        let layout = config.cache.layer(CACHE_LAYER_LAYOUT);
        assert_eq!(layout.handler, "memory");
        assert_eq!(layout.max_entries, 1000);
    }

    #[test]
    fn test_cache_layer_config_serialization() {
        let layer = CacheLayerConfig {
            handler: "memory".to_string(),
            url: None,
            max_entries: 500,
            default_ttl_secs: 60,
            max_bytes: 1_048_576,
        };
        let json = serde_json::to_value(&layer).unwrap();
        assert_eq!(json["handler"], "memory");
        assert_eq!(json["maxEntries"], 500);
        assert_eq!(json["defaultTtlSecs"], 60);
        assert_eq!(json["maxBytes"], 1_048_576);

        let round_trip: CacheLayerConfig = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip.max_entries, 500);
        assert_eq!(round_trip.default_ttl_secs, 60);
        assert_eq!(round_trip.max_bytes, 1_048_576);

        let omitted: CacheLayerConfig = serde_json::from_value(serde_json::json!({
            "handler": "memory",
            "maxEntries": 10,
            "defaultTtlSecs": 30
        }))
        .unwrap();
        assert_eq!(omitted.max_bytes, 0);
    }
}
