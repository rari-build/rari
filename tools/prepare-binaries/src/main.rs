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
#[command(about = "Prepare Rari binaries (and rari-use-cache addon) for platform packages", long_about = None)]
struct Args {
    #[arg(long)]
    all: bool,

    #[arg(long, help = "Build in debug mode (faster, for development)")]
    dev: bool,

    #[arg(long, help = "Build the rari-use-cache native addon in addition to the main binary")]
    addon: bool,

    #[arg(
        long,
        help = "Build the main rari binary (default when neither --bin nor --addon is set)"
    )]
    bin: bool,

    #[arg(long, value_name = "PLATFORM", help = "Restrict to a single platform (e.g. linux-x64)")]
    platform: Option<String>,
}

#[derive(Debug, Clone)]
struct Target {
    target: &'static str,
    platform: &'static str,
    binary_name: &'static str,
    package_dir: &'static str,
    addon_package_dir: &'static str,
}

const TARGETS: &[Target] = &[
    Target {
        target: "x86_64-unknown-linux-gnu",
        platform: "linux-x64",
        binary_name: "rari",
        package_dir: "packages/rari-linux-x64",
        addon_package_dir: "packages/use-cache-linux-x64",
    },
    Target {
        target: "aarch64-unknown-linux-gnu",
        platform: "linux-arm64",
        binary_name: "rari",
        package_dir: "packages/rari-linux-arm64",
        addon_package_dir: "packages/use-cache-linux-arm64",
    },
    Target {
        target: "x86_64-apple-darwin",
        platform: "darwin-x64",
        binary_name: "rari",
        package_dir: "packages/rari-darwin-x64",
        addon_package_dir: "packages/use-cache-darwin-x64",
    },
    Target {
        target: "aarch64-apple-darwin",
        platform: "darwin-arm64",
        binary_name: "rari",
        package_dir: "packages/rari-darwin-arm64",
        addon_package_dir: "packages/use-cache-darwin-arm64",
    },
    Target {
        target: "x86_64-pc-windows-msvc",
        platform: "win32-x64",
        binary_name: "rari.exe",
        package_dir: "packages/rari-win32-x64",
        addon_package_dir: "packages/use-cache-win32-x64",
    },
    Target {
        target: "aarch64-pc-windows-msvc",
        platform: "win32-arm64",
        binary_name: "rari.exe",
        package_dir: "packages/rari-win32-arm64",
        addon_package_dir: "packages/use-cache-win32-arm64",
    },
];

const ADDON_BUILD_DIR: &str = ".build/rari-use-cache";
const ADDON_CANONICAL_PACKAGE_DIR: &str = "packages/use-cache";
const ADDON_OUTPUT_FILE: &str = "rari_use_cache.node";

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

async fn build_binary(target: &str, project_root: &Path, dev_mode: bool) -> Result<bool> {
    let build_type = if dev_mode { "debug" } else { "release" };
    log(&format!("Building binary for {} ({})", target, build_type));

    let mut cmd = Command::new("cargo");
    cmd.arg("build");

    if !dev_mode {
        cmd.arg("--release");
    }

    cmd.args(["--target", target, "--bin", "rari"]).current_dir(project_root);

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

fn copy_binary_to_platform_package(
    target_info: &Target,
    project_root: &Path,
    dev_mode: bool,
) -> Result<bool> {
    let build_type = if dev_mode { "debug" } else { "release" };
    let source_path = project_root
        .join("target")
        .join(target_info.target)
        .join(build_type)
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
    if !target_info.platform.starts_with("win32") {
        let mut perms = fs::metadata(&dest_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_path, perms)?;

        if target_info.platform.starts_with("darwin") {
            if std::env::consts::OS != "macos" {
                log_warning("Skipping codesign: host OS is not macOS");
            } else {
                match dest_path.to_str() {
                    Some(path_str) => {
                        let sign_result = std::process::Command::new("codesign")
                            .args(["-s", "-", path_str])
                            .output();
                        match sign_result {
                            Ok(output) if output.status.success() => {
                                log_success(&format!("Ad-hoc signed: {}", dest_path.display()));
                            }
                            Ok(output) => {
                                log_warning(&format!(
                                    "codesign failed: {}",
                                    String::from_utf8_lossy(&output.stderr)
                                ));
                            }
                            Err(e) => {
                                log_warning(&format!("codesign not available: {}", e));
                            }
                        }
                    }
                    None => {
                        log_warning(&format!(
                            "Skipping codesign: path contains invalid UTF-8: {}",
                            dest_path.display()
                        ));
                    }
                }
            }
        }
    }

    log_success(&format!("Copied binary to: {}", dest_path.display()));
    Ok(true)
}

fn validate_binary(target_info: &Target, project_root: &Path, dev_mode: bool) -> Result<bool> {
    let binary_path =
        project_root.join(target_info.package_dir).join("bin").join(target_info.binary_name);

    if !binary_path.exists() {
        log_error(&format!("Binary not found: {}", binary_path.display()));
        return Ok(false);
    }

    #[cfg(unix)]
    if !target_info.platform.starts_with("win32") {
        let metadata = fs::metadata(&binary_path)?;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            log_error(&format!("Binary is not executable: {}", binary_path.display()));
            return Ok(false);
        }
    }

    let metadata = fs::metadata(&binary_path)?;
    let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
    let build_type = if dev_mode { "debug" } else { "release" };

    log_success(&format!(
        "Binary validated: {} ({:.2} MB, {})",
        binary_path.display(),
        size_mb,
        build_type
    ));
    Ok(true)
}

