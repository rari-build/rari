use crate::error::{LoadingStateError, RariError};
use crate::rsc::elements::ReactElement;
use crate::rsc::renderer::RscRenderer;
use crate::rsc::streaming::RscStream;
use crate::server::app_router::AppRouteMatch;
use crate::server::config::Config;
use crate::server::request_type::RenderMode;
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, warn};

#[derive(Debug, Clone)]
pub struct LayoutRenderContext {
    pub params: FxHashMap<String, String>,
    pub search_params: FxHashMap<String, Vec<String>>,
    pub headers: FxHashMap<String, String>,
    pub pathname: String,
}

#[derive(Debug, Clone)]
pub struct BoundaryInfo {
    pub id: String,
    pub has_fallback: bool,
}

#[derive(Debug, Clone)]
pub struct SuspenseDetectionResult {
    pub has_suspense: bool,
    pub boundary_count: usize,
    pub boundaries: Vec<BoundaryInfo>,
}

pub enum RenderResult {
    Static(String),
    StaticWithPayload { html: String, rsc_payload: String },
    Streaming(RscStream),
}

#[derive(Debug, Clone)]
pub struct BoundaryPosition {
    pub boundary_id: String,
    pub parent_path: Vec<usize>,
    pub is_in_content_area: bool,
    pub dom_path: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct LayoutStructure {
    pub has_navigation: bool,
    pub navigation_position: Option<usize>,
    pub content_position: Option<usize>,
    pub suspense_boundaries: Vec<BoundaryPosition>,
}

impl LayoutStructure {
    pub fn new() -> Self {
        Self {
            has_navigation: false,
            navigation_position: None,
            content_position: None,
            suspense_boundaries: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.has_navigation
            && let (Some(nav_pos), Some(content_pos)) =
                (self.navigation_position, self.content_position)
            && nav_pos >= content_pos
        {
            return false;
        }

        for boundary in &self.suspense_boundaries {
            if !boundary.is_in_content_area {
                return false;
            }
        }

        true
    }
}

impl Default for LayoutStructure {
    fn default() -> Self {
        Self::new()
    }
}

fn calculate_boundary_positions(
    layout_structure: &LayoutStructure,
) -> FxHashMap<String, Vec<usize>> {
    let mut positions = FxHashMap::default();

    for boundary in &layout_structure.suspense_boundaries {
        let mut dom_path = Vec::new();

        if layout_structure.has_navigation {
            if let Some(_nav_pos) = layout_structure.navigation_position
                && boundary.is_in_content_area
            {
                dom_path.push(1);
            }
        } else if boundary.is_in_content_area {
            dom_path.push(0);
        }

        for &index in &boundary.parent_path {
            dom_path.push(index);
        }

        positions.insert(boundary.boundary_id.clone(), dom_path);
    }

    positions
}

struct HtmlCache {
    cache: DashMap<u64, String>,
}

impl HtmlCache {
    fn new() -> Self {
        Self { cache: DashMap::new() }
    }

    fn get(&self, key: u64) -> Option<String> {
        self.cache.get(&key).map(|v| v.clone())
    }

    fn insert(&self, key: u64, html: String) {
        self.cache.insert(key, html);
    }
}

pub struct LayoutRenderer {
    renderer: Arc<tokio::sync::Mutex<RscRenderer>>,
    html_cache: Arc<HtmlCache>,
}

impl LayoutRenderer {
    pub fn new(renderer: Arc<tokio::sync::Mutex<RscRenderer>>) -> Self {
        Self { renderer, html_cache: Arc::new(HtmlCache::new()) }
    }

    fn generate_cache_key(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();

        route_match.route.path.hash(&mut hasher);

        let mut params: Vec<_> = context.params.iter().collect();
        params.sort_by_key(|(k, _)| *k);
        for (k, v) in params {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }

        let mut search_params: Vec<_> = context.search_params.iter().collect();
        search_params.sort_by_key(|(k, _)| *k);
        for (k, v) in search_params {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }

        hasher.finish()
    }

    pub async fn render_route(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        _request_context: Option<std::sync::Arc<crate::server::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            if let Some(loading_entry) = &route_match.loading {
                let loading_id = self.create_component_id(&loading_entry.file_path);
                Some(loading_id)
            } else {
                None
            }
        } else {
            None
        };
        let composition_script = self.build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            false,
        )?;

        let renderer = self.renderer.lock().await;

        if let Some(ref ctx) = _request_context
            && let Err(e) = renderer.runtime.set_request_context(ctx.clone()).await
        {
            tracing::warn!("Failed to set request context in runtime: {}", e);
        }

        let promise_result = renderer
            .runtime
            .execute_script("compose_and_render".to_string(), composition_script)
            .await?;

        let result = if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
            promise_result
        } else {
            let get_result_script = r#"globalThis.__rsc_render_result"#.to_string();
            renderer.runtime.execute_script("get_result".to_string(), get_result_script).await?
        };

        let rsc_data = result.get("rsc_data").ok_or_else(|| {
            tracing::error!("Failed to extract RSC data from result: {:?}", result);
            RariError::internal("No RSC data in render result")
        })?;

        let rsc_wire_format = {
            let mut serializer = renderer.serializer.lock();
            serializer
                .serialize_rsc_json(rsc_data)
                .map_err(|e| RariError::internal(format!("Failed to serialize RSC data: {e}")))?
        };

        if let Err(e) = Self::validate_html_structure(&rsc_wire_format, route_match) {
            tracing::warn!("HTML structure validation warning: {}", e);
        }

