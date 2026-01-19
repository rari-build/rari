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

pub async fn publish_package(package_path: &Path, is_prerelease: bool) -> Result<()> {
    let mut args = vec!["publish", "--access", "public"];

    if is_prerelease {
        args.push("--tag");
        args.push("next");
    }

    let otp = std::env::var("NPM_OTP").ok();
    if let Some(ref otp_value) = otp {
        args.push("--otp");
        args.push(otp_value);
    }

    let output = Command::new("npm").args(&args).current_dir(package_path).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if stderr.contains("EOTP") {
            anyhow::bail!(
                "npm publish requires a one-time password.\nPlease run: NPM_OTP=<code> just release"
            );
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
