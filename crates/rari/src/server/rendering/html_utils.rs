use crate::server::config::Config;
use axum::http::StatusCode;
use tracing::{debug, error, warn};

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
                debug!("Extracted asset links from {}: {} bytes", path, asset_links.len());
                return Some(asset_links);
            }
        }
    }

    debug!("No index.html found or no assets extracted, using shell without assets");
    None
}

pub async fn inject_assets_into_html(html: &str, config: &Config) -> Result<String, StatusCode> {
    let has_root_before = html.contains(r#"id="root""#);

    if has_root_before {
        debug!("Root element verified before asset injection");
    } else {
        warn!("Root element NOT found before asset injection - this may cause hydration issues");
    }

    let is_complete_document = is_complete_html_document(html);

    debug!(
        "inject_assets_into_html: is_complete_document={}, has_root_before={}, html_length={}",
        is_complete_document,
        has_root_before,
        html.len()
    );

    let result = if is_complete_document {
        debug!("Routing to inject_assets_into_complete_document");
        inject_assets_into_complete_document(html, config).await
    } else {
        debug!("Content fragment - routing to template injection");
        inject_content_into_template(html, config).await
    };

    match &result {
        Ok(final_html) => {
            let has_root_after = final_html.contains(r#"id="root""#);

            if has_root_before && !has_root_after {
                error!("CRITICAL: Root element was LOST during asset injection!");
                error!("This will cause hydration to fail in the browser.");

                warn!("Attempting recovery: returning original HTML without asset injection");

                let recovered_html = if html.trim_start().starts_with("<!DOCTYPE") {
                    html.to_string()
                } else {
                    format!("<!DOCTYPE html>\n{}", html)
                };

                warn!("Recovery completed: original HTML returned to preserve root element");
                return Ok(recovered_html);
            }

            if has_root_after {
                debug!("Root element successfully preserved after asset injection");
            } else if !has_root_before {
                debug!("No root element before or after injection (content fragment path)");
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
    debug!("Injecting assets into complete HTML document");

    let has_root_before = html.contains(r#"id="root""#);
    if !has_root_before {
        warn!("Root element missing before asset injection - this may cause hydration issues");
    }

    let template_path = if config.is_development() { "index.html" } else { "dist/index.html" };

    let template = match tokio::fs::read_to_string(template_path).await {
        Ok(t) => t,
        Err(e) => {
            debug!("Could not read template file {}: {}", template_path, e);
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
                debug!("Will inject asset: {}", &trimmed[..trimmed.len().min(60)]);
            } else {
                debug!(
                    "Asset already exists in HTML, skipping: {}",
                    &trimmed[..trimmed.len().min(50)]
                );
            }
        }
    }

    if asset_tags.is_empty() {
        debug!("No new assets to inject, all assets already present");
        if html.trim_start().starts_with("<!DOCTYPE") {
            return Ok(html.to_string());
        }
        return Ok(format!("<!DOCTYPE html>\n{}", html));
    }

    let assets = asset_tags.join("\n    ");
    debug!("Injecting {} new asset tags", asset_tags.len());

    let mut final_html = html.to_string();
    if let Some(body_end) = final_html.rfind("</body>") {
        final_html.insert_str(body_end, &format!("\n    {}\n  ", assets));
        debug!("Injected assets before </body> tag at position {}", body_end);
    } else {
        warn!("No </body> tag found in complete HTML document - cannot inject assets");
    }

    if !final_html.trim_start().starts_with("<!DOCTYPE") {
        final_html = format!("<!DOCTYPE html>\n{}", final_html);
    }

    let has_root_after = final_html.contains(r#"id="root""#);
    if has_root_before && !has_root_after {
        error!("Root element was lost during asset injection!");

        warn!("Returning original HTML to preserve root element");
        if html.trim_start().starts_with("<!DOCTYPE") {
            return Ok(html.to_string());
        }
        return Ok(format!("<!DOCTYPE html>\n{}", html));
    }

    debug!("Asset injection completed successfully, root element preserved");
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
    debug!("Injecting RSC payload into HTML ({} bytes)", rsc_payload.len());

    let escaped_payload =
        rsc_payload.replace('\\', "\\\\").replace('`', "\\`").replace("</script>", "<\\/script>");

    let script_tag = format!(
        r#"<script id="__RARI_RSC_PAYLOAD__" type="application/json">{}</script>"#,
        escaped_payload
    );

    if let Some(body_end) = html.rfind("</body>") {
        let mut result = html.to_string();
        result.insert_str(body_end, &script_tag);
        debug!("RSC payload injected before </body>");
        return result;
    }

    if let Some(html_end) = html.rfind("</html>") {
        let mut result = html.to_string();
        result.insert_str(html_end, &script_tag);
        debug!("RSC payload injected before </html>");
        return result;
    }

    debug!("No </body> or </html> found, appending RSC payload to end");
    format!("{}{}", html, script_tag)
}

async fn inject_content_into_template(
    content: &str,
    config: &Config,
) -> Result<String, StatusCode> {
    debug!("Injecting content fragment into template");

    let template_path = if config.is_development() { "index.html" } else { "dist/index.html" };

    let template = match tokio::fs::read_to_string(template_path).await {
        Ok(t) => t,
        Err(e) => {
            warn!("Could not read template file {}: {}", template_path, e);
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

                debug!("Injected content into <div id=\"root\"> in template");
                result
            } else {
                warn!("Could not find closing </div> for root element in template");
                template.replace(
                    r#"<div id="root"></div>"#,
                    &format!(r#"<div id="root">{}</div>"#, content),
                )
            }
        } else {
            warn!("Malformed root div in template");
            template.replace(
                r#"<div id="root"></div>"#,
                &format!(r#"<div id="root">{}</div>"#, content),
            )
        }
    } else {
        warn!("No <div id=\"root\"> found in template, using fallback");
        if let Some(body_end) = template.rfind("</body>") {
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
        }
    };

    if !final_html.contains(r#"id="root""#) {
        error!("CRITICAL: Root element missing in final HTML after template injection!");
        error!("This should never happen as template injection should always create root element");

        warn!("Attempting recovery with fallback HTML structure");
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

        warn!("Recovery completed: fallback HTML with root element returned");
        return Ok(recovered_html);
    }

    debug!("Template injection completed successfully with root element present");
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
