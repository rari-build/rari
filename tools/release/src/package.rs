use std::{fmt::Write, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::versions::{self, VersionSources};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    #[serde(flatten)]
    pub other: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub path: PathBuf,
    pub current_version: String,
    pub sources: VersionSources,
}

#[derive(Debug, Clone)]
pub struct PackageGroup {
    pub name: String,
    pub current_version: String,
    pub sources: VersionSources,
}

impl PackageGroup {
    pub async fn load_virtual(
        name: &str,
        parent_path: &str,
        platform_prefix: &str,
    ) -> Result<Self> {
        let parent_json = PathBuf::from(parent_path).join("package.json");
        let pkg_json: PackageJson = serde_json::from_str(&fs::read_to_string(&parent_json).await?)?;
        let platform_packages: Vec<String> = pkg_json
            .other
            .get("optionalDependencies")
            .and_then(|deps| deps.as_object())
            .map(|deps| {
                deps.keys().filter(|dep| dep.starts_with(platform_prefix)).cloned().collect()
            })
            .unwrap_or_default();
        if platform_packages.is_empty() {
            anyhow::bail!(
                "No `{platform_prefix}*` platform packages in optionalDependencies of {}",
                parent_json.display()
            );
        }

        let sources = versions::resolve(name, platform_packages, None).await;
        let Some(current_version) = sources.effective() else {
            anyhow::bail!(
                "Could not determine a version for {name}: not found on npm or in git tags"
            );
        };
        Ok(Self { name: name.to_string(), current_version, sources })
    }
}

pub async fn load_release_units() -> Result<Vec<ReleaseUnit>> {
    let (rari, create_app, binaries, use_cache, use_cache_binaries, lint) = tokio::join!(
        Package::load("rari", "packages/rari"),
        Package::load("create-rari-app", "packages/create-rari-app"),
        PackageGroup::load_virtual("rari-binaries", "packages/rari", "rari-"),
        Package::load("@rari/use-cache", "packages/use-cache"),
        PackageGroup::load_virtual(
            "@rari/use-cache-binaries",
            "packages/use-cache",
            "@rari/use-cache-",
        ),
        Package::load("@rari/lint", "packages/lint"),
    );

    Ok(vec![
        ReleaseUnit::Single(rari?),
        ReleaseUnit::Single(create_app?),
        ReleaseUnit::Virtual(binaries?),
        ReleaseUnit::Single(use_cache?),
        ReleaseUnit::Virtual(use_cache_binaries?),
        ReleaseUnit::Single(lint?),
    ])
}

#[derive(Debug, Clone)]
pub enum ReleaseUnit {
    Single(Package),
    Virtual(PackageGroup),
}

impl ReleaseUnit {
    pub fn name(&self) -> &str {
        match self {
            Self::Single(pkg) => &pkg.name,
            Self::Virtual(group) => &group.name,
        }
    }

    pub fn current_version(&self) -> &str {
        match self {
            Self::Single(pkg) => &pkg.current_version,
            Self::Virtual(group) => &group.current_version,
        }
    }

    pub fn sources(&self) -> &VersionSources {
        match self {
            Self::Single(pkg) => &pkg.sources,
            Self::Virtual(group) => &group.sources,
        }
    }

    pub fn packages(&self) -> Vec<&Package> {
        match self {
            Self::Single(pkg) => vec![pkg],
            Self::Virtual(_) => vec![],
        }
    }

    pub async fn update_version(&self, new_version: &str) -> Result<()> {
        match self {
            Self::Single(pkg) => pkg.update_version(new_version).await,
            Self::Virtual(_) => Ok(()),
        }
    }

