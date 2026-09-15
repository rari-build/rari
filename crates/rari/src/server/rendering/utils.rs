use std::fmt::Write;

use axum::http::StatusCode;
use cow_utils::CowUtils;
use tokio::fs;

use crate::server::config::Config;

fn dev_client_head(vite_port: u16) -> String {
    format!(
        r#"<script type="module" src="http://localhost:{vite_port}/@vite/client"></script>
<script type="module">
import 'http://localhost:{vite_port}/@id/virtual:rari-entry-client';
</script>
"#
    )
}

async fn load_client_head(config: &Config) -> Option<String> {
    if config.is_development() {
        return Some(dev_client_head(config.vite.port));
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

fn inject_tags_before_head_close(html: &str, tags: &str) -> String {
    let tags = tags.trim();
    if tags.is_empty() {
        return html.to_string();
    }

    let block = format!("{tags}\n");
    if let Some(head_end) = html.find("</head>") {
        let mut result = String::with_capacity(html.len() + block.len());
        result.push_str(&html[..head_end]);
        result.push_str(&block);
        result.push_str(&html[head_end..]);
        result
    } else {
        format!("{block}{html}")
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

    let mut final_html = inject_tags_before_head_close(html, &client_head);

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

    if let Some(head_end) = html.find("</head>") {
        let mut result = String::new();
        result.push_str(&html[..head_end]);
        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
        write!(
            result,
            r#"<script type="module" src="http://localhost:{vite_port}/@vite/client"></script>
<script type="module">
import 'http://localhost:{vite_port}/@id/virtual:rari-entry-client';
</script>
"#
        )
        .unwrap();
        result.push_str(&html[head_end..]);
        return result;
    }

    if let Some(body_end) = html.find("</body>") {
        let mut result = String::new();
        result.push_str(&html[..body_end]);
        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
        write!(
            result,
            r#"<script type="module" src="http://localhost:{vite_port}/@vite/client"></script>
<script type="module">
import 'http://localhost:{vite_port}/@id/virtual:rari-entry-client';
</script>
"#
        )
        .unwrap();
        result.push_str(&html[body_end..]);
        return result;
    }

    format!(
        r#"<script type="module" src="http://localhost:{vite_port}/@vite/client"></script>
<script type="module">
import 'http://localhost:{vite_port}/@id/virtual:rari-entry-client';
</script>
{html}"#
    )
}
