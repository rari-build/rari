use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

pub async fn get_recent_commits(package_path: &Path, limit: usize) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args([
            "log",
            "--oneline",
            "-n",
            &limit.to_string(),
            "--",
            &package_path.display().to_string(),
        ])
        .output()
        .await?;

    let commits = String::from_utf8_lossy(&output.stdout).lines().map(String::from).collect();

    Ok(commits)
}

pub async fn get_commits_since_tag(package_name: &str, package_path: &Path) -> Result<Vec<String>> {
    let tag_output =
        Command::new("git").args(["tag", "-l", &format!("{}@*", package_name)]).output().await?;

    let tags = String::from_utf8_lossy(&tag_output.stdout);
    let latest_tag = tags.lines().last();

    if let Some(tag) = latest_tag {
        let sha_output = Command::new("git").args(["rev-list", "-n", "1", tag]).output().await?;

        let sha = String::from_utf8_lossy(&sha_output.stdout).trim().to_string();

        let output = Command::new("git")
            .args([
                "log",
                &format!("{}..HEAD", sha),
                "--oneline",
                "--",
                &package_path.display().to_string(),
            ])
            .output()
            .await?;

        Ok(String::from_utf8_lossy(&output.stdout).lines().map(String::from).collect())
    } else {
        get_recent_commits(package_path, 10).await
    }
}

pub async fn add_and_commit(message: &str, cwd: &Path) -> Result<()> {
    Command::new("git").args(["add", "."]).current_dir(cwd).output().await?;

    Command::new("git").args(["commit", "-m", message]).current_dir(cwd).output().await?;

    Ok(())
}

pub async fn create_tag(tag: &str) -> Result<()> {
    Command::new("git").args(["tag", tag]).output().await?;

    Ok(())
}
