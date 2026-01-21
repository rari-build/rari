use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Parser, Debug)]
#[command(name = "prepare-binaries")]
#[command(about = "Prepare Rari binaries for platform packages", long_about = None)]
struct Args {
    #[arg(long)]
    all: bool,
}

#[derive(Debug, Clone)]
struct Target {
    target: &'static str,
    platform: &'static str,
    binary_name: &'static str,
    package_dir: &'static str,
}

const TARGETS: &[Target] = &[
    Target {
        target: "x86_64-unknown-linux-gnu",
        platform: "linux-x64",
        binary_name: "rari",
        package_dir: "packages/rari-linux-x64",
    },
    Target {
        target: "aarch64-unknown-linux-gnu",
        platform: "linux-arm64",
        binary_name: "rari",
        package_dir: "packages/rari-linux-arm64",
    },
    Target {
        target: "x86_64-apple-darwin",
        platform: "darwin-x64",
        binary_name: "rari",
        package_dir: "packages/rari-darwin-x64",
    },
    Target {
        target: "aarch64-apple-darwin",
        platform: "darwin-arm64",
        binary_name: "rari",
        package_dir: "packages/rari-darwin-arm64",
    },
    Target {
        target: "x86_64-pc-windows-msvc",
        platform: "win32-x64",
        binary_name: "rari.exe",
        package_dir: "packages/rari-win32-x64",
    },
];

fn log(message: &str) {
    println!("{} {}", "➜".cyan(), message);
}

fn log_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

fn log_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

fn log_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

fn get_current_platform_target() -> Option<&'static Target> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    TARGETS.iter().find(|target| {
        let parts: Vec<&str> = target.platform.split('-').collect();
        if parts.len() != 2 {
            return false;
        }
        let (target_os, target_arch) = (parts[0], parts[1]);

        let os_match = match os {
            "macos" => target_os == "darwin",
            "linux" => target_os == "linux",
            "windows" => target_os == "win32",
            _ => false,
        };

        let arch_match = match arch {
            "x86_64" => target_arch == "x64",
            "aarch64" => target_arch == "arm64",
            _ => false,
        };

        os_match && arch_match
    })
}

async fn check_rust_installed() -> Result<()> {
    let output = Command::new("cargo")
        .arg("--version")
        .output()
        .await
        .context("Failed to check cargo version")?;

    if output.status.success() {
        log_success("Rust/Cargo is installed");
        Ok(())
    } else {
        log_error("Rust/Cargo is not installed");
        log_error("Please install Rust: https://rustup.rs/");
        anyhow::bail!("Rust not installed");
    }
}

async fn install_target(target: &str) -> Result<()> {
    log(&format!("Installing Rust target: {}", target));

    let output = Command::new("rustup")
        .args(["target", "add", target])
        .output()
        .await
        .context("Failed to install target")?;

    if output.status.success() {
        log_success(&format!("Installed target: {}", target));
        Ok(())
    } else {
        log_warning(&format!("Failed to install target {}", target));
        log_warning("You may need to install additional system dependencies");
        Ok(())
    }
}

async fn build_binary(target: &str, project_root: &Path) -> Result<bool> {
    log(&format!("Building binary for {}", target));

    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--release", "--target", target, "--bin", "rari"]).current_dir(project_root);

    if target == "aarch64-unknown-linux-gnu" {
        cmd.env("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER", "aarch64-linux-gnu-gcc");
    }

    let output = cmd.output().await.context("Failed to execute cargo build")?;

    if output.status.success() {
        log_success(&format!("Built binary for {}", target));
        Ok(true)
    } else {
        log_error(&format!("Failed to build binary for {}", target));
        let stderr = String::from_utf8_lossy(&output.stderr);
        log_error(&format!("Error: {}", stderr));
        Ok(false)
    }
}

fn copy_binary_to_platform_package(target_info: &Target, project_root: &Path) -> Result<bool> {
    let source_path = project_root
        .join("target")
        .join(target_info.target)
        .join("release")
        .join(target_info.binary_name);

    let dest_dir = project_root.join(target_info.package_dir).join("bin");
    let dest_path = dest_dir.join(target_info.binary_name);

    if !source_path.exists() {
        log_error(&format!("Binary not found: {}", source_path.display()));
        return Ok(false);
    }

    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).context("Failed to create destination directory")?;
        log(&format!("Created directory: {}", dest_dir.display()));
    }

    fs::copy(&source_path, &dest_path).context("Failed to copy binary")?;

    #[cfg(unix)]
    if target_info.platform != "win32-x64" {
        let mut perms = fs::metadata(&dest_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_path, perms)?;
    }

    log_success(&format!("Copied binary to: {}", dest_path.display()));
    Ok(true)
}

