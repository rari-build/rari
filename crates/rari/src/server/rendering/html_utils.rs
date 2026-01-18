use crate::server::config::Config;
use axum::http::StatusCode;
use cow_utils::CowUtils;
use tracing::error;

pub async fn extract_asset_links_from_index_html() -> Option<String> {
    use tokio::fs;

    let possible_paths = vec!["dist/index.html", "build/index.html"];

    for path in possible_paths {
        if let Ok(content) = fs::read_to_string(path).await {
            let mut asset_links = String::new();
            let mut in_inline_script = false;
            let mut in_body = false;
            let mut script_base_indent = 0;
            let mut is_first_line = true;

            for line in content.lines() {
                let trimmed = line.trim();

                if trimmed.starts_with("<body") {
                    in_body = true;
                    continue;
                } else if trimmed.starts_with("</body>") {
                    in_body = false;
                    continue;
                }

                if in_body && !in_inline_script {
                    continue;
                }

                if (trimmed.starts_with("<script") && trimmed.contains("/assets/"))
                    || (trimmed.starts_with("<link") && trimmed.contains("/assets/"))
                {
                    if !is_first_line {
                        asset_links.push_str("    ");
                    }
                    asset_links.push_str(trimmed);
                    asset_links.push('\n');
                    is_first_line = false;
                    continue;
                }

                if trimmed.starts_with("<script") && !trimmed.contains("src=") && !in_body {
                    in_inline_script = true;

                    script_base_indent = line.len() - line.trim_start().len();
                    if !is_first_line {
                        asset_links.push_str("    ");
                    }
                    asset_links.push_str(trimmed);
                    asset_links.push('\n');
                    is_first_line = false;

                    if trimmed.contains("</script>") {
                        in_inline_script = false;
                    }
                    continue;
                }

                if in_inline_script {
                    let current_indent = line.len() - line.trim_start().len();
                    if current_indent >= script_base_indent {
                        let relative_indent = current_indent - script_base_indent;
                        asset_links.push_str("    ");
                        asset_links.push_str(&" ".repeat(relative_indent));
                        asset_links.push_str(trimmed);
                    } else {
                        asset_links.push_str("    ");
                        asset_links.push_str(trimmed);
                    }
                    asset_links.push('\n');
                    is_first_line = false;

                    if trimmed.contains("</script>") {
                        in_inline_script = false;
                    }
                    continue;
                }

                if trimmed.starts_with("<link")
                    && (trimmed.contains("preconnect") || trimmed.contains("dns-prefetch"))
                {
                    if !is_first_line {
                        asset_links.push_str("    ");
                    }
                    asset_links.push_str(trimmed);
                    asset_links.push('\n');
                    is_first_line = false;
                }
            }

            if !asset_links.is_empty() {
                return Some(asset_links);
            }
        }
    }

    None
}

pub async fn extract_body_scripts_from_index_html() -> Option<String> {
    use tokio::fs;

    let possible_paths = vec!["dist/index.html", "build/index.html"];

    for path in possible_paths {
        if let Ok(content) = fs::read_to_string(path).await {
            let mut body_scripts = String::new();
            let mut in_inline_script = false;
            let mut in_body = false;
            let mut script_base_indent = 0;
            let mut is_first_line = true;

            for line in content.lines() {
                let trimmed = line.trim();

                if trimmed.starts_with("<body") {
                    in_body = true;
                    continue;
                } else if trimmed.starts_with("</body>") {
                    in_body = false;
                    continue;
                }

                if !in_body && !in_inline_script {
                    continue;
                }

                if trimmed.starts_with("<script") && !trimmed.contains("src=") && in_body {
                    in_inline_script = true;

                    script_base_indent = line.len() - line.trim_start().len();
                    if !is_first_line {
                        body_scripts.push_str("    ");
                    }
                    body_scripts.push_str(trimmed);
                    body_scripts.push('\n');
                    is_first_line = false;

                    if trimmed.contains("</script>") {
                        in_inline_script = false;
                    }
                    continue;
                }

                if in_inline_script {
                    let current_indent = line.len() - line.trim_start().len();
                    if current_indent >= script_base_indent {
                        let relative_indent = current_indent - script_base_indent;
                        body_scripts.push_str("    ");
                        body_scripts.push_str(&" ".repeat(relative_indent));
                        body_scripts.push_str(trimmed);
                    } else {
                        body_scripts.push_str("    ");
                        body_scripts.push_str(trimmed);
                    }
                    body_scripts.push('\n');
                    is_first_line = false;

                    if trimmed.contains("</script>") {
                        in_inline_script = false;
                    }
                    continue;
                }
            }

            if !body_scripts.is_empty() {
                return Some(body_scripts);
            }
        }
    }

    None
}

