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

pub fn has_use_client_directive(code: &str) -> bool {
    for line in code.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("//") {
            continue;
        }

        if trimmed.starts_with("/*") {
            continue;
        }

        if trimmed == "'use client';"
            || trimmed == "\"use client\";"
            || trimmed == "'use client'"
            || trimmed == "\"use client\""
        {
            return true;
        }

        if !trimmed.starts_with("'use") && !trimmed.starts_with("\"use") {
            break;
        }
    }

    false
}

pub fn has_use_server_directive(code: &str) -> bool {
    for line in code.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("//") {
            continue;
        }

        if trimmed.starts_with("/*") {
            continue;
        }

        if trimmed.starts_with("'use server';")
            || trimmed.starts_with("\"use server\";")
            || trimmed == "'use server'"
            || trimmed == "\"use server\""
        {
            return true;
        }

        if !trimmed.starts_with("'use") && !trimmed.starts_with("\"use") {
            break;
        }
    }

    false
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
