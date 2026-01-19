use anyhow::Result;
use colored::Colorize;
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
    use std::io::{self, Write};

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

        if stderr.contains("EOTP") && otp.is_none() {
            println!("\n{}", "This operation requires a one-time password.".yellow());
            print!("Enter OTP from your authenticator: ");
            io::stdout().flush()?;

            let mut otp_input = String::new();
            io::stdin().read_line(&mut otp_input)?;
            let otp_input = otp_input.trim();

            let mut retry_args = vec!["publish", "--access", "public"];
            if is_prerelease {
                retry_args.push("--tag");
                retry_args.push("next");
            }
            retry_args.push("--otp");
            retry_args.push(otp_input);

            let retry_output =
                Command::new("npm").args(&retry_args).current_dir(package_path).output().await?;

            if !retry_output.status.success() {
                let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
                let retry_stdout = String::from_utf8_lossy(&retry_output.stdout);
                anyhow::bail!(
                    "Failed to publish package:\nstdout: {}\nstderr: {}",
                    retry_stdout,
                    retry_stderr
                );
            }

            return Ok(());
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
