use crate::rsc::rendering::layout::types::PageMetadata;
use crate::server::image::ImageOptimizer;
use cow_utils::CowUtils;

pub fn inject_metadata(
    html: &str,
    metadata: &PageMetadata,
    image_optimizer: Option<&ImageOptimizer>,
) -> String {
    let mut result = html.to_string();

    if let Some(title) = &metadata.title
        && let Some(title_start) = result.find("<title>")
        && let Some(title_end_rel) = result[title_start..].find("</title>")
    {
        let title_end_abs = title_start + title_end_rel + "</title>".len();
        result.replace_range(
            title_start..title_end_abs,
            &format!("<title>{}</title>", escape_html(title)),
        );
    }

    if let Some(description) = &metadata.description {
        if let Some(desc_start) = result.find(r#"<meta name="description""#) {
            if let Some(desc_end_rel) = result[desc_start..].find("/>") {
                let desc_end_abs = desc_start + desc_end_rel + "/>".len();
                result.replace_range(
                    desc_start..desc_end_abs,
                    &format!(
                        r#"<meta name="description" content="{}" />"#,
                        escape_html(description)
                    ),
                );
            }
        } else if let Some(head_end) = result.find("</head>") {
            result.insert_str(
                head_end,
                &format!(
                    r#"    <meta name="description" content="{}" />
"#,
                    escape_html(description)
                ),
            );
        }
    }

    if let Some(head_start) = result.find("<head")
        && let Some(head_open_end) = result[head_start..].find('>')
    {
        let insert_pos = head_start + head_open_end + 1;
        let mut critical_tags = String::new();

        if !result.contains(r#"<meta charset"#) {
            critical_tags.push_str(
                r#"
    <meta charset="UTF-8" />"#,
            );
        }

        if !result.contains(r#"<meta name="viewport""#) {
            let viewport_content =
                metadata.viewport.as_deref().unwrap_or("width=device-width, initial-scale=1.0");
            critical_tags.push_str(&format!(
                r#"
    <meta name="viewport" content="{}" />"#,
                escape_html(viewport_content)
            ));
        }

        if let Some(title) = &metadata.title
            && !result.contains("<title>")
        {
            critical_tags.push_str(&format!(
                r#"
    <title>{}</title>"#,
                escape_html(title)
            ));
        }

        if !critical_tags.is_empty() {
            result.insert_str(insert_pos, &critical_tags);
        }
    }

    if let Some(head_end) = result.find("</head>") {
        let mut meta_tags = String::new();

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
                    use crate::rsc::rendering::layout::types::OpenGraphImage;
                    let image_url = match image {
                        OpenGraphImage::Simple(url) => url.as_str(),
                        OpenGraphImage::Detailed(desc) => desc.url.as_str(),
                    };
                    meta_tags.push_str(&format!(
                        r#"    <meta property="og:image" content="{}" />
"#,
                        escape_html(image_url)
                    ));

                    if let OpenGraphImage::Detailed(desc) = image {
                        if let Some(width) = desc.width {
                            meta_tags.push_str(&format!(
                                r#"    <meta property="og:image:width" content="{}" />
"#,
                                width
                            ));
                        }
                        if let Some(height) = desc.height {
                            meta_tags.push_str(&format!(
                                r#"    <meta property="og:image:height" content="{}" />
"#,
                                height
                            ));
                        }
                        if let Some(alt) = &desc.alt {
                            meta_tags.push_str(&format!(
                                r#"    <meta property="og:image:alt" content="{}" />
"#,
                                escape_html(alt)
                            ));
                        }
                    }
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

        if let Some(icons) = &metadata.icons {
            if let Some(icon_value) = &icons.icon {
                use crate::rsc::rendering::layout::types::IconValue;
                match icon_value {
                    IconValue::Single(url) => {
                        meta_tags.push_str(&format!(
                            r#"    <link rel="icon" href="{}" />
"#,
                            escape_html(url)
                        ));
                    }
                    IconValue::Multiple(urls) => {
                        for url in urls {
                            meta_tags.push_str(&format!(
                                r#"    <link rel="icon" href="{}" />
"#,
                                escape_html(url)
                            ));
                        }
                    }
                    IconValue::Detailed(icon_list) => {
                        for icon in icon_list {
                            let rel = icon.rel.as_deref().unwrap_or("icon");
                            let mut attrs =
                                format!(r#"rel="{}" href="{}""#, rel, escape_html(&icon.url));
                            if let Some(icon_type) = &icon.icon_type {
                                attrs.push_str(&format!(r#" type="{}""#, escape_html(icon_type)));
                            }
                            if let Some(sizes) = &icon.sizes {
                                attrs.push_str(&format!(r#" sizes="{}""#, escape_html(sizes)));
                            }
                            meta_tags.push_str(&format!("    <link {} />\n", attrs));
                        }
                    }
                }
            }
            if let Some(apple_value) = &icons.apple {
                use crate::rsc::rendering::layout::types::IconValue;
                match apple_value {
                    IconValue::Single(url) => {
                        meta_tags.push_str(&format!(
                            r#"    <link rel="apple-touch-icon" href="{}" />
"#,
                            escape_html(url)
                        ));
                    }
                    IconValue::Multiple(urls) => {
                        for url in urls {
                            meta_tags.push_str(&format!(
                                r#"    <link rel="apple-touch-icon" href="{}" />
"#,
                                escape_html(url)
                            ));
                        }
                    }
                    IconValue::Detailed(apple_list) => {
                        for icon in apple_list {
                            let rel = icon.rel.as_deref().unwrap_or("apple-touch-icon");
                            let mut attrs =
                                format!(r#"rel="{}" href="{}""#, rel, escape_html(&icon.url));
                            if let Some(sizes) = &icon.sizes {
                                attrs.push_str(&format!(r#" sizes="{}""#, escape_html(sizes)));
                            }
                            meta_tags.push_str(&format!("    <link {} />\n", attrs));
                        }
                    }
                }
            }
            if let Some(other_list) = &icons.other {
                for icon in other_list {
                    let rel = icon.rel.as_deref().unwrap_or("icon");
                    let mut attrs = format!(r#"rel="{}" href="{}""#, rel, escape_html(&icon.url));
                    if let Some(icon_type) = &icon.icon_type {
                        attrs.push_str(&format!(r#" type="{}""#, escape_html(icon_type)));
                    }
                    if let Some(sizes) = &icon.sizes {
                        attrs.push_str(&format!(r#" sizes="{}""#, escape_html(sizes)));
                    }
                    if let Some(color) = &icon.color {
                        attrs.push_str(&format!(r#" color="{}""#, escape_html(color)));
                    }
                    meta_tags.push_str(&format!("    <link {} />\n", attrs));
                }
            }
        }

        if let Some(manifest) = &metadata.manifest {
            meta_tags.push_str(&format!(
                r#"    <link rel="manifest" href="{}" />
"#,
                escape_html(manifest)
            ));
        }

        if let Some(theme_color) = &metadata.theme_color {
            use crate::rsc::rendering::layout::types::ThemeColorMetadata;
            match theme_color {
                ThemeColorMetadata::Simple(color) => {
                    meta_tags.push_str(&format!(
                        r#"    <meta name="theme-color" content="{}" />
"#,
                        escape_html(color)
                    ));
                }
                ThemeColorMetadata::Detailed(colors) => {
                    for color_desc in colors {
                        let mut attrs = format!(
                            r#"name="theme-color" content="{}""#,
                            escape_html(&color_desc.color)
                        );
                        if let Some(media) = &color_desc.media {
                            attrs.push_str(&format!(r#" media="{}""#, escape_html(media)));
                        }
                        meta_tags.push_str(&format!("    <meta {} />\n", attrs));
                    }
                }
            }
        }

        if let Some(apple_web_app) = &metadata.apple_web_app {
            if let Some(title) = &apple_web_app.title {
                meta_tags.push_str(&format!(
                    r#"    <meta name="apple-mobile-web-app-title" content="{}" />
"#,
                    escape_html(title)
                ));
            }
            if let Some(status_bar_style) = &apple_web_app.status_bar_style {
                meta_tags.push_str(&format!(
                    r#"    <meta name="apple-mobile-web-app-status-bar-style" content="{}" />
"#,
                    escape_html(status_bar_style)
                ));
            }
            if let Some(capable) = apple_web_app.capable {
                meta_tags.push_str(&format!(
                    r#"    <meta name="mobile-web-app-capable" content="{}" />
"#,
                    if capable { "yes" } else { "no" }
                ));
            }
        }

        if !meta_tags.is_empty() {
            result.insert_str(head_end, &meta_tags);
        }
    }

    if let Some(optimizer) = image_optimizer
        && let Some(head_end) = result.find("</head>")
    {
        let preload_links = optimizer.get_preload_links();
        if !preload_links.is_empty() {
            let mut preload_html = String::new();
            for link in preload_links {
                preload_html.push_str("    ");
                preload_html.push_str(&link);
                preload_html.push('\n');
            }
            result.insert_str(head_end, &preload_html);
        }
    }

    result
}

fn escape_html(s: &str) -> String {
    s.cow_replace('&', "&amp;")
        .cow_replace('<', "&lt;")
        .cow_replace('>', "&gt;")
        .cow_replace('"', "&quot;")
        .cow_replace('\'', "&#x27;")
        .into_owned()
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
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
        };

        let result = inject_metadata(html, &metadata, None);

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

        use crate::rsc::rendering::layout::types::{OpenGraphImage, OpenGraphImageDescriptor};

        let metadata = PageMetadata {
            title: Some("Test".to_string()),
            description: None,
            keywords: None,
            open_graph: Some(OpenGraphMetadata {
                title: Some("OG Title".to_string()),
                description: Some("OG Description".to_string()),
                url: Some("https://example.com".to_string()),
                site_name: Some("Example Site".to_string()),
                images: Some(vec![
                    OpenGraphImage::Simple("https://example.com/simple.jpg".to_string()),
                    OpenGraphImage::Detailed(OpenGraphImageDescriptor {
                        url: "https://example.com/image.jpg".to_string(),
                        width: Some(1200),
                        height: Some(630),
                        alt: Some("Example Image".to_string()),
                    }),
                ]),
                og_type: Some("website".to_string()),
            }),
            twitter: None,
            robots: None,
            viewport: None,
            canonical: None,
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains(r#"<meta property="og:title" content="OG Title" />"#));
        assert!(result.contains(r#"<meta property="og:description" content="OG Description" />"#));
        assert!(result.contains(r#"<meta property="og:url" content="https://example.com" />"#));
        assert!(result.contains(r#"<meta property="og:site_name" content="Example Site" />"#));
        assert!(result.contains(r#"<meta property="og:type" content="website" />"#));
        assert!(
            result.contains(
                r#"<meta property="og:image" content="https://example.com/simple.jpg" />"#
            )
        );
        assert!(
            result.contains(
                r#"<meta property="og:image" content="https://example.com/image.jpg" />"#
            )
        );
        assert!(result.contains(r#"<meta property="og:image:width" content="1200" />"#));
        assert!(result.contains(r#"<meta property="og:image:height" content="630" />"#));
        assert!(result.contains(r#"<meta property="og:image:alt" content="Example Image" />"#));
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
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
        };

        let result = inject_metadata(html, &metadata, None);

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
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
        };

        let result = inject_metadata(html, &metadata, None);

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
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains("Test &amp; &lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"));
        assert!(
            result.contains(r#"Description with &quot;quotes&quot; and &#x27;apostrophes&#x27;"#)
        );
    }

    #[test]
    fn test_inject_default_meta_tags() {
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
            robots: None,
            viewport: None,
            canonical: None,
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains(r#"<meta charset="UTF-8" />"#));
        assert!(result.contains(
            r#"<meta name="viewport" content="width=device-width, initial-scale=1.0" />"#
        ));
    }

    #[test]
    fn test_no_duplicate_meta_tags() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
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
            robots: None,
            viewport: None,
            canonical: None,
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert_eq!(result.matches(r#"<meta charset"#).count(), 1);
        assert_eq!(result.matches(r#"<meta name="viewport""#).count(), 1);
    }

    #[test]
    fn test_custom_viewport_overrides_default() {
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
            robots: None,
            viewport: Some("width=1024, initial-scale=1.0".to_string()),
            canonical: None,
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(
            result.contains(r#"<meta name="viewport" content="width=1024, initial-scale=1.0" />"#)
        );
        assert!(!result.contains(
            r#"<meta name="viewport" content="width=device-width, initial-scale=1.0" />"#
        ));
    }
}
