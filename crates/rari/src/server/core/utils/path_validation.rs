use std::path::{Path, PathBuf};

use cow_utils::CowUtils;
use rari_error::RariError;
use tokio::fs;
use url::Url;

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

pub fn normalize_component_path(file_path: &str) -> String {
    let path = Path::new(file_path);

    if path.is_absolute() {
        let components: Vec<_> = path.components().collect();

        if let Some(src_idx) = components.iter().position(|c| c.as_os_str() == "src") {
            let after_src: PathBuf = components[src_idx..].iter().collect();
            return after_src.to_string_lossy().cow_replace('\\', "/").into_owned();
        } else if let Some(app_idx) = components.iter().position(|c| c.as_os_str() == "app") {
            let after_app: PathBuf = components[app_idx..].iter().collect();
            return after_app.to_string_lossy().cow_replace('\\', "/").into_owned();
        }
    }

    file_path.cow_replace('\\', "/").into_owned()
}

#[expect(clippy::missing_errors_doc)]
pub fn validate_component_path(file_path: &str) -> Result<(), RariError> {
    let normalized = normalize_component_path(file_path);

    if !normalized.starts_with("app/") && !normalized.starts_with("src/") {
        return Err(RariError::bad_request(
            "Invalid component path: must be within app/ or src/ directory",
        ));
    }

    let file_path = &normalized;

    if file_path.contains("..") {
        return Err(RariError::bad_request("Path traversal detected in component path"));
    }

    if file_path.contains("//") {
        return Err(RariError::bad_request("Invalid component path: contains '//'"));
    }

    let allowed_extensions = [".ts", ".tsx", ".js", ".jsx"];
    if !allowed_extensions.iter().any(|ext| file_path.ends_with(ext)) {
        return Err(RariError::bad_request(
            "Invalid file extension: must be .ts, .tsx, .js, or .jsx",
        ));
    }

    if file_path.contains('\0') {
        return Err(RariError::bad_request("Invalid path: contains null byte"));
    }

    Ok(())
}

#[expect(clippy::missing_errors_doc)]
pub fn build_vite_dev_module_url(
    vite_address: &str,
    module_path: &str,
    timestamp_ms: u128,
) -> Result<Url, RariError> {
    if module_path.contains("://") {
        return Err(RariError::bad_request("Invalid file path: URL schemes not allowed"));
    }
    if module_path.contains("//") {
        return Err(RariError::bad_request("Invalid file path: contains '//'"));
    }
    if module_path.contains("..") {
        return Err(RariError::bad_request("Path traversal detected in file path"));
    }
    if module_path.contains('\0') {
        return Err(RariError::bad_request("Invalid file path: contains null byte"));
    }
    if module_path.contains('\\') {
        return Err(RariError::bad_request("Invalid file path: backslashes not allowed"));
    }

    let base = Url::parse(&format!("http://{vite_address}/"))
        .map_err(|_| RariError::internal("Invalid Vite server address"))?;
    let relative = module_path.trim_start_matches('/');
    if relative.is_empty() {
        return Err(RariError::bad_request("Invalid file path: empty path"));
    }

    let mut url = base
        .join(relative)
        .map_err(|_| RariError::bad_request("Invalid file path for Vite module URL"))?;

    if url.origin() != base.origin() {
        return Err(RariError::bad_request("Invalid file path: host override not allowed"));
    }

    url.set_query(Some(&format!("t={timestamp_ms}")));

    if url.origin() != base.origin() {
        return Err(RariError::bad_request("Invalid file path: host override not allowed"));
    }

    Ok(url)
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::{env, fs};

    use super::*;

    fn test_temp_dir(name: &str) -> PathBuf {
        env::temp_dir().join(format!("rari-test-path-validation-{name}"))
    }

    #[tokio::test]
    async fn test_rejects_parent_directory_traversal() {
        let base = test_temp_dir("parent-traversal");
        fs::create_dir_all(&base).unwrap();

        let result = validate_safe_path(&base, "../etc/passwd").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("'..'"));
    }

    #[tokio::test]
    async fn test_rejects_multiple_parent_traversal() {
        let base = test_temp_dir("multiple-parent");
        fs::create_dir_all(&base).unwrap();

        let result = validate_safe_path(&base, "../../etc/passwd").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_hidden_traversal() {
        let base = test_temp_dir("hidden-traversal");
        fs::create_dir_all(&base).unwrap();

        let result = validate_safe_path(&base, "foo/../../../etc/passwd").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_double_slash() {
        let base = test_temp_dir("double-slash");
        fs::create_dir_all(&base).unwrap();

        let result = validate_safe_path(&base, "foo//bar").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rejects_null_byte() {
        let base = test_temp_dir("null-byte");
        fs::create_dir_all(&base).unwrap();

        let result = validate_safe_path(&base, "foo\0bar").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_accepts_valid_path() {
        let base = test_temp_dir("valid-path");
        fs::create_dir_all(&base).unwrap();

        let test_file = base.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(&base, "test.txt").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn test_accepts_nested_valid_path() {
        let base = test_temp_dir("nested-path");
        fs::create_dir_all(&base).unwrap();

        let nested_dir = base.join("foo").join("bar");
        fs::create_dir_all(&nested_dir).unwrap();
        let test_file = nested_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(&base, "foo/bar/test.txt").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_rejects_symlink_escape() {
        let base = test_temp_dir("symlink-escape");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        let outside_dir = test_temp_dir("symlink-outside");
        let _ = fs::remove_dir_all(&outside_dir);
        fs::create_dir_all(&outside_dir).unwrap();
        let outside_file = outside_dir.join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();

        let link_path = base.join("escape");
        let _ = fs::remove_file(&link_path);
        symlink(&outside_dir, &link_path).expect("Failed to create symlink for security test");

        let result = validate_safe_path(&base, "escape/secret.txt").await;
        assert!(result.is_err(), "Security failure: symlink escape was not rejected");
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
    fn test_vite_dev_module_url_pins_origin() {
        let url = build_vite_dev_module_url("127.0.0.1:5173", "src/app/page.tsx", 123).unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:5173/src/app/page.tsx?t=123");

        let with_slash =
            build_vite_dev_module_url("127.0.0.1:5173", "/src/components/Button.tsx", 1).unwrap();
        assert_eq!(with_slash.host_str(), Some("127.0.0.1"));
        assert_eq!(with_slash.port(), Some(5173));
    }

    #[test]
    fn test_vite_dev_module_url_rejects_ssrf_vectors() {
        assert!(build_vite_dev_module_url("127.0.0.1:5173", "http://evil.test/x", 1).is_err());
        assert!(build_vite_dev_module_url("127.0.0.1:5173", "//evil.test/x", 1).is_err());
        assert!(build_vite_dev_module_url("127.0.0.1:5173", "/src/../etc/passwd", 1).is_err());
        assert!(build_vite_dev_module_url("127.0.0.1:5173", "src//app/page.tsx", 1).is_err());
        assert!(build_vite_dev_module_url("127.0.0.1:5173", "src/app\0/page.tsx", 1).is_err());
    }

    #[tokio::test]
    async fn test_handles_leading_slash() {
        let base = test_temp_dir("leading-slash");
        fs::create_dir_all(&base).unwrap();

        let test_file = base.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let result = validate_safe_path(&base, "/test.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rejects_nonexistent_path() {
        let base = test_temp_dir("nonexistent");
        fs::create_dir_all(&base).unwrap();

        let result = validate_safe_path(&base, "nonexistent.txt").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
