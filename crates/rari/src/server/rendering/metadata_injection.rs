use std::fmt::Write;

use crate::{
    rendering::{
        layout::types::{IconValue, OpenGraphImage, PageMetadata, ThemeColorMetadata},
        r#static::escape_html,
    },
    server::image::ImageOptimizer,
};

#[expect(clippy::too_many_lines)]
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
                    r#"<meta name="description" content="{}" />
"#,
                    escape_html(description)
                ),
            );
        }
    }

    if let Some(head_start) = result.find("<head")
        && let Some(head_open_end) = result[head_start..].find('>')
    {
        let byte_after_head = result.as_bytes().get(head_start + 5);
        if let Some(&b) = byte_after_head {
            if b != b'>' && !b.is_ascii_whitespace() {
                return result;
            }
        } else {
            return result;
        }

        let insert_pos = head_start + head_open_end + 1;
        let mut critical_tags = String::new();

        if !result.contains(r"<meta charset") {
            critical_tags.push_str(
                r#"
<meta charset="UTF-8" />"#,
            );
        }

        if !result.contains(r#"<meta name="viewport""#) {
            let viewport_content =
                metadata.viewport.as_deref().unwrap_or("width=device-width, initial-scale=1.0");
            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
            write!(
                critical_tags,
                r#"
<meta name="viewport" content="{}" />"#,
                escape_html(viewport_content)
            )
            .unwrap();
        }

        if let Some(title) = &metadata.title
            && !result.contains("<title>")
        {
            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
            write!(
                critical_tags,
                r"
<title>{}</title>",
                escape_html(title)
            )
            .unwrap();
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
            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
            writeln!(meta_tags, r#"<meta name="keywords" content="{keywords_str}" />"#).unwrap();
        }

        let alternates_canonical = metadata.alternates.as_ref().and_then(|a| a.canonical.as_ref());
        let effective_canonical = alternates_canonical.or(metadata.canonical.as_ref());
        if let Some(canonical) = effective_canonical {
            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
            writeln!(meta_tags, r#"<link rel="canonical" href="{}" />"#, escape_html(canonical))
                .unwrap();
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
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="robots" content="{}" />"#,
                    robots_content.join(", ")
                )
                .unwrap();
            }
        }

        if let Some(og) = &metadata.open_graph {
            if let Some(og_title) = &og.title {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta property="og:title" content="{}" />"#,
                    escape_html(og_title)
                )
                .unwrap();
            }
            if let Some(og_description) = &og.description {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta property="og:description" content="{}" />"#,
                    escape_html(og_description)
                )
                .unwrap();
            }
            if let Some(og_url) = &og.url {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta property="og:url" content="{}" />"#,
                    escape_html(og_url)
                )
                .unwrap();
            }
            if let Some(og_site_name) = &og.site_name {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta property="og:site_name" content="{}" />"#,
                    escape_html(og_site_name)
                )
                .unwrap();
            }
            if let Some(og_type) = &og.og_type {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta property="og:type" content="{}" />"#,
                    escape_html(og_type)
                )
                .unwrap();
            }
            if let Some(images) = &og.images {
                for image in images {
                    let image_url = match image {
                        OpenGraphImage::Simple(url) => url.as_str(),
                        OpenGraphImage::Detailed(desc) => desc.url.as_str(),
                    };
                    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                    writeln!(
                        meta_tags,
                        r#"<meta property="og:image" content="{}" />"#,
                        escape_html(image_url)
                    )
                    .unwrap();

                    if let OpenGraphImage::Detailed(desc) = image {
                        if let Some(width) = desc.width {
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            writeln!(
                                meta_tags,
                                r#"<meta property="og:image:width" content="{width}" />"#
                            )
                            .unwrap();
                        }
                        if let Some(height) = desc.height {
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            writeln!(
                                meta_tags,
                                r#"<meta property="og:image:height" content="{height}" />"#
                            )
                            .unwrap();
                        }
                        if let Some(alt) = &desc.alt {
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            writeln!(
                                meta_tags,
                                r#"<meta property="og:image:alt" content="{}" />"#,
                                escape_html(alt)
                            )
                            .unwrap();
                        }
                    }
                }
            }
        }

        if let Some(twitter) = &metadata.twitter {
            if let Some(card) = &twitter.card {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="twitter:card" content="{}" />"#,
                    escape_html(card)
                )
                .unwrap();
            }
            if let Some(site) = &twitter.site {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="twitter:site" content="{}" />"#,
                    escape_html(site)
                )
                .unwrap();
            }
            if let Some(creator) = &twitter.creator {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="twitter:creator" content="{}" />"#,
                    escape_html(creator)
                )
                .unwrap();
            }
            if let Some(twitter_title) = &twitter.title {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="twitter:title" content="{}" />"#,
                    escape_html(twitter_title)
                )
                .unwrap();
            }
            if let Some(twitter_description) = &twitter.description {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="twitter:description" content="{}" />"#,
                    escape_html(twitter_description)
                )
                .unwrap();
            }
            if let Some(images) = &twitter.images {
                for image in images {
                    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                    writeln!(
                        meta_tags,
                        r#"<meta name="twitter:image" content="{}" />"#,
                        escape_html(image)
                    )
                    .unwrap();
                }
            }
        }

        if let Some(icons) = &metadata.icons {
            if let Some(icon_value) = &icons.icon {
                match icon_value {
                    IconValue::Single(url) => {
                        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                        writeln!(meta_tags, r#"<link rel="icon" href="{}" />"#, escape_html(url))
                            .unwrap();
                    }
                    IconValue::Multiple(urls) => {
                        for url in urls {
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            writeln!(
                                meta_tags,
                                r#"<link rel="icon" href="{}" />"#,
                                escape_html(url)
                            )
                            .unwrap();
                        }
                    }
                    IconValue::Detailed(icon_list) => {
                        for icon in icon_list {
                            let rel = icon.rel.as_deref().unwrap_or("icon");
                            let mut attrs = format!(
                                r#"rel="{}" href="{}""#,
                                escape_html(rel),
                                escape_html(&icon.url)
                            );
                            if let Some(icon_type) = &icon.icon_type {
                                #[expect(
                                    clippy::unwrap_used,
                                    reason = "write! to String never fails"
                                )]
                                write!(&mut attrs, r#" type="{}""#, escape_html(icon_type))
                                    .unwrap();
                            }
                            if let Some(sizes) = &icon.sizes {
                                #[expect(
                                    clippy::unwrap_used,
                                    reason = "write! to String never fails"
                                )]
                                write!(&mut attrs, r#" sizes="{}""#, escape_html(sizes)).unwrap();
                            }
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            writeln!(&mut meta_tags, "<link {attrs} />").unwrap();
                        }
                    }
                }
            }
            if let Some(apple_value) = &icons.apple {
                match apple_value {
                    IconValue::Single(url) => {
                        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                        writeln!(
                            meta_tags,
                            r#"<link rel="apple-touch-icon" href="{}" />"#,
                            escape_html(url)
                        )
                        .unwrap();
                    }
                    IconValue::Multiple(urls) => {
                        for url in urls {
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            writeln!(
                                meta_tags,
                                r#"<link rel="apple-touch-icon" href="{}" />"#,
                                escape_html(url)
                            )
                            .unwrap();
                        }
                    }
                    IconValue::Detailed(apple_list) => {
                        for icon in apple_list {
                            let rel = icon.rel.as_deref().unwrap_or("apple-touch-icon");
                            let mut attrs = format!(
                                r#"rel="{}" href="{}""#,
                                escape_html(rel),
                                escape_html(&icon.url)
                            );
                            if let Some(sizes) = &icon.sizes {
                                #[expect(
                                    clippy::unwrap_used,
                                    reason = "write! to String never fails"
                                )]
                                write!(&mut attrs, r#" sizes="{}""#, escape_html(sizes)).unwrap();
                            }
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            writeln!(&mut meta_tags, "<link {attrs} />").unwrap();
                        }
                    }
                }
            }
            if let Some(other_list) = &icons.other {
                for icon in other_list {
                    let rel = icon.rel.as_deref().unwrap_or("icon");
                    let mut attrs =
                        format!(r#"rel="{}" href="{}""#, escape_html(rel), escape_html(&icon.url));
                    if let Some(icon_type) = &icon.icon_type {
                        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                        write!(&mut attrs, r#" type="{}""#, escape_html(icon_type)).unwrap();
                    }
                    if let Some(sizes) = &icon.sizes {
                        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                        write!(&mut attrs, r#" sizes="{}""#, escape_html(sizes)).unwrap();
                    }
                    if let Some(color) = &icon.color {
                        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                        write!(&mut attrs, r#" color="{}""#, escape_html(color)).unwrap();
                    }
                    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                    writeln!(&mut meta_tags, "<link {attrs} />").unwrap();
                }
            }
        }

        if let Some(manifest) = &metadata.manifest {
            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
            writeln!(meta_tags, r#"<link rel="manifest" href="{}" />"#, escape_html(manifest))
                .unwrap();
        }

        if let Some(theme_color) = &metadata.theme_color {
            match theme_color {
                ThemeColorMetadata::Simple(color) => {
                    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                    writeln!(
                        meta_tags,
                        r#"<meta name="theme-color" content="{}" />"#,
                        escape_html(color)
                    )
                    .unwrap();
                }
                ThemeColorMetadata::Detailed(colors) => {
                    for color_desc in colors {
                        let mut attrs = format!(
                            r#"name="theme-color" content="{}""#,
                            escape_html(&color_desc.color)
                        );
                        if let Some(media) = &color_desc.media {
                            #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                            write!(&mut attrs, r#" media="{}""#, escape_html(media)).unwrap();
                        }
                        #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                        writeln!(&mut meta_tags, "<meta {attrs} />").unwrap();
                    }
                }
            }
        }

        if let Some(apple_web_app) = &metadata.apple_web_app {
            if let Some(title) = &apple_web_app.title {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="apple-mobile-web-app-title" content="{}" />"#,
                    escape_html(title)
                )
                .unwrap();
            }
            if let Some(status_bar_style) = &apple_web_app.status_bar_style {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="apple-mobile-web-app-status-bar-style" content="{}" />"#,
                    escape_html(status_bar_style)
                )
                .unwrap();
            }
            if let Some(capable) = apple_web_app.capable {
                #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                writeln!(
                    meta_tags,
                    r#"<meta name="mobile-web-app-capable" content="{}" />"#,
                    if capable { "yes" } else { "no" }
                )
                .unwrap();
            }
        }

        if let Some(alternates) = &metadata.alternates {
            if let Some(languages) = &alternates.languages {
                for (lang, url) in languages {
                    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                    writeln!(
                        meta_tags,
                        r#"<link rel="alternate" hreflang="{}" href="{}" />"#,
                        escape_html(lang),
                        escape_html(url)
                    )
                    .unwrap();
                }
            }
            if let Some(types) = &alternates.types {
                for (media_type, url) in types {
                    let title = url
                        .rsplit('/')
                        .next()
                        .and_then(|f| f.strip_suffix(".xml"))
                        .unwrap_or("Feed");
                    #[expect(clippy::unwrap_used, reason = "write! to String never fails")]
                    writeln!(
                        meta_tags,
                        r#"<link rel="alternate" type="{}" href="{}" title="{}" />"#,
                        escape_html(media_type),
                        escape_html(url),
                        escape_html(title)
                    )
                    .unwrap();
                }
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
                preload_html.push_str(&link);
                preload_html.push('\n');
            }
            result.insert_str(head_end, &preload_html);
        }
    }

    result
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;
    use crate::rendering::layout::types::{
        AlternatesMetadata, OpenGraphImage, OpenGraphImageDescriptor, OpenGraphMetadata,
        RobotsMetadata, TwitterMetadata,
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
            alternates: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains("<title>Test Page</title>"));
        assert!(result.contains(r#"<meta name="description" content="Test description" />"#));
        assert!(result.contains(r#"<meta name="keywords" content="test, page" />"#));
    }

    #[test]
    fn test_inject_open_graph() {
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

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
            alternates: None,
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
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

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
            alternates: None,
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
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

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
            alternates: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains(r#"<meta name="robots" content="noindex, follow, nocache" />"#));
    }

    #[test]
    fn test_escape_html() {
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

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
            alternates: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains("Test &amp; &lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"));
        assert!(result.contains(r"Description with &quot;quotes&quot; and &#39;apostrophes&#39;"));
    }

    #[test]
    fn test_inject_default_meta_tags() {
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

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
            alternates: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains(r#"<meta charset="UTF-8" />"#));
        assert!(result.contains(
            r#"<meta name="viewport" content="width=device-width, initial-scale=1.0" />"#
        ));

        let charset_pos =
            result.find(r#"<meta charset="UTF-8" />"#).expect("charset meta tag should be present");
        let viewport_pos = result
            .find(r#"<meta name="viewport" content="width=device-width, initial-scale=1.0" />"#)
            .expect("viewport meta tag should be present");
        let title_pos = result.find("<title>").expect("title tag should be present");

        assert!(charset_pos < title_pos, "charset meta tag should appear before title");
        assert!(viewport_pos < title_pos, "viewport meta tag should appear before title");
        assert!(charset_pos < viewport_pos, "charset should appear before viewport");
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
            alternates: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert_eq!(result.matches(r"<meta charset").count(), 1);
        assert_eq!(result.matches(r#"<meta name="viewport""#).count(), 1);
    }

    #[test]
    fn test_custom_viewport_overrides_default() {
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

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
            alternates: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(
            result.contains(r#"<meta name="viewport" content="width=1024, initial-scale=1.0" />"#)
        );
        assert!(!result.contains(
            r#"<meta name="viewport" content="width=device-width, initial-scale=1.0" />"#
        ));
    }

    #[test]
    fn test_no_injection_into_header_tag() {
        let html = r"<!DOCTYPE html>
<html>
<header>
    <title>This is not a head tag</title>
</header>
<body></body>
</html>";

        let metadata = PageMetadata {
            title: Some("Test".to_string()),
            description: Some("Test description".to_string()),
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
            alternates: None,
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(!result.contains(r#"<meta charset="UTF-8" />"#));
        assert!(!result.contains(r#"<meta name="viewport""#));
        assert!(result.contains("<title>Test</title>"));
    }

    #[test]
    fn test_inject_alternates_rss_feed() {
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

        let mut types = FxHashMap::default();
        types.insert("application/rss+xml".to_string(), "https://example.com/feed.xml".to_string());

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
            alternates: Some(AlternatesMetadata {
                canonical: None,
                languages: None,
                types: Some(types),
            }),
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains(
            r#"<link rel="alternate" type="application/rss+xml" href="https://example.com/feed.xml" title="feed" />"#
        ));
    }

    #[test]
    fn test_inject_alternates_languages() {
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

        let mut languages = FxHashMap::default();
        languages.insert("en".to_string(), "https://example.com/en".to_string());
        languages.insert("es".to_string(), "https://example.com/es".to_string());

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
            alternates: Some(AlternatesMetadata {
                canonical: Some("https://example.com".to_string()),
                languages: Some(languages),
                types: None,
            }),
        };

        let result = inject_metadata(html, &metadata, None);

        assert!(result.contains(r#"<link rel="canonical" href="https://example.com" />"#));
        assert!(
            result.contains(
                r#"<link rel="alternate" hreflang="en" href="https://example.com/en" />"#
            )
        );
        assert!(
            result.contains(
                r#"<link rel="alternate" hreflang="es" href="https://example.com/es" />"#
            )
        );
    }

    #[test]
    fn test_no_duplicate_canonical_when_both_set() {
        let html = r"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>";

        let metadata = PageMetadata {
            title: Some("Test".to_string()),
            description: None,
            keywords: None,
            open_graph: None,
            twitter: None,
            robots: None,
            viewport: None,
            canonical: Some("https://example.com/old".to_string()),
            icons: None,
            manifest: None,
            theme_color: None,
            apple_web_app: None,
            alternates: Some(AlternatesMetadata {
                canonical: Some("https://example.com/preferred".to_string()),
                languages: None,
                types: None,
            }),
        };

        let result = inject_metadata(html, &metadata, None);

        assert_eq!(result.matches(r#"rel="canonical""#).count(), 1);
        assert!(result.contains(r#"href="https://example.com/preferred""#));
        assert!(!result.contains(r#"href="https://example.com/old""#));
    }
}
