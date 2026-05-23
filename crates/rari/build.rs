use std::env;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use deno_ast::MediaType;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    let output_path = out_dir.join("deno_node_lazy_sources.rs");

    let deno_node_dir = find_deno_node_dir();
    let polyfills_dir = deno_node_dir.join("polyfills");
    println!("cargo:rerun-if-changed={}", polyfills_dir.display());

    let lib_rs_path = deno_node_dir.join("lib.rs");
    let (esm_files, js_files) = parse_lazy_file_lists(&lib_rs_path);

    generate_lazy_sources(&output_path, &polyfills_dir, &esm_files, &js_files);
}

fn find_deno_node_dir() -> PathBuf {
    let cargo_home = env::var("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .expect("Neither CARGO_HOME, HOME, nor USERPROFILE is set");
        PathBuf::from(home).join(".cargo")
    });

    let registry_src = cargo_home.join("registry").join("src");

    #[allow(clippy::disallowed_methods)]
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&registry_src)
        .expect("Failed to read cargo registry src directory")
        .filter_map(|entry| entry.ok())
        .flat_map(|index_dir| {
            std::fs::read_dir(index_dir.path())
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("deno_node-")))
                .map(|e| e.path())
                .collect::<Vec<_>>()
        })
        .filter(|p| p.join("polyfills").exists())
        .collect();

    candidates.sort();

    candidates.pop().unwrap_or_else(|| {
        panic!(
            "Could not find any deno_node crate in cargo registry at {}",
            registry_src.display()
        );
    })
}

type FileList = Vec<(String, String)>;

#[allow(clippy::type_complexity)]
fn parse_lazy_file_lists(lib_rs_path: &Path) -> (FileList, FileList) {
    #[allow(clippy::disallowed_methods)]
    let content = std::fs::read_to_string(lib_rs_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", lib_rs_path.display(), e));

    let esm_files = parse_section(&content, "lazy_loaded_esm");
    let js_files = parse_section(&content, "lazy_loaded_js");

    (esm_files, js_files)
}

fn parse_section(content: &str, section_name: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut in_section = false;
    let mut bracket_depth = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with(&format!("{section_name} = ["))
            || trimmed == format!("{section_name} = [")
        {
            in_section = true;
            bracket_depth = 1;
            continue;
        }

        if !in_section {
            continue;
        }

        if trimmed.contains('[') {
            bracket_depth += trimmed.matches('[').count();
        }
        if trimmed.contains(']') {
            bracket_depth -= trimmed.matches(']').count();
            if bracket_depth == 0 {
                break;
            }
        }

        if trimmed.starts_with("//") || trimmed.starts_with("dir ") {
            continue;
        }

        let trimmed = trimmed.trim_end_matches(',');
        if trimmed.is_empty() {
            continue;
        }

        if let Some((specifier, path)) = trimmed.split_once(" = ") {
            let specifier = specifier.trim().trim_matches('"');
            let path = path.trim().trim_matches('"');
            results.push((specifier.to_string(), path.to_string()));
        } else {
            let path = trimmed.trim_matches('"');
            if !path.is_empty() {
                let specifier = format!("ext:deno_node/{path}");
                results.push((specifier, path.to_string()));
            }
        }
    }

    results
}

fn generate_lazy_sources(
    output_path: &Path,
    polyfills_dir: &Path,
    esm_files: &[(String, String)],
    js_files: &[(String, String)],
) {
    let mut esm_entries = String::new();
    for (specifier, rel_path) in esm_files {
        let file_path = polyfills_dir.join(rel_path);
        #[allow(clippy::disallowed_methods)]
        let source = std::fs::read_to_string(&file_path).unwrap_or_else(|e| {
            panic!("Failed to read '{}': {}", file_path.display(), e);
        });
        let source = maybe_transpile(specifier, &source);
        let escaped = escape_string_for_rust(&source);
        writeln!(esm_entries, "    (\"{specifier}\", r###\"{escaped}\"###),")
            .expect("Failed to write ESM entry");
    }

    let mut js_entries = String::new();
    for (specifier, rel_path) in js_files {
        let file_path = polyfills_dir.join(rel_path);
        #[allow(clippy::disallowed_methods)]
        let source = std::fs::read_to_string(&file_path).unwrap_or_else(|e| {
            panic!("Failed to read '{}': {}", file_path.display(), e);
        });
        let source = maybe_transpile(specifier, &source);
        let escaped = escape_string_for_rust(&source);
        writeln!(js_entries, "    (\"{specifier}\", r###\"{escaped}\"###),")
            .expect("Failed to write JS entry");
    }

    let generated = format!(
        "// Auto-generated by build.rs - do not edit\n\
         // deno_node lazy-loaded sources, transpiled and embedded at compile time\n\n\
         pub static DENO_NODE_LAZY_ESM_SOURCES: &[(&str, &str)] = &[\n\
         {esm_entries}];\n\n\
         pub static DENO_NODE_LAZY_JS_SOURCES: &[(&str, &str)] = &[\n\
         {js_entries}];\n"
    );

    std::fs::write(output_path, generated).expect("Failed to write generated lazy sources");
}

fn maybe_transpile(specifier: &str, source: &str) -> String {
    let media_type = if specifier.starts_with("node:") {
        MediaType::TypeScript
    } else {
        MediaType::from_path(Path::new(specifier))
    };

    match media_type {
        MediaType::TypeScript | MediaType::Tsx | MediaType::Jsx => {}
        _ => return source.to_string(),
    }

    let specifier_url = url::Url::parse(specifier).unwrap_or_else(|_| {
        url::Url::parse(&format!("file:///{specifier}")).expect("invalid specifier")
    });

    let parsed = deno_ast::parse_module(deno_ast::ParseParams {
        specifier: specifier_url,
        text: source.into(),
        media_type,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })
    .unwrap_or_else(|e| panic!("Failed to parse '{specifier}': {e}"));

    parsed
        .transpile(
            &deno_ast::TranspileOptions {
                imports_not_used_as_values: deno_ast::ImportsNotUsedAsValues::Remove,
                ..Default::default()
            },
            &deno_ast::TranspileModuleOptions::default(),
            &deno_ast::EmitOptions {
                source_map: deno_ast::SourceMapOption::None,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("Failed to transpile '{specifier}': {e}"))
        .into_source()
        .text
}

#[allow(clippy::disallowed_methods)]
fn escape_string_for_rust(s: &str) -> String {
    s.replace("\"###", "\"####")
}
