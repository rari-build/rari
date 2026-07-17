//! Generic component dependency analysis shared across backends.
//!
//! Extracts local import specifiers from module source so the runtime can
//! build a dependency graph for topological loading. The `react` skip is
//! correct for any React-based backend: `react`/subpaths are the shared React
//! runtime, not user components.

use std::sync::OnceLock;

use regex::Regex;
use smallvec::SmallVec;

pub type DependencyList = SmallVec<[String; 4]>;

static IMPORT_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_import_regex() -> &'static Regex {
    IMPORT_REGEX.get_or_init(|| {
        #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
        Regex::new(r#"(?:import|from)\s*((?:['"])(.*?)(?:['"]))"#)
            .expect("Failed to compile dependency extraction regex")
    })
}

pub fn extract_dependencies(code: &str) -> DependencyList {
    let import_regex = get_import_regex();
    let mut dependencies = SmallVec::new();

    for captures in import_regex.captures_iter(code) {
        if captures.len() >= 3
            && let Some(import_path) = captures.get(2)
        {
            let import_path_str = import_path.as_str().to_string();
            if !import_path_str.starts_with("react")
                && (import_path_str.starts_with('.')
                    || import_path_str.starts_with('/')
                    || import_path_str.contains('/'))
            {
                dependencies.push(import_path_str);
            }
        }
    }

    dependencies
}
