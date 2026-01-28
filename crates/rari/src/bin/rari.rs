use clap::{Arg, Command};
use rari::error::RariError;
use rari::server::{
    Server,
    config::{Config, Mode},
    image::ImageOptimizer,
};

use rustls::crypto::CryptoProvider;
use tracing::error;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let matches = Command::new("rari")
        .version(env!("CARGO_PKG_VERSION"))
        .about("rari HTTP Server")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(
            Command::new("optimize-images").about("Pre-optimize local images for production").arg(
                Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .help("Enable verbose logging")
                    .action(clap::ArgAction::SetTrue),
            ),
        )
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

    if let Some(("optimize-images", sub_matches)) = matches.subcommand() {
        init_logging_for_subcommand(sub_matches)?;
        return run_optimize_images().await;
    }

    init_logging(&matches)?;

    CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .map_err(|_| "Failed to install rustls crypto provider")?;

    let config = load_configuration(&matches).await?;

    let server = Server::new(config).await.map_err(|e| {
        error!("Failed to create server: {}", e);
        e
    })?;

    let shutdown_signal = setup_shutdown_signal();

    tokio::select! {
        result = server.start() => {
            match result {
                Ok(_) => {}
                Err(e) => {
                    error!("Server error: {}", e);
                    return Err(e.into());
                }
            }
        }
        _ = shutdown_signal => {}
    }

    Ok(())
}

async fn run_optimize_images() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let project_path = std::env::current_dir()?;

    tracing::info!("Pre-optimizing local images...");

    let image_config_path = project_path.join("dist").join("server").join("image.json");

    let image_config = if image_config_path.exists() {
        let config_content = std::fs::read_to_string(&image_config_path)?;
        serde_json::from_str(&config_content)?
    } else {
        tracing::warn!("No image.json found, using default configuration");
        rari::server::image::ImageConfig::default()
    };

    let optimizer = ImageOptimizer::new(image_config, &project_path);

    match optimizer.preoptimize_local_images().await {
        Ok(count) => {
            tracing::info!("Successfully pre-optimized {} image variants", count);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to pre-optimize images: {}", e);
            Err(e.into())
        }
    }
}

#[allow(clippy::result_large_err)]
fn init_logging_for_subcommand(matches: &clap::ArgMatches) -> Result<(), RariError> {
    let verbose = matches.get_flag("verbose");

    let default_level = if verbose { "debug" } else { "info" };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(format!("rari={default_level}")))
        .map_err(|e| RariError::configuration(format!("Failed to create log filter: {e}")))?;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false)
                .compact(),
        )
        .init();

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

    let mut config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("No environment config found, using defaults: {}", e);
            Config::new(mode)
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
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }

    #[cfg(windows)]
    {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
        }
    }
}
