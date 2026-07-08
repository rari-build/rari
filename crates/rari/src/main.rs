use std::{
    env, error,
    io::{self, Write},
    path::PathBuf,
};

use clap::{Arg, ArgAction, Command};
use rari::server::{
    Server,
    config::{Config, Mode},
    image::{ImageConfig, ImageOptimizer, scan_for_image_usage},
};
use rari_error::RariError;
use rustls::crypto::{CryptoProvider, aws_lc_rs};
use tokio::fs;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error + Send + Sync>> {
    let matches = cli().get_matches();

    if let Some(("optimize-images", sub_matches)) = matches.subcommand() {
        init_logging_for_subcommand(sub_matches)?;
        CryptoProvider::install_default(aws_lc_rs::default_provider())
            .map_err(|_| "Failed to install rustls crypto provider")?;
        let dry_run = sub_matches.get_flag("dry-run");
        return run_optimize_images(dry_run).await;
    }

    if let Some(("scan-images", sub_matches)) = matches.subcommand() {
        return run_scan_images(sub_matches);
    }

    init_logging(&matches)?;

    CryptoProvider::install_default(aws_lc_rs::default_provider())
        .map_err(|_| "Failed to install rustls crypto provider")?;

    let config = load_configuration(&matches)?;

    let server = Server::new(config).await.map_err(|e| {
        tracing::error!("Failed to create server: {}", e);
        e
    })?;

    server.start_with_shutdown(setup_shutdown_signal()).await.map_err(|e| {
        tracing::error!("Server error: {}", e);
        e
    })?;

    Ok(())
}

fn cli() -> Command {
    Command::new("rari")
        .version(env!("CARGO_PKG_VERSION"))
        .about("rari HTTP Server")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(
            Command::new("optimize-images")
                .about("Pre-optimize local images for production")
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("Enable verbose logging")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .help("Preview images that would be optimized without performing writes")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("scan-images")
                .about("Scan source files for rari Image component usage")
                .arg(
                    Arg::new("src")
                        .long("src")
                        .value_name("DIR")
                        .help("Source directory to scan")
                        .required(true),
                )
                .arg(
                    Arg::new("extra")
                        .long("extra")
                        .value_name("DIR")
                        .help("Additional directory to scan")
                        .action(ArgAction::Append),
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
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Reduce log output")
                .action(ArgAction::SetTrue),
        )
}

async fn run_optimize_images(dry_run: bool) -> Result<(), Box<dyn error::Error + Send + Sync>> {
    let project_path = env::current_dir()?;

    let image_config_path = project_path.join("dist").join("server").join("image.json");

    let image_config: ImageConfig = if fs::try_exists(&image_config_path).await? {
        let config_content = fs::read_to_string(&image_config_path).await?;
        serde_json::from_str(&config_content)?
    } else {
        return Ok(());
    };

    let has_work =
        !image_config.preoptimize_manifest.is_empty() || !image_config.local_patterns.is_empty();

    if !has_work {
        return Ok(());
    }

    let optimizer = ImageOptimizer::new(image_config, &project_path);

    if dry_run {
        tracing::info!("Running in dry-run mode (preview only, no writes)...");
        match optimizer.preoptimize_local_images_preview().await {
            Ok(count) => {
                tracing::info!("[DRY RUN] Would pre-optimize {} image variants", count);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to preview images: {}", e);
                Err(e.into())
            }
        }
    } else {
        match optimizer.preoptimize_local_images().await {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("Successfully pre-optimized {} image variants", count);
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to pre-optimize images: {}", e);
                Err(e.into())
            }
        }
    }
}

fn run_scan_images(
    sub_matches: &clap::ArgMatches,
) -> Result<(), Box<dyn error::Error + Send + Sync>> {
    let src_dir = sub_matches
        .get_one::<String>("src")
        .ok_or_else(|| RariError::configuration("Source directory is required".to_string()))?;
    let extra_dirs = sub_matches
        .get_many::<String>("extra")
        .map(|values| values.map(PathBuf::from).collect::<Vec<_>>())
        .unwrap_or_default();

    let manifest = scan_for_image_usage(src_dir, &extra_dirs).map_err(|error| {
        RariError::configuration(format!("Failed to scan for image usage: {error}"))
    })?;

    let mut stdout = io::stdout().lock();
    stdout.write_all(serde_json::to_string(&manifest)?.as_bytes())?;
    stdout.write_all(b"\n")?;
    Ok(())
}

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

fn load_configuration(matches: &clap::ArgMatches) -> Result<Config, RariError> {
    let mode_str = matches
        .get_one::<String>("mode")
        .ok_or_else(|| RariError::configuration("Mode argument is required".to_string()))?;

    let mode = match mode_str.as_str() {
        "development" | "dev" => Mode::Development,
        "production" | "prod" => Mode::Production,
        mode => return Err(RariError::configuration(format!("Invalid mode: {mode}"))),
    };

    let mut config = Config::load_from_env_for_mode(mode)
        .map_err(|e| RariError::configuration(format!("Failed to load configuration: {e}")))?;

    if let Some(host) = matches.get_one::<String>("host") {
        config.server.host.clone_from(host);
    }

    if let Some(&port) = matches.get_one::<u16>("port") {
        config.server.port = port;
    }

    validate_configuration(&config)?;

    Ok(config)
}

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
        setup_shutdown_signal_unix().await;
    }

    #[cfg(windows)]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(unix)]
async fn setup_shutdown_signal_unix() {
    use signal_hook::{
        consts::{SIGINT, SIGTERM},
        iterator::Signals,
    };
    use tokio::task;

    let received = task::spawn_blocking(move || {
        let mut signals = Signals::new([SIGTERM, SIGINT]).ok()?;
        signals.forever().next();
        Some(())
    })
    .await;

    let use_fallback = match received {
        Ok(Some(())) => false,
        Ok(None) | Err(_) => true,
    };

    if use_fallback {
        tracing::error!("Failed to register Unix shutdown signal handler, falling back to tokio");
        setup_shutdown_signal_tokio().await;
    }
}

#[cfg(unix)]
async fn setup_shutdown_signal_tokio() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(sigterm) => sigterm,
        Err(error) => {
            use std::future;

            tracing::error!("Failed to create SIGTERM handler: {error}");
            future::pending::<()>().await;
            return;
        }
    };

    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(sigint) => sigint,
        Err(error) => {
            tracing::error!("Failed to create SIGINT handler: {error}");
            sigterm.recv().await;
            return;
        }
    };

    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }
}
