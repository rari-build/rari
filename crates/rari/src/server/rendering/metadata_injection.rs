use crate::rsc::rendering::layout::types::PageMetadata;

pub fn inject_metadata(html: &str, metadata: &PageMetadata) -> String {
    tracing::debug!(
        "inject_metadata called with title={:?}, description={:?}",
        metadata.title,
        metadata.description
    );
    let mut result = html.to_string();

    if let Some(title) = &metadata.title {
        if let Some(title_start) = result.find("<title>") {
            if let Some(title_end_rel) = result[title_start..].find("</title>") {
                let title_end_abs = title_start + title_end_rel + "</title>".len();
                tracing::debug!(
                    "Replacing title at position {}..{} with: {}",
                    title_start,
                    title_end_abs,
                    title
                );
                result.replace_range(
                    title_start..title_end_abs,
                    &format!("<title>{}</title>", escape_html(title)),
                );
                tracing::debug!("Title replaced successfully");
            }
        } else {
            tracing::warn!("No <title> tag found in HTML to replace");
        }
    } else {
        tracing::debug!("No title in metadata to inject");
    }

    if let Some(head_end) = result.find("</head>") {
        let mut meta_tags = String::new();

        if let Some(title) = &metadata.title
            && !result.contains("<title>")
        {
            meta_tags.push_str(&format!("    <title>{}</title>\n", escape_html(title)));
        }

        if let Some(description) = &metadata.description {
            meta_tags.push_str(&format!(
                r#"    <meta name="description" content="{}" />
"#,
                escape_html(description)
            ));
        }

        if let Some(keywords) = &metadata.keywords
            && !keywords.is_empty()
        {
            let keywords_str =
                keywords.iter().map(|k| escape_html(k)).collect::<Vec<_>>().join(", ");
            meta_tags.push_str(&format!(
                r#"    <meta name="keywords" content="{}" />
"#,
                keywords_str
            ));
        }

        if let Some(viewport) = &metadata.viewport
            && !result.contains(r#"<meta name="viewport""#)
        {
            meta_tags.push_str(&format!(
                r#"    <meta name="viewport" content="{}" />
"#,
                escape_html(viewport)
            ));
        }

        if let Some(canonical) = &metadata.canonical {
            meta_tags.push_str(&format!(
                r#"    <link rel="canonical" href="{}" />
"#,
                escape_html(canonical)
            ));
        }

        if let Some(robots) = &metadata.robots {
            let mut robots_content = Vec::new();
            if let Some(index) = robots.index {
                robots_content.push(if index { "index" } else { "noindex" });
            }
            if let Some(follow) = robots.follow {
                robots_content.push(if follow { "follow" } else { "nofollow" });
            }
            if let Some(nocache) = robots.nocache
                && nocache
            {
                robots_content.push("nocache");
            }
            if !robots_content.is_empty() {
                meta_tags.push_str(&format!(
                    r#"    <meta name="robots" content="{}" />
"#,
                    robots_content.join(", ")
                ));
            }
        }

        if let Some(og) = &metadata.open_graph {
            if let Some(og_title) = &og.title {
                meta_tags.push_str(&format!(
                    r#"    <meta property="og:title" content="{}" />
"#,
                    escape_html(og_title)
                ));
            }
            if let Some(og_description) = &og.description {
                meta_tags.push_str(&format!(
                    r#"    <meta property="og:description" content="{}" />
"#,
                    escape_html(og_description)
                ));
            }
            if let Some(og_url) = &og.url {
                meta_tags.push_str(&format!(
                    r#"    <meta property="og:url" content="{}" />
"#,
                    escape_html(og_url)
                ));
            }
            if let Some(og_site_name) = &og.site_name {
                meta_tags.push_str(&format!(
                    r#"    <meta property="og:site_name" content="{}" />
"#,
                    escape_html(og_site_name)
                ));
            }
            if let Some(og_type) = &og.og_type {
                meta_tags.push_str(&format!(
                    r#"    <meta property="og:type" content="{}" />
"#,
                    escape_html(og_type)
                ));
            }
            if let Some(images) = &og.images {
                for image in images {
                    meta_tags.push_str(&format!(
                        r#"    <meta property="og:image" content="{}" />
"#,
                        escape_html(image)
                    ));
                }
            }
        }

        if let Some(twitter) = &metadata.twitter {
            if let Some(card) = &twitter.card {
                meta_tags.push_str(&format!(
                    r#"    <meta name="twitter:card" content="{}" />
"#,
                    escape_html(card)
                ));
            }
            if let Some(site) = &twitter.site {
                meta_tags.push_str(&format!(
                    r#"    <meta name="twitter:site" content="{}" />
"#,
                    escape_html(site)
                ));
            }
            if let Some(creator) = &twitter.creator {
                meta_tags.push_str(&format!(
                    r#"    <meta name="twitter:creator" content="{}" />
"#,
                    escape_html(creator)
                ));
            }
            if let Some(twitter_title) = &twitter.title {
                meta_tags.push_str(&format!(
                    r#"    <meta name="twitter:title" content="{}" />
"#,
                    escape_html(twitter_title)
                ));
            }
            if let Some(twitter_description) = &twitter.description {
                meta_tags.push_str(&format!(
                    r#"    <meta name="twitter:description" content="{}" />
"#,
                    escape_html(twitter_description)
                ));
            }
            if let Some(images) = &twitter.images {
                for image in images {
                    meta_tags.push_str(&format!(
                        r#"    <meta name="twitter:image" content="{}" />
"#,
                        escape_html(image)
                    ));
                }
            }
        }

        if !meta_tags.is_empty() {
            result.insert_str(head_end, &meta_tags);
        }
    }

    result
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rsc::rendering::layout::types::{
        OpenGraphMetadata, RobotsMetadata, TwitterMetadata,
    };

    #[test]
    fn test_inject_basic_metadata() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Default Title</title>
</head>
<body></body>
</html>"#;

        let metadata = PageMetadata {
            title: Some("Test Page".to_string()),
            description: Some("Test description".to_string()),
            keywords: Some(vec!["test".to_string(), "page".to_string()]),
            open_graph: None,
            twitter: None,
            robots: None,
            viewport: None,
            canonical: None,
        };

        let result = inject_metadata(html, &metadata);

        assert!(result.contains("<title>Test Page</title>"));
        assert!(result.contains(r#"<meta name="description" content="Test description" />"#));
        assert!(result.contains(r#"<meta name="keywords" content="test, page" />"#));
    }

    #[test]
    fn test_inject_open_graph() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>"#;

        let metadata = PageMetadata {
            title: Some("Test".to_string()),
            description: None,
            keywords: None,
            open_graph: Some(OpenGraphMetadata {
                title: Some("OG Title".to_string()),
                description: Some("OG Description".to_string()),
                url: Some("https://example.com".to_string()),
                site_name: Some("Example Site".to_string()),
                images: Some(vec!["https://example.com/image.jpg".to_string()]),
                og_type: Some("website".to_string()),
            }),
            twitter: None,
            robots: None,
            viewport: None,
            canonical: None,
        };

        let result = inject_metadata(html, &metadata);

        assert!(result.contains(r#"<meta property="og:title" content="OG Title" />"#));
        assert!(result.contains(r#"<meta property="og:description" content="OG Description" />"#));
        assert!(result.contains(r#"<meta property="og:url" content="https://example.com" />"#));
        assert!(result.contains(r#"<meta property="og:site_name" content="Example Site" />"#));
        assert!(result.contains(r#"<meta property="og:type" content="website" />"#));
        assert!(
            result.contains(
                r#"<meta property="og:image" content="https://example.com/image.jpg" />"#
            )
        );
    }

    #[test]
    fn test_inject_twitter_metadata() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>"#;

        let metadata = PageMetadata {
            title: Some("Test".to_string()),
            description: None,
            keywords: None,
            open_graph: None,
            twitter: Some(TwitterMetadata {
                card: Some("summary_large_image".to_string()),
                site: Some("@example".to_string()),
                creator: Some("@creator".to_string()),
                title: Some("Twitter Title".to_string()),
                description: Some("Twitter Description".to_string()),
                images: Some(vec!["https://example.com/twitter.jpg".to_string()]),
            }),
            robots: None,
            viewport: None,
            canonical: None,
        };

        let result = inject_metadata(html, &metadata);

        assert!(result.contains(r#"<meta name="twitter:card" content="summary_large_image" />"#));
        assert!(result.contains(r#"<meta name="twitter:site" content="@example" />"#));
        assert!(result.contains(r#"<meta name="twitter:creator" content="@creator" />"#));
        assert!(result.contains(r#"<meta name="twitter:title" content="Twitter Title" />"#));
        assert!(
            result.contains(r#"<meta name="twitter:description" content="Twitter Description" />"#)
        );
        assert!(result.contains(
            r#"<meta name="twitter:image" content="https://example.com/twitter.jpg" />"#
        ));
    }

    #[test]
    fn test_inject_robots() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>"#;

        let metadata = PageMetadata {
            title: Some("Test".to_string()),
            description: None,
            keywords: None,
            open_graph: None,
            twitter: None,
            robots: Some(RobotsMetadata {
                index: Some(false),
                follow: Some(true),
                nocache: Some(true),
            }),
            viewport: None,
            canonical: None,
        };

        let result = inject_metadata(html, &metadata);

        assert!(result.contains(r#"<meta name="robots" content="noindex, follow, nocache" />"#));
    }

    #[test]
    fn test_escape_html() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>"#;

        let metadata = PageMetadata {
            title: Some("Test & <script>alert('xss')</script>".to_string()),
            description: Some("Description with \"quotes\" and 'apostrophes'".to_string()),
            keywords: None,
            open_graph: None,
            twitter: None,
            robots: None,
            viewport: None,
            canonical: None,
        };

        let result = inject_metadata(html, &metadata);

        assert!(result.contains("Test &amp; &lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"));
        assert!(
            result.contains(r#"Description with &quot;quotes&quot; and &#x27;apostrophes&#x27;"#)
        );
    }
}
