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
pub struct CacheConfig {
    pub routes: FxHashMap<String, String>,
    pub static_files: String,
    pub server_components: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            routes: FxHashMap::default(),
            static_files: "public, max-age=31536000, immutable".to_string(),
            server_components: "no-cache".to_string(),
        }
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
    pub caching: CacheConfig,
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
        assert_eq!(cache_control, "no-cache");
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

        let cache_config = CacheConfig {
            routes,
            static_files: "public, max-age=31536000, immutable".to_string(),
            server_components: "no-cache".to_string(),
        };

        let serialized = serde_json::to_string(&cache_config).unwrap();
        let deserialized: CacheConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(cache_config.static_files, deserialized.static_files);
        assert_eq!(cache_config.server_components, deserialized.server_components);
        assert_eq!(cache_config.routes.len(), deserialized.routes.len());
    }
}
