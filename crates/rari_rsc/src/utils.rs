use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::OnceLock,
};

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

pub fn hash_string(s: &str) -> String {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dependencies() {
        let code = r"
        import React from 'react';
        import { useState } from 'react';
        import Button from './Button';
        import { Card, CardContent } from '../components/Card';

        export default function Component() {
            return <div>Test</div>;
        }
        ";

        let dependencies = extract_dependencies(code);
        assert_eq!(dependencies.len(), 2);
        assert!(dependencies.contains(&"./Button".to_string()));
        assert!(dependencies.contains(&"../components/Card".to_string()));
    }
}
