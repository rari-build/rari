use smallvec::SmallVec;
use std::sync::OnceLock;

pub type DependencyList = SmallVec<[String; 4]>;

static IMPORT_REGEX: OnceLock<regex::Regex> = OnceLock::new();

fn get_import_regex() -> &'static regex::Regex {
    IMPORT_REGEX.get_or_init(|| {
        #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
        regex::Regex::new(r#"(?:import|from)\s*((?:['"])(.*?)(?:['"]))"#)
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

pub fn hash_string(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
#[allow(
    clippy::allow_attributes,
    clippy::unreadable_literal,
    clippy::needless_raw_string_hashes,
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp,
    clippy::bool_assert_comparison,
    clippy::redundant_clone,
    clippy::redundant_closure_for_method_calls,
    clippy::single_char_pattern,
    clippy::approx_constant,
    clippy::uninlined_format_args,
    clippy::module_inception,
    clippy::return_self_not_must_use,
    clippy::disallowed_methods,
    clippy::clone_on_ref_ptr,
    clippy::get_unwrap
)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dependencies() {
        let code = r#"
        import React from 'react';
        import { useState } from 'react';
        import Button from './Button';
        import { Card, CardContent } from '../components/Card';

        export default function Component() {
            return <div>Test</div>;
        }
        "#;

        let dependencies = extract_dependencies(code);
        assert_eq!(dependencies.len(), 2);
        assert!(dependencies.contains(&"./Button".to_string()));
        assert!(dependencies.contains(&"../components/Card".to_string()));
    }
}
