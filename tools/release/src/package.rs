use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    pub needs_build: bool,
}

#[derive(Debug, Clone)]
pub struct PackageGroup {
    pub name: String,
    pub packages: Vec<Package>,
    pub current_version: String,
}

impl PackageGroup {
    pub async fn new(name: String, packages: Vec<Package>) -> Result<Self> {
        if packages.is_empty() {
            anyhow::bail!("PackageGroup must contain at least one package");
        }

        let current_version = packages[0].current_version.clone();

        for pkg in &packages {
            if pkg.current_version != current_version {
                anyhow::bail!(
                    "All packages in group '{}' must have the same version. Found {} with version {} but expected {}",
                    name,
                    pkg.name,
                    pkg.current_version,
                    current_version
                );
            }
        }

        Ok(Self { name, packages, current_version })
    }

    pub async fn update_all_versions(&self, new_version: &str) -> Result<()> {
        for package in &self.packages {
            package.update_version(new_version).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ReleaseUnit {
    Single(Package),
    Group(PackageGroup),
}

impl ReleaseUnit {
    pub fn name(&self) -> &str {
        match self {
            Self::Single(pkg) => &pkg.name,
            Self::Group(group) => &group.name,
        }
    }

    pub fn current_version(&self) -> &str {
        match self {
            Self::Single(pkg) => &pkg.current_version,
            Self::Group(group) => &group.current_version,
        }
    }

    pub fn packages(&self) -> Vec<&Package> {
        match self {
            Self::Single(pkg) => vec![pkg],
            Self::Group(group) => group.packages.iter().collect(),
        }
    }

    pub fn needs_build(&self) -> bool {
        match self {
            Self::Single(pkg) => pkg.needs_build,
            Self::Group(group) => group.packages.iter().any(|p| p.needs_build),
        }
    }

    pub async fn update_version(&self, new_version: &str) -> Result<()> {
        match self {
            Self::Single(pkg) => pkg.update_version(new_version).await,
            Self::Group(group) => group.update_all_versions(new_version).await,
        }
    }

    pub fn paths(&self) -> Vec<&PathBuf> {
        match self {
            Self::Single(pkg) => vec![&pkg.path],
            Self::Group(group) => group.packages.iter().map(|p| &p.path).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleasedPackage {
    pub name: String,
    pub version: String,
    pub tag: String,
    pub commits: Vec<String>,
    pub previous_tag: Option<String>,
}

impl Package {
    pub async fn load(name: &str, path: &str, needs_build: bool) -> Result<Self> {
        let pkg_path = PathBuf::from(path);
        let pkg_json_path = pkg_path.join("package.json");
        let content = tokio::fs::read_to_string(&pkg_json_path).await?;
        let pkg_json: PackageJson = serde_json::from_str(&content)?;

        Ok(Self {
            name: name.to_string(),
            path: pkg_path,
            current_version: pkg_json.version,
            needs_build,
        })
    }

    pub async fn update_version(&self, new_version: &str) -> Result<()> {
        let pkg_json_path = self.path.join("package.json");
        let content = tokio::fs::read_to_string(&pkg_json_path).await?;

        let pkg_json: PackageJson = serde_json::from_str(&content)?;
        let old_version = &pkg_json.version;

        let version_pattern = format!(r#""version": "{}""#, regex::escape(old_version));
        let version_replacement = format!(r#""version": "{}""#, new_version);

        let re = regex::Regex::new(&version_pattern)?;
        let updated = re.replace(&content, version_replacement.as_str());

        if updated == content {
            anyhow::bail!("Failed to update version in package.json - pattern not found");
        }

        tokio::fs::write(&pkg_json_path, updated.as_ref()).await?;

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
                let mut new = v.clone();
                new.patch += 1;
                new.pre = semver::Prerelease::new("0").ok()?;
                new.to_string()
            }
            Self::Preminor => {
                let mut new = v.clone();
                new.minor += 1;
                new.patch = 0;
                new.pre = semver::Prerelease::new("0").ok()?;
                new.to_string()
            }
            Self::Premajor => {
                let mut new = v.clone();
                new.major += 1;
                new.minor = 0;
                new.patch = 0;
                new.pre = semver::Prerelease::new("0").ok()?;
                new.to_string()
            }
            Self::Prerelease => {
                let mut new = v.clone();
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

    pub fn label(&self, current: &str) -> String {
        match (*self).to_version(current) {
            Some(v) if *self != Self::Custom => format!("{:?} ({})", self, v),
            _ => format!("{:?}", self),
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
