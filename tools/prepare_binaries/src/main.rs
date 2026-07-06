mod common;
mod rari_binary;
mod use_cache_addon;

use std::{path::PathBuf, process};

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use common::{
    check_rust_installed, get_current_platform_target, install_target, log, log_error, log_success,
    log_warning,
};
use rari_binary::{build_binary, copy_binary_to_platform_package, validate_binary};
use use_cache_addon::{
    addon_stable_output_path_public, build_addon, copy_addon_to_platform_package, validate_addon,
};

#[derive(Parser, Debug)]
#[command(name = "prepare-binaries")]
#[command(about = "Prepare rari binaries and rari_use_cache addon for the current platform", long_about = None)]
struct Args {
    #[arg(long, help = "Build in debug mode (faster, for development)")]
    dev: bool,

    #[arg(long, help = "Build the rari_use_cache native addon in addition to the main binary")]
    addon: bool,

    #[arg(
        long,
        help = "Build the main rari binary (default when neither --bin nor --addon is set)"
    )]
    bin: bool,
}

#[expect(clippy::print_stdout)]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let do_build_bin = args.bin || !args.addon;
    let do_build_addon = args.addon || !args.bin;

    if !do_build_bin && !do_build_addon {
        anyhow::bail!(
            "Nothing to build: both --no-bin and --no-addon semantics engaged (pass --bin and/or --addon)"
        );
    }

    println!("{}", "🔧 Preparing rari artifacts for current platform".bold());
    println!();

    let project_root = PathBuf::from(".");

    let current_target = get_current_platform_target().context(
        "Unable to determine current platform target. Supported platforms: macOS (x64/ARM64), Linux (x64/ARM64), Windows (x64/ARM64).",
    )?;

    log(&format!("Building for: {}", current_target.platform.cyan()));

    if args.dev {
        log_warning("Building in debug mode (faster, but larger binaries)");
        println!("{}", "Use release mode for production builds".dimmed());
    }

    println!();

    if do_build_bin {
        check_rust_installed().await?;

        log(&format!("Installing Rust target: {}", current_target.target));
        install_target(current_target.target).await?;
    }

    println!();

    // ---- Binary ----
    if do_build_bin {
        log("Building binary...");
        let success = build_binary(current_target.target, &project_root, args.dev).await?;
        if !success {
            log_error("Failed to build binary for current platform");
            log_error("This may indicate a Rust compilation issue");
            process::exit(1);
        }

        println!();

        log("Copying binary to platform package...");
        let build_type = if args.dev { "debug" } else { "release" };
        let binary_path = project_root
            .join("target")
            .join(current_target.target)
            .join(build_type)
            .join(current_target.binary_name);

        if !binary_path.exists() {
            log_error(&format!("Binary not found: {}", binary_path.display()));
            process::exit(1);
        }

        copy_binary_to_platform_package(current_target, &project_root, args.dev)?;

        println!();

        log("Validating binary...");
        validate_binary(current_target, &project_root, args.dev)?;

        println!();
    }

    // ---- Addon ----
    if do_build_addon {
        log("Building rari_use_cache addon...");
        let success = build_addon(current_target, &project_root, args.dev).await?;
        if !success {
            log_error(&format!(
                "Failed to build addon for current platform ({})",
                current_target.platform
            ));
            process::exit(1);
        }

        println!();

        log("Copying addon to platform package...");
        let src = addon_stable_output_path_public(current_target, &project_root);
        if !src.exists() {
            log_error(&format!("Addon artifact missing: {}", src.display()));
            process::exit(1);
        }

        copy_addon_to_platform_package(current_target, &project_root, args.dev)?;

        println!();

        log("Validating addon...");
        validate_addon(current_target, &project_root)?;

        println!();
    }

    if do_build_bin {
        log_success("✨ Successfully prepared binary!");
    }
    if do_build_addon {
        log_success("✨ Successfully prepared addon!");
    }

    println!();
    println!("{}", "Platform package ready:".bold());
    if do_build_bin {
        println!("  • Binary → {}", current_target.package_dir.cyan());
    }
    if do_build_addon {
        println!("  • Addon → {}", current_target.addon_package_dir.cyan());
    }

    println!();
    println!("{}", "Next steps:".dimmed());
    println!("{}", "  1. Test the artifacts locally".dimmed());
    println!("{}", "  2. Use GitHub Actions for cross-platform builds".dimmed());
    println!("{}", "  3. Run the release script: pnpm run release".dimmed());

    Ok(())
}