        Ok(rsc_wire_format)
    }

    pub async fn render_route_optimized(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        mode: RenderMode,
        request_context: Option<std::sync::Arc<crate::server::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        match mode {
            RenderMode::Ssr => {
                match self
                    .render_route_to_html_direct(route_match, context, request_context)
                    .await?
                {
                    RenderResult::Static(html) => Ok(html),
                    RenderResult::StaticWithPayload { html, .. } => Ok(html),
                    RenderResult::Streaming(_) => Err(RariError::internal(
                        "Streaming result not supported in render_route_optimized",
                    )),
                }
            }
            RenderMode::RscNavigation => {
                self.render_route(route_match, context, request_context).await
            }
        }
    }

    fn validate_skeleton_positions(
        &self,
        layout_structure: &LayoutStructure,
    ) -> Result<(), RariError> {
        for boundary in &layout_structure.suspense_boundaries {
            if !boundary.is_in_content_area {
                warn!(
                    "Skeleton position validation failed: boundary '{}' is not in content area. This may cause layout shifts.",
                    boundary.boundary_id
                );
                return Err(RariError::internal(format!(
                    "Skeleton position validation failed: boundary '{}' is not in content area",
                    boundary.boundary_id
                )));
            }

            if boundary.dom_path.is_empty() {
                warn!(
                    "Skeleton position validation failed: boundary '{}' has empty DOM path. Cannot ensure position stability.",
                    boundary.boundary_id
                );
                return Err(RariError::internal(format!(
                    "Skeleton position validation failed: boundary '{}' has empty DOM path",
                    boundary.boundary_id
                )));
            }
        }

        Ok(())
    }

    fn validate_layout_structure(
        &self,
        rsc_data: &serde_json::Value,
        route_match: &AppRouteMatch,
    ) -> Result<LayoutStructure, RariError> {
        let mut layout_structure = LayoutStructure::new();

        fn traverse(
            value: &serde_json::Value,
            structure: &mut LayoutStructure,
            path: &mut Vec<usize>,
            position: &mut usize,
            in_content_area: bool,
        ) {
            match value {
                serde_json::Value::Array(arr) => {
                    if arr.len() >= 4 && arr[0].as_str() == Some("$") {
                        let tag = arr[1].as_str().unwrap_or("");

                        if tag == "nav"
                            || (tag == "header"
                                && arr.get(3).and_then(|p| p.get("role")).and_then(|r| r.as_str())
                                    == Some("navigation"))
                        {
                            structure.has_navigation = true;
                            if structure.navigation_position.is_none() {
                                structure.navigation_position = Some(*position);
                            }
                        }

                        if (tag == "main"
                            || tag == "article"
                            || (tag == "section"
                                && arr.get(3).and_then(|p| p.get("role")).and_then(|r| r.as_str())
                                    == Some("main")))
                            && structure.content_position.is_none()
                        {
                            structure.content_position = Some(*position);
                        }

                        if (tag == "react.suspense" || tag == "Suspense")
                            && let Some(props) = arr.get(3).and_then(|v| v.as_object())
                            && let Some(boundary_id) = props
                                .get("boundaryId")
                                .or_else(|| props.get("__boundary_id"))
                                .and_then(|v| v.as_str())
                        {
                            structure.suspense_boundaries.push(BoundaryPosition {
                                boundary_id: boundary_id.to_string(),
                                parent_path: path.clone(),
                                is_in_content_area: in_content_area
                                    || structure.content_position.is_some(),
                                dom_path: Vec::new(),
                            });
                        }

                        *position += 1;
                    }

                    for (idx, item) in arr.iter().enumerate() {
                        path.push(idx);
                        traverse(
                            item,
                            structure,
                            path,
                            position,
                            in_content_area || structure.content_position.is_some(),
                        );
                        path.pop();
                    }
                }
                serde_json::Value::Object(obj) => {
                    if obj.contains_key("__preSerializedSuspense") {
                        if let Some(rsc_array) = obj.get("rscArray") {
                            traverse(rsc_array, structure, path, position, in_content_area);
                        }
                    } else {
                        for (_key, value) in obj.iter() {
                            path.push(0);
                            traverse(value, structure, path, position, in_content_area);
                            path.pop();
                        }
                    }
                }
                _ => {}
            }
        }

        let mut path = Vec::new();
        let mut position = 0;
        traverse(rsc_data, &mut layout_structure, &mut path, &mut position, false);

        let boundary_positions = calculate_boundary_positions(&layout_structure);

        for boundary in &mut layout_structure.suspense_boundaries {
            if let Some(dom_path) = boundary_positions.get(&boundary.boundary_id) {
                boundary.dom_path = dom_path.clone();
            }
        }

        if !layout_structure.is_valid() {
            error!(
                "❌ INVALID LAYOUT STRUCTURE for route '{}'\n\
                 \n\
                 ╔═══════════════════════════════════════════════════════════════════════════╗\n\
                 ║ LAYOUT VALIDATION FAILED                                                  ║\n\
                 ╚═══════════════════════════════════════════════════════════════════════════╝\n\
                 \n\
                 📊 LAYOUT STRUCTURE:\n\
                 • Has navigation: {}\n\
                 • Navigation position: {:?}\n\
                 • Content position: {:?}\n\
                 • Suspense boundaries: {}\n\
                 \n\
                 🎯 BOUNDARY POSITIONS:",
                route_match.route.path,
                layout_structure.has_navigation,
                layout_structure.navigation_position,
                layout_structure.content_position,
                layout_structure.suspense_boundaries.len()
            );

            for (idx, boundary) in layout_structure.suspense_boundaries.iter().enumerate() {
                error!(
                    "   {}. Boundary '{}'\n\
                     • Parent path: {:?}\n\
                     • DOM path: {:?}\n\
                     • In content area: {}\n\
                     • Position: {}",
                    idx + 1,
                    boundary.boundary_id,
                    boundary.parent_path,
                    boundary.dom_path,
                    boundary.is_in_content_area,
                    idx
                );
            }

            if layout_structure.has_navigation
                && let (Some(nav_pos), Some(content_pos)) =
                    (layout_structure.navigation_position, layout_structure.content_position)
                && nav_pos >= content_pos
            {
                error!(
                    "\n\
                     ❌ ISSUE: Navigation position ({}) is not before content position ({})\n\
                     \n\
                     🔍 TROUBLESHOOTING:\n\
                     • Navigation elements must appear before main content in the DOM\n\
                     • Check your layout component structure\n\
                     • Ensure <nav> or navigation elements come before <main>\n\
                     \n\
                     💡 SOLUTION:\n\
                     • Reorder your layout components so navigation comes first\n\
                     • Use semantic HTML: <nav> before <main>",
                    nav_pos, content_pos
                );
            }

            for boundary in &layout_structure.suspense_boundaries {
                if !boundary.is_in_content_area {
                    error!(
                        "\n\
                         ❌ ISSUE: Suspense boundary '{}' is not in content area\n\
                         • Parent path: {:?}\n\
                         • DOM path: {:?}\n\
                         \n\
                         🔍 TROUBLESHOOTING:\n\
                         • Suspense boundaries must be inside the main content area\n\
                         • Boundaries outside content area can cause layout shifts\n\
                         • This violates proper Suspense boundary positioning\n\
                         \n\
                         💡 SOLUTION:\n\
                         • Move Suspense boundaries inside <main> or content area\n\
                         • Ensure boundaries are wrapped by content-area elements\n\
                         • Check that boundaries aren't in navigation or header",
                        boundary.boundary_id, boundary.parent_path, boundary.dom_path
                    );
                }
            }

            error!(
                "\n\
                 📚 REQUIREMENTS:\n\
                 • Navigation must precede content (layout best practice)\n\
                 • Suspense boundaries must be in content area\n\
                 • Proper positioning prevents layout shifts and hydration errors\n\
                 \n\
                 ═══════════════════════════════════════════════════════════════════════════"
            );
        }

        Ok(layout_structure)
    }

    fn detect_suspense_boundaries(
        &self,
        rsc_json: &serde_json::Value,
    ) -> Result<SuspenseDetectionResult, RariError> {
        let mut boundary_count = 0;
        let mut boundaries = Vec::new();

        fn traverse(
            value: &serde_json::Value,
            boundaries: &mut Vec<BoundaryInfo>,
            count: &mut usize,
        ) {
            match value {
                serde_json::Value::Array(arr) => {
                    if arr.len() >= 4
                        && arr[0].as_str() == Some("$")
                        && (arr[1].as_str() == Some("react.suspense")
                            || arr[1].as_str() == Some("Suspense"))
                    {
                        *count += 1;

                        if let Some(props) = arr.get(3).and_then(|v| v.as_object())
                            && let Some(boundary_id) = props
                                .get("boundaryId")
                                .or_else(|| props.get("__boundary_id"))
                                .and_then(|v| v.as_str())
                        {
                            boundaries.push(BoundaryInfo {
                                id: boundary_id.to_string(),
                                has_fallback: props.contains_key("fallback"),
                            });
                        }
                    }

                    for item in arr {
                        traverse(item, boundaries, count);
                    }
                }
                serde_json::Value::Object(obj) => {
                    if obj.contains_key("__preSerializedSuspense") {
                        if let Some(rsc_array) = obj.get("rscArray") {
                            traverse(rsc_array, boundaries, count);
                        }
                    } else {
                        for value in obj.values() {
                            traverse(value, boundaries, count);
                        }
                    }
                }
                _ => {}
            }
        }

        traverse(rsc_json, &mut boundaries, &mut boundary_count);

        Ok(SuspenseDetectionResult { has_suspense: boundary_count > 0, boundary_count, boundaries })
    }

    pub async fn render_route_to_html_direct(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        _request_context: Option<std::sync::Arc<crate::server::request_context::RequestContext>>,
    ) -> Result<RenderResult, RariError> {
        let cache_key = self.generate_cache_key(route_match, context);

        if let Some(cached_html) = self.html_cache.get(cache_key) {
            return Ok(RenderResult::Static(cached_html));
        }

        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            if let Some(loading_entry) = &route_match.loading {
                let loading_id = self.create_component_id(&loading_entry.file_path);
                Some(loading_id)
            } else {
                None
            }
        } else {
            None
        };

        let composition_script = self.build_composition_script(
            route_match,
            context,
            loading_component_id.as_deref(),
            true,
        )?;

        let renderer = self.renderer.lock().await;

        if let Some(ref ctx) = _request_context
            && let Err(e) = renderer.runtime.set_request_context(ctx.clone()).await
        {
            tracing::warn!("Failed to set request context in runtime: {}", e);
        }

        let promise_result = renderer
            .runtime
            .execute_script("compose_and_render".to_string(), composition_script.clone())
            .await?;

        let result = if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
            promise_result
        } else {
            let get_result_script = r#"globalThis.__rsc_render_result"#.to_string();
            renderer.runtime.execute_script("get_result".to_string(), get_result_script).await?
        };

        let rsc_data = result.get("rsc_data").ok_or_else(|| {
            tracing::error!("Failed to extract RSC data from result: {:?}", result);
            RariError::internal("No RSC data in render result")
        })?;

        let layout_structure = match self.validate_layout_structure(rsc_data, route_match) {
            Ok(structure) => {
                if !structure.is_valid() {
                    warn!(
                        "Layout structure validation failed for route '{}'. Falling back to non-streaming rendering.",
                        route_match.route.path
                    );

                    warn!(
                        "Component tree for route '{}': {} layouts, page: {}",
                        route_match.route.path,
                        route_match.layouts.len(),
                        route_match.route.file_path
                    );

                    for (idx, layout) in route_match.layouts.iter().enumerate() {
                        warn!(
                            "  Layout {}: {} (is_root: {})",
                            idx, layout.file_path, layout.is_root
                        );
                    }

                    structure
                } else {
                    if let Err(e) = self.validate_skeleton_positions(&structure) {
                        warn!(
                            "Skeleton position validation failed for route '{}': {}. Falling back to non-streaming rendering.",
                            route_match.route.path, e
                        );
                    }

                    structure
                }
            }
            Err(e) => {
                error!(
                    "Layout structure validation error for route '{}': {}",
                    route_match.route.path, e
                );

                error!(
                    "Component tree for route '{}': {} layouts, page: {}",
                    route_match.route.path,
                    route_match.layouts.len(),
                    route_match.route.file_path
                );

                for (idx, layout) in route_match.layouts.iter().enumerate() {
                    error!("  Layout {}: {} (is_root: {})", idx, layout.file_path, layout.is_root);
                }

                warn!(
                    "Falling back to static rendering for route {} due to validation error",
                    route_match.route.path
                );

                LayoutStructure::new()
            }
        };

        let suspense_detection = self.detect_suspense_boundaries(rsc_data)?;

        if suspense_detection.boundary_count > 0 {
            let rsc_str = serde_json::to_string(rsc_data).unwrap_or_default();
            let boundary_occurrences = rsc_str.matches("react.suspense").count();
            if boundary_occurrences != suspense_detection.boundary_count {
                tracing::warn!(
                    "⚠️ RSC data contains {} occurrences of 'react.suspense' but detected {} unique boundaries. Possible duplicates in RSC data!",
                    boundary_occurrences,
                    suspense_detection.boundary_count
                );
            }
        }

        if suspense_detection.has_suspense && layout_structure.is_valid() {
            tracing::info!(
                "Found {} Suspense boundaries with valid layout structure, enabling streaming for route {}",
                suspense_detection.boundary_count,
                route_match.route.path
            );

            let mut streaming_renderer =
                crate::rsc::streaming::StreamingRenderer::new(Arc::clone(&renderer.runtime));

            let stream = streaming_renderer
                .start_streaming_with_precomputed_data(
                    rsc_data.clone(),
                    suspense_detection.boundaries.clone(),
                    layout_structure,
                )
                .await?;

            return Ok(RenderResult::Streaming(stream));
        } else if suspense_detection.has_suspense && !layout_structure.is_valid() {
            warn!(
                "Found {} Suspense boundaries but layout structure is invalid, falling back to static rendering for route {}",
                suspense_detection.boundary_count, route_match.route.path
            );
        }

        let page_props = self.create_page_props(route_match, context)?;
        let page_component_id = self.create_component_id(&route_match.route.file_path);

        #[allow(clippy::disallowed_methods)]
        let layouts: Vec<serde_json::Value> = route_match
            .layouts
            .iter()
            .map(|layout| {
                serde_json::json!({
                    "componentId": self.create_component_id(&layout.file_path),
                    "isRoot": layout.is_root
                })
            })
            .collect();

        let result = renderer
            .runtime
            .execute_function(
                "renderRouteToHtml",
                vec![
                    serde_json::Value::String(page_component_id.clone()),
                    page_props.clone(),
                    serde_json::Value::Array(layouts),
                ],
            )
            .await?;

        if let Some(error) = result.get("error").and_then(|v| v.as_str())
            && !error.is_empty()
            && error != "null"
        {
            tracing::error!("JavaScript error: {}", error);
            return Err(RariError::internal(format!("JavaScript error: {}", error)));
        }

        let html = result
            .get("html")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                tracing::error!("Failed to extract HTML from result: {:?}", result);
                RariError::internal("No HTML in render result")
            })?
            .to_string();

        let rsc_payload = result.get("rsc").and_then(|v| v.as_str()).unwrap_or("").to_string();

        self.html_cache.insert(cache_key, html.clone());

        if !rsc_payload.is_empty() {
            Ok(RenderResult::StaticWithPayload { html, rsc_payload })
        } else {
            Ok(RenderResult::Static(html))
        }
    }

    fn validate_html_structure(html: &str, route_match: &AppRouteMatch) -> Result<(), RariError> {
        let root_layout_path =
            route_match.layouts.iter().find(|l| l.is_root).map(|l| l.file_path.as_str());

        if html.contains("<div><html")
            || html.contains("\"div\",null,{\"children\":[\"$\",\"html\"")
        {
            let error_msg = Self::create_wrapped_html_error_message(route_match, root_layout_path);
            return Err(RariError::internal(error_msg));
        }

        Ok(())
    }

    fn create_wrapped_html_error_message(
        route_match: &AppRouteMatch,
        root_layout_path: Option<&str>,
    ) -> String {
        let mut msg =
            String::from("🚨 React Hydration Mismatch Detected: Wrapped HTML Element\n\n");

        msg.push_str(
            "╔═══════════════════════════════════════════════════════════════════════════╗\n",
        );
        msg.push_str(
            "║ WHAT HAPPENED:                                                            ║\n",
        );
        msg.push_str(
            "╚═══════════════════════════════════════════════════════════════════════════╝\n",
        );
        msg.push_str("The root layout's <html> element is being wrapped in a container element.\n");
        msg.push_str(
            "This causes React hydration to fail because the server-rendered HTML structure\n",
        );
        msg.push_str("doesn't match what React expects on the client side.\n\n");

        msg.push_str(
            "React hydration is the process where React attaches event listeners and makes\n",
        );
        msg.push_str(
            "the server-rendered HTML interactive. When the structures don't match, React\n",
        );
        msg.push_str(
            "can't properly hydrate, leading to frozen inputs, broken interactions, and\n",
        );
        msg.push_str("console warnings.\n\n");

        msg.push_str("📍 AFFECTED FILES:\n");
        msg.push_str(&format!("   Route:        {}\n", route_match.route.path));
        msg.push_str(&format!("   Page:         {}\n", route_match.route.file_path));

        if let Some(layout_path) = root_layout_path {
            msg.push_str(&format!("   Root Layout:  {} ← CHECK THIS FILE\n", layout_path));
        }

        msg.push_str("\n🔧 HOW TO FIX (Step-by-Step):\n");
        msg.push_str("   Step 1: Open your root layout file");
        if let Some(layout_path) = root_layout_path {
            msg.push_str(&format!(" ({})", layout_path));
        }
        msg.push('\n');
        msg.push_str("   Step 2: Find the component's return statement\n");
        msg.push_str("   Step 3: Ensure <html> is the FIRST element returned (no wrapper divs)\n");
        msg.push_str("   Step 4: Verify <body> is a direct child of <html>\n");
        msg.push_str("   Step 5: Save the file and restart your dev server\n\n");

        msg.push_str("✅ CORRECT STRUCTURE:\n");
        msg.push_str("   export default function RootLayout({ children }) {\n");
        msg.push_str("     return (\n");
        msg.push_str("       <html lang=\"en\">  {/* ← html is the outermost element */}\n");
        msg.push_str("         <head>\n");
        msg.push_str("           <meta charSet=\"utf-8\" />\n");
        msg.push_str("         </head>\n");
        msg.push_str("         <body>{children}</body>\n");
        msg.push_str("       </html>\n");
        msg.push_str("     );\n");
        msg.push_str("   }\n\n");

        msg.push_str("❌ INCORRECT STRUCTURE (causes this error):\n");
        msg.push_str("   export default function RootLayout({ children }) {\n");
        msg.push_str("     return (\n");
        msg.push_str("       <div>  {/* ← PROBLEM: Don't wrap html in div! */}\n");
        msg.push_str("         <html lang=\"en\">\n");
        msg.push_str("           <body>{children}</body>\n");
        msg.push_str("         </html>\n");
        msg.push_str("       </div>\n");
        msg.push_str("     );\n");
        msg.push_str("   }\n\n");

        if !route_match.layouts.is_empty() {
            msg.push_str("📂 COMPONENT HIERARCHY:\n");
            msg.push_str("   Your current component tree for this route:\n\n");
            for (idx, layout) in route_match.layouts.iter().enumerate() {
                let indent = "   ".repeat(idx + 1);
                let marker = if layout.is_root { " ← ROOT LAYOUT (check this!)" } else { "" };
                msg.push_str(&format!("   {}└─ {}{}\n", indent, layout.file_path, marker));
            }
            msg.push_str(&format!(
                "   {}└─ {} (page component)\n",
                "   ".repeat(route_match.layouts.len() + 1),
                route_match.route.file_path
            ));
            msg.push('\n');
        }

        msg.push_str("💡 WHY THIS MATTERS:\n");
        msg.push_str("   • Input fields will freeze and become unresponsive\n");
        msg.push_str("   • Form submissions may fail\n");
        msg.push_str("   • Interactive elements won't work properly\n");
        msg.push_str("   • React will log hydration warnings in the browser console\n\n");

        msg.push_str("📚 LEARN MORE:\n");
        msg.push_str(
            "   • React Hydration: https://react.dev/reference/react-dom/client/hydrateRoot\n",
        );
        msg.push_str("   • Root Layout Pattern: Check your framework's documentation for root layout structure\n");

        msg
    }

    fn validate_rsc_wire_format(rsc_data: &str) -> Result<(), RariError> {
        if rsc_data.trim().is_empty() {
            let error_msg = Self::create_empty_rsc_error_message();
            return Err(RariError::internal(error_msg));
        }

        let trimmed = rsc_data.trim_start();
        if !trimmed.starts_with(char::is_numeric) {
            let warning_msg = Self::create_invalid_rsc_format_warning(
                "missing row ID at start",
                &trimmed.chars().take(50).collect::<String>(),
            );
            tracing::warn!("{}", warning_msg);
        }

        if !rsc_data.contains("[\"$\"") {
            let warning_msg = Self::create_invalid_rsc_format_warning(
                "missing React element markers",
                "Expected to find [\"$\"] markers in the output",
            );
            tracing::warn!("{}", warning_msg);
        }

        let lines: Vec<&str> = rsc_data.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            if !line.starts_with(char::is_numeric) {
                let warning_msg = format!(
                    "⚠️  RSC Wire Format Warning: Invalid Line Format\n\n\
                    Line {} does not start with a row ID.\n\n\
                    📍 Line Content (first 50 chars):\n   {}\n\n\
                    💡 Expected Format:\n   \
                    Each line should start with a numeric row ID followed by a colon.\n   \
                    Example: 0:[\"$\",\"html\",null,{{...}}]\n",
                    idx,
                    &line.chars().take(50).collect::<String>()
                );
                tracing::warn!("{}", warning_msg);
            }
        }

        Ok(())
    }

    fn create_empty_rsc_error_message() -> String {
        let mut msg = String::from("🚨 RSC Streaming Error: Empty Content\n\n");

        msg.push_str(
            "╔═══════════════════════════════════════════════════════════════════════════╗\n",
        );
        msg.push_str(
            "║ RENDERING ERROR:                                                          ║\n",
        );
        msg.push_str(
            "╚═══════════════════════════════════════════════════════════════════════════╝\n",
        );
        msg.push_str(
            "The RSC renderer produced empty output, which cannot be streamed to the client.\n",
        );
        msg.push_str("This means your component didn't render anything, or the rendering process failed.\n\n");

        msg.push_str("💡 COMMON CAUSES:\n");
        msg.push_str("   1. Component returned null, undefined, or false\n");
        msg.push_str("   2. Layout or page component failed to render (check for errors)\n");
        msg.push_str("   3. RSC serialization failed silently\n");
        msg.push_str("   4. Missing 'export default' in your component file\n");
        msg.push_str("   5. Async component that threw an error\n\n");

        msg.push_str("🔧 HOW TO FIX (Step-by-Step):\n");
        msg.push_str("   Step 1: Check your server logs for any error messages\n");
        msg.push_str("   Step 2: Verify your page component exports a default function:\n");
        msg.push_str("           • File should have: export default function Page() { ... }\n");
        msg.push_str("   Step 3: Ensure your component returns valid JSX:\n");
        msg.push_str("           • Must return a React element, not null/undefined\n");
        msg.push_str("   Step 4: If using async components, verify they properly await data:\n");
        msg.push_str("           • export default async function Page() { await data; return <div>...</div>; }\n");
        msg.push_str(
            "   Step 5: Check that all layout components in the chain return valid elements\n\n",
        );

        msg.push_str("✅ VALID COMPONENT EXAMPLES:\n\n");
        msg.push_str("   Basic Page Component:\n");
        msg.push_str("   ─────────────────────\n");
        msg.push_str("   export default function Page() {\n");
        msg.push_str("     return <div>Hello World</div>;\n");
        msg.push_str("   }\n\n");

        msg.push_str("   Async Page Component:\n");
        msg.push_str("   ─────────────────────\n");
        msg.push_str("   export default async function Page() {\n");
        msg.push_str("     const data = await fetchData();\n");
        msg.push_str("     return <div>{data.title}</div>;\n");
        msg.push_str("   }\n\n");

        msg.push_str("   Layout Component:\n");
        msg.push_str("   ─────────────────\n");
        msg.push_str("   export default function Layout({ children }) {\n");
        msg.push_str("     return (\n");
        msg.push_str("       <div>\n");
        msg.push_str("         <nav>Navigation</nav>\n");
        msg.push_str("         {children}\n");
        msg.push_str("       </div>\n");
        msg.push_str("     );\n");
        msg.push_str("   }\n\n");

        msg.push_str("❌ INVALID EXAMPLES (cause this error):\n\n");
        msg.push_str("   Missing export:\n");
        msg.push_str("   ───────────────\n");
        msg.push_str("   function Page() {  // ← Missing 'export default'\n");
        msg.push_str("     return <div>Hello</div>;\n");
        msg.push_str("   }\n\n");

        msg.push_str("   Returning null:\n");
        msg.push_str("   ───────────────\n");
        msg.push_str("   export default function Page() {\n");
        msg.push_str("     return null;  // ← Don't return null\n");
        msg.push_str("   }\n\n");

        msg.push_str("🔍 DEBUGGING CHECKLIST:\n");
        msg.push_str("   □ Component file has 'export default'\n");
        msg.push_str("   □ Component returns JSX (not null/undefined)\n");
        msg.push_str("   □ No errors in server logs\n");
        msg.push_str("   □ Async components properly await data\n");
        msg.push_str("   □ All layout components in the chain are valid\n");

        msg
    }

    fn create_invalid_rsc_format_warning(issue: &str, details: &str) -> String {
        let mut msg = String::from("⚠️  RSC Wire Format Warning: Invalid Structure\n\n");

        msg.push_str(
            "╔═══════════════════════════════════════════════════════════════════════════╗\n",
        );
        msg.push_str(
            "║ SERIALIZATION WARNING:                                                    ║\n",
        );
        msg.push_str(
            "╚═══════════════════════════════════════════════════════════════════════════╝\n",
        );
        msg.push_str(&format!("Issue:   {}\n", issue));
        msg.push_str(&format!("Details: {}\n\n", details));

        msg.push_str("The React Server Component (RSC) wire format appears to be malformed.\n");
        msg.push_str("This could prevent the client from properly parsing and hydrating your components.\n\n");

        msg.push_str("📋 RSC WIRE FORMAT SPECIFICATION:\n");
        msg.push_str("   The React Server Component wire format should follow this structure:\n\n");
        msg.push_str("   • Each line starts with a numeric row ID followed by a colon\n");
        msg.push_str("     Example: 0:[...]\n\n");
        msg.push_str("   • React elements are represented as JSON arrays:\n");
        msg.push_str("     Format: [\"$\", \"tagName\", key, props]\n");
        msg.push_str("     Example: [\"$\", \"div\", null, {\"className\": \"container\"}]\n\n");
        msg.push_str("   • Multiple rows are separated by newlines\n");
        msg.push_str("   • Props can contain nested elements (children)\n\n");

        msg.push_str("✅ VALID RSC WIRE FORMAT EXAMPLES:\n\n");
        msg.push_str("   Simple element:\n");
        msg.push_str("   0:[\"$\",\"div\",null,{\"children\":\"Hello World\"}]\n\n");

        msg.push_str("   Nested structure:\n");
        msg.push_str("   0:[\"$\",\"html\",null,{\"lang\":\"en\",\"children\":[\"$\",\"body\",null,{\"children\":\"...\"}]}]\n\n");

        msg.push_str("   Multiple rows:\n");
        msg.push_str("   0:[\"$\",\"div\",null,{\"children\":[\"$L1\"]}]\n");
        msg.push_str("   1:[\"$\",\"span\",null,{\"children\":\"Text\"}]\n\n");

        msg.push_str("💡 WHAT THIS WARNING MAY INDICATE:\n");
        msg.push_str("   • A bug in the RSC serializer (internal framework issue)\n");
        msg.push_str("   • Corrupted component output from a malformed component\n");
        msg.push_str("   • Incompatible React version or missing dependencies\n");
        msg.push_str("   • Component returning invalid data types\n\n");

        msg.push_str("🔧 TROUBLESHOOTING STEPS:\n");
        msg.push_str("   Step 1: Check the server logs for any serialization errors\n");
        msg.push_str("   Step 2: Verify your components return valid React elements:\n");
        msg.push_str("           • Should return JSX, not plain objects or strings\n");
        msg.push_str("   Step 3: Ensure you're using a compatible version of React:\n");
        msg.push_str("           • Check package.json for react and react-dom versions\n");
        msg.push_str("   Step 4: Look for components that might return unexpected values:\n");
        msg.push_str(
            "           • Avoid returning raw objects, arrays, or primitives at the root\n",
        );
        msg.push_str("   Step 5: If the issue persists, this may be a framework bug:\n");
        msg.push_str("           • Report the issue with the full error details\n\n");

        msg.push_str("🔍 DEBUGGING TIPS:\n");
        msg.push_str("   • Enable verbose logging to see the full RSC output\n");
        msg.push_str("   • Check if specific components cause the issue (comment them out)\n");
        msg.push_str("   • Verify all imports are correct and components are properly defined\n");
        msg.push_str("   • Test with a minimal component to isolate the problem\n");

        msg
    }

    pub async fn render_route_streaming(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<RscStream, RariError> {
        let has_root_layout = route_match.layouts.iter().any(|l| l.is_root);

        if has_root_layout {
            let html = self.render_route(route_match, context, None).await?;

            Self::validate_rsc_wire_format(&html)?;

            let (tx, rx) = mpsc::channel(1);
            let _ = tx
                .send(crate::rsc::streaming::RscStreamChunk {
                    data: html.into_bytes(),
                    chunk_type: crate::rsc::streaming::RscChunkType::InitialShell,
                    row_id: 0,
                    is_final: true,
                })
                .await;
            drop(tx);

            Ok(RscStream::new(rx))
        } else {
            self.render_route_streaming_progressive(route_match, context).await
        }
    }

    async fn render_route_streaming_progressive(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<RscStream, RariError> {
        let html = self.render_route(route_match, context, None).await?;

        Self::validate_rsc_wire_format(&html)?;

        let (tx, rx) = mpsc::channel(1);
        let _ = tx
            .send(crate::rsc::streaming::RscStreamChunk {
                data: html.into_bytes(),
                chunk_type: crate::rsc::streaming::RscChunkType::InitialShell,
                row_id: 0,
                is_final: true,
            })
            .await;
        drop(tx);

        Ok(RscStream::new(rx))
    }

    pub fn build_composition_script(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        loading_component_id: Option<&str>,
        use_suspense: bool,
    ) -> Result<String, RariError> {
        let page_props = self.create_page_props(route_match, context).map_err(|e| {
            error!("Failed to create page props for route '{}': {}", route_match.route.path, e);
            RariError::internal(format!(
                "Failed to create page props for route '{}' (component: {}): {}",
                route_match.route.path, route_match.route.file_path, e
            ))
        })?;

        let page_props_json = serde_json::to_string(&page_props).map_err(|e| {
            error!("Failed to serialize page props for route '{}': {}", route_match.route.path, e);
            RariError::internal(format!(
                "Failed to serialize page props for route '{}' (component: {}): {}",
                route_match.route.path, route_match.route.file_path, e
            ))
        })?;

        let page_component_id = self.create_component_id(&route_match.route.file_path);

        let page_render_script = if let Some(loading_id) = loading_component_id {
            format!(
                r#"
                const startPage = performance.now();
                const PageComponent = globalThis["{}"];
                if (!PageComponent || typeof PageComponent !== 'function') {{
                    throw new Error('Page component {} not found');
                }}

                const LoadingComponent = globalThis["{}"];
                if (!LoadingComponent || typeof LoadingComponent !== 'function') {{
                    console.warn('Loading component {} not found, rendering page without Suspense');
                    const pageProps = {};
                    const pageResult = PageComponent(pageProps);
                    var pageElement = pageResult && typeof pageResult.then === 'function'
                        ? await pageResult
                        : pageResult;
                }} else {{
                    const pageProps = {};
                    const useSuspense = {};

                    const isAsync = PageComponent.constructor.name === 'AsyncFunction';

                    if (isAsync && useSuspense) {{
                        try {{
                            const componentPathHash = '{}';
                            const boundaryId = 'page_boundary_' + componentPathHash;
                            const promiseId = 'page_promise_' + componentPathHash;

                        globalThis.__suspense_promises = globalThis.__suspense_promises || {{}};

                        globalThis.__deferred_async_components = globalThis.__deferred_async_components || [];
                        globalThis.__deferred_async_components.push({{
                            promiseId: promiseId,
                            boundaryId: boundaryId,
                            component: PageComponent,
                            props: pageProps,
                            componentPath: '{}'
                        }});

                        globalThis.__discovered_boundaries = globalThis.__discovered_boundaries || [];

                        globalThis.__pending_promises = globalThis.__pending_promises || [];
                        globalThis.__pending_promises.push({{
                            id: promiseId,
                            boundaryId: boundaryId,
                            componentPath: '{}'
                        }});

                        let loadingFallback;
                        try {{
                            loadingFallback = LoadingComponent();
                        }} catch (loadingError) {{
                            throw new Error('Failed to call LoadingComponent: ' + (loadingError.message || String(loadingError)));
                        }}

                        const fallbackForBoundary = {{
                            type: loadingFallback?.type || 'div',
                            props: loadingFallback?.props ? {{...loadingFallback.props}} : {{ children: 'Loading...' }},
                            key: null
                        }};

                        globalThis.__discovered_boundaries.push({{
                            id: boundaryId,
                            fallback: fallbackForBoundary,
                            parentId: 'content-slot',
                            parentPath: ['content-slot'],
                            isInContentArea: true,
                            positionHints: {{
                                inContentArea: true,
                                domPath: ['content-slot'],
                                isStable: true
                            }}
                        }});

                        const childrenPlaceholder = React.createElement('div', {{
                            'data-promise-ref': promiseId,
                            className: 'suspense-placeholder'
                        }}, 'Loading...');
                        let suspenseRscProps;
                        try {{
                            const fallbackRsc = await globalThis.renderToRsc(loadingFallback, globalThis.__rsc_client_components || {{}});

                            suspenseRscProps = {{
                                fallback: fallbackRsc,
                                boundaryId: boundaryId
                            }};
                        }} catch (renderError) {{
                            throw new Error('Failed to render Suspense boundary: ' + (renderError.message || String(renderError)));
                        }}

                        var pageElement = {{
                            __preSerializedSuspense: true,
                            rscArray: ["$", "react.suspense", null, suspenseRscProps]
                        }};

                        }} catch (asyncWrapError) {{
                            throw new Error('Failed to wrap async component in Suspense: ' + (asyncWrapError.message || String(asyncWrapError)));
                        }}
                    }} else if (isAsync && !useSuspense) {{
                        try {{
                            const pageResult = PageComponent(pageProps);

                            if (pageResult && typeof pageResult.then === 'function') {{
                                var pageElement = await pageResult;
                            }} else {{
                                var pageElement = pageResult;
                            }}
                        }} catch (asyncError) {{
                            throw new Error('Failed to await async component: ' + (asyncError.message || String(asyncError)));
                        }}
                    }} else {{
                        const pageResult = PageComponent(pageProps);

                        var pageElement = pageResult && typeof pageResult.then === 'function'
                            ? await pageResult
                            : pageResult;
                    }}
                }}
                timings.pageRender = performance.now() - startPage;
                "#,
                page_component_id,
                page_component_id,
                loading_id,
                loading_id,
                page_props_json,
                page_props_json,
                if use_suspense { "true" } else { "false" },
                route_match.route.file_path,
                route_match.route.file_path,
                route_match.route.file_path,
            )
        } else {
            format!(
                r#"
                const startPage = performance.now();
                const PageComponent = globalThis["{}"];
                if (!PageComponent || typeof PageComponent !== 'function') {{
                    throw new Error('Page component {} not found');
                }}

                const pageProps = {};
                const pageResult = PageComponent(pageProps);
                var pageElement = pageResult && typeof pageResult.then === 'function'
                    ? await pageResult
                    : pageResult;
                timings.pageRender = performance.now() - startPage;
                "#,
                page_component_id, page_component_id, page_props_json
            )
        };

        let mut script = format!(
            r#"
            (async () => {{
                const timings = {{}};
                const startTotal = performance.now();

                const React = globalThis.React || require('react');
                const ReactDOMServer = globalThis.ReactDOMServer || require('react-dom/server');

                globalThis.__discovered_boundaries = [];
                globalThis.__pending_promises = [];
                globalThis.__deferred_async_components = [];

                {}
            "#,
            page_render_script
        );

        let mut current_element = "pageElement".to_string();

        for (i, layout) in route_match.layouts.iter().rev().enumerate() {
            let layout_component_id = self.create_component_id(&layout.file_path);
            let layout_var = format!("layout{}", i);

            if layout.is_root {
                script.push_str(&format!(
                    r#"
                const startLayout{} = performance.now();
                const LayoutComponent{} = globalThis["{}"];
                if (!LayoutComponent{} || typeof LayoutComponent{} !== 'function') {{
                    throw new Error('Root layout component {} not found');
                }}

                const {} = React.createElement(LayoutComponent{}, {{ children: {}, pathname: {} }});
                timings.layout{} = performance.now() - startLayout{};
                "#,
                    i,
                    i,
                    layout_component_id,
                    i,
                    i,
                    layout_component_id,
                    layout_var,
                    i,
                    current_element,
                    serde_json::to_string(&context.pathname).unwrap_or_else(|_| "null".to_string()),
                    i,
                    i
                ));
            } else {
                script.push_str(&format!(
                    r#"
                const startLayout{} = performance.now();
                const LayoutComponent{} = globalThis["{}"];
                if (!LayoutComponent{} || typeof LayoutComponent{} !== 'function') {{
                    throw new Error('Layout component {} not found');
                }}

                const {} = React.createElement(LayoutComponent{}, {{ children: {}, pathname: {} }});
                timings.layout{} = performance.now() - startLayout{};
                "#,
                    i,
                    i,
                    layout_component_id,
                    i,
                    i,
                    layout_component_id,
                    layout_var,
                    i,
                    current_element,
                    serde_json::to_string(&context.pathname).unwrap_or_else(|_| "null".to_string()),
                    i,
                    i
                ));
            }

            current_element = layout_var;
        }

        script.push_str(r#"
                globalThis.__rsc_render_result = null;

                if (!globalThis.renderToRsc) {
                    globalThis.renderToRsc = async function(element, clientComponents = {}) {
                        if (element && typeof element === 'object' && element.__preSerializedSuspense) {
                            return element.rscArray;
                        }

                        if (!element) return null;

                        if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
                            return element;
                        }

                        if (Array.isArray(element)) {
                            const results = [];
                            for (const child of element) {
                                results.push(await globalThis.renderToRsc(child, clientComponents));
                            }
                            return results;
                        }

                        if (element && typeof element === 'object') {
                            const uniqueKey = element.key || null;
                            const props = element.props || {};

                            const typeCheck = element.type === React.Suspense;
                            const nameCheck = typeof element.type === 'function' && element.type.name === 'Suspense';
                            const stringCheck = element.type === 'react.suspense' || element.type === 'Suspense';
                            const propsCheck = props.boundaryId || props.__boundary_id;
                            const isSuspense = typeCheck || nameCheck || stringCheck || propsCheck;

                            if (isSuspense) {
                                const rscProps = {
                                    fallback: props.fallback ? await globalThis.renderToRsc(props.fallback, clientComponents) : null,
                                    children: props.children ? await globalThis.renderToRsc(props.children, clientComponents) : null,
                                    boundaryId: props.boundaryId || props.__boundary_id
                                };

                                if (rscProps.fallback === null) delete rscProps.fallback;
                                if (rscProps.children === null) delete rscProps.children;
                                if (!rscProps.boundaryId) delete rscProps.boundaryId;

                                return ["$", "react.suspense", uniqueKey, rscProps];
                            }

                            if (element.type) {
                                if (typeof element.type === 'string') {
                                    const { children: propsChildren, ...otherProps } = props;
                                    const actualChildren = element.children || propsChildren;

                                    const rscProps = {
                                        ...otherProps,
                                        children: actualChildren ? await globalThis.renderToRsc(actualChildren, clientComponents) : undefined
                                    };

                                    if (rscProps.children === undefined) {
                                        delete rscProps.children;
                                    }

                                    return ["$", element.type, uniqueKey, rscProps];
                                } else if (typeof element.type === 'function') {
                                    try {
                                        let result = element.type(props);

                                        if (result && typeof result.then === 'function') {
                                            result = await result;
                                        }

                                        return await globalThis.renderToRsc(result, clientComponents);
                                    } catch (error) {
                                        console.error('Error rendering function component:', error);
                                        return ["$", "div", uniqueKey, {
                                            children: `Error: ${error.message}`,
                                            style: { color: 'red', border: '1px solid red', padding: '10px' }
                                        }];
                                    }
                                }
                            }

                            return ["$", "div", uniqueKey, {
                                className: "rsc-unknown",
                                children: "Unknown element type"
                            }];
                        }

                        return element;
                    };
                }

                const startRSC = performance.now();
                const rscData = await globalThis.renderToRsc(ELEMENT_PLACEHOLDER, globalThis.__rsc_client_components || {});
                timings.rscConversion = performance.now() - startRSC;

                timings.total = performance.now() - startTotal;

                const deferredComponents = globalThis.__deferred_async_components || [];
                const hasAsync = deferredComponents.length > 0;
                const deferredCount = deferredComponents.length;

                const result = {
                    rsc_data: rscData,
                    boundaries: globalThis.__discovered_boundaries || [],
                    pending_promises: globalThis.__pending_promises || [],
                    has_suspense: (globalThis.__discovered_boundaries && globalThis.__discovered_boundaries.length > 0) ||
                                 (globalThis.__pending_promises && globalThis.__pending_promises.length > 0),
                    metadata: {
                        hasAsync: hasAsync,
                        deferredCount: deferredCount,
                        executionTime: timings.total
                    },
                    timings: timings,
                    success: true
                };

                try {
                    const jsonString = JSON.stringify(result);
                    const cleanResult = JSON.parse(jsonString);
                    globalThis.__rsc_render_result = cleanResult;
                    return cleanResult;
                } catch (jsonError) {
                    globalThis.__rsc_render_result = result;
                    return result;
                }
            })()
            "#);

        let script = script.replace("ELEMENT_PLACEHOLDER", &current_element);

        Ok(script)
    }

    fn create_component_id(&self, file_path: &str) -> String {
        let normalized =
            file_path.replace(".tsx", "").replace(".ts", "").replace("[", "_").replace("]", "_");
        format!("app/{}", normalized)
    }

    fn get_component_id(&self, file_path: &str) -> String {
        let path = std::path::Path::new(file_path);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown");

        let mut chars = stem.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    fn create_page_props(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<Value, RariError> {
        let params_value = if route_match.params.is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::to_value(&route_match.params)?
        };

        let search_params_value = if context.search_params.is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::to_value(&context.search_params)?
        };

        #[allow(clippy::disallowed_methods)]
        let result = serde_json::json!({
            "params": params_value,
            "searchParams": search_params_value
        });
        Ok(result)
    }

    #[allow(dead_code)]
    async fn render_loading_fallback(
        &self,
        loading_path: &str,
        context: &LayoutRenderContext,
    ) -> Result<ReactElement, RariError> {
        let component_id = self.create_component_id(loading_path);
        let renderer = self.renderer.lock().await;

        if !renderer.component_exists(&component_id) {
            let error = LoadingStateError::LoadingNotFound {
                path: loading_path.to_string(),
                message: format!("Component '{}' not found in runtime", component_id),
            };
            error!(
                "Loading component not found: {} (component ID: {})",
                loading_path, component_id
            );
            return Err(error.into());
        }

        let page_props = self.create_page_props(
            &AppRouteMatch {
                route: crate::server::app_router::AppRouteEntry {
                    path: String::new(),
                    file_path: loading_path.to_string(),
                    segments: vec![],
                    params: vec![],
                    is_dynamic: false,
                },
                params: FxHashMap::default(),
                layouts: vec![],
                loading: None,
                error: None,
                not_found: None,
                pathname: context.pathname.clone(),
            },
            context,
        )?;

        let page_props_json = serde_json::to_string(&page_props)
            .map_err(|e| RariError::internal(format!("Failed to serialize props: {}", e)))?;

        let script = format!(
            r#"
            (function() {{
                const props = {};
                const component = globalThis["{}"];
                if (!component || typeof component !== 'function') {{
                    throw new Error("Component not found: {}");
                }}

                const element = component(props);

                function serializeElement(el) {{
                    if (!el || typeof el !== 'object') {{
                        return el;
                    }}

                    if (Array.isArray(el)) {{
                        return el.map(serializeElement);
                    }}

                    if (el.type !== undefined && el.props !== undefined) {{
                        const result = {{
                            type: el.type,
                            props: {{}},
                            key: el.key || null,
                            ref: el.ref || null
                        }};

                        for (const [key, value] of Object.entries(el.props)) {{
                            if (key === 'children') {{
                                if (Array.isArray(value)) {{
                                    result.props.children = value.map(serializeElement);
                                }} else {{
                                    result.props.children = serializeElement(value);
                                }}
                            }} else {{
                                result.props[key] = value;
                            }}
                        }}

                        return result;
                    }}

                    return el;
                }}

                return serializeElement(element);
            }})();
            "#,
            page_props_json, component_id, component_id
        );

        let result = renderer
            .runtime
            .execute_script(format!("render_{}", component_id), script)
            .await
            .map_err(|e| {
                let error = LoadingStateError::RenderError {
                    path: loading_path.to_string(),
                    message: format!("Failed to execute loading component: {}", e),
                    source: Some(e.to_string()),
                };
                error!(
                    "Loading component render error at {}: {} (component ID: {})",
                    loading_path, e, component_id
                );
                RariError::from(error)
            })?;

        let element: ReactElement = serde_json::from_value(result.clone()).map_err(|e| {
            let error = LoadingStateError::InvalidOutput {
                path: loading_path.to_string(),
                message: format!("Failed to parse loading component output as ReactElement: {}", e),
                details: Some(format!("Output: {:?}", result)),
            };
            error!(
                "Invalid loading component output from {}: {} (component ID: {})",
                loading_path, e, component_id
            );
            RariError::from(error)
        })?;

        Ok(element)
    }

    #[allow(dead_code)]
    async fn render_page_element(
        &self,
        page_path: &str,
        context: &LayoutRenderContext,
    ) -> Result<ReactElement, RariError> {
        let component_id = self.create_component_id(page_path);

        let renderer = self.renderer.lock().await;

        let page_props = self.create_page_props(
            &AppRouteMatch {
                route: crate::server::app_router::AppRouteEntry {
                    path: String::new(),
                    file_path: page_path.to_string(),
                    segments: vec![],
                    params: vec![],
                    is_dynamic: false,
                },
                params: context.params.clone(),
                layouts: vec![],
                loading: None,
                error: None,
                not_found: None,
                pathname: context.pathname.clone(),
            },
            context,
        )?;

        let page_props_json = serde_json::to_string(&page_props)
            .map_err(|e| RariError::internal(format!("Failed to serialize props: {}", e)))?;

        let script = format!(
            r#"
            (function() {{
                const props = {};
                const component = globalThis["{}"];
                if (!component || typeof component !== 'function') {{
                    throw new Error("Component not found: {}");
                }}

                const element = component(props);

                function serializeElement(el) {{
                    if (!el || typeof el !== 'object') {{
                        return el;
                    }}

                    if (Array.isArray(el)) {{
                        return el.map(serializeElement);
                    }}

                    if (el.type !== undefined && el.props !== undefined) {{
                        const result = {{
                            type: el.type,
                            props: {{}},
                            key: el.key || null,
                            ref: el.ref || null
                        }};

                        for (const [key, value] of Object.entries(el.props)) {{
                            if (key === 'children') {{
                                if (Array.isArray(value)) {{
                                    result.props.children = value.map(serializeElement);
                                }} else {{
                                    result.props.children = serializeElement(value);
                                }}
                            }} else {{
                                result.props[key] = value;
                            }}
                        }}

                        return result;
                    }}

                    return el;
                }}

                return serializeElement(element);
            }})();
            "#,
            page_props_json, component_id, component_id
        );

        let result =
            renderer.runtime.execute_script(format!("render_{}", component_id), script).await?;

        let element: ReactElement = serde_json::from_value(result).map_err(|e| {
            RariError::internal(format!("Failed to parse page component result: {}", e))
        })?;

        Ok(element)
    }

    #[allow(dead_code)]
    async fn render_page_with_suspense(
        &self,
        page_path: &str,
        loading_fallback: ReactElement,
        context: &LayoutRenderContext,
    ) -> Result<ReactElement, RariError> {
        let boundary_id = format!("suspense_{}", uuid::Uuid::new_v4());

        let page_element = self.render_page_element(page_path, context).await.map_err(|e| {
            let error = LoadingStateError::SuspenseError {
                message: format!("Failed to render page component for Suspense boundary: {}", e),
                boundary_id: Some(boundary_id.clone()),
            };
            error!(
                "Suspense boundary creation failed for page '{}' (boundary ID: {}): {}",
                page_path, boundary_id, e
            );
            RariError::from(error)
        })?;

        let mut props = FxHashMap::default();

        let fallback_value = serde_json::to_value(&loading_fallback).map_err(|e| {
            let error = LoadingStateError::SuspenseError {
                message: format!(
                    "Failed to serialize loading fallback for Suspense boundary: {}",
                    e
                ),
                boundary_id: Some(boundary_id.clone()),
            };
            error!(
                "Failed to serialize loading fallback for Suspense boundary '{}': {}",
                boundary_id, e
            );
            RariError::from(error)
        })?;

        let children_value = serde_json::to_value(&page_element).map_err(|e| {
            let error = LoadingStateError::SuspenseError {
                message: format!("Failed to serialize page element for Suspense boundary: {}", e),
                boundary_id: Some(boundary_id.clone()),
            };
            error!(
                "Failed to serialize page element for Suspense boundary '{}': {}",
                boundary_id, e
            );
            RariError::from(error)
        })?;

        props.insert("fallback".to_string(), fallback_value);
        props.insert("children".to_string(), children_value);
        props.insert("__boundary_id".to_string(), serde_json::Value::String(boundary_id.clone()));

        Ok(ReactElement { tag: "react.suspense".to_string(), props, key: None })
    }

    #[allow(dead_code)]
    fn create_default_loading_element(&self) -> ReactElement {
        let mut props = FxHashMap::default();
        props.insert("className".to_string(), serde_json::Value::String("loading".to_string()));
        props.insert("children".to_string(), serde_json::Value::String("Loading...".to_string()));

        ReactElement { tag: "div".to_string(), props, key: None }
    }

    #[allow(dead_code)]
    async fn render_loading_fallback_safe(
        &self,
        loading_path: &str,
        context: &LayoutRenderContext,
    ) -> ReactElement {
        match self.render_loading_fallback(loading_path, context).await {
            Ok(element) => element,
            Err(e) => {
                if let Some(error_type) = e.get_property("error_type") {
                    match error_type {
                        "loading_not_found" => {
                            warn!(
                                "Loading component not found at '{}'. Using default loading element. Error: {}",
                                loading_path, e
                            );
                        }
                        "render_error" => {
                            error!(
                                "Loading component at '{}' failed to render. Using default loading element. Error: {}",
                                loading_path, e
                            );
                        }
                        "invalid_output" => {
                            error!(
                                "Loading component at '{}' produced invalid output. Using default loading element. Error: {}",
                                loading_path, e
                            );
                        }
                        _ => {
                            error!(
                                "Unknown error rendering loading component at '{}'. Using default loading element. Error: {}",
                                loading_path, e
                            );
                        }
                    }
                } else {
                    error!(
                        "Failed to render loading component at '{}'. Using default loading element. Error: {}",
                        loading_path, e
                    );
                }

                self.create_default_loading_element()
            }
        }
    }

    #[allow(dead_code)]
    async fn render_route_with_suspense_element(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        suspense_element: ReactElement,
        _request_context: Option<std::sync::Arc<crate::server::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        let composition_script =
            self.build_composition_script_with_element(route_match, context, suspense_element)?;

        let renderer = self.renderer.lock().await;

        if let Some(ref ctx) = _request_context
            && let Err(e) = renderer.runtime.set_request_context(ctx.clone()).await
        {
            tracing::warn!("Failed to set request context in runtime: {}", e);
        }

        let promise_result = renderer
            .runtime
            .execute_script("compose_and_render_suspense".to_string(), composition_script)
            .await?;

        let result = if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
            promise_result
        } else {
            let get_result_script = r#"globalThis.__rsc_render_result"#.to_string();
            renderer.runtime.execute_script("get_result".to_string(), get_result_script).await?
        };

        let rsc_data = result.get("rsc_data").ok_or_else(|| {
            tracing::error!("Failed to extract RSC data from result: {:?}", result);
            RariError::internal("No RSC data in render result")
        })?;

        let rsc_wire_format = {
            let mut serializer = renderer.serializer.lock();
            serializer
                .serialize_rsc_json(rsc_data)
                .map_err(|e| RariError::internal(format!("Failed to serialize RSC data: {e}")))?
        };

        if let Err(e) = Self::validate_html_structure(&rsc_wire_format, route_match) {
            tracing::warn!("HTML structure validation warning: {}", e);
        }

        Ok(rsc_wire_format)
    }

    #[allow(dead_code)]
    fn build_composition_script_with_element(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        element: ReactElement,
    ) -> Result<String, RariError> {
        let element_json = serde_json::to_string(&element)
            .map_err(|e| RariError::internal(format!("Failed to serialize element: {e}")))?;

        let mut script = format!(
            r#"
            (async () => {{
                const timings = {{}};
                const startTotal = performance.now();

                const React = globalThis.React || require('react');

                function reconstructReactElement(obj) {{
                    if (!obj || typeof obj !== 'object') {{
                        return obj;
                    }}

                    if (obj.$$typeof) {{
                        return obj;
                    }}

                    if (obj.type && obj.props) {{
                        const {{ type, props, key }} = obj;

                        let reconstructedProps = {{}};
                        for (const [propKey, propValue] of Object.entries(props)) {{
                            if (propValue && typeof propValue === 'object') {{
                                reconstructedProps[propKey] = reconstructReactElement(propValue);
                            }} else {{
                                reconstructedProps[propKey] = propValue;
                            }}
                        }}

                        return React.createElement(type, {{ key, ...reconstructedProps }});
                    }}

                    if (Array.isArray(obj)) {{
                        return obj.map(reconstructReactElement);
                    }}

                    return obj;
                }}

                const pageElementData = {};
                const pageElement = reconstructReactElement(pageElementData);
                timings.pageRender = 0;
            "#,
            element_json
        );

        let mut current_element = "pageElement".to_string();
        for (i, layout) in route_match.layouts.iter().rev().enumerate() {
            let layout_component_id = self.create_component_id(&layout.file_path);
            let layout_var = format!("layout{}", i);

            script.push_str(&format!(
                r#"
                const startLayout{} = performance.now();
                const LayoutComponent{} = globalThis["{}"];
                if (!LayoutComponent{} || typeof LayoutComponent{} !== 'function') {{
                    throw new Error('Layout component {} not found');
                }}

                const {} = LayoutComponent{}({{ children: {}, pathname: {} }});
                timings.layout{} = performance.now() - startLayout{};
                "#,
                i,
                i,
                layout_component_id,
                i,
                i,
                layout_component_id,
                layout_var,
                i,
                current_element,
                serde_json::to_string(&context.pathname).unwrap_or_else(|_| "null".to_string()),
                i,
                i
            ));

            current_element = layout_var;
        }

        script.push_str(r#"
                globalThis.__rsc_render_result = null;

                const traverseToRsc = globalThis.traverseToRsc;
                if (!traverseToRsc) {
                    throw new Error('traverseToRsc not available - RSC runtime not initialized');
                }

                if (!globalThis.renderToRsc) {
                    globalThis.renderToRsc = async function(element, clientComponents = {}) {
                        if (element && typeof element === 'object' && element.__preSerializedSuspense) {
                            return element.rscArray;
                        }

                        if (!element) return null;

                        if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
                            return element;
                        }

                        if (Array.isArray(element)) {
                            const results = [];
                            for (const child of element) {
                                results.push(await globalThis.renderToRsc(child, clientComponents));
                            }
                            return results;
                        }

                        if (element && typeof element === 'object') {
                            const uniqueKey = element.key || null;
                            const props = element.props || {};

                            const typeCheck = element.type === React.Suspense;
                            const nameCheck = typeof element.type === 'function' && element.type.name === 'Suspense';
                            const stringCheck = element.type === 'react.suspense' || element.type === 'Suspense';
                            const propsCheck = props.boundaryId || props.__boundary_id;
                            const isSuspense = typeCheck || nameCheck || stringCheck || propsCheck;

                            if (isSuspense) {
                                const rscProps = {
                                    fallback: props.fallback ? await globalThis.renderToRsc(props.fallback, clientComponents) : null,
                                    children: props.children ? await globalThis.renderToRsc(props.children, clientComponents) : null,
                                    boundaryId: props.boundaryId || props.__boundary_id
                                };

                                if (rscProps.fallback === null) delete rscProps.fallback;
                                if (rscProps.children === null) delete rscProps.children;
                                if (!rscProps.boundaryId) delete rscProps.boundaryId;

                                return ["$", "react.suspense", uniqueKey, rscProps];
                            }

                            if (element.type) {
                                if (typeof element.type === 'string') {
                                    const { children: propsChildren, ...otherProps } = props;
                                    const actualChildren = element.children || propsChildren;

                                    const rscProps = {
                                        ...otherProps,
                                        children: actualChildren ? await globalThis.renderToRsc(actualChildren, clientComponents) : undefined
                                    };

                                    if (rscProps.children === undefined) {
                                        delete rscProps.children;
                                    }

                                    return ["$", element.type, uniqueKey, rscProps];
                                } else if (typeof element.type === 'function') {
                                    try {
                                        let result = element.type(props);

                                        if (result && typeof result.then === 'function') {
                                            result = await result;
                                        }

                                        return await globalThis.renderToRsc(result, clientComponents);
                                    } catch (error) {
                                        console.error('Error rendering function component:', error);
                                        return ["$", "div", uniqueKey, {
                                            children: `Error: ${error.message}`,
                                            style: { color: 'red', border: '1px solid red', padding: '10px' }
                                        }];
                                    }
                                }
                            }

                            return ["$", "div", uniqueKey, {
                                className: "rsc-unknown",
                                children: "Unknown element type"
                            }];
                        }

                        return element;
                    };
                }

                const startRSC = performance.now();
                const rscData = await globalThis.renderToRsc(ELEMENT_PLACEHOLDER, globalThis.__rsc_client_components || {});
                timings.rscConversion = performance.now() - startRSC;

                timings.total = performance.now() - startTotal;

                globalThis.__rsc_render_result = { rsc_data: rscData, timings };
                return globalThis.__rsc_render_result;
            })()
            "#);

        let script = script.replace("ELEMENT_PLACEHOLDER", &current_element);

        Ok(script)
    }

    pub async fn render_loading(
        &self,
        loading_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = self.get_component_id(loading_path);

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, None).await
    }

    pub async fn render_error(
        &self,
        error_path: &str,
        error: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = self.get_component_id(error_path);

        let mut props = serde_json::Map::new();
        props.insert("error".to_string(), Value::String(error.to_string()));
        props.insert(
            "reset".to_string(),
            Value::String("() => window.location.reload()".to_string()),
        );

        let props_json = serde_json::to_string(&props)
            .map_err(|e| RariError::internal(format!("Failed to serialize error props: {e}")))?;

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, Some(&props_json)).await
    }

    pub async fn render_not_found(
        &self,
        not_found_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = self.get_component_id(not_found_path);

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, None).await
    }

    pub async fn component_exists(&self, component_id: &str) -> bool {
        let renderer = self.renderer.lock().await;
        renderer.component_exists(component_id)
    }

    pub async fn register_component(
        &self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        self.renderer.lock().await.register_component(component_id, component_code).await
    }
}

