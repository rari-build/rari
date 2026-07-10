use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::{fs, process::Command};

const NOTES_DIR: &str = ".github/release-notes";

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
        args.push("crates/rari_use_cache/**".to_string());
    }

    args.push("--output".to_string());
    args.push(changelog_path.display().to_string());

    let output = Command::new("git-cliff").args(&args).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "Failed to generate changelog for {package_name}:\nstdout: {stdout}\nstderr: {stderr}"
        );
    }

    Ok(())
}

/// Resolve manual release notes path.
///
/// Lookup order:
/// 1. Explicit `--notes-file` / override
/// 2. `.github/release-notes/<tag>.md` (e.g. `rari@0.15.0.md`, `v0.15.0.md`)
/// 3. `.github/release-notes/<version>.md` (shared across units, e.g. `0.15.0.md`)
pub fn resolve_notes_path(
    tag: &str,
    version: &str,
    override_path: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(path) = override_path {
        return path.exists().then(|| path.to_path_buf());
    }

    let dir = Path::new(NOTES_DIR);
    let candidates = [dir.join(format!("{tag}.md")), dir.join(format!("{version}.md"))];
    candidates.into_iter().find(|path| path.exists())
}

pub async fn load_manual_notes(
    tag: &str,
    version: &str,
    override_path: Option<&Path>,
) -> Result<Option<(PathBuf, String)>> {
    let Some(path) = resolve_notes_path(tag, version, override_path) else {
        return Ok(None);
    };

    let content = fs::read_to_string(&path).await?;
    let content = content.trim().to_string();
    if content.is_empty() {
        return Ok(None);
    }

    Ok(Some((path, content)))
}

pub fn compose_release_notes(manual: Option<&str>, auto: &str) -> String {
    let manual = manual.map(str::trim).filter(|s| !s.is_empty());
    let auto = auto.trim();

    match manual {
        Some(manual) if auto.is_empty() || auto == "See CHANGELOG.md for details." => {
            manual.to_string()
        }
        Some(manual) => format!("{manual}\n\n---\n\n{auto}"),
        None => auto.to_string(),
    }
}

/// Insert manual notes under the newly generated version heading in CHANGELOG.md.
pub async fn inject_manual_notes(package_path: &Path, version: &str, manual: &str) -> Result<()> {
    let manual = manual.trim();
    if manual.is_empty() {
        return Ok(());
    }

    let path = package_path.join("CHANGELOG.md");
    let content = fs::read_to_string(&path).await?;
    let marker = format!("## [{version}]");

    let Some(idx) = content.find(&marker) else {
        return Ok(());
    };

    let line_end = content[idx..].find('\n').map_or(content.len(), |i| idx + i + 1);
    let mut insert_at = line_end;
    if content[insert_at..].starts_with('\n') {
        insert_at += 1;
    }

    let mut new_content = String::with_capacity(content.len() + manual.len() + 2);
    new_content.push_str(&content[..insert_at]);
    new_content.push_str(manual);
    new_content.push_str("\n\n");
    new_content.push_str(&content[insert_at..]);
    fs::write(&path, new_content).await?;

    Ok(())
}

pub async fn generate_release_notes(
    tag: &str,
    package_name: &str,
    previous_tag: Option<&str>,
    version: &str,
    notes_file: Option<&Path>,
) -> Result<String> {
    let manual = load_manual_notes(tag, version, notes_file).await?;
    let auto = generate_auto_release_notes(tag, package_name, previous_tag).await?;
    Ok(compose_release_notes(manual.as_ref().map(|(_, body)| body.as_str()), &auto))
}

async fn generate_auto_release_notes(
    tag: &str,
    package_name: &str,
    previous_tag: Option<&str>,
) -> Result<String> {
    let range = previous_tag.map(|prev| format!("{prev}..{tag}"));

    let flags: Vec<&str> = if range.is_some() { vec![""] } else { vec!["--current", "--latest"] };

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
            args.push("crates/rari_use_cache/**".to_string());
            args.push("--tag-pattern".to_string());
            args.push("^@rari/use-cache@".to_string());
        } else if package_name == "@rari/use-cache-binaries" {
            args.push("--include-path".to_string());
            args.push("crates/rari_use_cache/**".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_prepends_manual_with_separator() {
        let out =
            compose_release_notes(Some("## Highlights\n\n- big thing"), "### Features\n\n- x");
        assert!(out.starts_with("## Highlights"));
        assert!(out.contains("\n\n---\n\n### Features"));
    }

    #[test]
    fn compose_manual_only_when_auto_placeholder() {
        let out =
            compose_release_notes(Some("## Highlights\n\n- only"), "See CHANGELOG.md for details.");
        assert_eq!(out, "## Highlights\n\n- only");
    }

    #[test]
    fn compose_auto_only_without_manual() {
        let out = compose_release_notes(None, "### Features\n\n- x");
        assert_eq!(out, "### Features\n\n- x");
    }
}
