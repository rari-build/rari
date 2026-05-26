use std::path::{Path, PathBuf};

#[cfg(test)]
use crate::utils::path_url::path_to_file_url;
#[cfg(test)]
use cow_utils::CowUtils;

pub struct DistPathResolver {
    project_root: PathBuf,
    dist_dir: PathBuf,
    #[cfg(test)]
    server_dir: String,
}

impl DistPathResolver {
    pub fn new(project_root: PathBuf) -> Self {
        let dist_dir = project_root.join("dist");
        Self {
            project_root,
            dist_dir,
            #[cfg(test)]
            server_dir: "server".to_string(),
        }
    }

    #[cfg(test)]
    fn get_dist_path(&self, component_id: &str) -> PathBuf {
        let filename = format!("{}.js", component_id);
        self.dist_dir.join(&self.server_dir).join(filename)
    }

    #[cfg(test)]
    pub fn get_dist_url(&self, component_id: &str) -> String {
        let path = self.get_dist_path(component_id);
        path_to_file_url(&path)
    }

    #[cfg(test)]
    pub fn file_path_to_component_id(&self, file_path: &Path) -> String {
        let relative_path = if file_path.is_absolute() {
            file_path.strip_prefix(&self.project_root).unwrap_or(file_path).to_path_buf()
        } else {
            file_path.to_path_buf()
        };

        let path_str = relative_path.to_string_lossy();

        path_str
            .cow_replace('\\', "/")
            .trim_end_matches(".tsx")
            .trim_end_matches(".ts")
            .trim_end_matches(".jsx")
            .trim_end_matches(".js")
            .cow_replace(|c: char| !c.is_alphanumeric() && c != '/' && c != '-', "_")
            .trim_start_matches("src/")
            .to_string()
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn dist_dir(&self) -> &Path {
        &self.dist_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_dist_path() {
        let resolver = DistPathResolver::new(PathBuf::from("/project"));
        let path = resolver.get_dist_path("pages/home");

        assert_eq!(path, PathBuf::from("/project/dist/server/pages/home.js"));
    }

    #[test]
    fn test_get_dist_url() {
        let resolver = DistPathResolver::new(PathBuf::from("/project"));
        let url = resolver.get_dist_url("components/Button");

        assert!(url.starts_with("file://"));
        assert!(url.contains("dist/server/components/Button.js"));
    }

    #[test]
    fn test_file_path_to_component_id() {
        let resolver = DistPathResolver::new(PathBuf::from("/project"));

        let id = resolver.file_path_to_component_id(Path::new("src/pages/home.tsx"));
        assert_eq!(id, "pages/home");

        let id = resolver.file_path_to_component_id(Path::new("components/Button.tsx"));
        assert_eq!(id, "components/Button");

        let id = resolver.file_path_to_component_id(Path::new("src/pages/[slug].tsx"));
        assert_eq!(id, "pages/_slug_");
    }

    #[test]
    fn test_file_path_to_component_id_absolute() {
        let resolver = DistPathResolver::new(PathBuf::from("/project"));

        let id = resolver.file_path_to_component_id(Path::new("/project/src/pages/about.tsx"));
        assert_eq!(id, "pages/about");
    }
}
