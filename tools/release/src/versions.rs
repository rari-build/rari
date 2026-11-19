use std::{collections::BTreeSet, time::Duration};

use semver::Version;
use tokio::{process::Command, task::JoinSet, time::timeout};

use crate::git::tag_pattern_for;

const NPM_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NpmStatus {
    Published { latest: Option<String>, next: Option<String> },
    NotPublished,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpmPackageVersion {
    pub name: String,
    pub status: NpmStatus,
}

impl NpmPackageVersion {
    fn latest(&self) -> Option<&str> {
        match &self.status {
            NpmStatus::Published { latest, .. } => latest.as_deref(),
            _ => None,
        }
    }

    fn label(&self) -> String {
        match &self.status {
            NpmStatus::Published { latest: Some(v), .. } => v.clone(),
            NpmStatus::Published { latest: None, .. } => "no latest".to_string(),
            NpmStatus::NotPublished => "unpublished".to_string(),
            NpmStatus::Unavailable => "unavailable".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VersionSources {
    pub local: Option<String>,
    pub npm: Vec<NpmPackageVersion>,
    pub latest_tag: Option<String>,
}

impl VersionSources {
    pub fn npm_reachable(&self) -> bool {
        self.npm.iter().all(|pkg| pkg.status != NpmStatus::Unavailable)
    }

    fn npm_latests(&self) -> BTreeSet<&str> {
        self.npm.iter().filter_map(NpmPackageVersion::latest).collect()
    }

    pub fn npm_is_mixed(&self) -> bool {
        let latests = self.npm_latests();
        latests.len() > 1
            || (!latests.is_empty()
                && self.npm.iter().any(|pkg| {
                    matches!(
                        pkg.status,
                        NpmStatus::NotPublished | NpmStatus::Published { latest: None, .. }
                    )
                }))
    }

    pub fn effective(&self) -> Option<String> {
        self.npm_latests()
            .into_iter()
            .chain(self.latest_tag.as_deref())
            .chain(self.local.as_deref())
            .filter_map(|v| Version::parse(v).ok())
            .max()
            .map(|v| v.to_string())
    }

    pub fn effective_prerelease(&self) -> Option<String> {
        let nexts = self.npm.iter().filter_map(|pkg| match &pkg.status {
            NpmStatus::Published { next, .. } => next.as_deref(),
            _ => None,
        });
        self.npm_latests()
            .into_iter()
            .chain(nexts)
            .chain(self.latest_tag.as_deref())
            .chain(self.local.as_deref())
            .filter_map(|v| Version::parse(v).ok())
            .max()
            .map(|v| v.to_string())
    }

    pub fn is_out_of_sync(&self) -> bool {
        if self.npm_is_mixed() {
            return true;
        }
        let known: BTreeSet<&str> = self
            .npm_latests()
            .into_iter()
            .chain(self.latest_tag.as_deref())
            .chain(self.local.as_deref())
            .collect();
        known.len() > 1
    }

    pub fn summary(&self) -> String {
        let mut parts = vec![self.npm_summary()];

        let nexts: BTreeSet<&str> = self
            .npm
            .iter()
            .filter_map(|pkg| match &pkg.status {
                NpmStatus::Published { next, .. } => next.as_deref(),
                _ => None,
            })
            .collect();
        if !nexts.is_empty() {
            parts.push(format!("next {}", nexts.into_iter().collect::<Vec<_>>().join("/")));
        }

        parts.push(format!("tag {}", self.latest_tag.as_deref().unwrap_or("none")));
        if let Some(local) = &self.local {
            parts.push(format!("local {local}"));
        }
        parts.join(" · ")
    }

    fn npm_summary(&self) -> String {
        let count = self.npm.len();
        let suffix = if count > 1 { format!(" ({count} packages)") } else { String::new() };

        let labels: BTreeSet<String> = self.npm.iter().map(NpmPackageVersion::label).collect();
        if labels.len() <= 1 {
            let label = labels.into_iter().next().unwrap_or_else(|| "none".to_string());
            return format!("npm {label}{suffix}");
        }

        let details: Vec<String> =
            self.npm.iter().map(|pkg| format!("{} {}", pkg.name, pkg.label())).collect();
        format!("npm mixed: {}", details.join(", "))
    }
}

pub async fn resolve(
    unit_name: &str,
    npm_packages: Vec<String>,
    local: Option<String>,
) -> VersionSources {
    let (npm, latest_tag) = tokio::join!(fetch_all(npm_packages), latest_tag_version(unit_name));
    VersionSources { local, npm, latest_tag }
}

async fn fetch_all(packages: Vec<String>) -> Vec<NpmPackageVersion> {
    let mut set = JoinSet::new();
    for (idx, name) in packages.into_iter().enumerate() {
        set.spawn(async move {
            let status = fetch_npm_dist_tags(&name).await;
            (idx, NpmPackageVersion { name, status })
        });
    }

    let mut results: Vec<(usize, NpmPackageVersion)> = Vec::new();
    while let Some(joined) = set.join_next().await {
        if let Ok(result) = joined {
            results.push(result);
        }
    }
    results.sort_by_key(|(idx, _)| *idx);
    results.into_iter().map(|(_, pkg)| pkg).collect()
}

async fn fetch_npm_dist_tags(package: &str) -> NpmStatus {
    let lookup = Command::new("npm")
        .args(["view", package, "dist-tags", "--json", "--prefer-online"])
        .kill_on_drop(true)
        .output();

    let Ok(Ok(output)) = timeout(NPM_TIMEOUT, lookup).await else {
        return NpmStatus::Unavailable;
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return if stderr.contains("E404") || stdout.contains("E404") {
            NpmStatus::NotPublished
        } else {
            NpmStatus::Unavailable
        };
    }

    let Ok(tags) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return NpmStatus::Unavailable;
    };
    let get = |key: &str| tags.get(key).and_then(|v| v.as_str()).map(String::from);

    NpmStatus::Published { latest: get("latest"), next: get("next") }
}

async fn latest_tag_version(unit_name: &str) -> Option<String> {
    let pattern = tag_pattern_for(unit_name);
    let prefix = pattern.trim_end_matches('*');

    let output = Command::new("git").args(["tag", "-l", &pattern]).output().await.ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|tag| tag.strip_prefix(prefix))
        .filter_map(|v| Version::parse(v).ok())
        .max()
        .map(|v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn published(name: &str, latest: &str) -> NpmPackageVersion {
        NpmPackageVersion {
            name: name.to_string(),
            status: NpmStatus::Published { latest: Some(latest.to_string()), next: None },
        }
    }

    fn with_status(name: &str, status: NpmStatus) -> NpmPackageVersion {
        NpmPackageVersion { name: name.to_string(), status }
    }

    fn sources(
        npm: Vec<NpmPackageVersion>,
        tag: Option<&str>,
        local: Option<&str>,
    ) -> VersionSources {
        VersionSources { local: local.map(String::from), npm, latest_tag: tag.map(String::from) }
    }

    #[test]
    fn effective_is_the_highest_known_version() {
        let s = sources(vec![published("rari", "0.17.10")], Some("0.17.11"), Some("0.17.10"));
        assert_eq!(s.effective().as_deref(), Some("0.17.11"));
        assert!(s.is_out_of_sync());
    }

    #[test]
    fn effective_compares_semver_not_strings() {
        let s = sources(vec![published("rari", "0.17.9")], Some("0.17.10"), None);
        assert_eq!(s.effective().as_deref(), Some("0.17.10"));
    }

    #[test]
    fn in_sync_when_all_sources_agree() {
        let s = sources(vec![published("rari", "1.2.3")], Some("1.2.3"), Some("1.2.3"));
        assert!(!s.is_out_of_sync());
        assert_eq!(s.summary(), "npm 1.2.3 · tag 1.2.3 · local 1.2.3");
    }

    #[test]
    fn falls_back_when_npm_unavailable() {
        let s = sources(
            vec![with_status("@rari/lint", NpmStatus::Unavailable)],
            Some("0.1.6"),
            Some("0.1.6"),
        );
        assert!(!s.npm_reachable());
        assert_eq!(s.effective().as_deref(), Some("0.1.6"));
        assert!(!s.is_out_of_sync());
        assert!(s.summary().contains("npm unavailable"));
    }

    #[test]
    fn binary_platforms_in_sync() {
        let s = sources(
            vec![published("rari-darwin-arm64", "0.17.10"), published("rari-linux-x64", "0.17.10")],
            Some("0.17.10"),
            None,
        );
        assert!(!s.is_out_of_sync());
        assert_eq!(s.summary(), "npm 0.17.10 (2 packages) · tag 0.17.10");
    }

    #[test]
    fn binary_platforms_mixed_versions() {
        let s = sources(
            vec![published("rari-darwin-arm64", "0.17.11"), published("rari-linux-x64", "0.17.10")],
            Some("0.17.11"),
            None,
        );
        assert!(s.npm_is_mixed());
        assert!(s.is_out_of_sync());
        assert_eq!(s.effective().as_deref(), Some("0.17.11"));
        assert!(s.summary().contains("rari-linux-x64 0.17.10"));
    }

    #[test]
    fn binary_platform_missing_is_mixed() {
        let s = sources(
            vec![
                published("rari-darwin-arm64", "0.17.10"),
                with_status("rari-linux-riscv64", NpmStatus::NotPublished),
            ],
            Some("0.17.10"),
            None,
        );
        assert!(s.is_out_of_sync());
        assert!(s.summary().contains("rari-linux-riscv64 unpublished"));
    }

    #[test]
    fn mixed_versions_still_flagged_when_one_lookup_fails() {
        let s = sources(
            vec![
                published("rari-darwin-arm64", "0.17.11"),
                published("rari-linux-x64", "0.17.10"),
                with_status("rari-win32-x64", NpmStatus::Unavailable),
            ],
            Some("0.17.11"),
            None,
        );
        assert!(!s.npm_reachable());
        assert!(s.npm_is_mixed());
        assert!(s.is_out_of_sync());
    }

    #[test]
    fn prerelease_base_includes_published_next() {
        let s = VersionSources {
            local: Some("0.17.10".to_string()),
            npm: vec![with_status(
                "rari",
                NpmStatus::Published {
                    latest: Some("0.17.10".to_string()),
                    next: Some("0.17.11-0".to_string()),
                },
            )],
            latest_tag: None,
        };
        assert_eq!(s.effective().as_deref(), Some("0.17.10"));
        assert_eq!(s.effective_prerelease().as_deref(), Some("0.17.11-0"));
    }

    #[test]
    fn prerelease_base_matches_stable_without_next() {
        let s = sources(vec![published("rari", "0.17.10")], Some("0.17.10"), Some("0.17.10"));
        assert_eq!(s.effective_prerelease(), s.effective());
    }

    #[test]
    fn unavailable_alone_is_not_mixed() {
        let s = sources(
            vec![
                published("rari-darwin-arm64", "0.17.10"),
                with_status("rari-win32-x64", NpmStatus::Unavailable),
            ],
            Some("0.17.10"),
            None,
        );
        assert!(!s.npm_is_mixed());
    }
}
