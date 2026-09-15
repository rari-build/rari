use crate::{
    rendering::layout::{IconDescriptor, IconValue, IconsMetadata, PageMetadata},
    server::routing::AppIconEntry,
};

pub fn resolve_app_icons_for_route(icons: &[AppIconEntry], route_path: &str) -> Vec<AppIconEntry> {
    let normalized = if route_path.is_empty() { "/" } else { route_path };
    let segments: Vec<&str> = if normalized == "/" {
        Vec::new()
    } else {
        normalized.split('/').filter(|s| !s.is_empty()).collect()
    };

    let mut candidates = Vec::with_capacity(segments.len() + 1);
    for i in (0..=segments.len()).rev() {
        let candidate =
            if i == 0 { "/".to_string() } else { format!("/{}", segments[..i].join("/")) };
        candidates.push(candidate);
    }

    let mut resolved = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();

    for candidate in &candidates {
        for icon in icons.iter().filter(|icon| &icon.path == candidate) {
            let key = format!("{}:{}", icon.kind, icon.file_path);
            if seen_keys.contains(&key) {
                continue;
            }
            if icon.kind == "favicon"
                && resolved.iter().any(|entry: &AppIconEntry| entry.kind == "favicon")
            {
                continue;
            }
            resolved.push(icon.clone());
            seen_keys.insert(key);
        }
    }

    resolved
}

pub fn inject_app_icons_into_metadata(
    icons: &[AppIconEntry],
    route_path: &str,
    metadata: &mut PageMetadata,
) {
    if icons.is_empty() {
        return;
    }

    let resolved = resolve_app_icons_for_route(icons, route_path);
    if resolved.is_empty() {
        return;
    }

    let mut icons_meta =
        metadata.icons.clone().unwrap_or(IconsMetadata { icon: None, apple: None, other: None });

    if icons_meta.icon.is_none() {
        let icon_list: Vec<IconDescriptor> = resolved
            .iter()
            .filter(|icon| icon.kind == "favicon" || icon.kind == "icon")
            .map(|icon| IconDescriptor {
                url: icon.url.clone(),
                rel: Some("icon".to_string()),
                icon_type: Some(icon.content_type.clone()),
                sizes: icon.sizes.clone(),
                color: None,
            })
            .collect();
        if !icon_list.is_empty() {
            icons_meta.icon = Some(IconValue::Detailed(icon_list));
        }
    }

    if icons_meta.apple.is_none() {
        let apple_list: Vec<IconDescriptor> = resolved
            .iter()
            .filter(|icon| icon.kind == "apple-icon")
            .map(|icon| IconDescriptor {
                url: icon.url.clone(),
                rel: Some("apple-touch-icon".to_string()),
                icon_type: Some(icon.content_type.clone()),
                sizes: icon.sizes.clone(),
                color: None,
            })
            .collect();
        if !apple_list.is_empty() {
            icons_meta.apple = Some(IconValue::Detailed(apple_list));
        }
    }

    if icons_meta.icon.is_some() || icons_meta.apple.is_some() || icons_meta.other.is_some() {
        metadata.icons = Some(icons_meta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, file_path: &str, kind: &str, url: &str) -> AppIconEntry {
        AppIconEntry {
            path: path.to_string(),
            file_path: file_path.to_string(),
            kind: kind.to_string(),
            url: url.to_string(),
            content_type: "image/png".to_string(),
            sizes: Some("32x32".to_string()),
            width: Some(32),
            height: Some(32),
        }
    }

    #[test]
    fn prefers_nearer_favicon_and_collects_icons() {
        let icons = vec![
            entry("/", "favicon.ico", "favicon", "/favicon.ico"),
            entry("/", "icon.png", "icon", "/icon.png"),
            entry("/blog", "icon.png", "icon", "/blog/icon.png"),
            entry("/blog", "apple-icon.png", "apple-icon", "/blog/apple-icon.png"),
        ];

        let resolved = resolve_app_icons_for_route(&icons, "/blog/post");
        assert!(resolved.iter().any(|i| i.url == "/blog/icon.png"));
        assert!(resolved.iter().any(|i| i.url == "/icon.png"));
        assert!(resolved.iter().any(|i| i.url == "/favicon.ico"));
        assert!(resolved.iter().any(|i| i.url == "/blog/apple-icon.png"));
    }

    #[test]
    fn injects_only_when_metadata_icons_missing() {
        let icons = vec![
            entry("/", "favicon.ico", "favicon", "/favicon.ico"),
            entry("/", "apple-icon.png", "apple-icon", "/apple-icon.png"),
        ];

        let mut metadata = PageMetadata {
            title: None,
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

        inject_app_icons_into_metadata(&icons, "/", &mut metadata);

        let icons_meta = metadata.icons.expect("icons injected");
        match icons_meta.icon.expect("icon") {
            IconValue::Detailed(list) => {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0].url, "/favicon.ico");
            }
            other => panic!("expected detailed icons, got {other:?}"),
        }
        match icons_meta.apple.expect("apple") {
            IconValue::Detailed(list) => {
                assert_eq!(list[0].url, "/apple-icon.png");
                assert_eq!(list[0].rel.as_deref(), Some("apple-touch-icon"));
            }
            other => panic!("expected detailed apple icons, got {other:?}"),
        }

        metadata.icons = Some(IconsMetadata {
            icon: Some(IconValue::Single("/custom.ico".to_string())),
            apple: Some(IconValue::Single("/custom-apple.png".to_string())),
            other: None,
        });
        inject_app_icons_into_metadata(&icons, "/", &mut metadata);
        let icons_meta = metadata.icons.expect("icons kept");
        assert!(matches!(icons_meta.icon, Some(IconValue::Single(_))));
        assert!(matches!(icons_meta.apple, Some(IconValue::Single(_))));
    }
}
