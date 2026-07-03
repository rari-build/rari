use std::sync::Arc;

use cow_utils::CowUtils;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[cfg(test)]
use crate::server::routing::types::RouteSegmentType;
use crate::server::routing::types::{ParamValue, RouteSegment};

fn parse_decoded_path_segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| urlencoding::decode(s).unwrap_or_else(|_| s.to_string().into()).into_owned())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AppRouteEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "componentId", default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    #[serde(default)]
    pub css: Vec<String>,
    pub segments: Vec<RouteSegment>,
    pub params: Vec<String>,
    #[serde(rename = "isDynamic")]
    pub is_dynamic: bool,
    #[serde(rename = "staticParams", default, skip_serializing_if = "Option::is_none")]
    pub static_params: Option<Vec<FxHashMap<String, serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LayoutEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "componentId", default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(rename = "parentPath", skip_serializing_if = "Option::is_none")]
    pub parent_path: Option<String>,
    #[serde(rename = "isRoot", default)]
    pub is_root: bool,
    #[serde(rename = "additionalPaths", default, skip_serializing_if = "Option::is_none")]
    pub additional_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplateEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "componentId", default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(rename = "parentPath", skip_serializing_if = "Option::is_none")]
    pub parent_path: Option<String>,
    #[serde(rename = "additionalPaths", default, skip_serializing_if = "Option::is_none")]
    pub additional_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LoadingEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "componentId", default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(rename = "additionalPaths", default, skip_serializing_if = "Option::is_none")]
    pub additional_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ErrorEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "componentId", default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(rename = "additionalPaths", default, skip_serializing_if = "Option::is_none")]
    pub additional_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NotFoundEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "componentId", default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(rename = "additionalPaths", default, skip_serializing_if = "Option::is_none")]
    pub additional_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AppRouteManifest {
    pub routes: Vec<AppRouteEntry>,
    pub layouts: Vec<LayoutEntry>,
    pub loading: Vec<LoadingEntry>,
    pub errors: Vec<ErrorEntry>,
    #[serde(rename = "notFound")]
    pub not_found: Vec<NotFoundEntry>,
    #[serde(default)]
    pub templates: Vec<TemplateEntry>,
    pub generated: String,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AppRouteMatch {
    pub route: AppRouteEntry,
    pub params: FxHashMap<String, ParamValue>,
    pub layouts: Vec<LayoutEntry>,
    pub loading: Option<LoadingEntry>,
    pub error: Option<ErrorEntry>,
    pub not_found: Option<NotFoundEntry>,
    pub templates: Vec<TemplateEntry>,
    pub pathname: String,
}

pub struct AppRouter {
    manifest: Arc<AppRouteManifest>,
}

impl AppRouter {
    pub fn new(manifest: AppRouteManifest) -> Self {
        Self { manifest: Arc::new(manifest) }
    }

    #[expect(clippy::missing_errors_doc)]
    pub async fn from_file(path: &str) -> Result<Self, RariError> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| RariError::io(format!("Failed to read manifest: {e}")))?;

        let manifest: AppRouteManifest = serde_json::from_str(&content)
            .map_err(|e| RariError::configuration(format!("Failed to parse manifest: {e}")))?;

        Ok(Self::new(manifest))
    }

    #[expect(clippy::missing_errors_doc)]
    pub fn match_route(&self, path: &str) -> Result<AppRouteMatch, RariError> {
        let normalized_path = Self::normalize_path(path);

        for route in &self.manifest.routes {
            if let Some(params) = Self::match_route_pattern(route, &normalized_path) {
                let layouts = self.resolve_layouts_for_route(route);
                let templates = self.resolve_templates_for_route(route);

                let loading = self.find_loading_for_route(route);

                let error = self.find_error_for_route(route);

                return Ok(AppRouteMatch {
                    route: route.clone(),
                    params,
                    layouts,
                    loading,
                    error,
                    not_found: None,
                    templates,
                    pathname: normalized_path,
                });
            }
        }

        Err(RariError::not_found(format!("No route found for path: {path}")))
    }

    pub fn create_not_found_match(&self, path: &str) -> Option<AppRouteMatch> {
        let normalized_path = Self::normalize_path(path);

        let not_found_entry = self.find_not_found(&normalized_path)?;

        let layouts = self.resolve_layouts(&normalized_path);
        let templates = self.resolve_templates(&normalized_path);
        let loading = self.find_loading(&normalized_path);
        let error = self.find_error(&normalized_path);

        let not_found_route = AppRouteEntry {
            path: normalized_path.clone(),
            file_path: not_found_entry.file_path.clone(),
            component_id: not_found_entry.component_id.clone(),
            css: not_found_entry.css.clone(),
            segments: vec![],
            params: vec![],
            is_dynamic: false,
            static_params: None,
        };

        Some(AppRouteMatch {
            route: not_found_route,
            params: FxHashMap::default(),
            layouts,
            loading,
            error,
            not_found: Some(not_found_entry),
            templates,
            pathname: normalized_path,
        })
    }

    fn match_route_pattern(
        route: &AppRouteEntry,
        path: &str,
    ) -> Option<FxHashMap<String, ParamValue>> {
        let route_segments = route.path.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>();
        let path_segments = parse_decoded_path_segments(path);

        let mut params = FxHashMap::default();
        let mut route_idx = 0;
        let mut path_idx = 0;

        while route_idx < route_segments.len() {
            let route_seg = route_segments[route_idx];

            if route_seg.starts_with("[[...") && route_seg.ends_with("]]") {
                let param_name = &route_seg[5..route_seg.len() - 2];

                if path_idx < path_segments.len() {
                    let remaining: Vec<String> = path_segments[path_idx..].to_vec();
                    params.insert(param_name.to_string(), ParamValue::Multiple(remaining));
                }

                return Some(params);
            }

            if route_seg.starts_with("[...") && route_seg.ends_with(']') {
                let param_name = &route_seg[4..route_seg.len() - 1];

                if path_idx >= path_segments.len() {
                    return None;
                }

                let remaining: Vec<String> = path_segments[path_idx..].to_vec();
                params.insert(param_name.to_string(), ParamValue::Multiple(remaining));

                return Some(params);
            }

            if route_seg.starts_with('[') && route_seg.ends_with(']') {
                if path_idx >= path_segments.len() {
                    return None;
                }

                let param_name = &route_seg[1..route_seg.len() - 1];
                params.insert(
                    param_name.to_string(),
                    ParamValue::Single(path_segments[path_idx].clone()),
                );

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

    #[expect(clippy::ref_option, reason = "Function signature matches API pattern")]
    fn matches_path_or_additional(
        path: &str,
        additional_paths: &Option<Vec<String>>,
        current_path: &str,
    ) -> bool {
        path == current_path
            || additional_paths.as_ref().is_some_and(|paths| {
                paths.iter().any(|additional_path| additional_path == current_path)
            })
    }

    fn normalized_dir(file_path: &str) -> String {
        let normalized = file_path.cow_replace('\\', "/").into_owned();
        normalized.rsplit_once('/').map(|(dir, _)| dir.to_string()).unwrap_or_default()
    }

    fn is_boundary_ancestor(boundary_file_path: &str, route_file_path: &str) -> bool {
        let boundary_dir = Self::normalized_dir(boundary_file_path);
        if boundary_dir.is_empty() {
            return true;
        }

        let route_dir = Self::normalized_dir(route_file_path);
        route_dir == boundary_dir || route_dir.starts_with(&format!("{boundary_dir}/"))
    }

    fn file_path_depth(file_path: &str) -> usize {
        let dir = Self::normalized_dir(file_path);
        if dir.is_empty() { 0 } else { dir.split('/').count() }
    }

    fn nearest_boundary_by_path<T, PathFn, AdditionalPathsFn, FilePathFn>(
        entries: &[T],
        route_path: &str,
        path_of: PathFn,
        additional_paths_of: AdditionalPathsFn,
        file_path_of: FilePathFn,
    ) -> Option<T>
    where
        T: Clone,
        PathFn: Fn(&T) -> &str,
        AdditionalPathsFn: Fn(&T) -> &Option<Vec<String>>,
        FilePathFn: Fn(&T) -> &str,
    {
        let segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        (0..=segments.len()).rev().find_map(|i| {
            let current_path =
                if i == 0 { "/".to_string() } else { format!("/{}", segments[..i].join("/")) };

            entries
                .iter()
                .filter(|entry| {
                    Self::matches_path_or_additional(
                        path_of(entry),
                        additional_paths_of(entry),
                        &current_path,
                    )
                })
                .max_by_key(|entry| Self::file_path_depth(file_path_of(entry)))
                .cloned()
        })
    }

    fn nearest_boundary_for_route<T, PathFn, AdditionalPathsFn, FilePathFn>(
        entries: &[T],
        route: &AppRouteEntry,
        path_of: PathFn,
        additional_paths_of: AdditionalPathsFn,
        file_path_of: FilePathFn,
    ) -> Option<T>
    where
        T: Clone,
        PathFn: Fn(&T) -> &str,
        AdditionalPathsFn: Fn(&T) -> &Option<Vec<String>>,
        FilePathFn: Fn(&T) -> &str,
    {
        entries
            .iter()
            .filter(|entry| {
                Self::is_boundary_ancestor(file_path_of(entry), &route.file_path)
                    || Self::matches_path_or_additional(
                        path_of(entry),
                        additional_paths_of(entry),
                        &route.path,
                    )
            })
            .max_by_key(|entry| Self::file_path_depth(file_path_of(entry)))
            .cloned()
    }

    fn resolve_layouts_for_route(&self, route: &AppRouteEntry) -> Vec<LayoutEntry> {
        let mut layouts: Vec<LayoutEntry> = self
            .manifest
            .layouts
            .iter()
            .filter(|layout| {
                Self::is_boundary_ancestor(&layout.file_path, &route.file_path)
                    || Self::matches_path_or_additional(
                        &layout.path,
                        &layout.additional_paths,
                        &route.path,
                    )
            })
            .cloned()
            .collect();

        layouts.sort_by_key(|layout| Self::file_path_depth(&layout.file_path));

        for layout in &mut layouts {
            layout.is_root =
                Self::normalized_dir(&layout.file_path).is_empty() || layout.path == "/";
        }

        layouts
    }

    pub fn resolve_layouts(&self, route_path: &str) -> Vec<LayoutEntry> {
        let mut layouts = Vec::new();
        let segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for i in 0..=segments.len() {
            let current_path =
                if i == 0 { "/".to_string() } else { format!("/{}", segments[..i].join("/")) };

            let mut matching_layouts: Vec<_> = self
                .manifest
                .layouts
                .iter()
                .filter(|layout| {
                    Self::matches_path_or_additional(
                        &layout.path,
                        &layout.additional_paths,
                        &current_path,
                    )
                })
                .cloned()
                .collect();

            matching_layouts.sort_by_key(|layout| Self::file_path_depth(&layout.file_path));

            for layout in matching_layouts {
                if layouts
                    .iter()
                    .any(|existing: &LayoutEntry| existing.file_path == layout.file_path)
                {
                    continue;
                }

                let mut layout_entry = layout.clone();
                layout_entry.is_root = Self::normalized_dir(&layout_entry.file_path).is_empty()
                    || layout_entry.path == "/";
                layouts.push(layout_entry);
            }
        }

        layouts
    }

    pub fn resolve_templates(&self, route_path: &str) -> Vec<TemplateEntry> {
        let mut templates = Vec::new();
        let segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for i in 0..=segments.len() {
            let current_path =
                if i == 0 { "/".to_string() } else { format!("/{}", segments[..i].join("/")) };

            let mut matching: Vec<_> = self
                .manifest
                .templates
                .iter()
                .filter(|template| {
                    Self::matches_path_or_additional(
                        &template.path,
                        &template.additional_paths,
                        &current_path,
                    )
                })
                .cloned()
                .collect();

            matching.sort_by_key(|t| Self::file_path_depth(&t.file_path));

            for template in matching {
                if templates
                    .iter()
                    .any(|existing: &TemplateEntry| existing.file_path == template.file_path)
                {
                    continue;
                }
                templates.push(template);
            }
        }

        templates
    }

    fn resolve_templates_for_route(&self, route: &AppRouteEntry) -> Vec<TemplateEntry> {
        let mut templates: Vec<_> = self
            .manifest
            .templates
            .iter()
            .filter(|template| {
                Self::is_boundary_ancestor(&template.file_path, &route.file_path)
                    || Self::matches_path_or_additional(
                        &template.path,
                        &template.additional_paths,
                        &route.path,
                    )
            })
            .cloned()
            .collect();

        templates.sort_by_key(|t| Self::file_path_depth(&t.file_path));
        templates
    }

    pub(crate) fn find_loading(&self, route_path: &str) -> Option<LoadingEntry> {
        Self::nearest_boundary_by_path(
            &self.manifest.loading,
            route_path,
            |loading| &loading.path,
            |loading| &loading.additional_paths,
            |loading| &loading.file_path,
        )
    }

    fn find_loading_for_route(&self, route: &AppRouteEntry) -> Option<LoadingEntry> {
        Self::nearest_boundary_for_route(
            &self.manifest.loading,
            route,
            |loading| &loading.path,
            |loading| &loading.additional_paths,
            |loading| &loading.file_path,
        )
    }

    pub(crate) fn find_error(&self, route_path: &str) -> Option<ErrorEntry> {
        Self::nearest_boundary_by_path(
            &self.manifest.errors,
            route_path,
            |error| &error.path,
            |error| &error.additional_paths,
            |error| &error.file_path,
        )
    }

    fn find_error_for_route(&self, route: &AppRouteEntry) -> Option<ErrorEntry> {
        Self::nearest_boundary_for_route(
            &self.manifest.errors,
            route,
            |error| &error.path,
            |error| &error.additional_paths,
            |error| &error.file_path,
        )
    }

    pub(crate) fn find_not_found(&self, route_path: &str) -> Option<NotFoundEntry> {
        Self::nearest_boundary_by_path(
            &self.manifest.not_found,
            route_path,
            |not_found| &not_found.path,
            |not_found| &not_found.additional_paths,
            |not_found| &not_found.file_path,
        )
    }

    fn normalize_path(path: &str) -> String {
        let path = path.trim();

        let path = path.split('?').next().unwrap_or(path);
        let path = path.split('#').next().unwrap_or(path);

        if path.is_empty() || !path.starts_with('/') {
            format!("/{path}")
        } else {
            path.to_string()
        }
    }

    pub fn manifest(&self) -> &AppRouteManifest {
        &self.manifest
    }

    pub fn warmup_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();

        for route in &self.manifest.routes {
            if !route.is_dynamic {
                paths.push(route.path.clone());
            } else if let Some(ref static_params) = route.static_params {
                for params in static_params {
                    let concrete_path = Self::expand_route_path(&route.path, params);
                    if let Some(p) = concrete_path {
                        paths.push(p);
                    }
                }
            }
        }

        paths
    }

    fn expand_route_path(
        route_path: &str,
        params: &FxHashMap<String, serde_json::Value>,
    ) -> Option<String> {
        let segments: Vec<&str> = route_path.split('/').collect();
        let mut result_segments: Vec<String> = Vec::new();

        for segment in &segments {
            if segment.is_empty() {
                continue;
            }

            if segment.starts_with("[[...") && segment.ends_with("]]") {
                let param_name = &segment[5..segment.len() - 2];
                if let Some(value) = params.get(param_name) {
                    match value {
                        serde_json::Value::Array(arr) => {
                            for item in arr {
                                if let Some(s) = item.as_str() {
                                    result_segments.push(s.to_string());
                                }
                            }
                        }
                        serde_json::Value::String(s) => {
                            result_segments.push(s.clone());
                        }
                        _ => {}
                    }
                }
            } else if segment.starts_with("[...") && segment.ends_with(']') {
                let param_name = &segment[4..segment.len() - 1];
                let value = params.get(param_name)?;
                match value {
                    serde_json::Value::Array(arr) => {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                result_segments.push(s.to_string());
                            }
                        }
                    }
                    serde_json::Value::String(s) => {
                        result_segments.push(s.clone());
                    }
                    _ => return None,
                }
            } else if segment.starts_with('[') && segment.ends_with(']') {
                let param_name = &segment[1..segment.len() - 1];
                let value = params.get(param_name)?;
                match value {
                    serde_json::Value::String(s) => {
                        result_segments.push(s.clone());
                    }
                    _ => return None,
                }
            } else {
                result_segments.push(segment.to_string());
            }
        }

        if result_segments.is_empty() {
            Some("/".to_string())
        } else {
            Some(format!("/{}", result_segments.join("/")))
        }
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn build_minimal_manifest() -> AppRouteManifest {
        AppRouteManifest {
            routes: vec![],
            layouts: vec![],
            loading: vec![],
            errors: vec![],
            not_found: vec![],
            templates: vec![],
            generated: "2026-01-01T00:00:00.000Z".to_string(),
        }
    }

    fn create_test_manifest() -> AppRouteManifest {
        AppRouteManifest {
            routes: vec![
                AppRouteEntry {
                    path: "/".to_string(),
                    file_path: "page.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    segments: vec![],
                    params: vec![],
                    is_dynamic: false,
                    static_params: None,
                },
                AppRouteEntry {
                    path: "/about".to_string(),
                    file_path: "about/page.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    segments: vec![RouteSegment {
                        segment_type: RouteSegmentType::Static,
                        value: "about".to_string(),
                        param: None,
                    }],
                    params: vec![],
                    is_dynamic: false,
                    static_params: None,
                },
                AppRouteEntry {
                    path: "/blog/[slug]".to_string(),
                    file_path: "blog/[slug]/page.tsx".to_string(),
                    component_id: None,
                    css: vec![],
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
                    static_params: None,
                },
                AppRouteEntry {
                    path: "/docs/[...slug]".to_string(),
                    file_path: "docs/[...slug]/page.tsx".to_string(),
                    component_id: None,
                    css: vec![],
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
                    static_params: None,
                },
            ],
            layouts: vec![
                LayoutEntry {
                    path: "/".to_string(),
                    file_path: "layout.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    parent_path: None,
                    additional_paths: None,
                    is_root: false,
                },
                LayoutEntry {
                    path: "/blog".to_string(),
                    file_path: "blog/layout.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    parent_path: Some("/".to_string()),
                    additional_paths: None,
                    is_root: false,
                },
            ],
            loading: vec![],
            errors: vec![],
            not_found: vec![],
            templates: vec![],
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
        assert_eq!(
            matched.params.get("slug").and_then(|p| p.as_string()),
            Some(&"hello-world".to_string())
        );
    }

    #[test]
    fn test_match_catch_all_route() {
        let router = AppRouter::new(create_test_manifest());
        let result = router.match_route("/docs/getting-started/installation");

        assert!(result.is_ok());
        let matched = result.unwrap();
        assert_eq!(matched.route.path, "/docs/[...slug]");
        let slug_vec = matched.params.get("slug").and_then(|p| p.as_vec());
        assert_eq!(
            slug_vec,
            Some(&vec!["getting-started".to_string(), "installation".to_string()])
        );
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

    #[test]
    fn test_match_root_route() {
        let router = AppRouter::new(create_test_manifest());
        let result = router.match_route("/");

        assert!(result.is_ok(), "Root path '/' should match");
        let matched = result.unwrap();
        assert_eq!(matched.route.path, "/");
        assert_eq!(matched.route.file_path, "page.tsx");
        assert!(matched.params.is_empty());
    }

    #[test]
    fn test_root_route_with_dynamic_sibling() {
        let manifest = AppRouteManifest {
            routes: vec![
                AppRouteEntry {
                    path: "/".to_string(),
                    file_path: "page.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    segments: vec![],
                    params: vec![],
                    is_dynamic: false,
                    static_params: None,
                },
                AppRouteEntry {
                    path: "/[slug]".to_string(),
                    file_path: "[slug]/page.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    segments: vec![RouteSegment {
                        segment_type: RouteSegmentType::Dynamic,
                        value: "[slug]".to_string(),
                        param: Some("slug".to_string()),
                    }],
                    params: vec!["slug".to_string()],
                    is_dynamic: true,
                    static_params: None,
                },
            ],
            layouts: vec![],
            loading: vec![],
            errors: vec![],
            not_found: vec![],
            templates: vec![],
            generated: "2025-01-10T00:00:00.000Z".to_string(),
        };

        let router = AppRouter::new(manifest);

        let result = router.match_route("/");
        assert!(result.is_ok(), "Root path '/' should match");
        let matched = result.unwrap();
        assert_eq!(matched.route.path, "/", "Should match root route, not [slug]");

        let result = router.match_route("/about");
        assert!(result.is_ok());
        let matched = result.unwrap();
        assert_eq!(matched.route.path, "/[slug]");
        assert_eq!(
            matched.params.get("slug").and_then(|p| p.as_string()),
            Some(&"about".to_string())
        );
    }

    #[test]
    fn test_resolve_layouts_with_additional_paths() {
        let manifest = AppRouteManifest {
            routes: vec![],
            layouts: vec![LayoutEntry {
                path: "/about".to_string(),
                file_path: "(marketing)/layout.tsx".to_string(),
                component_id: None,
                css: vec![],
                parent_path: Some("/".to_string()),
                is_root: false,
                additional_paths: Some(vec!["/pricing".to_string()]),
            }],
            loading: vec![],
            errors: vec![],
            not_found: vec![],
            templates: vec![],
            generated: "2026-01-01T00:00:00.000Z".to_string(),
        };
        let router = AppRouter::new(manifest);

        let layouts_for_about = router.resolve_layouts("/about");
        assert_eq!(layouts_for_about.len(), 1);
        assert_eq!(layouts_for_about[0].file_path, "(marketing)/layout.tsx");

        let layouts_for_pricing = router.resolve_layouts("/pricing");
        assert_eq!(layouts_for_pricing.len(), 1);
        assert_eq!(layouts_for_pricing[0].file_path, "(marketing)/layout.tsx");

        let layouts_for_unrelated = router.resolve_layouts("/dashboard");
        assert_eq!(layouts_for_unrelated.len(), 0);
    }

    #[test]
    fn test_match_route_resolves_group_layouts_by_file_path() {
        let manifest = AppRouteManifest {
            routes: vec![
                AppRouteEntry {
                    path: "/pricing".to_string(),
                    file_path: "(_public)/pricing/page.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    segments: vec![RouteSegment {
                        segment_type: RouteSegmentType::Static,
                        value: "pricing".to_string(),
                        param: None,
                    }],
                    params: vec![],
                    is_dynamic: false,
                    static_params: None,
                },
                AppRouteEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/(flow)/forgot/page.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    segments: vec![RouteSegment {
                        segment_type: RouteSegmentType::Static,
                        value: "forgot".to_string(),
                        param: None,
                    }],
                    params: vec![],
                    is_dynamic: false,
                    static_params: None,
                },
            ],
            layouts: vec![
                LayoutEntry {
                    path: "/".to_string(),
                    file_path: "layout.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    parent_path: None,
                    is_root: false,
                    additional_paths: None,
                },
                LayoutEntry {
                    path: "/contact".to_string(),
                    file_path: "(_public)/layout.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    parent_path: Some("/".to_string()),
                    is_root: false,
                    additional_paths: Some(vec!["/pricing".to_string()]),
                },
                LayoutEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/layout.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    parent_path: Some("/".to_string()),
                    is_root: false,
                    additional_paths: Some(vec!["/login".to_string(), "/signup".to_string()]),
                },
                LayoutEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/(flow)/layout.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    parent_path: Some("/".to_string()),
                    is_root: false,
                    additional_paths: Some(vec!["/reset".to_string()]),
                },
            ],
            loading: vec![],
            errors: vec![],
            not_found: vec![],
            templates: vec![],
            generated: "2026-01-01T00:00:00.000Z".to_string(),
        };
        let router = AppRouter::new(manifest);

        let pricing = router.match_route("/pricing").unwrap();
        let pricing_layouts: Vec<_> =
            pricing.layouts.iter().map(|layout| layout.file_path.as_str()).collect();
        assert_eq!(pricing_layouts, vec!["layout.tsx", "(_public)/layout.tsx"]);

        let forgot = router.match_route("/forgot").unwrap();
        let forgot_layouts: Vec<_> =
            forgot.layouts.iter().map(|layout| layout.file_path.as_str()).collect();
        assert_eq!(
            forgot_layouts,
            vec!["layout.tsx", "(auth)/layout.tsx", "(auth)/(flow)/layout.tsx"]
        );
    }

    #[test]
    fn test_resolve_layouts_falls_back_to_path() {
        let manifest = AppRouteManifest {
            routes: vec![],
            layouts: vec![LayoutEntry {
                path: "/".to_string(),
                file_path: "layout.tsx".to_string(),
                component_id: None,
                css: vec![],
                parent_path: None,
                is_root: true,
                additional_paths: None,
            }],
            loading: vec![],
            errors: vec![],
            not_found: vec![],
            templates: vec![],
            generated: "2026-01-01T00:00:00.000Z".to_string(),
        };
        let router = AppRouter::new(manifest);

        let layouts = router.resolve_layouts("/");
        assert_eq!(layouts.len(), 1);
    }

    #[test]
    fn test_find_loading_with_additional_paths() {
        let manifest = AppRouteManifest {
            loading: vec![LoadingEntry {
                path: "/about".to_string(),
                file_path: "(marketing)/loading.tsx".to_string(),
                component_id: None,
                css: vec![],
                additional_paths: Some(vec!["/pricing".to_string()]),
            }],
            ..build_minimal_manifest()
        };
        let router = AppRouter::new(manifest);

        let found = router.find_loading("/pricing");
        assert!(found.is_some());
        assert_eq!(found.unwrap().file_path, "(marketing)/loading.tsx");

        assert!(router.find_loading("/dashboard").is_none());
    }

    #[test]
    fn test_find_error_with_additional_paths() {
        let manifest = AppRouteManifest {
            errors: vec![ErrorEntry {
                path: "/about".to_string(),
                file_path: "(marketing)/error.tsx".to_string(),
                component_id: None,
                css: vec![],
                additional_paths: Some(vec!["/pricing".to_string()]),
            }],
            ..build_minimal_manifest()
        };
        let router = AppRouter::new(manifest);

        assert!(router.find_error("/pricing").is_some());
        assert!(router.find_error("/dashboard").is_none());
    }

    #[test]
    fn test_find_not_found_with_additional_paths() {
        let manifest = AppRouteManifest {
            not_found: vec![NotFoundEntry {
                path: "/about".to_string(),
                file_path: "(marketing)/not-found.tsx".to_string(),
                component_id: None,
                css: vec![],
                additional_paths: Some(vec!["/pricing".to_string()]),
            }],
            ..build_minimal_manifest()
        };
        let router = AppRouter::new(manifest);

        assert!(router.find_not_found("/pricing").is_some());
        assert!(router.find_not_found("/dashboard").is_none());
    }

    #[test]
    fn test_match_route_uses_nearest_group_boundary() {
        let manifest = AppRouteManifest {
            routes: vec![AppRouteEntry {
                path: "/forgot".to_string(),
                file_path: "(auth)/(flow)/forgot/page.tsx".to_string(),
                component_id: None,
                css: vec![],
                segments: vec![RouteSegment {
                    segment_type: RouteSegmentType::Static,
                    value: "forgot".to_string(),
                    param: None,
                }],
                params: vec![],
                is_dynamic: false,
                static_params: None,
            }],
            loading: vec![
                LoadingEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/loading.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    additional_paths: None,
                },
                LoadingEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/(flow)/loading.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    additional_paths: None,
                },
            ],
            errors: vec![
                ErrorEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/error.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    additional_paths: None,
                },
                ErrorEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/(flow)/error.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    additional_paths: None,
                },
            ],
            not_found: vec![
                NotFoundEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/not-found.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    additional_paths: None,
                },
                NotFoundEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/(flow)/not-found.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    additional_paths: None,
                },
            ],
            ..build_minimal_manifest()
        };
        let router = AppRouter::new(manifest);

        let matched = router.match_route("/forgot").unwrap();
        assert_eq!(matched.loading.unwrap().file_path, "(auth)/(flow)/loading.tsx");
        assert_eq!(matched.error.unwrap().file_path, "(auth)/(flow)/error.tsx");

        let not_found = router.find_not_found("/forgot").unwrap();
        assert_eq!(not_found.file_path, "(auth)/(flow)/not-found.tsx");
    }

    #[test]
    fn test_not_found_match_includes_all_group_layouts_for_same_path() {
        let manifest = AppRouteManifest {
            layouts: vec![
                LayoutEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/layout.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    parent_path: Some("/".to_string()),
                    is_root: false,
                    additional_paths: None,
                },
                LayoutEntry {
                    path: "/forgot".to_string(),
                    file_path: "(auth)/(flow)/layout.tsx".to_string(),
                    component_id: None,
                    css: vec![],
                    parent_path: Some("/".to_string()),
                    is_root: false,
                    additional_paths: None,
                },
            ],
            not_found: vec![NotFoundEntry {
                path: "/forgot".to_string(),
                file_path: "(auth)/(flow)/not-found.tsx".to_string(),
                component_id: None,
                css: vec![],
                additional_paths: None,
            }],
            ..build_minimal_manifest()
        };
        let router = AppRouter::new(manifest);

        let matched = router.create_not_found_match("/forgot").unwrap();
        let layout_paths: Vec<_> =
            matched.layouts.iter().map(|layout| layout.file_path.as_str()).collect();

        assert_eq!(layout_paths, vec!["(auth)/layout.tsx", "(auth)/(flow)/layout.tsx"]);
    }

    #[test]
    fn test_create_not_found_match_uses_requested_path_for_additional_paths() {
        let manifest = AppRouteManifest {
            layouts: vec![LayoutEntry {
                path: "/about".to_string(),
                file_path: "(marketing)/layout.tsx".to_string(),
                component_id: None,
                css: vec![],
                parent_path: Some("/".to_string()),
                is_root: false,
                additional_paths: Some(vec!["/pricing".to_string()]),
            }],
            loading: vec![LoadingEntry {
                path: "/about".to_string(),
                file_path: "(marketing)/loading.tsx".to_string(),
                component_id: None,
                css: vec![],
                additional_paths: Some(vec!["/pricing".to_string()]),
            }],
            errors: vec![ErrorEntry {
                path: "/about".to_string(),
                file_path: "(marketing)/error.tsx".to_string(),
                component_id: None,
                css: vec![],
                additional_paths: Some(vec!["/pricing".to_string()]),
            }],
            not_found: vec![NotFoundEntry {
                path: "/about".to_string(),
                file_path: "(marketing)/not-found.tsx".to_string(),
                component_id: None,
                css: vec![],
                additional_paths: Some(vec!["/pricing".to_string()]),
            }],
            ..build_minimal_manifest()
        };
        let router = AppRouter::new(manifest);

        let matched = router.create_not_found_match("/pricing").unwrap();
        assert_eq!(matched.pathname, "/pricing");
        assert_eq!(matched.not_found.unwrap().file_path, "(marketing)/not-found.tsx");
        assert_eq!(matched.layouts[0].file_path, "(marketing)/layout.tsx");
        assert_eq!(matched.loading.unwrap().file_path, "(marketing)/loading.tsx");
        assert_eq!(matched.error.unwrap().file_path, "(marketing)/error.tsx");
    }

    fn template_entry(path: &str, file_path: &str) -> TemplateEntry {
        TemplateEntry {
            path: path.to_string(),
            file_path: file_path.to_string(),
            component_id: None,
            css: vec![],
            parent_path: None,
            additional_paths: None,
        }
    }

    #[test]
    fn test_resolve_templates_chain() {
        let mut manifest = create_test_manifest();
        manifest.templates =
            vec![template_entry("/", "template.tsx"), template_entry("/blog", "blog/template.tsx")];
        let router = AppRouter::new(manifest);

        let templates = router.resolve_templates("/blog/hello-world");

        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].path, "/");
        assert_eq!(templates[1].path, "/blog");
    }

    #[test]
    fn test_resolve_templates_root_only() {
        let mut manifest = create_test_manifest();
        manifest.templates = vec![template_entry("/", "template.tsx")];
        let router = AppRouter::new(manifest);

        let templates = router.resolve_templates("/about");

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].path, "/");
    }

    #[test]
    fn test_resolve_templates_honors_additional_paths() {
        let mut manifest = create_test_manifest();
        let mut tpl = template_entry("/contact", "(_public)/template.tsx");
        tpl.additional_paths = Some(vec!["/pricing".to_string()]);
        manifest.templates = vec![tpl];
        let router = AppRouter::new(manifest);

        assert_eq!(router.resolve_templates("/contact").len(), 1);
        assert_eq!(router.resolve_templates("/pricing").len(), 1);
        assert_eq!(router.resolve_templates("/other").len(), 0);
    }

    #[test]
    fn test_resolve_templates_for_route_includes_ancestor_templates() {
        let mut manifest = create_test_manifest();
        manifest.templates = vec![template_entry("/dashboard", "dashboard/template.tsx")];
        let router = AppRouter::new(manifest);

        let templates = router.resolve_templates_for_route(&AppRouteEntry {
            path: "/dashboard/settings".to_string(),
            file_path: "dashboard/settings/page.tsx".to_string(),
            component_id: None,
            css: vec![],
            segments: vec![],
            params: vec![],
            is_dynamic: false,
            static_params: None,
        });

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].path, "/dashboard");
    }

    #[test]
    fn test_match_route_populates_templates() {
        let mut manifest = create_test_manifest();
        manifest.templates =
            vec![template_entry("/", "template.tsx"), template_entry("/blog", "blog/template.tsx")];
        let router = AppRouter::new(manifest);

        let matched = router.match_route("/blog/hello-world").unwrap();
        assert_eq!(matched.templates.len(), 2);
        assert_eq!(matched.templates[0].path, "/");
        assert_eq!(matched.templates[1].path, "/blog");
    }

    #[test]
    fn test_create_not_found_match_populates_templates() {
        let mut manifest = create_test_manifest();
        manifest.not_found = vec![NotFoundEntry {
            path: "/".to_string(),
            file_path: "not-found.tsx".to_string(),
            component_id: None,
            css: vec![],
            additional_paths: None,
        }];
        manifest.templates = vec![template_entry("/", "template.tsx")];
        let router = AppRouter::new(manifest);

        let matched = router.create_not_found_match("/missing").unwrap();
        assert_eq!(matched.templates.len(), 1);
        assert_eq!(matched.templates[0].path, "/");
    }
}
