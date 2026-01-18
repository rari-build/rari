use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::state::AppState;
use crate::target::Target;

pub async fn check_rust_installed(state: &mut AppState) -> Result<()> {
    state.add_log("Checking Rust installation...".to_string());

    let output = Command::new("cargo")
        .arg("--version")
        .output()
        .await
        .context("Failed to check cargo version")?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        state.add_log(format!("Rust/Cargo found: {}", version.trim()));
        Ok(())
    } else {
        state.add_log("Rust/Cargo is not installed".to_string());
        anyhow::bail!("Rust not installed. Please install from https://rustup.rs/");
    }
}

pub async fn install_target(target: &str, state: &mut AppState) -> Result<()> {
    state.add_log(format!("Installing Rust target: {}", target));

    let output = Command::new("rustup")
        .args(["target", "add", target])
        .output()
        .await
        .context("Failed to install target")?;

    if output.status.success() {
        state.add_log(format!("Installed target: {}", target));
        Ok(())
    } else {
        state.add_log(format!("Warning: Failed to install target {}", target));
        Ok(())
    }
}

pub async fn build_binary(
    target: &str,
    project_root: &Path,
    log_tx: mpsc::UnboundedSender<String>,
    progress_tx: mpsc::UnboundedSender<(usize, Option<usize>)>,
) -> Result<bool> {
    log_tx.send(format!("Building binary for {}...", target)).ok();

    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--release", "--target", target, "--bin", "rari"])
        .current_dir(project_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if target == "aarch64-unknown-linux-gnu" {
        cmd.env("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER", "aarch64-linux-gnu-gcc");
    }

    let mut child = cmd.spawn().context("Failed to spawn cargo build")?;

    if let Some(stderr) = child.stderr.take() {
        let log_tx_clone = log_tx.clone();
        let progress_tx_clone = progress_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut compiled_count = 0;
            let mut total_crates: Option<usize> = None;

            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && (trimmed.starts_with("Compiling")
                        || trimmed.starts_with("Finished")
                        || trimmed.starts_with("Building")
                        || trimmed.starts_with("Downloaded")
                        || trimmed.starts_with("Downloading")
                        || trimmed.contains("error")
                        || trimmed.contains("warning"))
                {
                    log_tx_clone.send(trimmed.to_string()).ok();

                    if trimmed.starts_with("Compiling") {
                        compiled_count += 1;

                        if total_crates.is_none()
                            && let Some(paren_start) = trimmed.rfind('(')
                            && let Some(slash_pos) = trimmed[paren_start..].find('/')
                        {
                            let total_str = &trimmed[paren_start + slash_pos + 1..];
                            if let Some(paren_end) = total_str.find(')')
                                && let Ok(total) = total_str[..paren_end].trim().parse::<usize>()
                            {
                                total_crates = Some(total);
                            }
                        }

                        progress_tx_clone.send((compiled_count, total_crates)).ok();
                    }
                }
            }
        });
    }

    let status = child.wait().await.context("Failed to wait for cargo build")?;

    if status.success() {
        log_tx.send(format!("Built binary for {}", target)).ok();
        Ok(true)
    } else {
        log_tx.send(format!("Failed to build {}", target)).ok();
        Ok(false)
    }
}

pub fn copy_binary_to_platform_package(
    target_info: &Target,
    project_root: &Path,
    state: &mut AppState,
) -> Result<bool> {
    let source_path = project_root
        .join("target")
        .join(target_info.target)
        .join("release")
        .join(target_info.binary_name);

    let dest_dir = project_root.join(target_info.package_dir).join("bin");
    let dest_path = dest_dir.join(target_info.binary_name);

    if !source_path.exists() {
        state.add_log(format!("Binary not found: {}", source_path.display()));
        return Ok(false);
    }

    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).context("Failed to create destination directory")?;
    }

    fs::copy(&source_path, &dest_path).context("Failed to copy binary")?;

    #[cfg(unix)]
    if target_info.platform != "win32-x64" {
        let mut perms = fs::metadata(&dest_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_path, perms)?;
    }

    state.add_log(format!("Copied binary to {}", target_info.package_dir));
    Ok(true)
}

pub fn validate_binary(
    target_info: &Target,
    project_root: &Path,
    state: &mut AppState,
) -> Result<Option<f64>> {
    let binary_path =
        project_root.join(target_info.package_dir).join("bin").join(target_info.binary_name);

    if !binary_path.exists() {
        state.add_log(format!("Binary not found: {}", binary_path.display()));
        return Ok(None);
    }

    #[cfg(unix)]
    if target_info.platform != "win32-x64" {
        let metadata = fs::metadata(&binary_path)?;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            state.add_log(format!("Binary is not executable: {}", binary_path.display()));
            return Ok(None);
        }
    }

    let metadata = fs::metadata(&binary_path)?;
    let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;

    state.add_log(format!("Validated {} ({:.2} MB)", target_info.platform, size_mb));
    Ok(Some(size_mb))
}
