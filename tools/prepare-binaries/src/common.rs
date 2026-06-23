use anyhow::{Context, Result};
use colored::Colorize;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct Target {
    pub target: &'static str,
    pub platform: &'static str,
    pub binary_name: &'static str,
    pub package_dir: &'static str,
    pub addon_package_dir: &'static str,
}

pub const TARGETS: &[Target] = &[
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

pub fn log(message: &str) {
    println!("{} {}", "➜".cyan(), message);
}

pub fn log_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

pub fn log_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

pub fn log_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

pub fn get_current_platform_target() -> Option<&'static Target> {
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

pub async fn check_rust_installed() -> Result<()> {
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

pub async fn install_target(target: &str) -> Result<()> {
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

pub async fn install_linux_cross_compiler() -> Result<()> {
    if std::env::consts::OS != "linux" {
        return Ok(());
    }

    log("Installing Linux ARM64 cross-compiler...");

    let output = Command::new("sh")
        .args([
            "-c",
            "sudo apt-get update && sudo apt-get install -y gcc-aarch64-linux-gnu",
        ])
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