fn addon_napi_output_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root.join(ADDON_BUILD_DIR).join(format!("rari-use-cache.{}.node", target_info.platform))
}

fn addon_stable_output_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root.join(ADDON_BUILD_DIR).join(target_info.platform).join(ADDON_OUTPUT_FILE)
}

fn addon_platform_package_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root.join(target_info.addon_package_dir).join(ADDON_OUTPUT_FILE)
}

fn addon_canonical_package_path(project_root: &Path) -> PathBuf {
    project_root.join(ADDON_CANONICAL_PACKAGE_DIR).join(ADDON_OUTPUT_FILE)
}

async fn build_addon(target_info: &Target, project_root: &Path, dev_mode: bool) -> Result<bool> {
    log(&format!(
        "Building rari-use-cache addon for {} ({})",
        target_info.platform,
        if dev_mode { "debug" } else { "release" }
    ));

    let manifest_dir = project_root.join("crates/rari-use-cache");
    let out_dir = project_root.join(ADDON_BUILD_DIR);
    fs::create_dir_all(&out_dir).context("Failed to create addon build dir")?;

    let abs_out_dir = out_dir.canonicalize().unwrap_or(out_dir.clone());

    let current_platform = get_current_platform_target();
    let is_current_platform = current_platform.is_some_and(|t| t.platform == target_info.platform);

    let mut args: Vec<String> = vec![
        "build".to_string(),
        "--platform".to_string(),
        "--cwd".to_string(),
        manifest_dir.to_string_lossy().to_string(),
    ];
    if !dev_mode {
        args.push("--release".to_string());
    }
    args.push("--strip".to_string());
    args.push("--no-js".to_string());

    if !is_current_platform {
        args.push("--target".to_string());
        args.push(target_info.target.to_string());
    }

    args.push("--output-dir".to_string());
    args.push(abs_out_dir.to_string_lossy().to_string());

    log(&format!("running: pnpm exec napi {}", args.join(" ")));

    let mut cmd = Command::new("pnpm");
    cmd.arg("exec")
        .arg("napi")
        .args(&args)
        .current_dir(project_root)
        .env("CARGO_CFG_TARGET_ARCH", std::env::consts::ARCH)
        .env("CARGO_CFG_TARGET_OS", std::env::consts::OS)
        .env(
            "CARGO_CFG_TARGET_ENV",
            if cfg!(target_env = "gnu") {
                "gnu"
            } else if cfg!(target_env = "msvc") {
                "msvc"
            } else {
                ""
            },
        );

    let output = cmd.output().await.context("Failed to execute napi build")?;

    if !output.status.success() {
        log_error(&format!("Failed to build addon for {}", target_info.platform));
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stderr.is_empty() {
            log_error(&format!("stderr: {}", stderr));
        }
        if !stdout.is_empty() {
            log_error(&format!("stdout: {}", stdout));
        }
        return Ok(false);
    }

    let src = addon_napi_output_path(target_info, project_root);
    if !src.exists() {
        log_error(&format!("expected addon artifact not found: {}", src.display()));
        return Ok(false);
    }

    let stable = addon_stable_output_path(target_info, project_root);
    if let Some(parent) = stable.parent() {
        fs::create_dir_all(parent).context("Failed to create per-platform addon build dir")?;
    }
    if src != stable {
        if stable.exists() {
            fs::remove_file(&stable).context("Failed to remove stale addon artifact")?;
        }
        fs::rename(&src, &stable).context("Failed to rename addon artifact")?;
    }

    if let Some(parent) = stable.parent() {
        for sibling in ["index.js", "index.d.ts"] {
            let p = parent.join(sibling);
            if p.exists() {
                let _ = fs::remove_file(&p);
            }
        }
    }

    if is_current_platform {
        let build_type = if dev_mode { "debug" } else { "release" };
        let target_dir = project_root.join("target").join(build_type);
        fs::create_dir_all(&target_dir).context("Failed to create target directory")?;

        let test_dest = target_dir.join(ADDON_OUTPUT_FILE);
        fs::copy(&stable, &test_dest).context("Failed to copy addon to target dir for tests")?;
        log_success(&format!("Copied addon to {} for local testing", test_dest.display()));
    }

    log_success(&format!("Built addon for {}", target_info.platform));
    Ok(true)
}

