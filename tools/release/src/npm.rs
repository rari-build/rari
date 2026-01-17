use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

pub async fn build_package(package_path: &Path) -> Result<()> {
    Command::new("pnpm").args(["build"]).current_dir(package_path).output().await?;

    Ok(())
}

pub async fn publish_package(package_path: &Path, is_prerelease: bool) -> Result<()> {
    let mut args = vec!["publish", "--access", "public"];

    if is_prerelease {
        args.push("--tag");
        args.push("next");
    }

    Command::new("npm").args(&args).current_dir(package_path).output().await?;

    Ok(())
}

pub async fn generate_changelog(tag: &str, project_root: &Path) -> Result<()> {
    Command::new("pnpm")
        .args(["exec", "git-cliff", "--tag", tag, "--output", "CHANGELOG.md"])
        .current_dir(project_root)
        .output()
        .await?;

    Ok(())
}
