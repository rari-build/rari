use cow_utils::CowUtils;

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

        if trimmed == "'use server';"
            || trimmed == "\"use server\";"
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
        r#"
if (!globalThis.{module_key}) {{
    globalThis.{module_key} = true;
    {code}
}}
"#,
        module_key = module_key,
        code = code
    )
}

pub fn extract_component_id(
    file_path: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path = std::path::Path::new(file_path);

    let relative_path = if path.is_absolute() {
        let components: Vec<_> = path.components().collect();
        if let Some(src_idx) = components.iter().position(|c| c.as_os_str() == "src") {
            let after_src: std::path::PathBuf = components[src_idx + 1..].iter().collect();
            after_src
        } else {
            return Err(format!("Path does not contain 'src' directory: {}", file_path).into());
        }
    } else {
        let src_dir = std::path::Path::new("src");
        if let Ok(rel) = path.strip_prefix(src_dir) {
            rel.to_path_buf()
        } else {
            path.to_path_buf()
        }
    };

    let component_id = relative_path
        .to_str()
        .ok_or("Invalid path encoding")?
        .trim_end_matches(".tsx")
        .trim_end_matches(".ts")
        .trim_end_matches(".jsx")
        .trim_end_matches(".js")
        .cow_replace('\\', "/");

    Ok(component_id.into_owned())
}

pub fn get_dist_path_for_component(
    file_path: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let component_id = extract_component_id(file_path)?;

    let dist_path =
        std::path::Path::new("dist").join("server").join(format!("{}.js", component_id));

    Ok(dist_path)
}
