mod common;
mod rari_binary;
mod use_cache_addon;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

use common::*;
use rari_binary::*;
use use_cache_addon::*;

#[derive(Parser, Debug)]
#[command(name = "prepare-binaries")]
#[command(about = "Prepare rari binaries (and rari-use-cache addon) for platform packages", long_about = None)]
struct Args {
    #[arg(long)]
    all: bool,

    #[arg(long, help = "Build in debug mode (faster, for development)")]
    dev: bool,

    #[arg(
        long,
        help = "Build the rari-use-cache native addon in addition to the main binary"
    )]
    addon: bool,

    #[arg(
        long,
        help = "Build the main rari binary (default when neither --bin nor --addon is set)"
    )]
    bin: bool,

    #[arg(
        long,
        value_name = "PLATFORM",
        help = "Restrict to a single platform (e.g. linux-x64)"
    )]
    platform: Option<String>,
}

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

    println!(
        "{}",
        "🔧 Preparing rari platform artifacts (binary and/or rari-use-cache addon)".bold()
    );
    println!();

    let project_root = PathBuf::from(".");

    let mut targets_to_build: Vec<&Target> = if args.all {
        log("Building for all platforms (cross-compilation mode)");
        TARGETS.iter().collect()
    } else if let Some(name) = &args.platform {
        let t = TARGETS
            .iter()
            .find(|t| t.platform == *name)
            .with_context(|| {
                format!(
                    "Unknown platform '{}'. Supported: {}",
                    name,
                    TARGETS
                        .iter()
                        .map(|t| t.platform)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        log(&format!(
            "Building for explicit platform only: {}",
            t.platform.cyan()
        ));
        vec![t]
    } else {
        let current_target = get_current_platform_target().context(
            "Unable to determine current platform target. Supported platforms: macOS (x64/ARM64), Linux (x64/ARM64), Windows (x64/ARM64). Use --platform <name> to override.",
        )?;
        log(&format!(
            "Building for current platform only: {}",
            current_target.platform.cyan()
        ));
        println!(
            "{}",
            "Use --all flag to build for all platforms (requires cross-compilation tools)".dimmed()
        );
        vec![current_target]
    };

    if args.all && args.platform.is_some() {
        log_warning("--platform overrides --all: only the requested platform will be built");
    }
    targets_to_build
        .retain(|t| !args.all || args.platform.as_deref().is_none_or(|p| p == t.platform));
    let _ = args.all;

    if args.dev {
        log_warning("Building in debug mode (faster, but larger binaries)");
        println!("{}", "Use release mode for production builds".dimmed());
    }

    println!();

    if do_build_bin {
        check_rust_installed().await?;

        if args.all && do_build_bin {
            install_linux_cross_compiler().await?;
        }
        log("Installing Rust targets...");
        for target_info in &targets_to_build {
            install_target(target_info.target).await?;
        }
    }

    println!();

    let mut bin_success = 0usize;
    let mut bin_failure = 0usize;
    let mut addon_success = 0usize;
    let mut addon_failure = 0usize;

    // ---- Binary ----
    if do_build_bin {
        log("Building binaries...");
        for target_info in &targets_to_build {
            let success = build_binary(target_info.target, &project_root, args.dev).await?;
            if success {
                bin_success += 1;
            } else {
                bin_failure += 1;
                if !args.all {
                    log_error("Failed to build binary for current platform");
                    log_error("This may indicate a Rust compilation issue");
                    std::process::exit(1);
                }
            }
        }

        println!();

        log("Copying binaries to platform packages...");
        for target_info in &targets_to_build {
            let build_type = if args.dev { "debug" } else { "release" };
            let binary_path = project_root
                .join("target")
                .join(target_info.target)
                .join(build_type)
                .join(target_info.binary_name);

            if binary_path.exists() {
                let success =
                    copy_binary_to_platform_package(target_info, &project_root, args.dev)?;
                if !success {
                    bin_failure += 1;
                }
            }
        }

        println!();

        log("Validating binaries...");
        for target_info in &targets_to_build {
            validate_binary(target_info, &project_root, args.dev)?;
        }

        println!();
    }

    // ---- Addon ----
    if do_build_addon {
        log("Building rari-use-cache addon...");
        for target_info in &targets_to_build {
            let success = build_addon(target_info, &project_root, args.dev).await?;
            if success {
                addon_success += 1;
            } else {
                addon_failure += 1;
                if !args.all {
                    log_error(&format!(
                        "Failed to build addon for current platform ({})",
                        target_info.platform
                    ));
                    std::process::exit(1);
                }
            }
        }

        println!();

        log("Copying addon to platform packages...");
        for target_info in &targets_to_build {
            let src = addon_stable_output_path_public(target_info, &project_root);
            if src.exists() {
                let success = copy_addon_to_platform_package(target_info, &project_root, args.dev)?;
                if !success {
                    addon_failure += 1;
                }
            } else {
                log_warning(&format!(
                    "Addon artifact missing for {}, skipping copy to platform package",
                    target_info.platform
                ));
            }
        }

        println!();

        log("Validating addons...");
        for target_info in &targets_to_build {
            let _ = validate_addon(target_info, &project_root);
        }

        println!();
    }

    let total_attempted = targets_to_build.len();

    let any_failure = bin_failure > 0 || addon_failure > 0;
    if !any_failure {
        if do_build_bin {
            log_success(&format!(
                "✨ Successfully prepared {} platform binaries!",
                bin_success
            ));
        }
        if do_build_addon {
            log_success(&format!(
                "✨ Successfully prepared {} platform addons!",
                addon_success
            ));
        }
        println!();
        println!("{}", "Platform packages ready:".bold());
        for target_info in &targets_to_build {
            if do_build_bin {
                println!(
                    "  • {} → {}",
                    target_info.platform.cyan(),
                    target_info.package_dir
                );
            }
            if do_build_addon {
                println!(
                    "  • {} → {}",
                    target_info.platform.cyan(),
                    target_info.addon_package_dir
                );
            }
        }
        println!();
        println!("{}", "Next steps:".dimmed());
        if !args.all {
            println!("{}", "  1. Test the artifacts locally".dimmed());
            println!(
                "{}",
                "  2. Use GitHub Actions for full cross-platform builds".dimmed()
            );
            println!(
                "{}",
                "  3. Or run with --all flag (requires cross-compilation setup)".dimmed()
            );
        } else {
            println!("{}", "  1. Test the artifacts locally".dimmed());
            println!(
                "{}",
                "  2. Run the release script: pnpm run release".dimmed()
            );
            println!("{}", "  3. Or publish individual packages".dimmed());
        }
    } else {
        if bin_success > 0 || addon_success > 0 {
            log_warning(&format!(
                "Partial success: {} bin(s), {} addon(s) of {} target(s)",
                bin_success, addon_success, total_attempted
            ));
            println!();
            println!("{}", "Successfully built:".bold());
            for target_info in &targets_to_build {
                if do_build_bin {
                    let build_type = if args.dev { "debug" } else { "release" };
                    let binary_path = project_root
                        .join("target")
                        .join(target_info.target)
                        .join(build_type)
                        .join(target_info.binary_name);
                    if binary_path.exists() {
                        println!("  • {} (binary)", target_info.platform.green());
                    }
                }
                if do_build_addon
                    && addon_stable_output_path_public(target_info, &project_root).exists()
                {
                    println!("  • {} (addon)", target_info.platform.green());
                }
            }
        } else {
            log_error("Failed to prepare any platform artifacts");
        }

        println!();
        println!("{}", "Troubleshooting:".bold());
        if args.all {
            println!("  • Cross-compilation requires additional tools:");
            println!("    - Linux: Install gcc-*-linux-gnu packages");
            println!("    - Windows: Install mingw-w64 toolchain");
            println!("    - Use GitHub Actions for reliable cross-platform builds");
            println!("  • Or build for current platform only (remove --all flag)");
        } else {
            println!("  • Ensure Rust is installed: https://rustup.rs/");
            println!("  • Check that all required dependencies are installed");
            println!(
                "  • For addon builds: ensure @napi-rs/cli is installed (npm i -g @napi-rs/cli)"
            );
        }

        if !args.all && any_failure {
            std::process::exit(1);
        }
    }

    Ok(())
}
