use anyhow::{Context, Result};
use cow_utils::CowUtils;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;

use crate::common::{Target, get_current_platform_target, log, log_error, log_success};

const ADDON_BUILD_DIR: &str = ".build/rari-use-cache";
const ADDON_OUTPUT_FILE: &str = "rari_use_cache.node";

fn addon_napi_output_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root
        .join(ADDON_BUILD_DIR)
        .join(format!("rari-use-cache.{}.node", target_info.platform))
}

fn addon_stable_output_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root
        .join(ADDON_BUILD_DIR)
        .join(target_info.platform)
        .join(ADDON_OUTPUT_FILE)
}

fn addon_platform_package_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root
        .join(target_info.addon_package_dir)
        .join(ADDON_OUTPUT_FILE)
}

pub async fn build_addon(
    target_info: &Target,
    project_root: &Path,
    dev_mode: bool,
) -> Result<bool> {
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

    let target_parts: Vec<&str> = target_info.target.split('-').collect();
    let target_arch = target_parts.first().unwrap_or(&"unknown");
    let target_os = if target_info.target.contains("darwin") {
        "macos"
    } else if target_info.target.contains("linux") {
        "linux"
    } else if target_info.target.contains("windows") {
        "windows"
    } else {
        "unknown"
    };
    let target_env = if target_info.target.contains("gnu") {
        "gnu"
    } else if target_info.target.contains("msvc") {
        "msvc"
    } else {
        ""
    };

    let mut cmd = Command::new("pnpm");
    cmd.arg("exec")
        .arg("napi")
        .args(&args)
        .current_dir(project_root)
        .env("CARGO_CFG_TARGET_ARCH", target_arch)
        .env("CARGO_CFG_TARGET_OS", target_os)
        .env("CARGO_CFG_TARGET_ENV", target_env);

    let output = cmd.output().await.context("Failed to execute napi build")?;

    if !output.status.success() {
        log_error(&format!(
            "Failed to build addon for {}",
            target_info.platform
        ));
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
        log_error(&format!(
            "expected addon artifact not found: {}",
            src.display()
        ));
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
        log_success(&format!(
            "Copied addon to {} for local testing",
            test_dest.display()
        ));
    }

    log_success(&format!("Built addon for {}", target_info.platform));
    Ok(true)
}

pub fn copy_addon_to_platform_package(
    target_info: &Target,
    project_root: &Path,
    _dev_mode: bool,
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

    let package_dir = project_root.join(target_info.addon_package_dir);
    generate_platform_package_files(target_info, &package_dir, project_root)?;

    Ok(true)
}

pub fn validate_addon(target_info: &Target, project_root: &Path) -> Result<bool> {
    let dest = addon_platform_package_path(target_info, project_root);
    if !dest.exists() {
        log_error(&format!("Addon not found: {}", dest.display()));
        return Ok(false);
    }
    let metadata = fs::metadata(&dest)?;
    let size_kb = metadata.len() as f64 / 1024.0;
    log_success(&format!(
        "Addon validated: {} ({:.2} KB)",
        dest.display(),
        size_kb
    ));
    Ok(true)
}

pub fn addon_stable_output_path_public(target_info: &Target, project_root: &Path) -> PathBuf {
    addon_stable_output_path(target_info, project_root)
}

fn generate_platform_package_files(
    target_info: &Target,
    package_dir: &Path,
    project_root: &Path,
) -> Result<()> {
    let package_name = package_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let (os, cpu) = match target_info.platform {
        "darwin-arm64" => ("darwin", "arm64"),
        "darwin-x64" => ("darwin", "x64"),
        "linux-arm64" => ("linux", "arm64"),
        "linux-x64" => ("linux", "x64"),
        "win32-arm64" => ("win32", "arm64"),
        "win32-x64" => ("win32", "x64"),
        _ => {
            return Err(anyhow::anyhow!(
                "Unrecognized platform '{}'. Expected one of: darwin-arm64, darwin-x64, linux-arm64, linux-x64, win32-arm64, win32-x64",
                target_info.platform
            ));
        }
    };

    let template_package_json_path =
        project_root.join(".github/templates/package-json/use-cache-platform.json");
    let template_index_js_path = project_root.join(".github/templates/js/use-cache-platform.js");

    let package_json_template = fs::read_to_string(&template_package_json_path)
        .context("Failed to read package.json template")?;

    let package_json = package_json_template
        .cow_replace("{NAME}", &package_name)
        .cow_replace("{VERSION}", "0.0.0-dev")
        .cow_replace("{DESCRIPTION}", target_info.platform)
        .cow_replace("{OS}", os)
        .cow_replace("{CPU}", cpu)
        .into_owned();

    let index_js =
        fs::read_to_string(&template_index_js_path).context("Failed to read index.js template")?;

    fs::write(package_dir.join("package.json"), package_json)
        .context("Failed to write platform package.json")?;
    fs::write(package_dir.join("index.js"), index_js)
        .context("Failed to write platform index.js")?;

    log_success(&format!("Generated package files for {}", package_name));
    Ok(())
}
