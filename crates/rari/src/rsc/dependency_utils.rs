use crate::error::RariError;
use smallvec::SmallVec;

type DependencyList = SmallVec<[String; 4]>;

fn compile_regex(pattern: &str, context: &str) -> Result<regex::Regex, RariError> {
    regex::Regex::new(pattern)
        .map_err(|_| RariError::js_execution(format!("Failed to compile {context} regex")))
}

pub fn extract_dependencies(code: &str) -> DependencyList {
    let import_regex = match compile_regex(
        r#"(?:import|from)\s*((?:['"])(.*?)(?:['"]))"#,
        "dependency extraction",
    ) {
        Ok(regex) => regex,
        Err(_) => return SmallVec::new(),
    };
    let mut dependencies = SmallVec::new();

    for captures in import_regex.captures_iter(code) {
        if captures.len() >= 3
            && let Some(import_path) = captures.get(2)
        {
            let import_path_str = import_path.as_str().to_string();
            if !import_path_str.starts_with("react")
                && (import_path_str.starts_with(".")
                    || import_path_str.starts_with("/")
                    || import_path_str.contains("/"))
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
