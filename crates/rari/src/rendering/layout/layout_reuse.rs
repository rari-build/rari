use rustc_hash::FxHashMap;
use serde_json::Value;

use crate::server::routing::app_router::{AppRouter, LayoutEntry};

pub fn pathname_from_router_state_header(state: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(state).ok()?;
    let pathname = parsed.get("pathname")?.as_str()?;
    Some(pathname.to_string())
}

pub fn tree_from_router_state_header(state: &str) -> Option<Value> {
    let parsed: Value = serde_json::from_str(state).ok()?;
    parsed.get("tree").cloned()
}

pub fn segment_path_from_tree(tree: &Value) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = tree;
    while let Some(arr) = current.as_array() {
        if let Some(segment) = arr.first().and_then(Value::as_str)
            && !segment.is_empty()
        {
            path.push(segment.to_string());
        }
        match arr.get(1).and_then(|parallel| parallel.get("children")) {
            Some(child) => current = child,
            None => break,
        }
    }
    path
}

pub fn layout_paths_from_segments(segments: &[String]) -> Vec<String> {
    let mut paths = vec!["/".to_string()];
    let mut acc = String::new();
    for segment in segments {
        acc.push('/');
        acc.push_str(segment);
        paths.push(acc.clone());
    }
    paths
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

pub fn intersect_reuse_with_client_tree(
    file_shared: Vec<String>,
    client_tree: Option<&Value>,
    from_pathname: &str,
) -> Vec<String> {
    let client_segments = match client_tree {
        Some(tree) => segment_path_from_tree(tree),
        None => from_pathname
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    };
    let client_paths = layout_paths_from_segments(&client_segments);
    let client_set: rustc_hash::FxHashSet<&str> = client_paths.iter().map(String::as_str).collect();

    let mut out = Vec::new();
    for path in file_shared {
        if !client_set.contains(path.as_str()) {
            break;
        }
        out.push(path);
    }
    out
}

pub fn shared_layout_paths_for_navigation(
    app_router: &AppRouter,
    from_pathname: &str,
    to_pathname: &str,
    client_tree: Option<&Value>,
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
    let file_shared = shared_layout_paths(&from_layouts, &to_layouts);
    intersect_reuse_with_client_tree(file_shared, client_tree, from_pathname)
}

#[expect(clippy::implicit_hasher)]
pub fn router_state_from_headers(headers: &FxHashMap<String, String>) -> Option<String> {
    headers.get("rari-router-state").cloned().or_else(|| headers.get("Rari-Router-State").cloned())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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
            name: None,
            skip_parents: false,
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
        let state = r#"{"pathname":"/blog/a","search":"","tree":["",{"children":["blog",{"children":["a",{}]}]}]}"#;
        assert_eq!(pathname_from_router_state_header(state).as_deref(), Some("/blog/a"));
        assert!(tree_from_router_state_header(state).is_some());
    }

    #[test]
    fn tree_from_router_state_optional() {
        let state = r#"{"pathname":"/","search":""}"#;
        assert_eq!(pathname_from_router_state_header(state).as_deref(), Some("/"));
        assert!(tree_from_router_state_header(state).is_none());
    }

    #[test]
    fn segment_path_from_tree_walks_children() {
        let tree = json!(["", {"children": ["blog", {"children": ["a", {}]}]}]);
        assert_eq!(segment_path_from_tree(&tree), vec!["blog", "a"]);
    }

    #[test]
    fn intersect_truncates_when_client_tree_is_shallower() {
        let file_shared = vec!["/".to_string(), "/blog".to_string()];
        let tree = json!(["", {"children": ["", {}]}]);
        let out = intersect_reuse_with_client_tree(file_shared, Some(&tree), "/blog/a");
        assert_eq!(out, vec!["/"]);
    }

    #[test]
    fn intersect_keeps_file_shared_when_tree_matches_from_route() {
        let file_shared = vec!["/".to_string(), "/blog".to_string()];
        let tree = json!(["", {"children": ["blog", {"children": ["a", {}]}]}]);
        let out = intersect_reuse_with_client_tree(file_shared, Some(&tree), "/blog/a");
        assert_eq!(out, vec!["/", "/blog"]);
    }
}
