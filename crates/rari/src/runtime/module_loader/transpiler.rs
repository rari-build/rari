pub const TYPESCRIPT_EXTENSION: &str = ".ts";
pub const TYPESCRIPT_JSX_EXTENSION: &str = ".tsx";
pub const JSX_EXTENSION: &str = ".jsx";
pub const JAVASCRIPT_EXTENSION: &str = ".js";

pub fn needs_typescript_transpilation(specifier: &str) -> bool {
    specifier.ends_with(TYPESCRIPT_EXTENSION) || specifier.ends_with(TYPESCRIPT_JSX_EXTENSION)
}

pub fn needs_jsx_transpilation(specifier: &str) -> bool {
    specifier.ends_with(JSX_EXTENSION)
}

pub fn needs_transpilation(specifier: &str) -> bool {
    needs_typescript_transpilation(specifier) || needs_jsx_transpilation(specifier)
}

pub fn get_module_type(_specifier: &str) -> &'static str {
    "module"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typescript_detection() {
        assert!(needs_typescript_transpilation("file.ts"));
        assert!(needs_typescript_transpilation("file.tsx"));
        assert!(!needs_typescript_transpilation("file.js"));
        assert!(!needs_typescript_transpilation("file.jsx"));
    }

    #[test]
    fn test_jsx_detection() {
        assert!(needs_jsx_transpilation("file.jsx"));
        assert!(!needs_jsx_transpilation("file.js"));
        assert!(!needs_jsx_transpilation("file.ts"));
    }

    #[test]
    fn test_transpilation_needed() {
        assert!(needs_transpilation("file.ts"));
        assert!(needs_transpilation("file.tsx"));
        assert!(needs_transpilation("file.jsx"));
        assert!(!needs_transpilation("file.js"));
    }
}
