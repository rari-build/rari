use std::path::{Path, PathBuf};

use cow_utils::CowUtils;
use rari_error::RariError;
use sha2::{Digest, Sha256};

const DIST_DIR: &str = "dist";

pub fn short_hash(value: &str) -> String {
    let hash = Sha256::digest(value.as_bytes());
    hex::encode(hash)[..8].to_string()
}

pub fn readable_component_id(project_relative_path: &str) -> String {
    let without_extension = project_relative_path
        .trim_end_matches(".tsx")
        .trim_end_matches(".ts")
        .trim_end_matches(".jsx")
        .trim_end_matches(".js");

    without_extension
        .strip_prefix("src/")
        .unwrap_or(without_extension)
        .chars()
        .map(
            |c| {
                if c.is_ascii_alphanumeric() || c == '/' || c == '-' || c == '_' { c } else { '_' }
            },
        )
        .collect()
}

fn is_use_client_directive(trimmed: &str) -> bool {
    matches!(trimmed, "'use client';" | "\"use client\";" | "'use client'" | "\"use client\"")
}

fn is_use_server_directive(trimmed: &str) -> bool {
    matches!(trimmed, "'use server';" | "\"use server\";" | "'use server'" | "\"use server\"")
}

fn is_closed_string_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    let Some((&quote, rest)) = bytes.split_first() else {
        return false;
    };
    if quote != b'\'' && quote != b'"' {
        return false;
    }
    let body = if rest.last() == Some(&b';') { &rest[..rest.len() - 1] } else { rest };
    body.last() == Some(&quote)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DirectiveFlags {
    use_client: bool,
    use_server: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DirectiveScan {
    present: DirectiveFlags,
    top_level: DirectiveFlags,
}

/// Strip leading complete `/* ... */` comments. Tracks open block comments across lines.
/// Returns the next prologue-significant token and advances `remaining` past it.
fn next_directive_candidate<'a>(
    remaining: &mut &'a str,
    in_block_comment: &mut bool,
) -> Option<&'a str> {
    let mut rest = remaining.trim_start();

    if *in_block_comment {
        let end = rest.find("*/")?;
        rest = rest[end + 2..].trim_start();
        *in_block_comment = false;
    }

    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ';');

        if rest.is_empty() {
            *remaining = rest;
            return None;
        }

        if rest.starts_with("//") {
            *remaining = "";
            return None;
        }

        if let Some(after_open) = rest.strip_prefix("/*") {
            if let Some(end) = after_open.find("*/") {
                rest = after_open[end + 2..].trim_start();
                continue;
            }
            *in_block_comment = true;
            *remaining = "";
            return None;
        }

        break;
    }

    let bytes = rest.as_bytes();
    if bytes.first().is_some_and(|b| *b == b'\'' || *b == b'"') {
        let quote = bytes[0];
        let Some(close_rel) = rest[1..].find(quote as char) else {
            *remaining = "";
            return Some(rest);
        };
        let mut end = 1 + close_rel + 1;
        if rest.as_bytes().get(end) == Some(&b';') {
            end += 1;
        }
        let token = &rest[..end];
        *remaining = rest[end..].trim_start();
        return Some(token);
    }

    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let token = &rest[..end];
    *remaining = rest[end..].trim_start();
    Some(token)
}

fn scan_directives(code: &str) -> DirectiveScan {
    let mut in_block_comment = false;
    let mut scan = DirectiveScan::default();
    let mut saw_first_string = false;

    for line in code.lines() {
        let mut remaining = line;

        while let Some(token) = next_directive_candidate(&mut remaining, &mut in_block_comment) {
            let is_string = token.starts_with('\'') || token.starts_with('"');
            if !is_string {
                return scan;
            }

            let is_client = is_use_client_directive(token);
            let is_server = is_use_server_directive(token);

            if !saw_first_string {
                saw_first_string = true;
                scan.top_level.use_client = is_client;
                scan.top_level.use_server = is_server;
            }

            if !is_closed_string_token(token) {
                return scan;
            }

            if is_client {
                scan.present.use_client = true;
            }
            if is_server {
                scan.present.use_server = true;
            }
        }
    }

    scan
}

pub fn has_use_client_directive(code: &str) -> bool {
    scan_directives(code).present.use_client
}

pub fn has_use_server_directive(code: &str) -> bool {
    scan_directives(code).present.use_server
}

pub fn has_top_level_use_client_directive(code: &str) -> bool {
    scan_directives(code).top_level.use_client
}

