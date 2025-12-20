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
                let imports: Vec<&str> = named_imports.split(',').map(|s| s.trim()).collect();

                result.push_str("if (typeof React === 'undefined') { var React = globalThis.React || { createElement: function() { return null; }, Fragment: function() { return null; } }; }\n");

                for import in imports {
                    let import_name = import.trim();
                    if !import_name.is_empty() {
                        if import_name.contains(" as ") {
                            let parts: Vec<&str> = import_name.split(" as ").collect();
                            if parts.len() == 2 {
                                let original_name = parts[0].trim();
                                let alias_name = parts[1].trim();
                                result.push_str(&format!(
                                        "if (typeof {} === 'undefined') {{ var {} = globalThis.React?.{} || globalThis.{} || (function(props) {{ return props?.children || null; }}); }}\n",
                                        alias_name, alias_name, original_name, alias_name
                                    ));
                            }
                        } else {
                            result.push_str(&format!(
                                    "if (typeof {} === 'undefined') {{ var {} = globalThis.React?.{} || globalThis.{} || (function(props) {{ return props?.children || null; }}); }}\n",
                                    import_name, import_name, import_name, import_name
                                ));
                        }
                    }
                }
                continue;
            }

            if react_default_import_regex.is_match(trimmed) {
                result.push_str("if (typeof React === 'undefined') { var React = globalThis.React || { createElement: function() { return null; }, Fragment: function() { return null; } }; }\n");
                continue;
            }
        } else if trimmed.starts_with("import") && !trimmed.contains("from 'react'") {
            if let Some(captures) = named_imports_regex.captures(trimmed)
                && let Some(named_imports_match) = captures.get(1)
            {
                let named_imports = named_imports_match.as_str();
                let imports: Vec<&str> = named_imports.split(',').map(|s| s.trim()).collect();

                for import in imports {
                    let import_name = import.trim();
                    if !import_name.is_empty() {
                        if import_name.contains(" as ") {
                            let parts: Vec<&str> = import_name.split(" as ").collect();
                            if parts.len() == 2 {
                                let original_name = parts[0].trim();
                                let alias_name = parts[1].trim();
                                result.push_str(&format!(
                                        "if (typeof {} === 'undefined') {{ var {} = (globalThis['~rsc'].functions && globalThis['~rsc'].functions.{} && globalThis['~rsc'].functions.{}.['~rsc_original']) ? globalThis['~rsc'].functions.{}.['~rsc_original'] : (globalThis['~rsc'].functions && globalThis['~rsc'].functions.{}) || globalThis.{} || (function(...args) {{ return Promise.resolve(null); }}); }}\n",
                                        alias_name, alias_name, original_name, original_name, original_name, original_name, alias_name
                                    ));
                            }
                        } else {
                            result.push_str(&format!(
                                    "if (typeof {} === 'undefined') {{ var {} = (globalThis['~rsc'].functions && globalThis['~rsc'].functions.{} && globalThis['~rsc'].functions.{}.['~rsc_original']) ? globalThis['~rsc'].functions.{}.['~rsc_original'] : (globalThis['~rsc'].functions && globalThis['~rsc'].functions.{}) || globalThis.{} || (function(...args) {{ return Promise.resolve(null); }}); }}\n",
                                    import_name, import_name, import_name, import_name, import_name, import_name, import_name
                                ));
                        }
                    }
                }
                continue;
            }

            if let Some(captures) = default_import_regex.captures(trimmed)
                && let Some(import_name_match) = captures.get(1)
            {
                let import_name = import_name_match.as_str();
                result.push_str(&format!(
                        "if (typeof {} === 'undefined') {{ var {} = globalThis.{} || (function(...args) {{ return Promise.resolve(null); }}); }}\n",
                        import_name, import_name, import_name
                    ));
                continue;
            }

            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}
