use crate::error::RariError;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

pub fn validate_safe_path(base: &Path, requested: &str) -> Result<PathBuf, RariError> {
    if requested.contains("..") {
        warn!(
            requested = %requested,
            "Rejected path containing '..' traversal pattern"
        );
        return Err(RariError::bad_request("Invalid path: contains '..' pattern"));
    }

    if requested.contains("//") {
        warn!(
            requested = %requested,
            "Rejected path containing '//' pattern"
        );
        return Err(RariError::bad_request("Invalid path: contains '//' pattern"));
    }

    if requested.contains('\0') {
        warn!(
            requested = %requested,
            "Rejected path containing null byte"
        );
        return Err(RariError::bad_request("Invalid path: contains null byte"));
    }

    if cfg!(unix)
        && requested.starts_with('/')
        && requested.len() > 1
        && requested.chars().nth(1) == Some('/')
    {
        warn!(
            requested = %requested,
            "Rejected absolute path"
        );
        return Err(RariError::bad_request("Invalid path: absolute paths not allowed"));
    }

    if cfg!(windows) && requested.len() >= 2 {
        let chars: Vec<char> = requested.chars().collect();
        if chars.len() >= 2 && chars[1] == ':' && chars[0].is_ascii_alphabetic() {
            warn!(
                requested = %requested,
                "Rejected Windows drive path"
            );
            return Err(RariError::bad_request("Invalid path: drive paths not allowed"));
        }
        if requested.starts_with("\\\\") || requested.starts_with("//") {
            warn!(
                requested = %requested,
                "Rejected UNC path"
            );
            return Err(RariError::bad_request("Invalid path: UNC paths not allowed"));
        }
    }

    let requested_clean = requested.trim_start_matches('/');

    let path = base.join(requested_clean);

    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            debug!(
                requested = %requested,
                base = %base.display(),
                error = %e,
                "Path canonicalization failed (file may not exist)"
            );
            return Err(RariError::not_found("File not found"));
        }
    };

    let canonical_base = match base.canonicalize() {
        Ok(b) => b,
        Err(e) => {
            warn!(
                base = %base.display(),
                error = %e,
                "Base directory canonicalization failed"
            );
            return Err(RariError::internal("Invalid base directory configuration"));
        }
    };

    if !canonical_path.starts_with(&canonical_base) {
        warn!(
            requested = %requested,
            canonical_path = %canonical_path.display(),
            canonical_base = %canonical_base.display(),
            "Path traversal attempt detected: resolved path outside base directory"
        );
        return Err(RariError::bad_request("Path traversal detected"));
    }

    debug!(
        requested = %requested,
        resolved = %canonical_path.display(),
        "Path validation successful"
    );

    Ok(canonical_path)
}

pub fn validate_component_path(file_path: &str) -> Result<(), RariError> {
    if !file_path.starts_with("app/") && !file_path.starts_with("src/") {
        warn!(
            file_path = %file_path,
            "Rejected component path: must start with app/ or src/"
        );
        return Err(RariError::bad_request(
            "Invalid component path: must be within app/ or src/ directory",
        ));
    }

    if file_path.contains("..") {
        warn!(
            file_path = %file_path,
            "Rejected component path containing '..'"
        );
        return Err(RariError::bad_request("Path traversal detected in component path"));
    }

    if file_path.contains("//") {
        warn!(
            file_path = %file_path,
            "Rejected component path containing '//'"
        );
        return Err(RariError::bad_request("Invalid component path: contains '//'"));
    }

    let allowed_extensions = [".ts", ".tsx", ".js", ".jsx"];
    if !allowed_extensions.iter().any(|ext| file_path.ends_with(ext)) {
        warn!(
            file_path = %file_path,
            "Rejected component path with invalid extension"
        );
        return Err(RariError::bad_request(
            "Invalid file extension: must be .ts, .tsx, .js, or .jsx",
        ));
    }

    if file_path.contains('\0') {
        warn!(
            file_path = %file_path,
            "Rejected component path containing null byte"
        );
        return Err(RariError::bad_request("Invalid path: contains null byte"));
    }

    debug!(
        file_path = %file_path,
        "Component path validation successful"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_rejects_parent_directory_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let result = validate_safe_path(base, "../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("'..'"));
    }

    #[test]
    fn test_rejects_multiple_parent_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let result = validate_safe_path(base, "../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_hidden_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let result = validate_safe_path(base, "foo/../../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_double_slash() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let result = validate_safe_path(base, "foo//bar");
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_null_byte() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let result = validate_safe_path(base, "foo\0bar");
        assert!(result.is_err());
    }

    #[test]
    fn test_accepts_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let test_file = base.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(base, "test.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file.canonicalize().unwrap());
    }

    #[test]
    fn test_accepts_nested_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let nested_dir = base.join("foo").join("bar");
        fs::create_dir_all(&nested_dir).unwrap();
        let test_file = nested_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(base, "foo/bar/test.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file.canonicalize().unwrap());
    }

    #[test]
    fn test_rejects_symlink_escape() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let outside_dir = TempDir::new().unwrap();
        let outside_file = outside_dir.path().join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_path = base.join("escape");
            if symlink(outside_dir.path(), &link_path).is_ok() {
                let result = validate_safe_path(base, "escape/secret.txt");
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn test_component_path_validation() {
        assert!(validate_component_path("app/page.tsx").is_ok());
        assert!(validate_component_path("src/components/Button.tsx").is_ok());
        assert!(validate_component_path("app/api/route.ts").is_ok());

        assert!(validate_component_path("lib/utils.ts").is_err());
        assert!(validate_component_path("public/image.png").is_err());

        assert!(validate_component_path("app/../etc/passwd").is_err());
        assert!(validate_component_path("src//components/Button.tsx").is_err());

        assert!(validate_component_path("app/page.html").is_err());
        assert!(validate_component_path("src/data.json").is_err());

        assert!(validate_component_path("app/page\0.tsx").is_err());
    }

    #[test]
    fn test_handles_leading_slash() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let test_file = base.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(base, "/test.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rejects_nonexistent_path() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        let result = validate_safe_path(base, "nonexistent.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
