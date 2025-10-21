use crate::error::RariError;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteSegmentType {
    Static,
    Dynamic,
    CatchAll,
    OptionalCatchAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSegment {
    #[serde(rename = "type")]
    pub segment_type: RouteSegmentType,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRouteEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub segments: Vec<RouteSegment>,
    pub params: Vec<String>,
    #[serde(rename = "isDynamic")]
    pub is_dynamic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "parentPath", skip_serializing_if = "Option::is_none")]
    pub parent_path: Option<String>,
    #[serde(rename = "isRoot", default)]
    pub is_root: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotFoundEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRouteManifest {
    pub routes: Vec<AppRouteEntry>,
    pub layouts: Vec<LayoutEntry>,
    pub loading: Vec<LoadingEntry>,
    pub errors: Vec<ErrorEntry>,
    #[serde(rename = "notFound")]
    pub not_found: Vec<NotFoundEntry>,
    pub generated: String,
}

#[derive(Debug, Clone)]
pub struct AppRouteMatch {
    pub route: AppRouteEntry,
    pub params: FxHashMap<String, String>,
    pub layouts: Vec<LayoutEntry>,
    pub loading: Option<LoadingEntry>,
    pub error: Option<ErrorEntry>,
    pub not_found: Option<NotFoundEntry>,
    pub pathname: String,
}

pub struct AppRouter {
    manifest: Arc<AppRouteManifest>,
}

impl AppRouter {
    pub fn new(manifest: AppRouteManifest) -> Self {
        Self { manifest: Arc::new(manifest) }
    }