fn copy_addon_to_platform_package(
    target_info: &Target,
    project_root: &Path,
    dev_mode: bool,
) -> Result<bool> {
    let src = addon_stable_output_path(target_info, project_root);
    if !src.exists() {
        log_error(&format!("Addon artifact not found: {}", src.display()));
        return Ok(false);
    }

    let dest = addon_platform_package_path(target_info, project_root);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).context("Failed to create addon package dir")?;
    }
    fs::copy(&src, &dest).context("Failed to copy addon artifact")?;
    log_success(&format!("Copied addon to: {}", dest.display()));

    let _ = dev_mode;
    Ok(true)
}

fn copy_addon_canonical(target_info: &Target, project_root: &Path) -> Result<bool> {
    let src = addon_stable_output_path(target_info, project_root);
    if !src.exists() {
        log_error(&format!("Addon artifact not found: {}", src.display()));
        return Ok(false);
    }

    let dest = addon_canonical_package_path(project_root);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).context("Failed to create canonical addon package dir")?;
    }
    fs::copy(&src, &dest).context("Failed to copy addon to canonical location")?;
    log_success(&format!("Copied addon to canonical dev location: {}", dest.display()));
    Ok(true)
}

fn validate_addon(target_info: &Target, project_root: &Path) -> Result<bool> {
    let dest = addon_platform_package_path(target_info, project_root);
    if !dest.exists() {
        log_error(&format!("Addon not found: {}", dest.display()));
        return Ok(false);
    }
    let metadata = fs::metadata(&dest)?;
    let size_kb = metadata.len() as f64 / 1024.0;
    log_success(&format!("Addon validated: {} ({:.2} KB)", dest.display(), size_kb));
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

    let do_build_bin = args.bin || !args.addon;
    let do_build_addon = args.addon || !args.bin;

    if !do_build_bin && !do_build_addon {
        anyhow::bail!(
            "Nothing to build: both --no-bin and --no-addon semantics engaged (pass --bin and/or --addon)"
        );
    }

    println!(
        "{}",
        "🔧 Preparing Rari platform artifacts (binary and/or rari-use-cache addon)".bold()
    );
    println!();

    let project_root = PathBuf::from(".");

    let mut targets_to_build: Vec<&Target> = if args.all {
        log("Building for all platforms (cross-compilation mode)");
        TARGETS.iter().collect()
    } else if let Some(name) = &args.platform {
        let t = TARGETS.iter().find(|t| t.platform == *name).with_context(|| {
            format!(
                "Unknown platform '{}'. Supported: {}",
                name,
                TARGETS.iter().map(|t| t.platform).collect::<Vec<_>>().join(", ")
            )
        })?;
        log(&format!("Building for explicit platform only: {}", t.platform.cyan()));
        vec![t]
    } else {
        let current_target = get_current_platform_target().context(
            "Unable to determine current platform target. Supported platforms: macOS (x64/ARM64), Linux (x64/ARM64), Windows (x64/ARM64). Use --platform <name> to override.",
        )?;
        log(&format!("Building for current platform only: {}", current_target.platform.cyan()));
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
            let src = addon_stable_output_path(target_info, &project_root);
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

        if !args.all
            && args.platform.is_none()
            && let Some(target_info) = targets_to_build.first()
        {
            println!();
            log("Copying addon to canonical dev package location...");
            copy_addon_canonical(target_info, &project_root)?;
        }

        println!();
    }

    let total_attempted = targets_to_build.len();

    let any_failure = bin_failure > 0 || addon_failure > 0;
    if !any_failure {
        if do_build_bin {
            log_success(&format!("✨ Successfully prepared {} platform binaries!", bin_success));
        }
        if do_build_addon {
            log_success(&format!("✨ Successfully prepared {} platform addons!", addon_success));
        }
        println!();
        println!("{}", "Platform packages ready:".bold());
        for target_info in &targets_to_build {
            if do_build_bin {
                println!("  • {} → {}", target_info.platform.cyan(), target_info.package_dir);
            }
            if do_build_addon {
                println!("  • {} → {}", target_info.platform.cyan(), target_info.addon_package_dir);
            }
        }
        println!();
        println!("{}", "Next steps:".dimmed());
        if !args.all {
            println!("{}", "  1. Test the artifacts locally".dimmed());
            println!("{}", "  2. Use GitHub Actions for full cross-platform builds".dimmed());
            println!(
                "{}",
                "  3. Or run with --all flag (requires cross-compilation setup)".dimmed()
            );
        } else {
            println!("{}", "  1. Test the artifacts locally".dimmed());
            println!("{}", "  2. Run the release script: pnpm run release".dimmed());
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
                if do_build_addon && addon_stable_output_path(target_info, &project_root).exists() {
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
