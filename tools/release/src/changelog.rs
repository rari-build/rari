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
    } else if package_name == "@rari/use-cache" {
        args.push("--include-path".to_string());
        args.push("crates/rari-use-cache/**".to_string());
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

pub async fn generate_release_notes(
    tag: &str,
    package_name: &str,
    previous_tag: Option<&str>,
) -> Result<String> {
    let range = previous_tag.map(|prev| format!("{}..{}", prev, tag));

    let flags: Vec<&str> = if range.is_some() {
        vec![""]
    } else {
        vec!["--current", "--latest"]
    };

    for flag in flags {
        let mut args = Vec::new();

        if !flag.is_empty() {
            args.push(flag.to_string());
        }

        if package_name == "rari-binaries" {
            args.push("--include-path".to_string());
            args.push("crates/**".to_string());
            args.push("--include-path".to_string());
            args.push("Cargo.toml".to_string());
            args.push("--include-path".to_string());
            args.push("Cargo.lock".to_string());
            args.push("--tag-pattern".to_string());
            args.push("^v".to_string());
        } else if package_name == "rari" {
            args.push("--include-path".to_string());
            args.push("packages/rari/**".to_string());
            args.push("--include-path".to_string());
            args.push("crates/rari/**".to_string());
            args.push("--tag-pattern".to_string());
            args.push("^rari@".to_string());
        } else if package_name == "create-rari-app" {
            args.push("--include-path".to_string());
            args.push("packages/create-rari-app/**".to_string());
            args.push("--tag-pattern".to_string());
            args.push("^create-rari-app@".to_string());
        } else if package_name == "@rari/use-cache" {
            args.push("--include-path".to_string());
            args.push("packages/use-cache/**".to_string());
            args.push("--include-path".to_string());
            args.push("crates/rari-use-cache/**".to_string());
            args.push("--tag-pattern".to_string());
            args.push("^@rari/use-cache@".to_string());
        } else if package_name == "@rari/use-cache-binaries" {
            args.push("--include-path".to_string());
            args.push("crates/rari-use-cache/**".to_string());
            args.push("--include-path".to_string());
            args.push("packages/use-cache-*/**".to_string());
            args.push("--tag-pattern".to_string());
            args.push("^use-cache-binaries@".to_string());
        }

        if let Some(ref r) = range {
            args.push(r.clone());
        }

        let output = Command::new("git-cliff").args(&args).output().await?;

        if output.status.success() {
            let notes = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !notes.is_empty() {
                let notes = notes
                    .lines()
                    .skip_while(|line| line.starts_with("## "))
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string();
                if !notes.is_empty() {
                    return Ok(notes);
                }
            }
        }
    }

    let _ = tag;
    Ok("See CHANGELOG.md for details.".to_string())
}
