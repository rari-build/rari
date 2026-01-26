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

pub async fn push_changes() -> Result<()> {
    let output = Command::new("git").args(["push"]).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("Failed to push commits:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    let output = Command::new("git").args(["push", "--tags"]).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("Failed to push tags:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    Ok(())
}

pub async fn get_repo_info() -> Result<(String, String)> {
    let output =
        Command::new("git").args(["config", "--get", "remote.origin.url"]).output().await?;

    if !output.status.success() {
        anyhow::bail!("Failed to get git remote URL");
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let parts: Vec<&str> = if url.starts_with("git@") {
        url.trim_start_matches("git@github.com:").trim_end_matches(".git").split('/').collect()
    } else {
        url.trim_start_matches("https://github.com/").trim_end_matches(".git").split('/').collect()
    };

    if parts.len() >= 2 {
        Ok((parts[0].to_string(), parts[1].to_string()))
    } else {
        anyhow::bail!("Could not parse GitHub repository from URL: {}", url);
    }
}