pub async fn inject_assets_into_html(html: &str, config: &Config) -> Result<String, StatusCode> {
    let has_root_before = html.contains(r#"id="root""#);
    let is_complete_document = is_complete_html_document(html);

    let result = if is_complete_document {
        inject_assets_into_complete_document(html, config).await
    } else {
        inject_content_into_template(html, config).await
    };

    match &result {
        Ok(final_html) => {
            let has_root_after = final_html.contains(r#"id="root""#);

            if has_root_before && !has_root_after {
                error!("CRITICAL: Root element was LOST during asset injection!");
                error!("This will cause hydration to fail in the browser.");

                let recovered_html = if html.trim_start().starts_with("<!DOCTYPE") {
                    html.to_string()
                } else {
                    format!("<!DOCTYPE html>\n{}", html)
                };

                return Ok(recovered_html);
            }
        }
        Err(e) => {
            error!("Asset injection failed with error: {:?}", e);
        }
    }

    result
}

pub fn is_complete_html_document(html: &str) -> bool {
    let trimmed = html.trim_start();
    let trimmed_lower = trimmed.cow_to_lowercase();
    let has_doctype_or_html =
        trimmed_lower.starts_with("<!doctype") || trimmed_lower.starts_with("<html");
    let has_body = html.contains("<body");

    has_doctype_or_html && has_body
}

async fn inject_assets_into_complete_document(
    html: &str,
    config: &Config,
) -> Result<String, StatusCode> {
    let has_root_before = html.contains(r#"id="root""#);

    let template_path = if config.is_development() { "index.html" } else { "dist/index.html" };

    let template = match tokio::fs::read_to_string(template_path).await {
        Ok(t) => t,
        Err(_) => {
            let trimmed_lower = html.trim_start().cow_to_lowercase();
            if trimmed_lower.starts_with("<!doctype") {
                return Ok(html.to_string());
            }
            return Ok(format!("<!DOCTYPE html>\n{}", html));
        }
    };

    let mut asset_tags = Vec::new();
    let mut head_content = Vec::new();
    let mut body_content = Vec::new();
    let mut in_head = false;
    let mut in_body = false;

    for line in template.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("<head") {
            in_head = true;
        } else if trimmed.starts_with("</head>") {
            in_head = false;
        } else if trimmed.starts_with("<body") {
            in_body = true;
        } else if trimmed.starts_with("</body>") {
            in_body = false;
        }

        if (trimmed.contains("<link") && trimmed.contains("stylesheet") && trimmed.contains("href"))
            || (trimmed.contains("<script") && trimmed.contains("src"))
        {
            let asset_signature = extract_asset_signature(trimmed);
            if !html.contains(&asset_signature) {
                asset_tags.push(trimmed.to_string());
            }
        } else if trimmed.contains("<script")
            && !trimmed.contains("src")
            && !trimmed.contains("type=\"module\"")
        {
            if !html.contains(trimmed) {
                if in_body {
                    body_content.push(trimmed.to_string());
                } else if in_head {
                    head_content.push(trimmed.to_string());
                }
            }
        } else if ((trimmed.contains("<link") && !trimmed.contains("stylesheet"))
            || trimmed.contains("<meta") && trimmed.contains("name="))
            && !html.contains(trimmed)
        {
            head_content.push(trimmed.to_string());
        }
    }

    let inline_scripts = extract_inline_scripts_with_location(&template);
    for (script, is_in_body) in inline_scripts {
        if !html.contains(&script) {
            if is_in_body {
                body_content.push(script);
            } else {
                head_content.push(script);
            }
        }
    }

    if asset_tags.is_empty() && head_content.is_empty() && body_content.is_empty() {
        let trimmed_lower = html.trim_start().cow_to_lowercase();
        if trimmed_lower.starts_with("<!doctype") {
            return Ok(html.to_string());
        }
        return Ok(format!("<!DOCTYPE html>\n{}", html));
    }

    let mut stylesheets = Vec::new();
    let mut scripts = Vec::new();

    for tag in asset_tags {
        if tag.contains("<link") && tag.contains("stylesheet") {
            stylesheets.push(tag);
        } else {
            scripts.push(tag);
        }
    }

    let mut final_html = html.to_string();

    if !head_content.is_empty() {
        let head_content_html = head_content.join("\n    ");
        if let Some(head_end) = final_html.find("</head>") {
            final_html.insert_str(head_end, &format!("    {}\n  ", head_content_html));
        }
    }

    if !stylesheets.is_empty() {
        let stylesheets_html = stylesheets.join("\n    ");
        if let Some(head_end) = final_html.find("</head>") {
            final_html.insert_str(head_end, &format!("    {}\n  ", stylesheets_html));
        }
    }

    if !scripts.is_empty() {
        let scripts_html = scripts.join("\n    ");
        if let Some(body_end) = final_html.rfind("</body>") {
            final_html.insert_str(body_end, &format!("\n    {}\n  ", scripts_html));
        }
    }

    if !body_content.is_empty() {
        let body_content_html = body_content.join("\n    ");
        if let Some(body_end) = final_html.rfind("</body>") {
            final_html.insert_str(body_end, &format!("\n    {}\n  ", body_content_html));
        }
    }

    let trimmed_lower = final_html.trim_start().cow_to_lowercase();
    if !trimmed_lower.starts_with("<!doctype") {
        final_html = format!("<!DOCTYPE html>\n{}", final_html);
    }

    let has_root_after = final_html.contains(r#"id="root""#);
    if has_root_before && !has_root_after {
        error!("Root element was lost during asset injection!");

        let trimmed_lower = html.trim_start().cow_to_lowercase();
        if trimmed_lower.starts_with("<!doctype") {
            return Ok(html.to_string());
        }
        return Ok(format!("<!DOCTYPE html>\n{}", html));
    }

    Ok(final_html)
}

fn extract_asset_signature(asset_tag: &str) -> String {
    if asset_tag.contains("<script")
        && let Some(src_start) = asset_tag.find("src=\"")
    {
        let src_start = src_start + 5;
        if let Some(src_end) = asset_tag[src_start..].find('"') {
            return format!("src=\"{}\"", &asset_tag[src_start..src_start + src_end]);
        }
    }

    if asset_tag.contains("<link")
        && let Some(href_start) = asset_tag.find("href=\"")
    {
        let href_start = href_start + 6;
        if let Some(href_end) = asset_tag[href_start..].find('"') {
            return format!("href=\"{}\"", &asset_tag[href_start..href_start + href_end]);
        }
    }

    asset_tag.trim().to_string()
}

fn extract_inline_scripts_with_location(html: &str) -> Vec<(String, bool)> {
    let mut scripts = Vec::new();
    let mut in_script = false;
    let mut in_body = false;
    let mut current_script = String::new();
    let mut script_in_body = false;

    for line in html.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("<body") {
            in_body = true;
        } else if trimmed.starts_with("</body>") {
            in_body = false;
        }

        if trimmed.starts_with("<script")
            && !trimmed.contains("src=")
            && !trimmed.contains("type=\"module\"")
        {
            in_script = true;
            script_in_body = in_body;
            current_script.clear();
            current_script.push_str(line);
            current_script.push('\n');

            if trimmed.contains("</script>") {
                scripts.push((current_script.trim().to_string(), script_in_body));
                in_script = false;
                current_script.clear();
            }
        } else if in_script {
            current_script.push_str(line);
            current_script.push('\n');

            if trimmed.contains("</script>") {
                scripts.push((current_script.trim().to_string(), script_in_body));
                in_script = false;
                current_script.clear();
            }
        }
    }

    scripts
}

pub fn inject_rsc_payload(html: &str, rsc_payload: &str) -> String {
    let escaped_payload = rsc_payload.cow_replace("</script>", "<\\/script>");

    let script_tag = format!(
        r#"<script id="__RARI_RSC_PAYLOAD__" type="application/json">{}</script>"#,
        escaped_payload
    );

    if let Some(body_end) = html.rfind("</body>") {
        let mut result = html.to_string();
        result.insert_str(body_end, &script_tag);
        return result;
    }

    if let Some(html_end) = html.rfind("</html>") {
        let mut result = html.to_string();
        result.insert_str(html_end, &script_tag);
        return result;
    }

    format!("{}{}", html, script_tag)
}

async fn inject_content_into_template(
    content: &str,
    config: &Config,
) -> Result<String, StatusCode> {
    let template_path = if config.is_development() { "index.html" } else { "dist/index.html" };

    let template = match tokio::fs::read_to_string(template_path).await {
        Ok(t) => t,
        Err(_) => {
            return Ok(format!(
                r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
</head>
<body>
  <div id="root">{}</div>
</body>
</html>"#,
                content
            ));
        }
    };

    let final_html = if let Some(root_start) = template.find(r#"<div id="root""#) {
        if let Some(root_close) = template[root_start..].find('>') {
            let close_pos = root_start + root_close + 1;

            if let Some(root_end) = template[close_pos..].find("</div>") {
                let end_pos = close_pos + root_end;

                let mut result = String::new();
                result.push_str(&template[..close_pos]);
                result.push_str(content);
                result.push_str(&template[end_pos..]);

                result
            } else {
                template
                    .cow_replace(
                        r#"<div id="root"></div>"#,
                        &format!(r#"<div id="root">{}</div>"#, content),
                    )
                    .into_owned()
            }
        } else {
            template
                .cow_replace(
                    r#"<div id="root"></div>"#,
                    &format!(r#"<div id="root">{}</div>"#, content),
                )
                .into_owned()
        }
    } else if let Some(body_end) = template.rfind("</body>") {
        let mut result = template.clone();
        result.insert_str(body_end, &format!(r#"<div id="root">{}</div>"#, content));
        result
    } else {
        format!(
            r#"<!DOCTYPE html>
    <html>
    <head>
      <meta charset="UTF-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    </head>
    <body>
      <div id="root">{}</div>
    </body>
    </html>"#,
            content
        )
    };

    if !final_html.contains(r#"id="root""#) {
        error!("CRITICAL: Root element missing in final HTML after template injection!");
        error!("This should never happen as template injection should always create root element");

        let recovered_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
</head>
<body>
  <div id="root">{}</div>
</body>
</html>"#,
            content
        );

        return Ok(recovered_html);
    }

    Ok(final_html)
}

pub fn inject_vite_client(html: &str, vite_port: u16) -> String {
    if html.contains("/@vite/client") || html.contains("@vite/client") {
        return html.to_string();
    }

    if let Some(head_end) = html.find("</head>") {
        let mut result = String::new();
        result.push_str(&html[..head_end]);
        result.push_str(&format!(
            r#"  <script type="module" src="http://localhost:{}/@vite/client"></script>
  <script type="module">
    import 'http://localhost:{}/@id/virtual:rari-entry-client';
  </script>
"#,
            vite_port, vite_port
        ));
        result.push_str(&html[head_end..]);
        return result;
    }

    if let Some(body_end) = html.find("</body>") {
        let mut result = String::new();
        result.push_str(&html[..body_end]);
        result.push_str(&format!(
            r#"  <script type="module" src="http://localhost:{}/@vite/client"></script>
  <script type="module">
    import 'http://localhost:{}/@id/virtual:rari-entry-client';
  </script>
"#,
            vite_port, vite_port
        ));
        result.push_str(&html[body_end..]);
        return result;
    }

    format!(
        r#"<script type="module" src="http://localhost:{}/@vite/client"></script>
<script type="module">
  import 'http://localhost:{}/@id/virtual:rari-entry-client';
</script>
{}"#,
        vite_port, vite_port, html
    )
}
