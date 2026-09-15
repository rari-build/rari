use axum::http::StatusCode;
use cow_utils::CowUtils;
use tokio::fs;

use crate::{rendering::r#static::RscHtmlRenderer, server::config::Config};

async fn load_client_head(config: &Config) -> Option<String> {
    if config.is_development() {
        return Some(RscHtmlRenderer::generate_dev_client_head(config.vite.port));
    }

    let path = config.public_dir().join("rari-client-head.html");
    match fs::read_to_string(&path).await {
        Ok(content) => Some(content),
        Err(_) => {
            tracing::warn!(path = %path.display(), "Client head file not found");
            None
        }
    }
}

#[expect(clippy::missing_errors_doc)]
pub async fn inject_assets_into_html(html: &str, config: &Config) -> Result<String, StatusCode> {
    let is_complete_document = is_complete_html_document(html);

    let result = if is_complete_document {
        inject_assets_into_complete_document(html, config).await
    } else {
        inject_content_into_template(html, config).await
    };

    if let Err(e) = &result {
        tracing::error!("Asset injection failed with error: {:?}", e);
    }

    result
}

fn is_complete_html_document(html: &str) -> bool {
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
    let Some(client_head) = load_client_head(config).await else {
        let trimmed_lower = html.trim_start().cow_to_lowercase();
        if trimmed_lower.starts_with("<!doctype") {
            return Ok(html.to_string());
        }
        return Ok(format!("<!DOCTYPE html>\n{html}"));
    };

    let mut final_html = RscHtmlRenderer::inject_head_tags(html, &client_head);

    let trimmed_lower = final_html.trim_start().cow_to_lowercase();
    if !trimmed_lower.starts_with("<!doctype") {
        final_html = format!("<!DOCTYPE html>\n{final_html}");
    }

    Ok(final_html)
}

async fn inject_content_into_template(
    content: &str,
    config: &Config,
) -> Result<String, StatusCode> {
    let client_head = load_client_head(config).await.unwrap_or_default();

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
{client_head}
</head>
<body>
{content}
</body>
</html>"#
    ))
}

pub fn inject_vite_client(html: &str, vite_port: u16) -> String {
    if html.contains("/@vite/client") || html.contains("@vite/client") {
        return html.to_string();
    }

    let client_head = RscHtmlRenderer::generate_dev_client_head(vite_port);

    if let Some(head_end) = html.find("</head>") {
        let mut result = String::with_capacity(html.len() + client_head.len());
        result.push_str(&html[..head_end]);
        result.push_str(&client_head);
        result.push_str(&html[head_end..]);
        return result;
    }

    if let Some(body_end) = html.find("</body>") {
        let mut result = String::with_capacity(html.len() + client_head.len());
        result.push_str(&html[..body_end]);
        result.push_str(&client_head);
        result.push_str(&html[body_end..]);
        return result;
    }

    format!("{client_head}{html}")
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use std::{
        env, fs, process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::server::config::{Config, Mode};

    #[tokio::test]
    async fn test_inject_assets_dedupes_existing_client_head_tags() {
        let public_dir = env::temp_dir().join(format!(
            "rari-client-head-dedupe-{}-{}",
            process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).expect("time").as_nanos()
        ));
        fs::create_dir_all(&public_dir).expect("temp public dir");

        let client_head = r#"<link rel="stylesheet" href="/assets/app.css" />
<script type="module" src="/assets/entry.js"></script>
"#;
        fs::write(public_dir.join("rari-client-head.html"), client_head).expect("write head");

        let mut config = Config::new(Mode::Production);
        config.static_files.prod_public_dir = public_dir.clone();

        let html = r#"<!DOCTYPE html>
<html>
<head>
<link rel="stylesheet" href="/assets/app.css" />
<script type="module" src="/assets/entry.js"></script>
</HEAD>
<body></body>
</html>"#;

        let result = inject_assets_into_html(html, &config).await.expect("inject");
        assert_eq!(
            result.matches(r#"href="/assets/app.css""#).count(),
            1,
            "stylesheet must appear once"
        );
        assert_eq!(
            result.matches(r#"src="/assets/entry.js""#).count(),
            1,
            "script must appear once"
        );
        assert!(result.contains("</HEAD>"), "must preserve original head close casing");

        let _ = fs::remove_dir_all(public_dir);
    }
}
