use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use cow_utils::CowUtils;
use tokio::process::Command;

use crate::common::{Target, get_current_platform_target, log, log_error, log_success};

const ADDON_BUILD_DIR: &str = ".build/rari_use_cache";
const ADDON_OUTPUT_FILE: &str = "rari_use_cache.node";
const USE_CACHE_INDEX_JS: &str = r"import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const __dirname = dirname(fileURLToPath(import.meta.url))

const addon = require(join(__dirname, 'rari_use_cache.node'))

export default addon
";

fn addon_napi_output_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root.join(ADDON_BUILD_DIR).join(format!("rari_use_cache.{}.node", target_info.platform))
}

fn napi_artifact_name(platform: &str) -> String {
    let abi = if platform.starts_with("linux") {
        "-gnu"
    } else if platform.starts_with("win32") {
        "-msvc"
    } else {
        ""
    };
    format!("rari_use_cache.{platform}{abi}.node")
}

fn find_fresh_napi_build_output(manifest_dir: &Path, target_info: &Target) -> Option<PathBuf> {
    let primary = manifest_dir.join(napi_artifact_name(target_info.platform));
    if primary.exists() {
        return Some(primary);
    }

    if let Ok(entries) = fs::read_dir(manifest_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "node")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("rari_use_cache"))
            {
                return Some(path);
            }
        }
    }

    None
}

fn cleanup_manifest_build_outputs(manifest_dir: &Path, target_info: &Target) {
    let primary = manifest_dir.join(napi_artifact_name(target_info.platform));
    let _ = fs::remove_file(&primary);
    for name in ["index.js", "index.d.ts"] {
        let _ = fs::remove_file(manifest_dir.join(name));
    }
    if let Ok(entries) = fs::read_dir(manifest_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "node")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("rari_use_cache"))
            {
                let _ = fs::remove_file(path);
            }
        }
    }
}

fn addon_stable_output_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root.join(ADDON_BUILD_DIR).join(target_info.platform).join(ADDON_OUTPUT_FILE)
}

fn addon_platform_package_path(target_info: &Target, project_root: &Path) -> PathBuf {
    project_root.join(target_info.addon_package_dir).join(ADDON_OUTPUT_FILE)
}

pub async fn build_addon(
    target_info: &Target,
    project_root: &Path,
    dev_mode: bool,
) -> Result<bool> {
    log(&format!(
        "Building rari_use_cache addon for {} ({})",
        target_info.platform,
        if dev_mode { "debug" } else { "release" }
    ));

    let manifest_dir = project_root.join("crates/rari_use_cache");

    let current_platform = get_current_platform_target();
    let is_current_platform = current_platform.is_some_and(|t| t.platform == target_info.platform);

    let mut args: Vec<String> =
        vec!["build".to_string(), "--strip".to_string(), "--platform".to_string()];
    if !dev_mode {
        args.push("--release".to_string());
    }

    if !is_current_platform {
        args.push("--target".to_string());
        args.push(target_info.target.to_string());
    }

    log(&format!("running: (cd {}) pnpm exec napi {}", manifest_dir.display(), args.join(" ")));

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
    let cmd_str = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };

    let mut cmd = Command::new(cmd_str);
    cmd.arg("exec")
        .arg("napi")
        .args(&args)
        .current_dir(&manifest_dir)
        .env("CARGO_CFG_TARGET_ARCH", target_arch)
        .env("CARGO_CFG_TARGET_OS", target_os)
        .env("CARGO_CFG_TARGET_ENV", target_env);

    let output = cmd.output().await.context("Failed to execute napi build")?;

    if !output.status.success() {
        log_error(&format!("Failed to build addon for {}", target_info.platform));
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stderr.is_empty() {
            log_error(&format!("stderr: {stderr}"));
        }
        if !stdout.is_empty() {
            log_error(&format!("stdout: {stdout}"));
        }
        return Ok(false);
    }

    let Some(src) = find_fresh_napi_build_output(&manifest_dir, target_info) else {
        log_error(&format!("expected addon artifact not found under {}", manifest_dir.display()));
        return Ok(false);
    };

    let stable = addon_stable_output_path(target_info, project_root);
    if let Some(parent) = stable.parent() {
        fs::create_dir_all(parent).context("Failed to create per-platform addon build dir")?;
    }
    if stable.exists() {
        fs::remove_file(&stable).context("Failed to remove stale addon artifact")?;
    }
    if src != stable {
        fs::copy(&src, &stable).context("Failed to copy addon artifact")?;
    }

    cleanup_manifest_build_outputs(&manifest_dir, target_info);

    let legacy = addon_napi_output_path(target_info, project_root);
    if legacy.exists() {
        let _ = fs::remove_file(&legacy);
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

#[expect(
    clippy::cast_precision_loss,
    reason = "File size in bytes to KB conversion, precision loss acceptable for display"
)]
pub fn validate_addon(target_info: &Target, project_root: &Path) -> Result<bool> {
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

pub fn addon_stable_output_path_public(target_info: &Target, project_root: &Path) -> PathBuf {
    addon_stable_output_path(target_info, project_root)
}

fn render_use_cache_platform_package_json(
    template: &str,
    platform: &str,
    version: &str,
    os: &str,
    cpu: &str,
) -> String {
    template
        .cow_replace("{PLATFORM}", platform)
        .cow_replace("{VERSION}", version)
        .cow_replace("{OS}", os)
        .cow_replace("{CPU}", cpu)
        .into_owned()
}

fn generate_platform_package_files(
    target_info: &Target,
    package_dir: &Path,
    project_root: &Path,
) -> Result<()> {
    let package_name = package_dir.file_name().unwrap_or_default().to_string_lossy();

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

    let package_json_template = fs::read_to_string(&template_package_json_path)
        .context("Failed to read package.json template")?;

    let package_json = render_use_cache_platform_package_json(
        &package_json_template,
        target_info.platform,
        "0.0.0-dev",
        os,
        cpu,
    );

    fs::write(package_dir.join("package.json"), package_json)
        .context("Failed to write platform package.json")?;
    fs::write(package_dir.join("index.js"), USE_CACHE_INDEX_JS)
        .context("Failed to write platform index.js")?;

    log_success(&format!("Generated package files for {package_name}"));
    Ok(())
}
