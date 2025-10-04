use clap::{Arg, Command};
use rari::error::RariError;
use rari::server::{
    Server,
    config::{Config, Mode},
};

use rustls::crypto::CryptoProvider;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let matches = Command::new("rari")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Rari HTTP Server")
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help("Server mode: development or production")
                .value_parser(["development", "dev", "production", "prod"])
                .default_value("development"),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path (TOML format)")
                .required(false),
        )
        .arg(
            Arg::new("host")
                .short('H')
                .long("host")
                .value_name("HOST")
                .help("Server host address")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Server port")
                .value_parser(clap::value_parser!(u16))
                .default_value("3000"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Reduce log output")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    init_logging(&matches)?;

    info!("Starting Rari Server v{}", env!("CARGO_PKG_VERSION"));

    CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .map_err(|_| "Failed to install rustls crypto provider")?;

    let config = load_configuration(&matches).await?;

    info!("Configuration loaded successfully");
    info!("Server mode: {}", config.mode);

    let server = Server::new(config).await.map_err(|e| {
        error!("Failed to create server: {}", e);
        e
    })?;

    info!("Server initialized, starting...");

    let shutdown_signal = setup_shutdown_signal();

    tokio::select! {
        result = server.start() => {
            match result {
                Ok(_) => info!("Server stopped normally"),
                Err(e) => {
                    error!("Server error: {}", e);
                    return Err(e.into());
                }
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, stopping server...");
        }
    }

    info!("Rari Server shutdown complete");
    Ok(())
}

#[allow(clippy::result_large_err)]
fn init_logging(matches: &clap::ArgMatches) -> Result<(), RariError> {
    let verbose = matches.get_flag("verbose");
    let quiet = matches.get_flag("quiet");

    let default_level = if verbose {
        "debug"
    } else if quiet {
        "warn"
    } else {
        "info"
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(format!("rari={default_level}")))
        .map_err(|e| RariError::configuration(format!("Failed to create log filter: {e}")))?;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(verbose)
                .with_line_number(verbose)
                .compact(),
        )
        .init();

    Ok(())
}

async fn load_configuration(matches: &clap::ArgMatches) -> Result<Config, RariError> {
    let mode_str = matches
        .get_one::<String>("mode")
        .ok_or_else(|| RariError::configuration("Mode argument is required".to_string()))?;

    let mode = match mode_str.as_str() {
        "development" | "dev" => Mode::Development,
        "production" | "prod" => Mode::Production,
        mode => return Err(RariError::configuration(format!("Invalid mode: {mode}"))),
    };

    let mut config = if let Some(config_file) = matches.get_one::<String>("config") {
        info!("Loading configuration from file: {}", config_file);
        Config::from_file(config_file).map_err(|e| {
            RariError::configuration(format!("Failed to load config file '{config_file}': {e}"))
        })?
    } else {
        match Config::from_env() {
            Ok(config) => {
                info!("Configuration loaded from environment variables");
                config
            }
            Err(e) => {
                warn!("Failed to load config from environment: {}, using defaults", e);
                Config::new(mode)
            }
        }
    };

    config.mode = mode;

    if let Some(host) = matches.get_one::<String>("host") {
        config.server.host = host.to_string();
    }

    if let Some(&port) = matches.get_one::<u16>("port") {
        config.server.port = port;
    }

    validate_configuration(&config)?;

    Ok(config)
}

#[allow(clippy::result_large_err)]
fn validate_configuration(config: &Config) -> Result<(), RariError> {
    if config.server.port == 0 {
        return Err(RariError::configuration("Server port cannot be 0".to_string()));
    }

    if config.vite.port == 0 {
        return Err(RariError::configuration("Vite port cannot be 0".to_string()));
    }

    if config.server.port == config.vite.port {
        return Err(RariError::configuration(
            "Server and Vite ports cannot be the same".to_string(),
        ));
    }

    if config.server.host.is_empty() {
        return Err(RariError::configuration("Server host cannot be empty".to_string()));
    }

    if !config.public_dir().exists() {
        warn!("Public directory does not exist: {}", config.public_dir().display());
        warn!("Static files will not be served correctly");
    }

    info!("Configuration validation passed");
    Ok(())
}

async fn setup_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");

        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to create SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C");
            }
        }
    }

    #[cfg(windows)]
    {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_parsing() {
        let dev_cases = ["development", "dev"];
        let prod_cases = ["production", "prod"];

        for case in dev_cases {
            let mode = match case {
                "development" | "dev" => Mode::Development,
                "production" | "prod" => Mode::Production,
                _ => panic!("Invalid mode"),
            };
            assert!(matches!(mode, Mode::Development));
        }

        for case in prod_cases {
            let mode = match case {
                "development" | "dev" => Mode::Development,
                "production" | "prod" => Mode::Production,
                _ => panic!("Invalid mode"),
            };
            assert!(matches!(mode, Mode::Production));
        }
    }

    #[tokio::test]
    async fn test_config_validation() {
        let mut config = Config::default();

        assert!(validate_configuration(&config).is_ok());

        config.server.port = 0;
        assert!(validate_configuration(&config).is_err());

        config.server.port = 3000;
        config.vite.port = 3000;
        assert!(validate_configuration(&config).is_err());
    }
}
