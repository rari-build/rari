use crate::server::config::Config;
use axum::http::StatusCode;
use tracing::error;

pub async fn extract_asset_links_from_index_html() -> Option<String> {
    use tokio::fs;

    let possible_paths = vec!["dist/index.html", "build/index.html"];

    for path in possible_paths {
        if let Ok(content) = fs::read_to_string(path).await {
            let mut asset_links = String::new();

            for line in content.lines() {
                let trimmed = line.trim();
                if (trimmed.starts_with("<script") && trimmed.contains("/assets/"))
                    || (trimmed.starts_with("<link") && trimmed.contains("/assets/"))
                {
                    asset_links.push_str("    ");
                    asset_links.push_str(trimmed);
                    asset_links.push('\n');
                }
            }

            if !asset_links.is_empty() {
                return Some(asset_links);
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
    let has_doctype_or_html = trimmed.starts_with("<!DOCTYPE") || trimmed.starts_with("<html");
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
            if html.trim_start().starts_with("<!DOCTYPE") {
                return Ok(html.to_string());
            }
            return Ok(format!("<!DOCTYPE html>\n{}", html));
        }
    };

    let mut asset_tags = Vec::new();
    for line in template.lines() {
        let trimmed = line.trim();
        if (trimmed.contains("<link") && trimmed.contains("stylesheet") && trimmed.contains("href"))
            || (trimmed.contains("<script") && trimmed.contains("src"))
        {
            let asset_signature = extract_asset_signature(trimmed);
            if !html.contains(&asset_signature) {
                asset_tags.push(trimmed.to_string());
            }
        }
    }

    if asset_tags.is_empty() {
        if html.trim_start().starts_with("<!DOCTYPE") {
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

    if !final_html.trim_start().starts_with("<!DOCTYPE") {
        final_html = format!("<!DOCTYPE html>\n{}", final_html);
    }

    let has_root_after = final_html.contains(r#"id="root""#);
    if has_root_before && !has_root_after {
        error!("Root element was lost during asset injection!");

        if html.trim_start().starts_with("<!DOCTYPE") {
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

pub fn inject_rsc_payload(html: &str, rsc_payload: &str) -> String {
    let escaped_payload = rsc_payload.replace("</script>", "<\\/script>");

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
                template.replace(
                    r#"<div id="root"></div>"#,
                    &format!(r#"<div id="root">{}</div>"#, content),
                )
            }
        } else {
            template.replace(
                r#"<div id="root"></div>"#,
                &format!(r#"<div id="root">{}</div>"#, content),
            )
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
