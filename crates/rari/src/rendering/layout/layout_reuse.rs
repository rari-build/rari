use rustc_hash::FxHashMap;
use serde_json::Value;

use crate::server::routing::app_router::{AppRouter, LayoutEntry};

pub fn pathname_from_router_state_header(state: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(state).ok()?;
    let pathname = parsed.get("pathname")?.as_str()?;
    Some(pathname.to_string())
}

pub fn shared_layout_paths(
    from_layouts: &[LayoutEntry],
    to_layouts: &[LayoutEntry],
) -> Vec<String> {
    let mut shared = Vec::new();
    for (from, to) in from_layouts.iter().zip(to_layouts.iter()) {
        if from.file_path != to.file_path || from.path != to.path {
            break;
        }
        shared.push(from.path.clone());
    }
    shared
}

pub fn shared_layout_paths_for_navigation(
    app_router: &AppRouter,
    from_pathname: &str,
    to_pathname: &str,
) -> Vec<String> {
    if from_pathname == to_pathname {
        return Vec::new();
    }

    let from_layouts = match app_router.match_route(from_pathname) {
        Ok(route_match) => route_match.layouts,
        Err(_) => app_router.resolve_layouts(from_pathname),
    };
    let to_layouts = match app_router.match_route(to_pathname) {
        Ok(route_match) => route_match.layouts,
        Err(_) => app_router.resolve_layouts(to_pathname),
    };
    shared_layout_paths(&from_layouts, &to_layouts)
}

#[expect(clippy::implicit_hasher)]
pub fn router_state_from_headers(headers: &FxHashMap<String, String>) -> Option<String> {
    headers.get("rari-router-state").cloned().or_else(|| headers.get("Rari-Router-State").cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::routing::app_router::LayoutEntry;

    fn layout(path: &str, file_path: &str) -> LayoutEntry {
        LayoutEntry {
            path: path.to_string(),
            file_path: file_path.to_string(),
            component_id: None,
            css: vec![],
            parent_path: None,
            is_root: path == "/",
            additional_paths: None,
        }
    }

    #[test]
    fn shared_prefix_stops_at_first_divergence() {
        let from = vec![
            layout("/", "app/layout.tsx"),
            layout("/blog", "app/blog/layout.tsx"),
            layout("/blog/a", "app/blog/a/layout.tsx"),
        ];
        let to = vec![
            layout("/", "app/layout.tsx"),
            layout("/blog", "app/blog/layout.tsx"),
            layout("/blog/b", "app/blog/b/layout.tsx"),
        ];
        assert_eq!(shared_layout_paths(&from, &to), vec!["/", "/blog"]);
    }

    #[test]
    fn pathname_from_router_state_reads_json() {
        let state = r#"{"pathname":"/blog/a","search":"","tree":["",{}]}"#;
        assert_eq!(pathname_from_router_state_header(state).as_deref(), Some("/blog/a"));
    }
}
