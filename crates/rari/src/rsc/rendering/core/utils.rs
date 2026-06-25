use std::fmt::Write;

use regex::Regex;

pub fn transform_imports_for_hmr(source: &str) -> String {
    let react_named_imports_regex =
        match Regex::new(r"import\s+React,?\s*\{\s*([^}]+)\s*\}\s+from\s+['\x22]react['\x22]") {
            Ok(regex) => regex,
            Err(_) => return source.to_string(),
        };

    let react_default_import_regex =
        match Regex::new(r"import\s+React\s+from\s+['\x22]react['\x22]") {
            Ok(regex) => regex,
            Err(_) => return source.to_string(),
        };

    let named_imports_regex =
        match Regex::new(r"import\s+\{\s*([^}]+)\s*\}\s+from\s+['\x22]([^'\x22]+)['\x22]") {
            Ok(regex) => regex,
            Err(_) => return source.to_string(),
        };

    let default_import_regex =
        match Regex::new(r"import\s+(\w+)\s+from\s+['\x22]([^'\x22]+)['\x22]") {
            Ok(regex) => regex,
            Err(_) => return source.to_string(),
        };

    let mut result = String::new();
    let lines: Vec<&str> = source.lines().collect();

    for line in lines {
        let trimmed = line.trim();

        if trimmed.starts_with("import") && trimmed.contains("from 'react'") {
            if let Some(captures) = react_named_imports_regex.captures(trimmed)
                && let Some(named_imports_match) = captures.get(1)
            {
                let named_imports = named_imports_match.as_str();
                let imports: Vec<&str> = named_imports.split(',').map(str::trim).collect();

                result.push_str(
                    "if (typeof React === 'undefined') { var React = globalThis.React; }\n",
                );

                for import in imports {
                    let import_name = import.trim();
                    if !import_name.is_empty() {
                        if import_name.contains(" as ") {
                            let parts: Vec<&str> = import_name.split(" as ").collect();
                            if parts.len() == 2 {
                                let original_name = parts[0].trim();
                                let alias_name = parts[1].trim();
                                let _ = writeln!(
                                    result,
                                    "if (typeof {alias_name} === 'undefined') {{ var {alias_name} = globalThis.React?.{original_name} || globalThis.{alias_name} || (function(props) {{ return props?.children || null; }}); }}"
                                );
                            }
                        } else {
                            let _ = writeln!(
                                result,
                                "if (typeof {import_name} === 'undefined') {{ var {import_name} = globalThis.React?.{import_name} || globalThis.{import_name} || (function(props) {{ return props?.children || null; }}); }}"
                            );
                        }
                    }
                }
                continue;
            }

            if react_default_import_regex.is_match(trimmed) {
                result.push_str(
                    "if (typeof React === 'undefined') { var React = globalThis.React; }\n",
                );
                continue;
            }
        } else if trimmed.starts_with("import") && !trimmed.contains("from 'react'") {
            if let Some(captures) = named_imports_regex.captures(trimmed)
                && let Some(named_imports_match) = captures.get(1)
            {
                let named_imports = named_imports_match.as_str();
                let imports: Vec<&str> = named_imports.split(',').map(str::trim).collect();

                for import in imports {
                    let import_name = import.trim();
                    if !import_name.is_empty() {
                        if import_name.contains(" as ") {
                            let parts: Vec<&str> = import_name.split(" as ").collect();
                            if parts.len() == 2 {
                                let original_name = parts[0].trim();
                                let alias_name = parts[1].trim();
                                let _ = writeln!(
                                    result,
                                    "if (typeof {alias_name} === 'undefined') {{ var {alias_name} = (globalThis['~rsc'].functions && globalThis['~rsc'].functions.{original_name} && globalThis['~rsc'].functions.{original_name}['~rsc_original']) ? globalThis['~rsc'].functions.{original_name}['~rsc_original'] : (globalThis['~rsc'].functions && globalThis['~rsc'].functions.{original_name}) || globalThis.{alias_name} || (function(...args) {{ return Promise.resolve(null); }}); }}"
                                );
                            }
                        } else {
                            let _ = writeln!(
                                result,
                                "if (typeof {import_name} === 'undefined') {{ var {import_name} = (globalThis['~rsc'].functions && globalThis['~rsc'].functions.{import_name} && globalThis['~rsc'].functions.{import_name}['~rsc_original']) ? globalThis['~rsc'].functions.{import_name}['~rsc_original'] : (globalThis['~rsc'].functions && globalThis['~rsc'].functions.{import_name}) || globalThis.{import_name} || (function(...args) {{ return Promise.resolve(null); }}); }}"
                            );
                        }
                    }
                }
                continue;
            }

            if let Some(captures) = default_import_regex.captures(trimmed)
                && let Some(import_name_match) = captures.get(1)
            {
                let import_name = import_name_match.as_str();
                let _ = writeln!(
                    result,
                    "if (typeof {import_name} === 'undefined') {{ var {import_name} = globalThis.{import_name} || (function(...args) {{ return Promise.resolve(null); }}); }}"
                );
                continue;
            }

            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}