pub fn create_layout_context(
    params: FxHashMap<String, String>,
    search_params: FxHashMap<String, Vec<String>>,
    headers: FxHashMap<String, String>,
    pathname: String,
) -> LayoutRenderContext {
    LayoutRenderContext { params, search_params, headers, pathname }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::JsExecutionRuntime;
    use std::sync::Arc;

    #[test]
    fn test_get_component_id() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        assert_eq!(renderer.get_component_id("app/page.tsx"), "Page");
        assert_eq!(renderer.get_component_id("app/layout.tsx"), "Layout");
        assert_eq!(renderer.get_component_id("app/loading.tsx"), "Loading");
        assert_eq!(renderer.get_component_id("app/error.tsx"), "Error");
    }

    #[test]
    fn test_create_page_props() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let mut params = FxHashMap::default();
        params.insert("id".to_string(), "123".to_string());

        let mut search_params = FxHashMap::default();
        search_params.insert("q".to_string(), vec!["test".to_string()]);

        let context = LayoutRenderContext {
            params: params.clone(),
            search_params: search_params.clone(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params,
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let props = renderer.create_page_props(&route_match, &context).unwrap();
        assert!(props.get("params").is_some());
        assert!(props.get("searchParams").is_some());
    }

    #[test]
    fn test_wrapped_html_error_message_contains_key_info() {
        use crate::server::app_router::{AppRouteEntry, LayoutEntry};

        let route_match = AppRouteMatch {
            route: AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![LayoutEntry {
                path: "/".to_string(),
                file_path: "app/layout.tsx".to_string(),
                parent_path: None,
                is_root: true,
            }],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let error_msg =
            LayoutRenderer::create_wrapped_html_error_message(&route_match, Some("app/layout.tsx"));

        assert!(error_msg.contains("Hydration Mismatch"));
        assert!(error_msg.contains("app/layout.tsx"));
        assert!(error_msg.contains("/test"));
        assert!(error_msg.contains("HOW TO FIX"));
        assert!(error_msg.contains("CORRECT STRUCTURE"));
        assert!(error_msg.contains("INCORRECT STRUCTURE"));
        assert!(error_msg.contains("Step 1"));
        assert!(error_msg.contains("ROOT"));
    }

    #[test]
    fn test_empty_rsc_error_message_contains_key_info() {
        let error_msg = LayoutRenderer::create_empty_rsc_error_message();

        assert!(error_msg.contains("Empty Content"));
        assert!(error_msg.contains("COMMON CAUSES"));
        assert!(error_msg.contains("HOW TO FIX"));
        assert!(error_msg.contains("VALID COMPONENT EXAMPLES"));
        assert!(error_msg.contains("export default"));
        assert!(error_msg.contains("Step 1"));
        assert!(error_msg.contains("DEBUGGING CHECKLIST"));
    }

    #[test]
    fn test_invalid_rsc_format_warning_contains_key_info() {
        let warning_msg = LayoutRenderer::create_invalid_rsc_format_warning(
            "missing row ID",
            "Expected numeric row ID at start",
        );

        assert!(warning_msg.contains("Invalid Structure"));
        assert!(warning_msg.contains("missing row ID"));
        assert!(warning_msg.contains("Expected numeric row ID at start"));
        assert!(warning_msg.contains("RSC WIRE FORMAT SPECIFICATION"));
        assert!(warning_msg.contains("VALID RSC WIRE FORMAT EXAMPLES"));
        assert!(warning_msg.contains("TROUBLESHOOTING STEPS"));
        assert!(warning_msg.contains("Step 1"));
    }

    #[test]
    fn test_detect_suspense_boundaries_no_suspense() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let rsc_json = serde_json::json!([
            "$",
            "div",
            null,
            {
                "children": "Hello World"
            }
        ]);

        let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
        assert!(!result.has_suspense);
        assert_eq!(result.boundary_count, 0);
        assert_eq!(result.boundaries.len(), 0);
    }

    #[test]
    fn test_detect_suspense_boundaries_with_suspense() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let rsc_json = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "boundaryId": "test_boundary_123",
                "fallback": ["$", "div", null, {"children": "Loading..."}],
                "children": ["$", "div", null, {"children": "Content"}]
            }
        ]);

        let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
        assert!(result.has_suspense);
        assert_eq!(result.boundary_count, 1);
        assert_eq!(result.boundaries.len(), 1);
        assert_eq!(result.boundaries[0].id, "test_boundary_123");
        assert!(result.boundaries[0].has_fallback);
    }

    #[test]
    fn test_detect_suspense_boundaries_nested() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let rsc_json = serde_json::json!([
            "$",
            "div",
            null,
            {
                "children": [
                    "$",
                    "react.suspense",
                    null,
                    {
                        "boundaryId": "nested_boundary",
                        "fallback": ["$", "span", null, {"children": "Loading..."}]
                    }
                ]
            }
        ]);

        let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
        assert!(result.has_suspense);
        assert_eq!(result.boundary_count, 1);
        assert_eq!(result.boundaries[0].id, "nested_boundary");
    }

    #[test]
    fn test_detect_suspense_boundaries_pre_serialized() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let rsc_json = serde_json::json!({
            "__preSerializedSuspense": true,
            "rscArray": [
                "$",
                "react.suspense",
                null,
                {
                    "boundaryId": "pre_serialized_boundary",
                    "fallback": ["$", "div", null, {"children": "Loading..."}]
                }
            ]
        });

        let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
        assert!(result.has_suspense);
        assert_eq!(result.boundary_count, 1);
        assert_eq!(result.boundaries[0].id, "pre_serialized_boundary");
    }

    #[test]
    fn test_detect_suspense_boundaries_multiple() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let rsc_json = serde_json::json!([
            "$",
            "div",
            null,
            {
                "children": [
                    [
                        "$",
                        "react.suspense",
                        null,
                        {
                            "boundaryId": "boundary_1",
                            "fallback": ["$", "div", null, {"children": "Loading 1..."}]
                        }
                    ],
                    [
                        "$",
                        "react.suspense",
                        null,
                        {
                            "__boundary_id": "boundary_2",
                            "fallback": ["$", "div", null, {"children": "Loading 2..."}]
                        }
                    ]
                ]
            }
        ]);

        let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
        assert!(result.has_suspense);
        assert_eq!(result.boundary_count, 2);
        assert_eq!(result.boundaries.len(), 2);
        assert_eq!(result.boundaries[0].id, "boundary_1");
        assert_eq!(result.boundaries[1].id, "boundary_2");
    }

    #[test]
    fn test_detect_suspense_boundaries_alternative_tag() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let rsc_json = serde_json::json!([
            "$",
            "Suspense",
            null,
            {
                "boundaryId": "alt_boundary",
                "fallback": ["$", "div", null, {"children": "Loading..."}]
            }
        ]);

        let result = renderer.detect_suspense_boundaries(&rsc_json).unwrap();
        assert!(result.has_suspense);
        assert_eq!(result.boundary_count, 1);
        assert_eq!(result.boundaries[0].id, "alt_boundary");
    }

    #[test]
    fn test_build_composition_script_with_use_suspense_true() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script = renderer
            .build_composition_script(&route_match, &context, Some("app/test/loading"), true)
            .unwrap();

        assert!(script.contains("const useSuspense = true"));
        assert!(script.contains("if (isAsync && useSuspense)"));
    }

    #[test]
    fn test_build_composition_script_with_use_suspense_false() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script = renderer
            .build_composition_script(&route_match, &context, Some("app/test/loading"), false)
            .unwrap();

        assert!(script.contains("const useSuspense = false"));
        assert!(script.contains("else if (isAsync && !useSuspense)"));
        assert!(script.contains("RSC Navigation mode - awaiting async component directly"));
    }

    #[test]
    fn test_layout_structure_new() {
        let layout_structure = LayoutStructure::new();
        assert!(!layout_structure.has_navigation);
        assert!(layout_structure.navigation_position.is_none());
        assert!(layout_structure.content_position.is_none());
        assert_eq!(layout_structure.suspense_boundaries.len(), 0);
    }

    #[test]
    fn test_layout_structure_is_valid_empty() {
        let layout_structure = LayoutStructure::new();
        assert!(layout_structure.is_valid());
    }

    #[test]
    fn test_layout_structure_is_valid_navigation_before_content() {
        let layout_structure = LayoutStructure {
            has_navigation: true,
            navigation_position: Some(0),
            content_position: Some(1),
            suspense_boundaries: Vec::new(),
        };
        assert!(layout_structure.is_valid());
    }

    #[test]
    fn test_layout_structure_is_invalid_navigation_after_content() {
        let layout_structure = LayoutStructure {
            has_navigation: true,
            navigation_position: Some(1),
            content_position: Some(0),
            suspense_boundaries: Vec::new(),
        };
        assert!(!layout_structure.is_valid());
    }

    #[test]
    fn test_layout_structure_is_invalid_boundary_outside_content() {
        let layout_structure = LayoutStructure {
            has_navigation: true,
            navigation_position: Some(0),
            content_position: Some(1),
            suspense_boundaries: vec![BoundaryPosition {
                boundary_id: "test_boundary".to_string(),
                parent_path: vec![0],
                is_in_content_area: false,
                dom_path: vec![0],
            }],
        };
        assert!(!layout_structure.is_valid());
    }

    #[test]
    fn test_layout_structure_is_valid_boundary_in_content() {
        let layout_structure = LayoutStructure {
            has_navigation: true,
            navigation_position: Some(0),
            content_position: Some(1),
            suspense_boundaries: vec![BoundaryPosition {
                boundary_id: "test_boundary".to_string(),
                parent_path: vec![1, 0],
                is_in_content_area: true,
                dom_path: vec![1, 1, 0],
            }],
        };
        assert!(layout_structure.is_valid());
    }

    #[test]
    fn test_validate_layout_structure_no_navigation() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let rsc_json = serde_json::json!([
            "$",
            "div",
            null,
            {
                "children": "Hello World"
            }
        ]);

        let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
        assert!(!result.has_navigation);
        assert!(result.navigation_position.is_none());
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_layout_structure_with_navigation() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let rsc_json = serde_json::json!([
            "$",
            "div",
            null,
            {
                "children": [
                    ["$", "nav", null, {"children": "Navigation"}],
                    ["$", "main", null, {"children": "Content"}]
                ]
            }
        ]);

        let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
        assert!(result.has_navigation);
        assert!(result.navigation_position.is_some());
        assert!(result.content_position.is_some());
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_layout_structure_with_suspense_in_content() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let rsc_json = serde_json::json!([
            "$",
            "div",
            null,
            {
                "children": [
                    ["$", "nav", null, {"children": "Navigation"}],
                    [
                        "$",
                        "main",
                        null,
                        {
                            "children": [
                                "$",
                                "react.suspense",
                                null,
                                {
                                    "boundaryId": "test_boundary",
                                    "fallback": ["$", "div", null, {"children": "Loading..."}]
                                }
                            ]
                        }
                    ]
                ]
            }
        ]);

        let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
        assert!(result.has_navigation);
        assert_eq!(result.suspense_boundaries.len(), 1);
        assert_eq!(result.suspense_boundaries[0].boundary_id, "test_boundary");
        assert!(result.suspense_boundaries[0].is_in_content_area);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_layout_structure_detects_header_with_navigation_role() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let rsc_json = serde_json::json!([
            "$",
            "div",
            null,
            {
                "children": [
                    ["$", "header", null, {"role": "navigation", "children": "Header Nav"}],
                    ["$", "main", null, {"children": "Content"}]
                ]
            }
        ]);

        let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
        assert!(result.has_navigation);
        assert!(result.navigation_position.is_some());
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_layout_structure_detects_section_with_main_role() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let rsc_json = serde_json::json!([
            "$",
            "div",
            null,
            {
                "children": [
                    ["$", "nav", null, {"children": "Navigation"}],
                    ["$", "section", null, {"role": "main", "children": "Main Content"}]
                ]
            }
        ]);

        let result = renderer.validate_layout_structure(&rsc_json, &route_match).unwrap();
        assert!(result.has_navigation);
        assert!(result.content_position.is_some());
        assert!(result.is_valid());
    }

    #[test]
    fn test_composition_script_includes_layout_structure_markers() {
        use crate::server::app_router::LayoutEntry;

        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![LayoutEntry {
                path: "/".to_string(),
                file_path: "app/layout.tsx".to_string(),
                parent_path: None,
                is_root: true,
            }],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script = renderer.build_composition_script(&route_match, &context, None, true).unwrap();

        assert!(!script.contains("'data-content-slot': true"));
        assert!(!script.contains("const contentSlot = React.createElement"));
        assert!(!script.contains("'data-layout-root': true"));
        assert!(!script.contains("const layoutRoot = React.createElement"));
    }

    #[test]
    fn test_calculate_boundary_positions_with_navigation() {
        let layout_structure = LayoutStructure {
            has_navigation: true,
            navigation_position: Some(0),
            content_position: Some(1),
            suspense_boundaries: vec![
                BoundaryPosition {
                    boundary_id: "boundary1".to_string(),
                    parent_path: vec![0],
                    is_in_content_area: true,
                    dom_path: Vec::new(),
                },
                BoundaryPosition {
                    boundary_id: "boundary2".to_string(),
                    parent_path: vec![1, 2],
                    is_in_content_area: true,
                    dom_path: Vec::new(),
                },
            ],
        };

        let positions = calculate_boundary_positions(&layout_structure);

        assert_eq!(positions.get("boundary1"), Some(&vec![1, 0]));
        assert_eq!(positions.get("boundary2"), Some(&vec![1, 1, 2]));
    }

    #[test]
    fn test_calculate_boundary_positions_without_navigation() {
        let layout_structure = LayoutStructure {
            has_navigation: false,
            navigation_position: None,
            content_position: Some(0),
            suspense_boundaries: vec![
                BoundaryPosition {
                    boundary_id: "boundary1".to_string(),
                    parent_path: vec![0],
                    is_in_content_area: true,
                    dom_path: Vec::new(),
                },
                BoundaryPosition {
                    boundary_id: "boundary2".to_string(),
                    parent_path: vec![1, 2],
                    is_in_content_area: true,
                    dom_path: Vec::new(),
                },
            ],
        };

        let positions = calculate_boundary_positions(&layout_structure);

        assert_eq!(positions.get("boundary1"), Some(&vec![0, 0]));
        assert_eq!(positions.get("boundary2"), Some(&vec![0, 1, 2]));
    }

    #[test]
    fn test_calculate_boundary_positions_outside_content_area() {
        let layout_structure = LayoutStructure {
            has_navigation: true,
            navigation_position: Some(0),
            content_position: Some(1),
            suspense_boundaries: vec![BoundaryPosition {
                boundary_id: "boundary1".to_string(),
                parent_path: vec![0, 1],
                is_in_content_area: false,
                dom_path: Vec::new(),
            }],
        };

        let positions = calculate_boundary_positions(&layout_structure);

        assert_eq!(positions.get("boundary1"), Some(&vec![0, 1]));
    }

    #[test]
    fn test_mode_consistency_both_modes_generate_render_to_rsc() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false).unwrap();

        assert!(script_ssr.contains("globalThis.renderToRsc"));
        assert!(script_rsc.contains("globalThis.renderToRsc"));

        assert!(script_ssr.contains("__preSerializedSuspense"));
        assert!(script_rsc.contains("__preSerializedSuspense"));

        assert!(script_ssr.contains("Comprehensive Suspense detection"));
        assert!(script_rsc.contains("Comprehensive Suspense detection"));
    }

    #[test]
    fn test_mode_consistency_suspense_serialization_format() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false).unwrap();

        assert!(script_ssr.contains(r#"["$", "react.suspense", uniqueKey, rscProps]"#));
        assert!(script_rsc.contains(r#"["$", "react.suspense", uniqueKey, rscProps]"#));

        assert!(script_ssr.contains("boundaryId: props.boundaryId || props.__boundary_id"));
        assert!(script_rsc.contains("boundaryId: props.boundaryId || props.__boundary_id"));
    }

    #[test]
    fn test_mode_consistency_metadata_structure() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false).unwrap();

        assert!(script_ssr.contains("rsc_data: rscData"));
        assert!(script_rsc.contains("rsc_data: rscData"));

        assert!(script_ssr.contains("boundaries: globalThis.__discovered_boundaries"));
        assert!(script_rsc.contains("boundaries: globalThis.__discovered_boundaries"));

        assert!(script_ssr.contains("pending_promises: globalThis.__pending_promises"));
        assert!(script_rsc.contains("pending_promises: globalThis.__pending_promises"));

        assert!(script_ssr.contains("metadata: {"));
        assert!(script_rsc.contains("metadata: {"));

        assert!(script_ssr.contains("hasAsync: hasAsync"));
        assert!(script_rsc.contains("hasAsync: hasAsync"));

        assert!(script_ssr.contains("deferredCount: deferredCount"));
        assert!(script_rsc.contains("deferredCount: deferredCount"));
    }

    #[test]
    fn test_mode_consistency_async_component_handling_with_loading() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script_ssr = renderer
            .build_composition_script(&route_match, &context, Some("app/test/loading"), true)
            .unwrap();
        let script_rsc = renderer
            .build_composition_script(&route_match, &context, Some("app/test/loading"), false)
            .unwrap();

        assert!(script_ssr.contains("const useSuspense = true"));
        assert!(script_ssr.contains("if (isAsync && useSuspense)"));
        assert!(script_ssr.contains("__preSerializedSuspense: true"));

        assert!(script_rsc.contains("const useSuspense = false"));
        assert!(script_rsc.contains("else if (isAsync && !useSuspense)"));
        assert!(script_rsc.contains("RSC Navigation mode - awaiting async component directly"));
    }

    #[test]
    fn test_mode_consistency_boundary_id_format() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script_ssr = renderer
            .build_composition_script(&route_match, &context, Some("app/test/loading"), true)
            .unwrap();
        let script_rsc = renderer
            .build_composition_script(&route_match, &context, Some("app/test/loading"), false)
            .unwrap();

        assert!(script_ssr.contains("const boundaryId = 'page_boundary_'"));
        assert!(script_rsc.contains("const boundaryId = 'page_boundary_'"));

        assert!(script_ssr.contains("const componentPathHash ="));
        assert!(script_rsc.contains("const componentPathHash ="));
    }

    #[test]
    fn test_mode_consistency_wrapper_elements() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false).unwrap();

        assert!(!script_ssr.contains("const contentSlot = React.createElement"));
        assert!(!script_ssr.contains("'data-content-slot': true"));
        assert!(!script_rsc.contains("const contentSlot = React.createElement"));
        assert!(!script_rsc.contains("'data-content-slot': true"));

        assert_eq!(
            script_ssr.contains("const contentSlot"),
            script_rsc.contains("const contentSlot"),
            "Both modes should have same contentSlot behavior"
        );
    }

    #[test]
    fn test_mode_consistency_error_handling() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false).unwrap();

        assert!(script_ssr.contains("console.error('Error rendering function component:', error)"));
        assert!(script_rsc.contains("console.error('Error rendering function component:', error)"));

        assert!(script_ssr.contains(r#"children: `Error: ${error.message}`"#));
        assert!(script_rsc.contains(r#"children: `Error: ${error.message}`"#));

        assert!(
            script_ssr
                .contains("style: { color: 'red', border: '1px solid red', padding: '10px' }")
        );
        assert!(
            script_rsc
                .contains("style: { color: 'red', border: '1px solid red', padding: '10px' }")
        );
    }

    #[test]
    fn test_mode_consistency_rsc_props_cleanup() {
        let renderer = LayoutRenderer::new(Arc::new(tokio::sync::Mutex::new(RscRenderer::new(
            Arc::new(JsExecutionRuntime::new(None)),
        ))));

        let route_match = AppRouteMatch {
            route: crate::server::app_router::AppRouteEntry {
                path: "/test".to_string(),
                file_path: "app/test/page.tsx".to_string(),
                segments: vec![],
                params: vec![],
                is_dynamic: false,
            },
            params: FxHashMap::default(),
            layouts: vec![],
            loading: None,
            error: None,
            not_found: None,
            pathname: "/test".to_string(),
        };

        let context = LayoutRenderContext {
            params: FxHashMap::default(),
            search_params: FxHashMap::default(),
            headers: FxHashMap::default(),
            pathname: "/test".to_string(),
        };

        let script_ssr =
            renderer.build_composition_script(&route_match, &context, None, true).unwrap();
        let script_rsc =
            renderer.build_composition_script(&route_match, &context, None, false).unwrap();

        assert!(script_ssr.contains("if (rscProps.fallback === null) delete rscProps.fallback"));
        assert!(script_rsc.contains("if (rscProps.fallback === null) delete rscProps.fallback"));

        assert!(script_ssr.contains("if (rscProps.children === null) delete rscProps.children"));
        assert!(script_rsc.contains("if (rscProps.children === null) delete rscProps.children"));

        assert!(script_ssr.contains("if (!rscProps.boundaryId) delete rscProps.boundaryId"));
        assert!(script_rsc.contains("if (!rscProps.boundaryId) delete rscProps.boundaryId"));

        assert!(script_ssr.contains("if (rscProps.children === undefined)"));
        assert!(script_rsc.contains("if (rscProps.children === undefined)"));
    }
}
