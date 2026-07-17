use std::path::{Path, PathBuf};

use cow_utils::CowUtils;
/// Re-export the core path-safety validator.
pub use rari_core::utils::path_validation::validate_safe_path;
use rari_error::RariError;

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

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

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
}