    pub fn paths(&self) -> Vec<&PathBuf> {
        match self {
            Self::Single(pkg) => vec![&pkg.path],
            Self::Virtual(_) => vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleasedPackage {
    pub name: String,
    pub version: String,
    pub tag: String,
    pub release_notes: String,
    pub previous_tag: Option<String>,
}

pub fn release_tag(unit_name: &str, version: &str) -> String {
    if unit_name == "rari-binaries" {
        format!("v{version}")
    } else if unit_name == "@rari/use-cache-binaries" {
        format!("use-cache-binaries@{version}")
    } else {
        format!("{unit_name}@{version}")
    }
}

impl Package {
    pub async fn load(name: &str, path: &str) -> Result<Self> {
        let pkg_path = PathBuf::from(path);
        let pkg_json_path = pkg_path.join("package.json");

        let content = fs::read_to_string(&pkg_json_path).await?;
        let pkg_json: PackageJson = serde_json::from_str(&content)?;

        let sources =
            versions::resolve(name, vec![name.to_string()], Some(pkg_json.version.clone())).await;
        let current_version = sources.effective().unwrap_or(pkg_json.version);

        Ok(Self { name: name.to_string(), path: pkg_path, current_version, sources })
    }

    pub async fn update_version(&self, new_version: &str) -> Result<()> {
        let pkg_json_path = self.path.join("package.json");
        let content = fs::read_to_string(&pkg_json_path).await?;

        let pkg_json: PackageJson = serde_json::from_str(&content)?;
        let old_version = &pkg_json.version;

        let version_pattern = format!(r#""version": "{}""#, regex::escape(old_version));
        let version_replacement = format!(r#""version": "{new_version}""#);

        let re = regex::Regex::new(&version_pattern)?;
        let updated = re.replace(&content, version_replacement.as_str());

        if updated == content {
            anyhow::bail!("Failed to update version in package.json - pattern not found");
        }

        fs::write(&pkg_json_path, updated.as_ref()).await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReleaseType {
    Patch,
    Minor,
    Major,
    Prepatch,
    Preminor,
    Premajor,
    Prerelease,
    Custom,
}

impl ReleaseType {
    pub fn to_version(self, current: &str) -> Option<String> {
        let v = semver::Version::parse(current).ok()?;
        Some(match self {
            Self::Patch => semver::Version::new(v.major, v.minor, v.patch + 1).to_string(),
            Self::Minor => semver::Version::new(v.major, v.minor + 1, 0).to_string(),
            Self::Major => semver::Version::new(v.major + 1, 0, 0).to_string(),
            Self::Prepatch => {
                let mut new = v;
                new.patch += 1;
                new.pre = semver::Prerelease::new("0").ok()?;
                new.to_string()
            }
            Self::Preminor => {
                let mut new = v;
                new.minor += 1;
                new.patch = 0;
                new.pre = semver::Prerelease::new("0").ok()?;
                new.to_string()
            }
            Self::Premajor => {
                let mut new = v;
                new.major += 1;
                new.minor = 0;
                new.patch = 0;
                new.pre = semver::Prerelease::new("0").ok()?;
                new.to_string()
            }
            Self::Prerelease => {
                let mut new = v;
                if new.pre.is_empty() {
                    new.patch += 1;
                    new.pre = semver::Prerelease::new("0").ok()?;
                } else {
                    let pre_str = new.pre.as_str();
                    if let Ok(num) = pre_str.parse::<u64>() {
                        new.pre = semver::Prerelease::new(&(num + 1).to_string()).ok()?;
                    }
                }
                new.to_string()
            }
            Self::Custom => current.to_string(),
        })
    }

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "Consistent API with other methods in this impl block"
    )]
    pub fn label(&self, current: &str) -> String {
        match (*self).to_version(current) {
            Some(v) if *self != Self::Custom => format!("{self:?} ({v})"),
            _ => format!("{self:?}"),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Patch,
            Self::Minor,
            Self::Major,
            Self::Prepatch,
            Self::Preminor,
            Self::Premajor,
            Self::Prerelease,
            Self::Custom,
        ]
    }
}

impl ReleasedPackage {
    pub fn create_github_release_url(&self, owner: &str, repo: &str) -> String {
        let (title_text, tag_text) = if self.name == "rari-binaries" {
            (format!("v{}", self.version), format!("v{}", self.version))
        } else if self.name == "@rari/use-cache-binaries" {
            (
                format!("use-cache-binaries@{}", self.version),
                format!("use-cache-binaries@{}", self.version),
            )
        } else {
            (format!("{}@{}", self.name, self.version), self.tag.clone())
        };

        let title = urlencoding::encode(&title_text);
        let tag = urlencoding::encode(&tag_text);

        let mut body = self.release_notes.clone();

        if !body.contains("**Full Changelog**")
            && let Some(prev_tag) = &self.previous_tag
        {
            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
            write!(
                &mut body,
                "\n\n**Full Changelog**: https://github.com/{owner}/{repo}/compare/{prev_tag}...{tag_text}"
            ).unwrap();
        }

        let body_encoded = urlencoding::encode(&body);

        format!(
            "https://github.com/{owner}/{repo}/releases/new?tag={tag}&title={title}&body={body_encoded}"
        )
    }
}
