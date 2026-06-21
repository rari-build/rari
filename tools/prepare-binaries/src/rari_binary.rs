use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tokio::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::common::{Target, log, log_error, log_success, log_warning};

pub async fn build_binary(target: &str, project_root: &Path, dev_mode: bool) -> Result<bool> {
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

pub fn copy_binary_to_platform_package(
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

pub fn validate_binary(target_info: &Target, project_root: &Path, dev_mode: bool) -> Result<bool> {
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