pub fn has_top_level_use_server_directive(code: &str) -> bool {
    scan_directives(code).top_level.use_server
}

pub fn wrap_server_action_module(code: &str, module_id: &str) -> String {
    if code.contains("Self-registering Production Component") {
        return code.to_string();
    }

    let module_key = format!("__module_loaded_{}", module_id.cow_replace(&['/', '-'][..], "_"));

    format!(
        r"
if (!globalThis.{module_key}) {{
    globalThis.{module_key} = true;
    {code}
}}
"
    )
}

#[expect(clippy::missing_errors_doc)]
pub fn extract_component_id(file_path: &str) -> Result<String, RariError> {
    let path = Path::new(file_path);

    let project_relative_path = if path.is_absolute() {
        let components: Vec<_> = path.components().collect();
        if let Some(src_idx) = components.iter().rposition(|c| c.as_os_str() == "src") {
            components[src_idx..].iter().collect()
        } else {
            return Err(RariError::validation(format!(
                "Path does not contain 'src' directory: {file_path}"
            )));
        }
    } else {
        let normalized = file_path.cow_replace('\\', "/");
        if normalized.starts_with("src/") {
            path.to_path_buf()
        } else {
            Path::new("src").join(path)
        }
    };

    let project_relative_path = project_relative_path
        .to_str()
        .ok_or_else(|| RariError::validation("Invalid path encoding"))?
        .cow_replace('\\', "/");

    Ok(format!(
        "{}_{}",
        readable_component_id(&project_relative_path),
        short_hash(&project_relative_path)
    ))
}

#[expect(clippy::missing_errors_doc)]
pub fn get_dist_path_for_component(file_path: &str) -> Result<PathBuf, RariError> {
    let component_id = extract_component_id(file_path)?;

    let dist_path = Path::new(DIST_DIR).join("server").join(format!("{component_id}.js"));

    Ok(dist_path)
}

#[cfg(test)]
#[expect(clippy::unwrap_used, clippy::expect_used)]
mod analysis_golden_tests {
    use std::{fs, path::PathBuf};

    use serde::Deserialize;

    use super::*;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/analysis").join(name)
    }

    #[derive(Debug, Deserialize)]
    struct ComponentIdFixture {
        cases: Vec<ComponentIdCase>,
    }

    #[derive(Debug, Deserialize)]
    struct ComponentIdCase {
        input: String,
        readable: String,
        id: String,
    }

    #[derive(Debug, Deserialize)]
    struct DirectiveFixture {
        cases: Vec<DirectiveCase>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[expect(clippy::struct_excessive_bools)]
    struct DirectiveCase {
        id: String,
        source: String,
        has_use_client: bool,
        has_use_server: bool,
        top_level_use_client: bool,
        top_level_use_server: bool,
    }

    #[test]
    fn component_ids_match_shared_goldens() {
        let fixture: ComponentIdFixture = serde_json::from_str(
            &fs::read_to_string(fixture_path("component-ids.json")).expect("read fixture"),
        )
        .expect("parse fixture");

        for case in fixture.cases {
            assert_eq!(
                readable_component_id(&case.input),
                case.readable,
                "readable mismatch for {}",
                case.input
            );
            assert_eq!(
                extract_component_id(&case.input).unwrap(),
                case.id,
                "id mismatch for {}",
                case.input
            );
            assert_eq!(
                short_hash(&case.input),
                case.id.rsplit_once('_').expect("id has hash").1,
                "hash mismatch for {}",
                case.input
            );
        }
    }

    #[test]
    fn directives_match_shared_goldens() {
        let fixture: DirectiveFixture = serde_json::from_str(
            &fs::read_to_string(fixture_path("directives.json")).expect("read fixture"),
        )
        .expect("parse fixture");

        for case in fixture.cases {
            assert_eq!(
                has_use_client_directive(&case.source),
                case.has_use_client,
                "hasUseClient mismatch for {}",
                case.id
            );
            assert_eq!(
                has_use_server_directive(&case.source),
                case.has_use_server,
                "hasUseServer mismatch for {}",
                case.id
            );
            assert_eq!(
                has_top_level_use_client_directive(&case.source),
                case.top_level_use_client,
                "topLevelUseClient mismatch for {}",
                case.id
            );
            assert_eq!(
                has_top_level_use_server_directive(&case.source),
                case.top_level_use_server,
                "topLevelUseServer mismatch for {}",
                case.id
            );
        }
    }
}
