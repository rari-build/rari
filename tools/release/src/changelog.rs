use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

pub async fn generate(tag: &str, package_name: &str, package_path: &Path) -> Result<()> {
    let changelog_path = package_path.join("CHANGELOG.md");

    let mut args = vec![
        "--tag".to_string(),
        tag.to_string(),
        "--include-path".to_string(),
        format!("{}/**", package_path.display()),
    ];

    if package_name == "rari" {
        args.push("--include-path".to_string());
        args.push("crates/rari/**".to_string());
    }

    args.push("--output".to_string());
    args.push(changelog_path.display().to_string());

    let output = Command::new("git-cliff").args(&args).output().await?;

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
