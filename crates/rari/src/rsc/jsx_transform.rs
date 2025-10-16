use crate::error::RariError;
use smallvec::SmallVec;
use std::path::PathBuf;

use oxc::allocator::Allocator;
use oxc::codegen::{Codegen, CodegenOptions};
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{JsxOptions, JsxRuntime, TransformOptions, Transformer};

type DependencyList = SmallVec<[String; 4]>;
type ErrorList = SmallVec<[String; 2]>;
type ExportList = SmallVec<[String; 3]>;

const MAX_JS_FILE_SIZE: usize = 52_428_800;

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

pub fn transform_jsx(jsx_code: &str, component_id: &str) -> Result<String, RariError> {
    if jsx_code.len() > MAX_JS_FILE_SIZE {
        let error_msg = format!(
            "JSX input too large to process safely: {} bytes (max: {} bytes)",
            jsx_code.len(),
            MAX_JS_FILE_SIZE
        );
        return Err(RariError::js_execution(error_msg));
    }

    if jsx_code.contains("// Self-registering Production Component:")
        || jsx_code.contains("// Self-registering Non-JSX Module:")
        || (jsx_code.contains("globalThis.__server_functions")
            && jsx_code.contains("globalThis.__rari_manual_register"))
    {
        return Ok(jsx_code.to_string());
    }

    let is_non_jsx = !jsx_code.contains("<") && !jsx_code.contains(">");

    if is_non_jsx {
        return transform_non_jsx_module(jsx_code, component_id);
    }

    let allocator = Allocator::default();

    let preprocessed_source = jsx_code
        .replace("'use server'", "// 'use server'")
        .replace("\"use server\"", "// \"use server\"");

    let source_type = SourceType::default().with_module(true).with_jsx(true).with_typescript(true);

    let parse_result = Parser::new(&allocator, &preprocessed_source, source_type).parse();

    if !parse_result.errors.is_empty() {
        let error_messages = parse_result
            .errors
            .into_iter()
            .map(|e| e.message.to_string())
            .collect::<ErrorList>()
            .join("\n");

        return Err(RariError::js_execution(format!(
            "JSX parsing failed for component '{component_id}':\n{error_messages}"
        )));
    }

    let mut program = parse_result.program;

    let semantic_builder = SemanticBuilder::new();
    let semantic_result = semantic_builder.build(&program);

    if !semantic_result.errors.is_empty() {
        let error_messages = semantic_result
            .errors
            .into_iter()
            .map(|e| e.message.to_string())
            .collect::<ErrorList>()
            .join("\n");

        return Err(RariError::js_execution(format!(
            "Semantic analysis failed for component '{component_id}':\n{error_messages}"
        )));
    }

    let semantic_info = semantic_result.semantic;

    let jsx_options = JsxOptions {
        runtime: JsxRuntime::Automatic,
        import_source: Some("react".to_string()),
        ..JsxOptions::default()
    };

    let transform_options = TransformOptions { jsx: jsx_options, ..Default::default() };
    let source_path = PathBuf::from(component_id);
    let transformer = Transformer::new(&allocator, &source_path, &transform_options);
    let scoping_data = semantic_info.into_scoping();
    let transformer_return = transformer.build_with_scoping(scoping_data, &mut program);

    if !transformer_return.errors.is_empty() {
        let error_messages = transformer_return
            .errors
            .into_iter()
            .map(|e| e.message.to_string())
            .collect::<ErrorList>()
            .join("\n");
        return Err(RariError::js_execution(format!(
            "JSX transformation failed for component '{component_id}':\n{error_messages}"
        )));
    }

    let codegen_options = CodegenOptions::default();
    let codegen = Codegen::new();
    let printed_code = codegen.with_options(codegen_options).build(&program);
    let raw_js_code = printed_code.code;

    if raw_js_code.len() > MAX_JS_FILE_SIZE {
        let error_msg = format!(
            "Transformed JavaScript too large: {} bytes (max: {} bytes)",
            raw_js_code.len(),
            MAX_JS_FILE_SIZE
        );
        return Err(RariError::js_execution(error_msg));
    }

    let mut processed_js_code = raw_js_code;

    let template_regex = compile_regex(r"(`[^`]*\$\{[^}]*}[^`]*`)", "template literals")?;
    let mut replacements = Vec::new();

    for capture in template_regex.captures_iter(&processed_js_code) {
        if let Some(template) = capture.get(1) {
            let template_str = template.as_str();
            let parts: Vec<&str> = template_str.split("${").collect();
            if parts.len() > 1 {
                let first_part = parts[0].trim_start_matches('`');

                let mut processed_parts = vec![format!("'{}'", first_part)];
                for part in parts.iter().skip(1) {
                    if let Some(idx) = part.find('}') {
                        let expr = &part[..idx];
                        let after = &part[idx + 1..];
                        processed_parts.push(expr.to_string());
                        if !after.is_empty() && after != "`" {
                            processed_parts.push(format!("'{}'", after.trim_end_matches('`')));
                        }
                    }
                }

                let replacement = processed_parts.join(" + ");
                replacements.push((template_str.to_string(), replacement));
            }
        }
    }

    for (template, replacement) in replacements {
        processed_js_code = processed_js_code.replace(&template, &replacement);
    }

    processed_js_code =
        processed_js_code.replace(" && React.createElement", " ? React.createElement : null");

    let conditional_string_regex =
        compile_regex(r#"(\w+(?:\.\w+)*)\s*&&\s*["']([^"']*)['"]\s*"#, "conditional string")?;
    processed_js_code =
        conditional_string_regex.replace_all(&processed_js_code, "$1 ? '$2' : null").to_string();

    let jsx_conditional_regex =
        compile_regex(r#"\{([^{}]+?)\s*&&\s*\(([^{}]*<[^>]+>[^{}]*)\)\}"#, "JSX conditional")?;

    processed_js_code = jsx_conditional_regex
        .replace_all(&processed_js_code, |caps: &regex::Captures| {
            let condition = &caps[1];
            let jsx_content = &caps[2];
            format!("{{{} ? ({}) : null}}", condition.trim(), jsx_content.trim())
        })
        .to_string();

    let jsx_conditional_simple_regex = compile_regex(
        r#"\{([^{}]+?)\s*&&\s*(<[^>]+(?:/>|>[^<]*</[^>]+>))\}"#,
        "simple JSX conditional",
    )?;

    processed_js_code = jsx_conditional_simple_regex
        .replace_all(&processed_js_code, |caps: &regex::Captures| {
            let condition = &caps[1];
            let jsx_element = &caps[2];
            format!("{{{} ? {} : null}}", condition.trim(), jsx_element.trim())
        })
        .to_string();

    let logical_or_regex = compile_regex(
        r#"([a-zA-Z_$][a-zA-Z0-9_$]*(?:\.[a-zA-Z_$][a-zA-Z0-9_$]*)*)\s*\|\|\s*([a-zA-Z_$][a-zA-Z0-9_$]*(?:\.[a-zA-Z_$][a-zA-Z0-9_$]*)*)"#,
        "logical OR",
    )?;

    processed_js_code =
        logical_or_regex.replace_all(&processed_js_code, "$1 ? $1 : $2").to_string();

    if processed_js_code.len() > MAX_JS_FILE_SIZE {
        let error_msg = format!(
            "Final JavaScript too large after post-processing: {} bytes (max: {} bytes)",
            processed_js_code.len(),
            MAX_JS_FILE_SIZE
        );
        return Err(RariError::js_execution(error_msg));
    }

    let import_regex = compile_regex(
        r#"(import\s+(?:[\w\s{},*]+\s+from\s+)?['"].*?['"];?)"#,
        "import extraction",
    )?;
    let imports: Vec<&str> = import_regex
        .find_iter(&processed_js_code)
        .map(|m| m.as_str())
        .filter(|import_line| {
            !import_line.contains("from 'react'") && !import_line.contains("from \"react\"")
        })
        .collect();

    let import_string = imports.join("\n");

    let code_without_imports = import_regex.replace_all(&processed_js_code, "").to_string();

    let export_default_regex = compile_regex(r#"export\s+default\s+([^;]+);?"#, "export default")?;
    let export_named_regex = compile_regex(
        r#"export\s+((?:async\s+)?function\s+\w+[^;]*|const\s+\w+[^;]*|let\s+\w+[^;]*|var\s+\w+[^;]*|class\s+\w+[^;]*)"#,
        "export named",
    )?;

    let default_export_expression =
        if let Some(captures) = export_default_regex.captures(&code_without_imports) {
            if let Some(export_match) = captures.get(1) {
                let export_str = export_match.as_str().trim();
                if (export_str.starts_with("function") || export_str.starts_with("async function"))
                    && export_str.contains("(")
                {
                    if export_str.starts_with("async function") {
                        let parts: Vec<&str> = export_str.split(&[' ', '(']).collect();
                        if parts.len() > 2 && !parts[2].is_empty() {
                            parts[2].to_string()
                        } else {
                            export_str.to_string()
                        }
                    } else {
                        let parts: Vec<&str> = export_str.split(&[' ', '(']).collect();
                        if parts.len() > 1 && !parts[1].is_empty() {
                            parts[1].to_string()
                        } else {
                            export_str.to_string()
                        }
                    }
                } else {
                    export_str.to_string()
                }
            } else {
                "null".to_string()
            }
        } else {
            "null".to_string()
        };

    let mut named_exports = Vec::new();
    for captures in export_named_regex.captures_iter(&code_without_imports) {
        if let Some(export_match) = captures.get(1) {
            let export_str = export_match.as_str();
            if export_str.starts_with("function ") {
                if let Some(name) = export_str.split_whitespace().nth(1) {
                    named_exports
                        .push(name.trim_matches(|c: char| !c.is_alphanumeric()).to_string());
                }
            } else if export_str.starts_with("const ")
                || export_str.starts_with("let ")
                || export_str.starts_with("var ")
            {
                if let Some(name) = export_str.split_whitespace().nth(1) {
                    named_exports
                        .push(name.trim_matches(|c: char| !c.is_alphanumeric()).to_string());
                }
            } else if export_str.starts_with("class ")
                && let Some(name) = export_str.split_whitespace().nth(1)
            {
                named_exports.push(name.trim_matches(|c: char| !c.is_alphanumeric()).to_string());
            }
        }
    }

    let component_hash = hash_string(component_id);
    let registration_key = format!("Component_{component_hash}");

    let default_export_expression_or_fallback_val = if default_export_expression != "null" {
        default_export_expression.clone()
    } else if named_exports.len() == 1 {
        named_exports[0].clone()
    } else {
        "null".to_string()
    };

    let export_expression_for_direct_import_val = if default_export_expression != "null" {
        default_export_expression
    } else if !named_exports.is_empty() {
        if named_exports.len() == 1 {
            named_exports[0].clone()
        } else {
            format!("{{ {} }}", named_exports.join(", "))
        }
    } else {
        "function() { return null; }".to_string()
    };

    let module_code = format!(
        r#"// Transformed ES Module for: {component_id}
"use module"; // Ensure this is treated as a module
{imports}

{code_without_imports}

(function() {{
    const componentKey = "{component_id}";
    const registrationKey = "{registration_key}";
    let mainExport = null;

    if (typeof {default_export_expression_or_fallback} !== 'undefined') {{
        mainExport = {default_export_expression_or_fallback};
    }}

    if (mainExport !== null) {{
        if (!globalThis[componentKey]) {{
            globalThis[componentKey] = mainExport;
        }}

        if (!globalThis[registrationKey]) {{
            globalThis[registrationKey] = mainExport;
        }}

        globalThis.__server_functions = globalThis.__server_functions || {{}};

        if (typeof mainExport === 'object') {{
            for (const exportName in mainExport) {{
                if (typeof mainExport[exportName] === 'function') {{
                    globalThis.__server_functions[exportName] = mainExport[exportName];
                }}
            }}
        }}
        else if (typeof mainExport === 'function') {{
            const fnName = mainExport.name || `fn_${component_id}`;

            globalThis.__server_functions[fnName] = mainExport;
        }}
  }} else {{
    }}

    globalThis.__rari_manual_register = globalThis.__rari_manual_register || {{}};
    globalThis.__rari_manual_register[componentKey] = () => {{
        if (mainExport !== null) {{
            globalThis[componentKey] = mainExport;
            globalThis[registrationKey] = mainExport;
            return true;
        }}
        return false;
    }};
}})();

export const __rari_main_export = {export_expression_for_direct_import};
export function __rari_register() {{ /* Compatibility stub */ return true; }}

export const __registry_proxy = new Proxy({{}}, {{
  get: function(target, prop) {{
    if (globalThis.__server_functions && typeof globalThis.__server_functions[prop] === 'function') {{
      return globalThis.__server_functions[prop];
    }}
    if (typeof globalThis[prop] === 'function') {{
      return globalThis[prop];
    }}
    return undefined;
  }}
}});
"#,
        imports = import_string,
        code_without_imports = code_without_imports.trim(),
        default_export_expression_or_fallback = default_export_expression_or_fallback_val,
        export_expression_for_direct_import = export_expression_for_direct_import_val,
        component_id = component_id,
        registration_key = registration_key
    );

    Ok(module_code)
}

fn transform_non_jsx_module(ts_code: &str, module_id_arg: &str) -> Result<String, RariError> {
    let allocator = Allocator::default();

    let preprocessed_source = ts_code
        .replace("'use server'", "// 'use server'")
        .replace("\"use server\"", "// \"use server\"");

    let source_type = SourceType::default().with_module(true).with_typescript(true);

    let parse_result = Parser::new(&allocator, &preprocessed_source, source_type).parse();

    if !parse_result.errors.is_empty() {
        let error_messages = parse_result
            .errors
            .into_iter()
            .map(|e| e.message.to_string())
            .collect::<ErrorList>()
            .join("\n");

        return Err(RariError::js_execution(format!(
            "TypeScript parsing failed for module '{module_id_arg}':\n{error_messages}"
        )));
    }

    let mut program = parse_result.program;

    let semantic_builder = SemanticBuilder::new();
    let semantic_result = semantic_builder.build(&program);

    if !semantic_result.errors.is_empty() {
        let error_messages = semantic_result
            .errors
            .into_iter()
            .map(|e| e.message.to_string())
            .collect::<ErrorList>()
            .join("\n");

        return Err(RariError::js_execution(format!(
            "Semantic analysis failed for module '{module_id_arg}':\n{error_messages}"
        )));
    }

    let semantic_info = semantic_result.semantic;

    let transform_options = TransformOptions::default();
    let source_path = PathBuf::from(module_id_arg);
    let transformer = Transformer::new(&allocator, &source_path, &transform_options);
    let scoping_data = semantic_info.into_scoping();
    let transformer_return = transformer.build_with_scoping(scoping_data, &mut program);

    if !transformer_return.errors.is_empty() {
        let error_messages = transformer_return
            .errors
            .into_iter()
            .map(|e| e.message.to_string())
            .collect::<ErrorList>()
            .join("\n");
        return Err(RariError::js_execution(format!(
            "TypeScript transformation failed for module '{module_id_arg}':\n{error_messages}"
        )));
    }

    let codegen_options = CodegenOptions::default();
    let codegen = Codegen::new();
    let printed_code = codegen.with_options(codegen_options).build(&program);
    let processed_code_val = printed_code.code;

    let export_fn_regex = match regex::Regex::new(r"export\s+(async\s+)?function\s+(\w+)") {
        Ok(regex) => regex,
        Err(_) => {
            return Err(RariError::js_execution(
                "Failed to compile export function regex".to_string(),
            ));
        }
    };
    let export_const_regex = match regex::Regex::new(r"export\s+const\s+(\w+)\s*=") {
        Ok(regex) => regex,
        Err(_) => {
            return Err(RariError::js_execution(
                "Failed to compile export const regex".to_string(),
            ));
        }
    };
    let export_default_regex = match regex::Regex::new(r"export\s+default\s+([\w\d_]+);?") {
        Ok(regex) => regex,
        Err(_) => {
            return Err(RariError::js_execution(
                "Failed to compile export default regex".to_string(),
            ));
        }
    };
    let export_default_fn_regex =
        match regex::Regex::new(r"export\s+default\s+function\s+([\w\d_]+)") {
            Ok(regex) => regex,
            Err(_) => {
                return Err(RariError::js_execution(
                    "Failed to compile export default function regex".to_string(),
                ));
            }
        };
    let export_object_default_regex =
        match regex::Regex::new(r"export\s*\{\s*([\w\d_]+)\s+as\s+default\s*\};?") {
            Ok(regex) => regex,
            Err(_) => {
                return Err(RariError::js_execution(
                    "Failed to compile export object default regex".to_string(),
                ));
            }
        };
    let import_regex = match regex::Regex::new(r"import\s+.*?from\s+.*?;") {
        Ok(regex) => regex,
        Err(_) => {
            return Err(RariError::js_execution("Failed to compile import regex".to_string()));
        }
    };

    let mut exported_names = Vec::new();
    let mut default_export_name = "null".to_string();

    for cap in export_fn_regex.captures_iter(&processed_code_val) {
        if let Some(name) = cap.get(2) {
            exported_names.push(name.as_str().to_string());
        }
    }
    for cap in export_const_regex.captures_iter(&processed_code_val) {
        if let Some(name) = cap.get(1) {
            exported_names.push(name.as_str().to_string());
        }
    }
    if let Some(cap) = export_default_fn_regex.captures(&processed_code_val)
        && let Some(name) = cap.get(1)
    {
        default_export_name = name.as_str().to_string();
    } else if let Some(cap) = export_object_default_regex.captures(&processed_code_val)
        && let Some(name) = cap.get(1)
    {
        default_export_name = name.as_str().to_string();
    } else if let Some(cap) = export_default_regex.captures(&processed_code_val)
        && let Some(name) = cap.get(1)
    {
        default_export_name = name.as_str().to_string();
    }

    let mut processed_code_without_exports = processed_code_val.clone();

    processed_code_without_exports =
        import_regex.replace_all(&processed_code_without_exports, "").to_string();

    processed_code_without_exports = export_default_fn_regex
        .replace_all(&processed_code_without_exports, "function $1")
        .to_string();
    processed_code_without_exports =
        export_fn_regex.replace_all(&processed_code_without_exports, "$1function $2").to_string();
    processed_code_without_exports =
        export_const_regex.replace_all(&processed_code_without_exports, "const $1").to_string();
    processed_code_without_exports =
        export_object_default_regex.replace_all(&processed_code_without_exports, "").to_string();
    processed_code_without_exports =
        export_default_regex.replace_all(&processed_code_without_exports, "$1").to_string();

    let registration_key_val = format!("Component_{}", hash_string(module_id_arg));

    let main_export_expression_val = if default_export_name != "null" {
        default_export_name.clone()
    } else if exported_names.len() == 1 {
        exported_names[0].clone()
    } else if !exported_names.is_empty() {
        let exports_object_fields = exported_names
            .iter()
            .map(|name| format!("{name}: {name}"))
            .collect::<ExportList>()
            .join(", ");
        format!("{{ {exports_object_fields} }}")
    } else {
        "null".to_string()
    };

    let main_export_expression_or_fallback_val = main_export_expression_val.clone();

    let exported_names_check_and_assign_val = exported_names
        .iter()
        .map(|name| format!("if (typeof {name} !== 'undefined') potentialExports.{name} = {name};"))
        .collect::<ExportList>()
        .join("\n            ");

    let self_registration_script_for_individual_server_fns_val = exported_names
        .iter()
        .map(|name| {
            format!(
                "
                        if (typeof {name} !== 'undefined') {{
                globalThis.{name} = {name};

                globalThis.__server_functions = globalThis.__server_functions || {{}};
                globalThis.__server_functions['{name}'] = {name};

                registeredAny = true;
            }}"
            )
        })
        .collect::<ExportList>()
        .join("\n            ");

    let module_code = format!(
        r#"// Self-registering Non-JSX Module: {module_id}
"use module";

// Original module code with exports removed for self-registration
{processed_code_without_exports}

// Self-registration logic
(function() {{
    try {{
        const moduleKey = "{module_id}";
        const registrationKey = "{registration_key}";
        let mainExport = null;
        let exportedFunctions = {{}};

        globalThis.__server_functions = globalThis.__server_functions || {{}};

        {exported_names_server_fn_register}

    if (typeof {main_export_expression_or_fallback} !== 'undefined') {{
        mainExport = {main_export_expression_or_fallback};
    }} else {{
        const potentialExports = {{}};
        {exported_names_check_and_assign} // exported_names_check_and_assign
        if(Object.keys(potentialExports).length > 0) {{
            if (Object.keys(potentialExports).length === 1) {{
                 mainExport = potentialExports[Object.keys(potentialExports)[0]];
            }} else {{
                 mainExport = potentialExports;
            }}
        }}
    }}

    if (mainExport !== null) {{
        if (!globalThis[moduleKey]) {{
            globalThis[moduleKey] = mainExport;
        }}

        if (!globalThis[registrationKey]) {{
            globalThis[registrationKey] = mainExport;
        }}

        globalThis.__server_functions = globalThis.__server_functions || {{}};

        if (typeof mainExport === 'object') {{
            for (const exportName in mainExport) {{
                if (typeof mainExport[exportName] === 'function') {{
                    globalThis.__server_functions[exportName] = mainExport[exportName];
                }}
            }}
        }}
        else if (typeof mainExport === 'function') {{
            const fnName = mainExport.name || `fn_\${module_id}`;

            globalThis.__server_functions[fnName] = mainExport;
        }}
    }} else {{
        if (Object.keys(exportedFunctions).length > 0) {{
            if (!globalThis[moduleKey]) {{
                globalThis[moduleKey] = exportedFunctions;
            }}

            if (!globalThis[registrationKey]) {{
                globalThis[registrationKey] = exportedFunctions;
            }}

            mainExport = exportedFunctions;
        }}
}}

    globalThis.__server_functions = globalThis.__server_functions || {{}};

    for (const key in globalThis) {{
        if (typeof globalThis[key] === 'function' &&
            !key.startsWith('__')) {{

            globalThis.__server_functions[key] = globalThis[key];
        }}
    }}

    globalThis.__rari_manual_register = globalThis.__rari_manual_register || {{}};
    globalThis.__rari_manual_register[moduleKey] = () => {{
        if (mainExport !== null) {{
            globalThis[moduleKey] = mainExport;
            globalThis[registrationKey] = mainExport;
            return true;
        }}
        let registeredAnyFunction = false;

        {self_registration_script_for_individual_server_fns}

        for (const key in globalThis) {{
            if (typeof globalThis[key] === 'function' &&
                !key.startsWith('__') &&
                key !== '__server_functions') {{

                globalThis.__server_functions = globalThis.__server_functions || {{}};
                globalThis.__server_functions[key] = globalThis[key];

                registeredAnyFunction = true;
            }}
        }}

        const hasRscFunctions = globalThis.__server_functions && Object.keys(globalThis.__server_functions).length > 0;

        if (hasRscFunctions) {{
            globalThis[moduleKey] = globalThis.__server_functions;
            globalThis[registrationKey] = globalThis.__server_functions;
            return true;
        }}

        return false;
    }};
    }} catch (error) {{
        globalThis.__server_functions = globalThis.__server_functions || {{}};
        globalThis.__rsc_modules = globalThis.__rsc_modules || {{}};

        for (const key in globalThis) {{
            if (typeof globalThis[key] === 'function' && !key.startsWith('__')) {{
                globalThis.__server_functions[key] = globalThis[key];
            }}
        }}
    }}
}})();
"#,
        module_id = module_id_arg,
        processed_code_without_exports = processed_code_without_exports,
        registration_key = registration_key_val,
        main_export_expression_or_fallback = main_export_expression_or_fallback_val,
        exported_names_check_and_assign = exported_names_check_and_assign_val,
        self_registration_script_for_individual_server_fns =
            self_registration_script_for_individual_server_fns_val,
        exported_names_server_fn_register = exported_names
            .iter()
            .map(|name| format!(
                r#"        // Register individual function: {name}
        if (typeof {name} === 'function') {{
            // Register in generic registry
            globalThis.__server_functions = globalThis.__server_functions || {{}};
            globalThis.__server_functions['{name}'] = {name};

        }} else {{
        }}"#
            ))
            .collect::<ExportList>()
            .join("\n        ")
    );

    Ok(module_code)
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

    #[test]
    fn test_transform_jsx() {
        let code = r#"
        import React from 'react';
        export default function Component() {
            return <div>Test</div>;
        }
        "#;

        let result = transform_jsx(code, "TestComponent");
        assert!(result.is_ok());
        let js_code = result.expect("Transform should succeed in test");

        assert!(js_code.contains("TestComponent"));

        assert!(js_code.contains("_jsx") || js_code.contains("React.createElement"));

        assert!(js_code.contains("__rsc_modules") || js_code.contains("__server_functions"));
    }

    #[test]
    fn test_transform_server_component() {
        let code = r#"
        'use server';
        import React from 'react';
        export default function ServerComponent() {
            return <div>Server Component</div>;
        }
        "#;

        let result = transform_jsx(code, "ServerComponent");
        assert!(result.is_ok());
        let js_code = result.expect("Transform should succeed in test");

        assert!(js_code.contains("ServerComponent"));
    }

    #[test]
    fn test_no_component_reference_in_output() {
        let code = r#"
        import React from 'react';
        const CustomComponent = () => <div>Custom</div>;
        export { CustomComponent };
        "#;

        let result = transform_jsx(code, "CustomComponent");
        assert!(result.is_ok());

        let js_code = result.expect("Transform should succeed in test");

        assert!(js_code.contains("CustomComponent"));

        assert!(
            js_code.contains("globalThis.__rsc_modules")
                || js_code.contains("__server_functions")
                || js_code.contains("__registry_proxy")
        );
    }
}
