use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

pub async fn generate(tag: &str, package_name: &str, package_path: &Path) -> Result<()> {
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
