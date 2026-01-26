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

pub async fn install_dependencies(project_root: &Path) -> Result<()> {
    let output = Command::new("pnpm").args(["install"]).current_dir(project_root).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("Failed to install dependencies:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    Ok(())
}

pub async fn publish_package(
    package_path: &Path,
    is_prerelease: bool,
    otp: Option<&str>,
) -> Result<()> {
    let mut args = vec!["publish", "--access", "public"];

    if is_prerelease {
        args.push("--tag");
        args.push("next");
    }

    let env_otp = std::env::var("NPM_OTP").ok();
    let otp_value = otp.or(env_otp.as_deref());

    if let Some(otp_code) = otp_value {
        args.push("--otp");
        args.push(otp_code);
    }

    let output = Command::new("pnpm").args(&args).current_dir(package_path).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if stderr.contains("EOTP") {
            if otp_value.is_some() {
                anyhow::bail!(
                    "npm publish failed: OTP code is invalid or expired.\nPlease restart with a fresh OTP code."
                );
            } else {
                anyhow::bail!(
                    "npm publish requires a one-time password.\nPlease provide an OTP code."
                );
            }
        }

        anyhow::bail!("Failed to publish package:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    Ok(())
}

pub async fn generate_changelog(tag: &str, project_root: &Path) -> Result<()> {
    let output = Command::new("git-cliff")
        .args(["--tag", tag, "--output", "CHANGELOG.md"])
        .current_dir(project_root)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("Failed to generate changelog:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    Ok(())
}
