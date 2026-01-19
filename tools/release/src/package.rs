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

        let mut pkg_json: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(obj) = pkg_json.as_object_mut() {
            obj.insert("version".to_string(), serde_json::Value::String(new_version.to_string()));
        }

        let updated = serde_json::to_string_pretty(&pkg_json)?;
        tokio::fs::write(&pkg_json_path, format!("{}\n", updated)).await?;

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