    pub async fn from_file(path: &str) -> Result<Self, RariError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| RariError::io(format!("Failed to read manifest: {e}")))?;

        let manifest: AppRouteManifest = serde_json::from_str(&content)
            .map_err(|e| RariError::configuration(format!("Failed to parse manifest: {e}")))?;

        Ok(Self::new(manifest))
    }

    pub fn match_route(&self, path: &str) -> Result<AppRouteMatch, RariError> {
        let normalized_path = Self::normalize_path(path);
        debug!("Matching route for path: {}", normalized_path);

        for route in &self.manifest.routes {
            if let Some(params) = self.match_route_pattern(route, &normalized_path) {
                debug!("Matched route: {} -> {}", normalized_path, route.path);

                let layouts = self.resolve_layouts(&route.path);

                let loading = self.find_loading(&route.path);

                let error = self.find_error(&route.path);

                let not_found = self.find_not_found(&route.path);

                return Ok(AppRouteMatch {
                    route: route.clone(),
                    params,
                    layouts,
                    loading,
                    error,
                    not_found,
                    pathname: normalized_path,
                });
            }
        }

        Err(RariError::not_found(format!("No route found for path: {}", path)))
    }

    pub fn create_not_found_match(&self, path: &str) -> Option<AppRouteMatch> {
        let normalized_path = Self::normalize_path(path);

        let not_found_entry = self.find_not_found("/")?;

        let layouts = self.resolve_layouts("/");
        let loading = self.find_loading("/");
        let error = self.find_error("/");

        let not_found_route = AppRouteEntry {
            path: normalized_path.clone(),
            file_path: not_found_entry.file_path.clone(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
        };

        Some(AppRouteMatch {
            route: not_found_route,
            params: FxHashMap::default(),
            layouts,
            loading,
            error,
            not_found: Some(not_found_entry),
            pathname: normalized_path,
        })
    }

    fn match_route_pattern(
        &self,
        route: &AppRouteEntry,
        path: &str,
    ) -> Option<FxHashMap<String, String>> {
        let route_segments = route.path.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>();
        let path_segments = path.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>();

        let mut params = FxHashMap::default();
        let mut route_idx = 0;
        let mut path_idx = 0;

        while route_idx < route_segments.len() {
            let route_seg = route_segments[route_idx];

            if route_seg.starts_with("[[...") && route_seg.ends_with("]]") {
                let param_name = &route_seg[5..route_seg.len() - 2];

                if path_idx < path_segments.len() {
                    let remaining: Vec<String> =
                        path_segments[path_idx..].iter().map(|s| s.to_string()).collect();
                    params.insert(param_name.to_string(), remaining.join("/"));
                }

                return Some(params);
            }

            if route_seg.starts_with("[...") && route_seg.ends_with(']') {
                let param_name = &route_seg[4..route_seg.len() - 1];

                if path_idx >= path_segments.len() {
                    return None;
                }

                let remaining: Vec<String> =
                    path_segments[path_idx..].iter().map(|s| s.to_string()).collect();
                params.insert(param_name.to_string(), remaining.join("/"));

                return Some(params);
            }

            if route_seg.starts_with('[') && route_seg.ends_with(']') {
                if path_idx >= path_segments.len() {
                    return None;
                }

                let param_name = &route_seg[1..route_seg.len() - 1];
                params.insert(param_name.to_string(), path_segments[path_idx].to_string());

                path_idx += 1;
                route_idx += 1;
                continue;
            }

            if path_idx >= path_segments.len() || route_seg != path_segments[path_idx] {
                return None;
            }

            path_idx += 1;
            route_idx += 1;
        }

        if path_idx == path_segments.len() { Some(params) } else { None }
    }

    pub fn resolve_layouts(&self, route_path: &str) -> Vec<LayoutEntry> {
        let mut layouts = Vec::new();
        let segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for i in 0..=segments.len() {
            let current_path =
                if i == 0 { "/".to_string() } else { format!("/{}", segments[..i].join("/")) };

            if let Some(layout) = self.manifest.layouts.iter().find(|l| l.path == current_path) {
                let mut layout_entry = layout.clone();
                layout_entry.is_root = layout_entry.path == "/";
                layouts.push(layout_entry);
            }
        }

        layouts
    }

    fn find_loading(&self, route_path: &str) -> Option<LoadingEntry> {
        let segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for i in (0..=segments.len()).rev() {
            let current_path =
                if i == 0 { "/".to_string() } else { format!("/{}", segments[..i].join("/")) };

            if let Some(loading) = self.manifest.loading.iter().find(|l| l.path == current_path) {
                return Some(loading.clone());
            }
        }

        None
    }

    fn find_error(&self, route_path: &str) -> Option<ErrorEntry> {
        let segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for i in (0..=segments.len()).rev() {
            let current_path =
                if i == 0 { "/".to_string() } else { format!("/{}", segments[..i].join("/")) };

            if let Some(error) = self.manifest.errors.iter().find(|e| e.path == current_path) {
                return Some(error.clone());
            }
        }

        None
    }

    pub fn find_not_found(&self, route_path: &str) -> Option<NotFoundEntry> {
        let segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for i in (0..=segments.len()).rev() {
            let current_path =
                if i == 0 { "/".to_string() } else { format!("/{}", segments[..i].join("/")) };

            if let Some(not_found) = self.manifest.not_found.iter().find(|n| n.path == current_path)
            {
                return Some(not_found.clone());
            }
        }

        None
    }

    fn normalize_path(path: &str) -> String {
        let path = path.trim();

        let path = path.split('?').next().unwrap_or(path);
        let path = path.split('#').next().unwrap_or(path);

        if path.is_empty() || !path.starts_with('/') {
            format!("/{}", path)
        } else {
            path.to_string()
        }
    }

    pub fn manifest(&self) -> &AppRouteManifest {
        &self.manifest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manifest() -> AppRouteManifest {
        AppRouteManifest {
            routes: vec![
                AppRouteEntry {
                    path: "/".to_string(),
                    file_path: "page.tsx".to_string(),
                    segments: vec![],
                    params: vec![],
                    is_dynamic: false,
                },
                AppRouteEntry {
                    path: "/about".to_string(),
                    file_path: "about/page.tsx".to_string(),
                    segments: vec![RouteSegment {
                        segment_type: RouteSegmentType::Static,
                        value: "about".to_string(),
                        param: None,
                    }],
                    params: vec![],
                    is_dynamic: false,
                },
                AppRouteEntry {
                    path: "/blog/[slug]".to_string(),
                    file_path: "blog/[slug]/page.tsx".to_string(),
                    segments: vec![
                        RouteSegment {
                            segment_type: RouteSegmentType::Static,
                            value: "blog".to_string(),
                            param: None,
                        },
                        RouteSegment {
                            segment_type: RouteSegmentType::Dynamic,
                            value: "[slug]".to_string(),
                            param: Some("slug".to_string()),
                        },
                    ],
                    params: vec!["slug".to_string()],
                    is_dynamic: true,
                },
                AppRouteEntry {
                    path: "/docs/[...slug]".to_string(),
                    file_path: "docs/[...slug]/page.tsx".to_string(),
                    segments: vec![
                        RouteSegment {
                            segment_type: RouteSegmentType::Static,
                            value: "docs".to_string(),
                            param: None,
                        },
                        RouteSegment {
                            segment_type: RouteSegmentType::CatchAll,
                            value: "[...slug]".to_string(),
                            param: Some("slug".to_string()),
                        },
                    ],
                    params: vec!["slug".to_string()],
                    is_dynamic: true,
                },
            ],
            layouts: vec![
                LayoutEntry {
                    path: "/".to_string(),
                    file_path: "layout.tsx".to_string(),
                    parent_path: None,
                    is_root: false,
                },
                LayoutEntry {
                    path: "/blog".to_string(),
                    file_path: "blog/layout.tsx".to_string(),
                    parent_path: Some("/".to_string()),
                    is_root: false,
                },
            ],
            loading: vec![],
            errors: vec![],
            not_found: vec![],
            generated: "2025-09-30T00:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn test_match_static_route() {
        let router = AppRouter::new(create_test_manifest());
        let result = router.match_route("/about");

        assert!(result.is_ok());
        let matched = result.unwrap();
        assert_eq!(matched.route.path, "/about");
        assert!(matched.params.is_empty());
    }

    #[test]
    fn test_match_dynamic_route() {
        let router = AppRouter::new(create_test_manifest());
        let result = router.match_route("/blog/hello-world");

        assert!(result.is_ok());
        let matched = result.unwrap();
        assert_eq!(matched.route.path, "/blog/[slug]");
        assert_eq!(matched.params.get("slug"), Some(&"hello-world".to_string()));
    }

    #[test]
    fn test_match_catch_all_route() {
        let router = AppRouter::new(create_test_manifest());
        let result = router.match_route("/docs/getting-started/installation");

        assert!(result.is_ok());
        let matched = result.unwrap();
        assert_eq!(matched.route.path, "/docs/[...slug]");
        assert_eq!(matched.params.get("slug"), Some(&"getting-started/installation".to_string()));
    }

    #[test]
    fn test_resolve_layouts() {
        let router = AppRouter::new(create_test_manifest());
        let layouts = router.resolve_layouts("/blog/[slug]");

        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].path, "/");
        assert_eq!(layouts[1].path, "/blog");
    }

    #[test]
    fn test_root_layout_detection() {
        let router = AppRouter::new(create_test_manifest());
        let layouts = router.resolve_layouts("/blog/[slug]");

        assert_eq!(layouts.len(), 2);
        assert!(layouts[0].is_root, "Root layout (/) should have is_root = true");
        assert_eq!(layouts[0].path, "/");
        assert!(!layouts[1].is_root, "Nested layout (/blog) should have is_root = false");
        assert_eq!(layouts[1].path, "/blog");
    }

    #[test]
    fn test_root_layout_only() {
        let router = AppRouter::new(create_test_manifest());
        let layouts = router.resolve_layouts("/");

        assert_eq!(layouts.len(), 1);
        assert!(layouts[0].is_root, "Root layout should have is_root = true");
        assert_eq!(layouts[0].path, "/");
    }

    #[test]
    fn test_not_found() {
        let router = AppRouter::new(create_test_manifest());
        let result = router.match_route("/nonexistent");

        assert!(result.is_err());
    }
}
