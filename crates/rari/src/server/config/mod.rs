use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

pub static GLOBAL_CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Mode {
    #[default]
    Development,
    Production,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Development => write!(f, "development"),
            Mode::Production => write!(f, "production"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub origin: Option<String>,
    pub enable_logging: bool,
    pub timeout_seconds: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            origin: None,
            enable_logging: true,
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct RedirectConfig {
    pub allowed_hosts: Vec<String>,
    pub allow_relative: bool,
    pub allow_subdomains: bool,
}

impl Default for RedirectConfig {
    fn default() -> Self {
        Self { allowed_hosts: vec![], allow_relative: true, allow_subdomains: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CspConfig {
    pub script_src: Vec<String>,
    pub style_src: Vec<String>,
    pub img_src: Vec<String>,
    pub font_src: Vec<String>,
    pub connect_src: Vec<String>,
    pub default_src: Vec<String>,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
    #[serde(default = "default_revalidate_rpm")]
    pub revalidate_requests_per_minute: u32,
}

fn default_revalidate_rpm() -> u32 {
    10
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: 100,
            burst_size: 200,
            revalidate_requests_per_minute: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControlConfig {
    pub routes: FxHashMap<String, String>,
    pub static_files: String,
    pub server_components: String,
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
pub struct RscHtmlConfig {
    pub enabled: bool,
    pub timeout_ms: u64,
    pub cache_template: bool,
    pub pretty_print: bool,
}

impl Default for RscHtmlConfig {
    fn default() -> Self {
        Self { enabled: true, timeout_ms: 5000, cache_template: true, pretty_print: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct StreamingConfig {
    pub enabled: bool,
    pub buffer_size: usize,
    pub resolution_timeout_ms: u64,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self { enabled: true, buffer_size: 64, resolution_timeout_ms: 5000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RscConfig {
    pub enable_ssr: bool,
    pub enable_streaming: bool,
    pub component_cache_size: usize,
    pub render_timeout_ms: u64,
    pub script_execution_timeout_ms: u64,
    pub enable_hot_reload: bool,
    pub hmr_reload_enabled: bool,
    pub hmr_max_retry_attempts: usize,
    pub hmr_reload_timeout_ms: u64,
    pub hmr_parallel_reloads: bool,
    pub hmr_debounce_delay_ms: u64,
    pub hmr_max_history_size: usize,
    pub hmr_enable_memory_monitoring: bool,
}

impl Default for RscConfig {
    fn default() -> Self {
        Self {
            enable_ssr: true,
            enable_streaming: true,
            component_cache_size: 2000,
            render_timeout_ms: 8000,
            script_execution_timeout_ms: 3000,
            enable_hot_reload: true,
            hmr_reload_enabled: true,
            hmr_max_retry_attempts: 3,
            hmr_reload_timeout_ms: 5000,
            hmr_parallel_reloads: true,
            hmr_debounce_delay_ms: 150,
            hmr_max_history_size: 100,
            hmr_enable_memory_monitoring: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub streaming: StreamingConfig,
    #[serde(default)]
    pub cors: CorsConfig,
    #[serde(default)]
    pub redirect: RedirectConfig,
    #[serde(default)]
    pub csp: CspConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub images: crate::server::image::ImageConfig,
}

impl Config {
    pub fn new(mode: Mode) -> Self {
        let default_config = Self::default();
        Self {
            mode,
            vite: ViteConfig {
                port: default_config.vite.port,
                host: default_config.server.host.clone(),
                ..default_config.vite
            },
            rsc: RscConfig {
                enable_hot_reload: mode != Mode::Production,
                hmr_reload_enabled: mode != Mode::Production,
                ..default_config.rsc
            },
            rsc_html: RscHtmlConfig {
                pretty_print: mode == Mode::Development,
                ..default_config.rsc_html
            },
            ..default_config
        }
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        if let Ok(mode_str) = std::env::var("RARI_MODE") {
            config.mode = match mode_str.to_lowercase().as_str() {
                "development" | "dev" => Mode::Development,
                "production" | "prod" => Mode::Production,
                _ => return Err(ConfigError::InvalidMode(mode_str)),
            };
        }

        if let Ok(host) = std::env::var("RARI_HOST") {
            config.server.host = host;
        }

        if let Ok(port_str) = std::env::var("RARI_PORT") {
            config.server.port =
                port_str.parse().map_err(|_| ConfigError::InvalidPort(port_str))?;
        }

        if let Ok(origin) = std::env::var("RARI_ORIGIN") {
            config.server.origin = Some(origin);
        }

        if let Ok(vite_host) = std::env::var("RARI_VITE_HOST") {
            config.vite.host = vite_host;
        }

        if let Ok(vite_port_str) = std::env::var("RARI_VITE_PORT") {
            config.vite.port =
                vite_port_str.parse().map_err(|_| ConfigError::InvalidVitePort(vite_port_str))?;
        }

        if let Ok(public_dir) = std::env::var("RARI_PUBLIC_DIR") {
            config.static_files.dev_public_dir = PathBuf::from(public_dir);
        }

        if let Ok(dist_dir) = std::env::var("RARI_DIST_DIR") {
            config.static_files.prod_public_dir = PathBuf::from(dist_dir);
        }

        if let Ok(timeout_str) = std::env::var("RARI_SCRIPT_EXECUTION_TIMEOUT_MS") {
            config.rsc.script_execution_timeout_ms = timeout_str
                .parse()
                .map_err(|_| ConfigError::InvalidTimeout(timeout_str.clone()))?;
        }

        if let Ok(disable_hmr) = std::env::var("DISABLE_HMR_RELOAD") {
            config.rsc.hmr_reload_enabled = disable_hmr.to_lowercase() != "true"
                && disable_hmr != "1"
                && disable_hmr.to_lowercase() != "yes";
        }

        if let Ok(max_retry_str) = std::env::var("RARI_HMR_MAX_RETRY_ATTEMPTS") {
            config.rsc.hmr_max_retry_attempts = max_retry_str.parse().map_err(|_| {
                ConfigError::InvalidConfig("RARI_HMR_MAX_RETRY_ATTEMPTS".to_string())
            })?;
        }

        if let Ok(timeout_str) = std::env::var("RARI_HMR_RELOAD_TIMEOUT_MS") {
            config.rsc.hmr_reload_timeout_ms = timeout_str.parse().map_err(|_| {
                ConfigError::InvalidConfig("RARI_HMR_RELOAD_TIMEOUT_MS".to_string())
            })?;
        }

        if let Ok(parallel_str) = std::env::var("RARI_HMR_PARALLEL_RELOADS") {
            config.rsc.hmr_parallel_reloads = parallel_str.to_lowercase() == "true"
                || parallel_str == "1"
                || parallel_str.to_lowercase() == "yes";
        }

        if let Ok(debounce_str) = std::env::var("RARI_HMR_DEBOUNCE_DELAY_MS") {
            config.rsc.hmr_debounce_delay_ms = debounce_str.parse().map_err(|_| {
                ConfigError::InvalidConfig("RARI_HMR_DEBOUNCE_DELAY_MS".to_string())
            })?;
        }

        if let Ok(history_size_str) = std::env::var("RARI_HMR_MAX_HISTORY_SIZE") {
            config.rsc.hmr_max_history_size = history_size_str
                .parse()
                .map_err(|_| ConfigError::InvalidConfig("RARI_HMR_MAX_HISTORY_SIZE".to_string()))?;
        }

        if let Ok(memory_monitoring_str) = std::env::var("RARI_HMR_ENABLE_MEMORY_MONITORING") {
            config.rsc.hmr_enable_memory_monitoring = memory_monitoring_str.to_lowercase()
                == "true"
                || memory_monitoring_str == "1"
                || memory_monitoring_str.to_lowercase() == "yes";
        }

        if let Ok(rsc_html_enabled_str) = std::env::var("RARI_RSC_HTML_ENABLED") {
            config.rsc_html.enabled = rsc_html_enabled_str.to_lowercase() == "true"
                || rsc_html_enabled_str == "1"
                || rsc_html_enabled_str.to_lowercase() == "yes";
        }

        if let Ok(rsc_html_timeout_str) = std::env::var("RARI_RSC_HTML_TIMEOUT_MS") {
            config.rsc_html.timeout_ms = rsc_html_timeout_str
                .parse()
                .map_err(|_| ConfigError::InvalidConfig("RARI_RSC_HTML_TIMEOUT_MS".to_string()))?;
        }

        if let Ok(rsc_html_cache_template_str) = std::env::var("RARI_RSC_HTML_CACHE_TEMPLATE") {
            config.rsc_html.cache_template = rsc_html_cache_template_str.to_lowercase() == "true"
                || rsc_html_cache_template_str == "1"
                || rsc_html_cache_template_str.to_lowercase() == "yes";
        }

        if let Ok(rsc_html_pretty_print_str) = std::env::var("RARI_RSC_HTML_PRETTY_PRINT") {
            config.rsc_html.pretty_print = rsc_html_pretty_print_str.to_lowercase() == "true"
                || rsc_html_pretty_print_str == "1"
                || rsc_html_pretty_print_str.to_lowercase() == "yes";
        }

        if let Ok(loading_enabled_str) = std::env::var("RARI_LOADING_ENABLED") {
            config.loading.enabled = loading_enabled_str.to_lowercase() == "true"
                || loading_enabled_str == "1"
                || loading_enabled_str.to_lowercase() == "yes";
        }

        if let Ok(min_display_time_str) = std::env::var("RARI_LOADING_MIN_DISPLAY_TIME_MS") {
            config.loading.min_display_time_ms = min_display_time_str.parse().map_err(|_| {
                ConfigError::InvalidConfig("RARI_LOADING_MIN_DISPLAY_TIME_MS".to_string())
            })?;
        }

        if let Ok(cache_loading_str) = std::env::var("RARI_LOADING_CACHE_COMPONENTS") {
            config.loading.cache_loading_components = cache_loading_str.to_lowercase() == "true"
                || cache_loading_str == "1"
                || cache_loading_str.to_lowercase() == "yes";
        }

        if let Ok(streaming_enabled_str) = std::env::var("RARI_STREAMING_ENABLED") {
            config.streaming.enabled = streaming_enabled_str.to_lowercase() == "true"
                || streaming_enabled_str == "1"
                || streaming_enabled_str.to_lowercase() == "yes";
        }

        if let Ok(buffer_size_str) = std::env::var("RARI_STREAMING_BUFFER_SIZE") {
            config.streaming.buffer_size = buffer_size_str.parse().map_err(|_| {
                ConfigError::InvalidConfig("RARI_STREAMING_BUFFER_SIZE".to_string())
            })?;
        }

        if let Ok(resolution_timeout_str) = std::env::var("RARI_STREAMING_RESOLUTION_TIMEOUT_MS") {
            config.streaming.resolution_timeout_ms =
                resolution_timeout_str.parse().map_err(|_| {
                    ConfigError::InvalidConfig("RARI_STREAMING_RESOLUTION_TIMEOUT_MS".to_string())
                })?;
        }

        if let Ok(script_src) = std::env::var("RARI_CSP_SCRIPT_SRC") {
            config.csp.script_src = script_src.split_whitespace().map(|s| s.to_string()).collect();
        }

        if let Ok(style_src) = std::env::var("RARI_CSP_STYLE_SRC") {
            config.csp.style_src = style_src.split_whitespace().map(|s| s.to_string()).collect();
        }

        if let Ok(img_src) = std::env::var("RARI_CSP_IMG_SRC") {
            config.csp.img_src = img_src.split_whitespace().map(|s| s.to_string()).collect();
        }

        if let Ok(font_src) = std::env::var("RARI_CSP_FONT_SRC") {
            config.csp.font_src = font_src.split_whitespace().map(|s| s.to_string()).collect();
        }

        if let Ok(connect_src) = std::env::var("RARI_CSP_CONNECT_SRC") {
            config.csp.connect_src =
                connect_src.split_whitespace().map(|s| s.to_string()).collect();
        }

        if let Ok(rate_limit_enabled) = std::env::var("RARI_RATE_LIMIT_ENABLED") {
            config.rate_limit.enabled = rate_limit_enabled.to_lowercase() == "true"
                || rate_limit_enabled == "1"
                || rate_limit_enabled.to_lowercase() == "yes";
        }

        if let Ok(rate_limit_rps) = std::env::var("RARI_RATE_LIMIT_RPS") {
            config.rate_limit.requests_per_second = rate_limit_rps
                .parse()
                .map_err(|_| ConfigError::InvalidConfig("RARI_RATE_LIMIT_RPS".to_string()))?;
        }

        if let Ok(rate_limit_burst) = std::env::var("RARI_RATE_LIMIT_BURST") {
            config.rate_limit.burst_size = rate_limit_burst
                .parse()
                .map_err(|_| ConfigError::InvalidConfig("RARI_RATE_LIMIT_BURST".to_string()))?;
        }

        if let Ok(revalidate_rpm) = std::env::var("RARI_REVALIDATE_RATE_LIMIT_RPM") {
            config.rate_limit.revalidate_requests_per_minute =
                revalidate_rpm.parse().map_err(|_| {
                    ConfigError::InvalidConfig("RARI_REVALIDATE_RATE_LIMIT_RPM".to_string())
                })?;
        }

        Ok(config)
    }

    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path).map_err(ConfigError::FileRead)?;

        toml::from_str(&contents).map_err(ConfigError::TomlParse)
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ConfigError> {
        let contents = toml::to_string_pretty(self).map_err(ConfigError::TomlSerialize)?;

        std::fs::write(path, contents).map_err(ConfigError::FileWrite)
    }

    pub fn get() -> Option<&'static Config> {
        GLOBAL_CONFIG.get()
    }

    pub fn set_global(config: Config) -> Result<(), Box<Config>> {
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

    pub fn hmr_reload_enabled(&self) -> bool {
        self.is_development() && self.rsc.hmr_reload_enabled
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

        for (pattern, cache_control) in &self.caching.routes {
            if Self::matches_pattern(pattern, path) {
                return cache_control;
            }
        }

        &self.caching.server_components
    }

    pub fn matches_pattern(pattern: &str, path: &str) -> bool {
        if let Some(prefix) = pattern.strip_suffix("/*") {
            return path.starts_with(prefix);
        }

        if pattern.contains('*') {
            let regex_pattern = pattern.replace("*", ".*").replace("/", "\\/");
            if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                return regex.is_match(path);
            }
        }

        pattern == path
    }

    pub fn csp_config(&self) -> CspConfig {
        let mut config = self.csp.clone();

        if !config.script_src.contains(&"'unsafe-inline'".to_string()) {
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

        directives.join("; ")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid mode: {0}")]
    InvalidMode(String),

    #[error("Invalid port: {0}")]
    InvalidPort(String),

    #[error("Invalid Vite port: {0}")]
    InvalidVitePort(String),

    #[error("Invalid timeout: {0}")]
    InvalidTimeout(String),

    #[error("Invalid config value for {0}")]
    InvalidConfig(String),

    #[error("Failed to read config file: {0}")]
    FileRead(std::io::Error),

    #[error("Failed to write config file: {0}")]
    FileWrite(std::io::Error),

    #[error("Failed to parse TOML: {0}")]
    TomlParse(toml::de::Error),

    #[error("Failed to serialize TOML: {0}")]
    TomlSerialize(toml::ser::Error),
}

#[cfg(test)]
mod tests {
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
    fn test_config_new_with_mode() {
        let config = Config::new(Mode::Production);
        assert_eq!(config.mode, Mode::Production);
        assert!(!config.rsc.enable_hot_reload);
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
        assert!(Config::matches_pattern("/api/*", "/api/users"));
        assert!(Config::matches_pattern("/api/*", "/api/products/123"));
        assert!(!Config::matches_pattern("/api/*", "/blog/posts"));

        assert!(Config::matches_pattern("/blog/*/comments", "/blog/post-1/comments"));
        assert!(!Config::matches_pattern("/blog/*/comments", "/blog/post-1"));

        assert!(Config::matches_pattern("*", "/any/path"));
        assert!(Config::matches_pattern("*.js", "/static/app.js"));
        assert!(!Config::matches_pattern("*.js", "/static/app.css"));
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
    fn test_streaming_config_default() {
        let streaming_config = StreamingConfig::default();
        assert!(streaming_config.enabled);
        assert_eq!(streaming_config.buffer_size, 64);
        assert_eq!(streaming_config.resolution_timeout_ms, 5000);
    }

    #[test]
    fn test_config_includes_streaming() {
        let config = Config::default();
        assert!(config.streaming.enabled);
        assert_eq!(config.streaming.buffer_size, 64);
        assert_eq!(config.streaming.resolution_timeout_ms, 5000);
    }

    #[test]
    fn test_streaming_config_serialization() {
        let streaming_config =
            StreamingConfig { enabled: false, buffer_size: 128, resolution_timeout_ms: 10000 };

        let serialized = serde_json::to_string(&streaming_config).unwrap();
        let deserialized: StreamingConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(streaming_config.enabled, deserialized.enabled);
        assert_eq!(streaming_config.buffer_size, deserialized.buffer_size);
        assert_eq!(streaming_config.resolution_timeout_ms, deserialized.resolution_timeout_ms);
    }
}
