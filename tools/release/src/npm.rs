use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

pub async fn check_npm_login() -> Result<()> {
    let output = Command::new("npm").args(["whoami"]).output().await?;

    if !output.status.success() {
        anyhow::bail!(
            "You are not logged into npm. Please run 'npm login' before proceeding with the release."
        );
    }

    Ok(())
}

pub async fn build_package(package_path: &Path) -> Result<()> {
    let output = Command::new("pnpm").args(["build"]).current_dir(package_path).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("Failed to build package:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    Ok(())
}

pub async fn publish_package(
    package_path: &Path,
    is_prerelease: bool,
    otp: Option<&str>,
) -> Result<()> {
    let pack_output =
        Command::new("pnpm").args(["pack"]).current_dir(package_path).output().await?;

    if !pack_output.status.success() {
        let stderr = String::from_utf8_lossy(&pack_output.stderr);
        let stdout = String::from_utf8_lossy(&pack_output.stdout);
        anyhow::bail!("Failed to pack package:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    let pack_output_str = String::from_utf8_lossy(&pack_output.stdout);
    let tgz_filename = pack_output_str
        .lines()
        .last()
        .and_then(|line| line.split_whitespace().last())
        .ok_or_else(|| anyhow::anyhow!("Could not determine packed tarball filename"))?;

    let mut args = vec!["publish", tgz_filename, "--access", "public"];

    if is_prerelease {
        args.push("--tag");
        args.push("next");
    }

    let env_otp = std::env::var("NPM_OTP").ok().or_else(|| std::env::var("PNPM_CONFIG_OTP").ok());
    let otp_value = otp.or(env_otp.as_deref());

    if let Some(otp_code) = otp_value {
        args.push("--otp");
        args.push(otp_code);
    }

    let mut cmd = Command::new("npm");
    cmd.args(&args).current_dir(package_path);

    let output = cmd.output().await?;

    let _ = tokio::fs::remove_file(package_path.join(tgz_filename)).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("Failed to publish package:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    Ok(())
}

pub async fn generate_changelog(tag: &str, package_name: &str, package_path: &Path) -> Result<()> {
    let changelog_path = package_path.join("CHANGELOG.md");

    let output = Command::new("git-cliff")
        .args([
            "--tag",
            tag,
            "--include-path",
            &format!("{}/**", package_path.display()),
            "--output",
            &changelog_path.display().to_string(),
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "Failed to generate changelog for {}:\nstdout: {}\nstderr: {}",
            package_name,
            stdout,
            stderr
        );
    }

    Ok(())
}
