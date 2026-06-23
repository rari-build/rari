use cow_utils::CowUtils;
use std::path::{Path, PathBuf};

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        use std::path::Component;
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(last) = components.last()
                    && matches!(last, Component::Normal(_))
                {
                    components.pop();
                } else if components
                    .first()
                    .is_some_and(|c| matches!(c, Component::RootDir | Component::Prefix(_)))
                {
                } else {
                    components.push(component);
                }
            }
            _ => {
                components.push(component);
            }
        }
    }

    components.iter().collect()
}

pub fn path_to_file_url(path: &Path) -> String {
    let absolute_path = if path.is_absolute() {
        normalize_path(path)
    } else {
        let joined = std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf());

        normalize_path(&joined)
    };

    url::Url::from_file_path(&absolute_path)
        .map(|url| url.to_string())
        .unwrap_or_else(|()| {
            let path_str = absolute_path
                .to_string_lossy()
                .cow_replace('\\', "/")
                .into_owned();
            if path_str.starts_with('/') {
                format!("file://{path_str}")
            } else {
                format!("file:///{path_str}")
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_unix_path() {
        let path = PathBuf::from("/home/user/file.js");
        let url = path_to_file_url(&path);
        assert!(url.starts_with("file:///"));
        assert!(url.contains("file.js"));
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_path() {
        let path = PathBuf::from(r"C:\Users\file.js");
        let url = path_to_file_url(&path);
        assert!(url.starts_with("file:///"));
        assert!(url.contains("C:/"));
        assert!(!url.contains("\\"));
    }

    #[test]
    fn test_relative_path_fallback() {
        let path = PathBuf::from("relative/file.js");
        let url = path_to_file_url(&path);
        assert!(url.starts_with("file:///"));
        assert!(url.contains("file.js"));
    }

    #[test]
    fn test_dot_segment_relative_path() {
        let path = PathBuf::from("./file.js");
        let url = path_to_file_url(&path);
        assert!(url.starts_with("file:///"));
        assert!(url.contains("file.js"));

        let url2 = path_to_file_url(&path);
        assert_eq!(url, url2);
    }

    #[test]
    fn test_parent_directory_segment() {
        let path = PathBuf::from("a/../file.js");
        let url = path_to_file_url(&path);
        assert!(url.starts_with("file:///"));
        assert!(url.contains("file.js"));

        let url2 = path_to_file_url(&path);
        assert_eq!(url, url2);
    }

    #[test]
    fn test_complex_dot_segments() {
        let path = PathBuf::from("./a/b/../c/./file.js");
        let url = path_to_file_url(&path);
        assert!(url.starts_with("file:///"));
        assert!(url.contains("file.js"));

        let url2 = path_to_file_url(&path);
        assert_eq!(url, url2);
    }

    #[test]
    fn test_dot_segments_are_normalized() {
        let relative = PathBuf::from("a/../file.js");
        let url = path_to_file_url(&relative);

        assert!(
            !url.contains(".."),
            "URL contains unnormalized '..' segments: {url}. This will break Deno module cache!"
        );
    }
}
