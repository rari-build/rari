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
            host: "127.0.0.1".to_string(),
            port: 3001,
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
pub struct RscConfig {
    pub enable_ssr: bool,
    pub enable_streaming: bool,
    pub component_cache_size: usize,
    pub render_timeout_ms: u64,
    pub enable_hot_reload: bool,
}

impl Default for RscConfig {
    fn default() -> Self {
        Self {
            enable_ssr: true,
            enable_streaming: true,
            component_cache_size: 1000,
            render_timeout_ms: 5000,
            enable_hot_reload: true,
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
}

impl Config {
    pub fn new(mode: Mode) -> Self {
        let default_config = Self::default();
        Self {
            mode,
            vite: ViteConfig {
                port: default_config.server.port + 1,
                host: default_config.server.host.clone(),
                ..default_config.vite
            },
            rsc: RscConfig { enable_hot_reload: mode != Mode::Production, ..default_config.rsc },
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
            config.vite.host = config.server.host.clone();
        }

        if let Ok(port_str) = std::env::var("RARI_PORT") {
            config.server.port =
                port_str.parse().map_err(|_| ConfigError::InvalidPort(port_str))?;
            config.vite.port = config.server.port + 1;
        }

        if let Ok(origin) = std::env::var("RARI_ORIGIN") {
            config.server.origin = Some(origin);
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
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid mode: {0}")]
    InvalidMode(String),

    #[error("Invalid port: {0}")]
    InvalidPort(String),

    #[error("Invalid Vite port: {0}")]
    InvalidVitePort(String),

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
        assert_eq!(config.vite.port, 3001);
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
        assert_eq!(config.vite_address(), "127.0.0.1:3001");
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
}