fn validate_binary(target_info: &Target, project_root: &Path) -> Result<bool> {
    let binary_path =
        project_root.join(target_info.package_dir).join("bin").join(target_info.binary_name);

    if !binary_path.exists() {
        log_error(&format!("Binary not found: {}", binary_path.display()));
        return Ok(false);
    }

    #[cfg(unix)]
    if target_info.platform != "win32-x64" {
        let metadata = fs::metadata(&binary_path)?;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            log_error(&format!("Binary is not executable: {}", binary_path.display()));
            return Ok(false);
        }
    }

    let metadata = fs::metadata(&binary_path)?;
    let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;

    log_success(&format!("Binary validated: {} ({:.2} MB)", binary_path.display(), size_mb));
    Ok(true)
}

async fn install_linux_cross_compiler() -> Result<()> {
    if std::env::consts::OS != "linux" {
        return Ok(());
    }

    log("Installing Linux ARM64 cross-compiler...");

    let output = Command::new("sh")
        .args(["-c", "sudo apt-get update && sudo apt-get install -y gcc-aarch64-linux-gnu"])
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            log_success("Installed Linux ARM64 cross-compiler");
        }
        _ => {
            log_warning("Failed to install Linux ARM64 cross-compiler");
            log_warning(
                "You may need to install it manually: sudo apt-get install gcc-aarch64-linux-gnu",
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("{}\n", "🔧 Preparing Rari binaries for platform packages".bold());

    let project_root = PathBuf::from(".");

    let targets_to_build: Vec<&Target> = if args.all {
        log("Building for all platforms (cross-compilation mode)");
        TARGETS.iter().collect()
    } else {
        let current_target = get_current_platform_target().context(
            "Unable to determine current platform target. Supported platforms: macOS (x64/ARM64), Linux (x64/ARM64), Windows (x64)",
        )?;
        log(&format!("Building for current platform only: {}", current_target.platform.cyan()));
        println!(
            "{}",
            "Use --all flag to build for all platforms (requires cross-compilation tools)".dimmed()
        );
        vec![current_target]
    };

    println!();

    check_rust_installed().await?;

    if args.all {
        install_linux_cross_compiler().await?;
    }

    log("Installing Rust targets...");
    for target_info in &targets_to_build {
        install_target(target_info.target).await?;
    }

    println!();

    log("Building binaries...");
    let mut success_count = 0;
    let mut failure_count = 0;

    for target_info in &targets_to_build {
        let success = build_binary(target_info.target, &project_root).await?;
        if success {
            success_count += 1;
        } else {
            failure_count += 1;
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
        let binary_path = project_root
            .join("target")
            .join(target_info.target)
            .join("release")
            .join(target_info.binary_name);

        if binary_path.exists() {
            let success = copy_binary_to_platform_package(target_info, &project_root)?;
            if !success {
                failure_count += 1;
            }
        }
    }

    println!();

    log("Validating binaries...");
    for target_info in &targets_to_build {
        validate_binary(target_info, &project_root)?;
    }

    println!();

    let total_attempted = targets_to_build.len();

    if failure_count == 0 {
        log_success(&format!("✨ Successfully prepared {} platform binaries!", success_count));
        println!();
        println!("{}", "Platform packages ready:".bold());
        for target_info in &targets_to_build {
            println!("  • {} → {}", target_info.platform.cyan(), target_info.package_dir);
        }
        println!();
        println!("{}", "Next steps:".dimmed());
        if !args.all {
            println!("{}", "  1. Test the binary locally".dimmed());
            println!("{}", "  2. Use GitHub Actions for full cross-platform builds".dimmed());
            println!(
                "{}",
                "  3. Or run with --all flag (requires cross-compilation setup)".dimmed()
            );
        } else {
            println!("{}", "  1. Test the binaries locally".dimmed());
            println!("{}", "  2. Run the release script: pnpm run release".dimmed());
            println!("{}", "  3. Or publish individual packages".dimmed());
        }
    } else {
        if success_count > 0 {
            log_warning(&format!(
                "Partial success: {}/{} binaries built",
                success_count, total_attempted
            ));
            println!();
            println!("{}", "Successfully built:".bold());
            for target_info in &targets_to_build {
                let binary_path = project_root
                    .join("target")
                    .join(target_info.target)
                    .join("release")
                    .join(target_info.binary_name);
                if binary_path.exists() {
                    println!("  • {}", target_info.platform.green());
                }
            }
        } else {
            log_error("Failed to prepare any platform binaries");
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
        }

        if !args.all && failure_count > 0 {
            std::process::exit(1);
        }
    }

    Ok(())
}
