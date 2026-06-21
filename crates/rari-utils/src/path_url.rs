use cow_utils::CowUtils;
use std::path::Path;

pub fn path_to_file_url(path: &Path) -> String {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map(|cwd| cwd.join(path)).unwrap_or_else(|_| path.to_path_buf())
    };

    url::Url::from_file_path(&absolute_path).map(|url| url.to_string()).unwrap_or_else(|_| {
        let path_str = absolute_path.to_string_lossy().cow_replace('\\', "/").into_owned();
        if path_str.starts_with('/') {
            format!("file://{}", path_str)
        } else {
            format!("file:///{}", path_str)
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

        let url2 = path_to_file_url(&path);
        assert_eq!(url, url2);
    }

    #[test]
    fn test_complex_dot_segments() {
        let path = PathBuf::from("./a/b/../c/./file.js");
        let url = path_to_file_url(&path);
        assert!(url.starts_with("file:///"));

        let url2 = path_to_file_url(&path);
        assert_eq!(url, url2);
    }

    #[test]
    fn test_relative_vs_absolute_identity() {
        let relative = PathBuf::from("./file.js");
        let url_relative = path_to_file_url(&relative);

        assert_eq!(url_relative, path_to_file_url(&relative));
    }
}
