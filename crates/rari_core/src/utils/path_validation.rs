//! Path safety utilities for preventing directory traversal attacks.
//!
//! These functions validate that a requested path stays within the intended
//! base directory, rejecting `..` segments, null bytes, and symlink escapes.

use std::path::{Path, PathBuf};

use rari_error::RariError;
use tokio::fs;

/// Validate that `requested` is a safe sub-path of `base`.
///
/// Rejects paths containing `..`, `//`, null bytes, leading `/` (on unix),
/// and ensures the resolved canonical path is a descendant of `base`.
/// Symlinks pointing outside `base` are also rejected because `canonicalize`
/// resolves them and the prefix check catches the escape.
#[expect(clippy::missing_errors_doc)]
pub async fn validate_safe_path(base: &Path, requested: &str) -> Result<PathBuf, RariError> {
    if requested.contains("..") {
        return Err(RariError::bad_request("Invalid path: contains '..' pattern"));
    }

    if requested.contains("//") {
        return Err(RariError::bad_request("Invalid path: contains '//' pattern"));
    }

    if requested.contains('\0') {
        return Err(RariError::bad_request("Invalid path: contains null byte"));
    }

    if cfg!(unix)
        && requested.starts_with('/')
        && requested.len() > 1
        && requested.chars().nth(1) == Some('/')
    {
        return Err(RariError::bad_request("Invalid path: absolute paths not allowed"));
    }

    if cfg!(windows) && requested.len() >= 2 {
        let chars: Vec<char> = requested.chars().collect();
        if chars.len() >= 2 && chars[1] == ':' && chars[0].is_ascii_alphabetic() {
            return Err(RariError::bad_request("Invalid path: drive paths not allowed"));
        }
        if requested.starts_with("\\\\") || requested.starts_with("//") {
            return Err(RariError::bad_request("Invalid path: UNC paths not allowed"));
        }
    }

    let requested_clean = requested.trim_start_matches('/');

    let path = base.join(requested_clean);

    let canonical_path =
        fs::canonicalize(&path).await.map_err(|_| RariError::not_found("File not found"))?;

    let canonical_base = fs::canonicalize(base)
        .await
        .map_err(|_| RariError::internal("Invalid base directory configuration"))?;

    if !canonical_path.starts_with(&canonical_base) {
        return Err(RariError::bad_request("Path traversal detected"));
    }

    Ok(canonical_path)
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rari-test-path-validation-{name}"))
    }

    #[tokio::test]
    async fn test_rejects_parent_directory_traversal() {
        let base = test_temp_dir("parent-traversal");
        std::fs::create_dir_all(&base).unwrap();
        tokio::fs::write(base.join(".gitkeep"), b"").await.unwrap();

        let result = validate_safe_path(&base, "../etc/passwd").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("'..'"));
    }

    #[tokio::test]
    async fn test_rejects_multiple_parent_traversal() {
        let base = test_temp_dir("multiple-parent");
        std::fs::create_dir_all(&base).unwrap();
        tokio::fs::write(base.join(".gitkeep"), b"").await.unwrap();

        let result = validate_safe_path(&base, "../../etc/passwd").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_hidden_traversal() {
        let base = test_temp_dir("hidden-traversal");
        std::fs::create_dir_all(&base).unwrap();
        tokio::fs::write(base.join(".gitkeep"), b"").await.unwrap();

        let result = validate_safe_path(&base, "foo/../../../etc/passwd").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_double_slash() {
        let base = test_temp_dir("double-slash");
        std::fs::create_dir_all(&base).unwrap();
        tokio::fs::write(base.join(".gitkeep"), b"").await.unwrap();

        let result = validate_safe_path(&base, "foo//bar").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_null_byte() {
        let base = test_temp_dir("null-byte");
        std::fs::create_dir_all(&base).unwrap();
        tokio::fs::write(base.join(".gitkeep"), b"").await.unwrap();

        let result = validate_safe_path(&base, "foo\0bar").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_accepts_valid_path() {
        let base = test_temp_dir("valid-path");
        std::fs::create_dir_all(&base).unwrap();
        let test_file = base.join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(&base, "test.txt").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn test_accepts_nested_valid_path() {
        let base = test_temp_dir("nested-path");
        std::fs::create_dir_all(&base).unwrap();
        let nested_dir = base.join("foo").join("bar");
        std::fs::create_dir_all(&nested_dir).unwrap();
        let test_file = nested_dir.join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(&base, "foo/bar/test.txt").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let base = test_temp_dir("symlink-escape");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        let outside_dir = test_temp_dir("symlink-outside");
        let _ = std::fs::remove_dir_all(&outside_dir);
        std::fs::create_dir_all(&outside_dir).unwrap();
        let outside_file = outside_dir.join("secret.txt");
        std::fs::write(&outside_file, "secret").unwrap();

        let link_path = base.join("escape");
        let _ = std::fs::remove_file(&link_path);
        symlink(&outside_dir, &link_path).expect("Failed to create symlink for security test");

        let result = validate_safe_path(&base, "escape/secret.txt").await;
        assert!(result.is_err(), "Security failure: symlink escape was not rejected");
    }

    #[tokio::test]
    async fn test_handles_leading_slash() {
        let base = test_temp_dir("leading-slash");
        std::fs::create_dir_all(&base).unwrap();
        let test_file = base.join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(&base, "/test.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rejects_nonexistent_path() {
        let base = test_temp_dir("nonexistent");
        std::fs::create_dir_all(&base).unwrap();

        let result = validate_safe_path(&base, "nonexistent.txt").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
