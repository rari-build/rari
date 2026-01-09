use crate::error::RariError;
use crate::rsc::rendering::core::RscRenderer;
use crate::rsc::rendering::streaming::RscStream;
use crate::server::config::Config;
use crate::server::routing::app_router::AppRouteMatch;
use crate::server::types::request::RenderMode;
use cow_utils::CowUtils;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;

use super::{constants::*, error_messages, types::*, utils};

struct LayoutHtmlCache {
    cache: DashMap<u64, String>,
}

impl LayoutHtmlCache {
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
    html_cache: Arc<LayoutHtmlCache>,
}

impl LayoutRenderer {
    pub fn new(renderer: Arc<tokio::sync::Mutex<RscRenderer>>) -> Self {
        Self { renderer, html_cache: Arc::new(LayoutHtmlCache::new()) }
    }

    pub async fn check_page_not_found(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<bool, RariError> {
        let page_props = utils::create_page_props(route_match, context)?;
        let page_props_json = serde_json::to_string(&page_props)?;

        fn convert_route_path_to_dist_path(path: &str) -> String {
            use regex::Regex;
            let re = Regex::new(r"\[+([^\]]+)\]+")
                .expect("Invalid regex pattern for route path conversion");
            re.replace_all(path, |caps: &regex::Captures| {
                let param = &caps[1];
                let bracket_count = caps[0].matches('[').count();
                let underscores = "_".repeat(bracket_count);
                format!("{}{}{}", underscores, param, underscores)
            })
            .to_string()
        }

        let dist_server_path = std::env::current_dir()
            .ok()
            .map(|p| p.join("dist/server"))
            .and_then(|p| p.canonicalize().ok());

        let base_path = match dist_server_path {
            Some(path) => path,
            None => return Ok(false),
        };

        let js_filename = route_match
            .route
            .file_path
            .cow_replace(".tsx", ".js")
            .cow_replace(".ts", ".js")
            .into_owned();
        let dist_filename = convert_route_path_to_dist_path(&js_filename);
        let page_file_path = base_path.join("app").join(&dist_filename);

        if !page_file_path.exists() {
            return Ok(false);
        }

        let page_path = format!("file://{}", page_file_path.display());

        let check_script = format!(
            r#"
            (async () => {{
                try {{
                    const module = await import("{}");

                    if (typeof module.getData === 'function') {{
                        const pageProps = {};
                        const result = await module.getData(pageProps);
                        return {{ notFound: result?.notFound === true }};
                    }}

                    return {{ notFound: false }};
                }} catch (error) {{
                    console.error('[check_page_not_found] Error:', error);
                    return {{ notFound: false }};
                }}
            }})()
            "#,
            page_path, page_props_json
        );

        let renderer = self.renderer.lock().await;
        let result =
            renderer.runtime.execute_script("check_not_found".to_string(), check_script).await?;

        let not_found = result.get("notFound").and_then(|v| v.as_bool()).unwrap_or(false);

        Ok(not_found)
    }

    pub async fn render_route(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        request_context: Option<
            std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        >,
    ) -> Result<String, RariError> {
        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            if let Some(loading_entry) = &route_match.loading {
                let loading_id = utils::create_component_id(&loading_entry.file_path);
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
            loading_component_id.is_some(),
        )?;

        let renderer = self.renderer.lock().await;

        if let Some(ref ctx) = request_context
            && let Err(e) = renderer.runtime.set_request_context(ctx.clone()).await
        {
            error!("Failed to set request context: {}", e);
        }

        renderer
            .runtime
            .execute_script(
                "enable_streaming".to_string(),
                "globalThis.__RARI_STREAMING_SUSPENSE__ = true;".to_string(),
            )
            .await?;

        let resolve_helper = include_str!("js/resolve_lazy_helper.js");
        renderer
            .runtime
            .execute_script("inject_lazy_resolver".to_string(), resolve_helper.to_string())
            .await?;

        let promise_result = renderer
            .runtime
            .execute_script("compose_and_render".to_string(), composition_script)
            .await?;

        let result = if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
            promise_result
        } else {
            renderer
                .runtime
                .execute_script("get_result".to_string(), JS_GET_RESULT.to_string())
                .await?
        };

        let rsc_data = result.get("rsc_data").ok_or_else(|| {
            tracing::error!("Failed to extract RSC data from result: {:?}", result);
            RariError::internal("No RSC data in render result")
        })?;

        let (mut rsc_wire_format, pending_promises) = {
            let mut serializer = renderer.serializer.lock();

            serializer.reset_for_new_request();

            let wire_format = serializer
                .serialize_rsc_json(rsc_data)
                .map_err(|e| RariError::internal(format!("Failed to serialize RSC data: {e}")))?;
            let promises = serializer.pending_lazy_promises.clone();
            serializer.pending_lazy_promises.clear();
            serializer.seen_lazy_promise_ids.clear();
            (wire_format, promises)
        };

        for lazy_promise in pending_promises {
            let resolve_script = format!(
                "(async () => {{ return await globalThis.__RARI_RESOLVE_LAZY__('{}'); }})()",
                lazy_promise.promise_id
            );

            let result = renderer
                .runtime
                .execute_script(
                    format!("resolve_promise_{}", lazy_promise.promise_id),
                    resolve_script,
                )
                .await;

            match result {
                Ok(result) => {
                    if let Some(success) = result.get("success").and_then(|v| v.as_bool())
                        && !success
                    {
                        let error_msg =
                            result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                        tracing::error!(
                            "Failed to resolve lazy promise {}: {}",
                            lazy_promise.promise_id,
                            error_msg
                        );
                        continue;
                    }

                    let resolved_content = result.get("data").unwrap_or(&result);

                    let wire_format = {
                        let mut serializer = renderer.serializer.lock();
                        match serializer.serialize_rsc_json(resolved_content) {
                            Ok(wire_format) => wire_format,
                            Err(e) => {
                                tracing::error!("Failed to serialize resolved content: {}", e);
                                continue;
                            }
                        }
                    };

                    for line in wire_format.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }

                        let line_to_append = if let Some(colon_pos) = line.find(':') {
                            if line.starts_with(|c: char| c.is_ascii_digit()) {
                                let row_id_str = &line[..colon_pos];
                                if row_id_str.parse::<usize>().is_ok() {
                                    if line.contains("I[") {
                                        line.to_string()
                                    } else {
                                        let content = &line[colon_pos + 1..];
                                        format!("{}:{}", lazy_promise.lazy_row_id, content)
                                    }
                                } else {
                                    line.to_string()
                                }
                            } else {
                                line.to_string()
                            }
                        } else {
                            line.to_string()
                        };

                        rsc_wire_format.push('\n');
                        rsc_wire_format.push_str(&line_to_append);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to resolve lazy promise {}: {}",
                        lazy_promise.promise_id,
                        e
                    );
                }
            }
        }

        if let Err(e) = Self::validate_html_structure(&rsc_wire_format, route_match) {
            error!("HTML structure validation failed: {}", e);
        }

        Ok(rsc_wire_format)
    }

    pub async fn render_route_by_mode(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        mode: RenderMode,
        request_context: Option<
            std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        >,
    ) -> Result<String, RariError> {
        match mode {
            RenderMode::Ssr => {
                match self
                    .render_route_to_html_direct(route_match, context, request_context)
                    .await?
                {
                    RenderResult::Static(html) => Ok(html),
                    RenderResult::Streaming(_) => Err(RariError::internal(
                        "Streaming result not supported in render_route_by_mode",
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
                return Err(RariError::internal(format!(
                    "Skeleton position validation failed: boundary '{}' is not in content area",
                    boundary.boundary_id
                )));
            }

            if boundary.dom_path.is_empty() {
                return Err(RariError::internal(format!(
                    "Skeleton position validation failed: boundary '{}' has empty DOM path",
                    boundary.boundary_id
                )));
            }
        }

        Ok(())
    }

    #[cfg(test)]
    pub(super) fn validate_layout_structure(
        &self,
        rsc_data: &serde_json::Value,
        route_match: &AppRouteMatch,
    ) -> Result<LayoutStructure, RariError> {
        self.validate_layout_structure_impl(rsc_data, route_match)
    }

    fn validate_layout_structure_impl(
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

                        if (tag == "$Sreact.suspense"
                            || tag == "react.suspense"
                            || tag == "Suspense")
                            && let Some(props) = arr.get(3).and_then(|v| v.as_object())
                            && let Some(boundary_id) =
                                props.get("~boundaryId").and_then(|v| v.as_str())
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
                    if obj.contains_key("~preSerializedSuspense") {
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

        let boundary_positions = utils::calculate_boundary_positions(&layout_structure);

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

    #[cfg(test)]
    pub(super) fn detect_suspense_boundaries(
        &self,
        rsc_json: &serde_json::Value,
    ) -> Result<SuspenseDetectionResult, RariError> {
        self.detect_suspense_boundaries_impl(rsc_json)
    }

    fn detect_suspense_boundaries_impl(
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
                        && (arr[1].as_str() == Some("$Sreact.suspense")
                            || arr[1].as_str() == Some("react.suspense")
                            || arr[1].as_str() == Some("Suspense"))
                    {
                        *count += 1;

                        if let Some(props) = arr.get(3).and_then(|v| v.as_object())
                            && let Some(boundary_id) =
                                props.get("~boundaryId").and_then(|v| v.as_str())
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
                    if obj.contains_key("~preSerializedSuspense") {
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
        request_context: Option<
            std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        >,
    ) -> Result<RenderResult, RariError> {
        let cache_key = utils::generate_cache_key(route_match, context);
        let is_not_found = route_match.not_found.is_some();

        if is_not_found && let Some(cached_html) = self.html_cache.get(cache_key) {
            return Ok(RenderResult::Static(cached_html));
        }

        let loading_enabled = Config::get().map(|config| config.loading.enabled).unwrap_or(true);

        let loading_component_id = if loading_enabled {
            if let Some(loading_entry) = &route_match.loading {
                let loading_id = utils::create_component_id(&loading_entry.file_path);
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

        if let Some(ref ctx) = request_context
            && let Err(e) = renderer.runtime.set_request_context(ctx.clone()).await
        {
            error!("Failed to set request context: {}", e);
        }

        let is_not_found = route_match.not_found.is_some();

        if !is_not_found {
            renderer
                .runtime
                .execute_script(
                    "enable_streaming".to_string(),
                    "globalThis.__RARI_STREAMING_SUSPENSE__ = true;".to_string(),
                )
                .await?;

            let resolve_helper = include_str!("js/resolve_lazy_helper.js");
            renderer
                .runtime
                .execute_script("inject_lazy_resolver".to_string(), resolve_helper.to_string())
                .await?;
        }

        let promise_result = renderer
            .runtime
            .execute_script("compose_and_render".to_string(), composition_script.clone())
            .await?;

        let result = if promise_result.is_object() && promise_result.get("rsc_data").is_some() {
            promise_result
        } else {
            renderer
                .runtime
                .execute_script("get_result".to_string(), JS_GET_RESULT.to_string())
                .await?
        };

        let rsc_data = result.get("rsc_data").ok_or_else(|| {
            tracing::error!("Failed to extract RSC data from result: {:?}", result);
            RariError::internal("No RSC data in render result")
        })?;

        let _layout_structure = match self.validate_layout_structure_impl(rsc_data, route_match) {
            Ok(structure) => {
                if !structure.is_valid() {
                    structure
                } else {
                    if let Err(e) = self.validate_skeleton_positions(&structure) {
                        error!("Skeleton position validation failed: {}", e);
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

                LayoutStructure::new()
            }
        };

        let _suspense_detection = self.detect_suspense_boundaries_impl(rsc_data)?;

        if !is_not_found {
            let (rsc_wire_format, pending_promises) = {
                let mut serializer = renderer.serializer.lock();

                serializer.reset_for_new_request();

                let wire_format = serializer.serialize_rsc_json(rsc_data).map_err(|e| {
                    RariError::internal(format!("Failed to serialize RSC data: {}", e))
                })?;

                let promises = serializer.pending_lazy_promises.clone();
                serializer.pending_lazy_promises.clear();
                serializer.seen_lazy_promise_ids.clear();

                (wire_format, promises)
            };

            let runtime_for_task = Arc::clone(&renderer.runtime);
            let serializer_for_task = Arc::clone(&renderer.serializer);

            drop(renderer);

            let wire_lines: Vec<String> = rsc_wire_format.lines().map(|s| s.to_string()).collect();

            let (chunk_sender, chunk_receiver) =
                mpsc::channel::<crate::rsc::rendering::streaming::RscStreamChunk>(64);

            tokio::spawn(async move {
                let mut row_id = 0u32;

                for line in wire_lines {
                    if line.trim().is_empty() {
                        continue;
                    }

                    row_id += 1;

                    let chunk_type = if line.contains(":I[") {
                        crate::rsc::rendering::streaming::RscChunkType::ModuleImport
                    } else {
                        crate::rsc::rendering::streaming::RscChunkType::InitialShell
                    };

                    let chunk = crate::rsc::rendering::streaming::RscStreamChunk {
                        data: format!("{}\n", line).into_bytes(),
                        chunk_type,
                        row_id,
                        is_final: false,
                        boundary_id: None,
                    };

                    if chunk_sender.send(chunk).await.is_err() {
                        break;
                    }
                }

                for lazy_promise in pending_promises {
                    let resolve_script = format!(
                        "(async () => {{ return await globalThis.__RARI_RESOLVE_LAZY__('{}'); }})()",
                        lazy_promise.promise_id
                    );

                    let result = runtime_for_task
                        .execute_script(
                            format!("resolve_promise_{}", lazy_promise.promise_id),
                            resolve_script,
                        )
                        .await;

                    match result {
                        Ok(result) => {
                            if let Some(success) = result.get("success").and_then(|v| v.as_bool())
                                && !success
                            {
                                let error_msg = result
                                    .get("error")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown error");
                                tracing::error!(
                                    "Failed to resolve lazy promise {}: {}",
                                    lazy_promise.promise_id,
                                    error_msg
                                );
                                continue;
                            }

                            let resolved_content = result.get("data").unwrap_or(&result);

                            let wire_format = {
                                let mut serializer = serializer_for_task.lock();
                                match serializer.serialize_rsc_json(resolved_content) {
                                    Ok(wire_format) => wire_format,
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to serialize resolved content: {}",
                                            e
                                        );
                                        continue;
                                    }
                                }
                            };

                            let lines: Vec<&str> = wire_format.lines().collect();
                            let mut content_row_id: Option<u32> = None;

                            for line in lines.iter() {
                                if line.trim().is_empty() {
                                    continue;
                                }

                                if let Some(colon_pos) = line.find(':')
                                    && let Ok(row_id) = line[..colon_pos].parse::<u32>()
                                {
                                    let content = &line[colon_pos + 1..];

                                    if !content.starts_with("I[") && content_row_id.is_none() {
                                        content_row_id = Some(row_id);
                                        break;
                                    }
                                }
                            }

                            let mut rows_to_send: Vec<(u32, String)> = Vec::new();

                            for line in lines.iter() {
                                if line.trim().is_empty() {
                                    continue;
                                }

                                if let Some(colon_pos) = line.find(':')
                                    && let Ok(row_id) = line[..colon_pos].parse::<u32>()
                                {
                                    let content = &line[colon_pos + 1..];

                                    if Some(row_id) == content_row_id {
                                        let renumbered_line =
                                            format!("{}:{}", lazy_promise.lazy_row_id, content);
                                        rows_to_send
                                            .push((lazy_promise.lazy_row_id, renumbered_line));
                                    } else {
                                        rows_to_send.push((row_id, line.to_string()));
                                    }
                                }
                            }

                            rows_to_send.sort_by_key(|(row_id, _)| *row_id);

                            for (_, line_to_send) in rows_to_send {
                                let chunk = crate::rsc::rendering::streaming::RscStreamChunk {
                                    data: format!("{}\n", line_to_send).into_bytes(),
                                    chunk_type: crate::rsc::rendering::streaming::RscChunkType::BoundaryUpdate,
                                    row_id: lazy_promise.lazy_row_id,
                                    is_final: false,
                                    boundary_id: Some(lazy_promise.promise_id.clone()),
                                };

                                if chunk_sender.send(chunk).await.is_err() {
                                    tracing::error!(
                                        "[Streaming] Failed to send chunk for lazy promise {}",
                                        lazy_promise.promise_id
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to resolve lazy promise {}: {}",
                                lazy_promise.promise_id,
                                e
                            );
                        }
                    }
                }

                let final_chunk = crate::rsc::rendering::streaming::RscStreamChunk {
                    data: Vec::new(),
                    chunk_type: crate::rsc::rendering::streaming::RscChunkType::StreamComplete,
                    row_id: 0,
                    is_final: true,
                    boundary_id: None,
                };

                let _ = chunk_sender.send(final_chunk).await;
            });

            let stream = crate::rsc::rendering::streaming::RscStream::new(chunk_receiver);
            return Ok(RenderResult::Streaming(stream));
        }

        let page_props = utils::create_page_props(route_match, context)?;
        let page_component_id = utils::create_component_id(&route_match.route.file_path);

        #[allow(clippy::disallowed_methods)]
        let layouts: Vec<serde_json::Value> = route_match
            .layouts
            .iter()
            .map(|layout| {
                serde_json::json!({
                    "componentId": utils::create_component_id(&layout.file_path),
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

        self.html_cache.insert(cache_key, html.clone());

        Ok(RenderResult::Static(html))
    }

    fn validate_html_structure(html: &str, route_match: &AppRouteMatch) -> Result<(), RariError> {
        let root_layout_path =
            route_match.layouts.iter().find(|l| l.is_root).map(|l| l.file_path.as_str());

        if html.contains("<div><html")
            || html.contains("\"div\",null,{\"children\":[\"$\",\"html\"")
        {
            let error_msg =
                error_messages::create_wrapped_html_error_message(route_match, root_layout_path);
            return Err(RariError::internal(error_msg));
        }

        Ok(())
    }

    fn validate_rsc_wire_format(rsc_data: &str) -> Result<(), RariError> {
        if rsc_data.trim().is_empty() {
            let error_msg = error_messages::create_empty_rsc_error_message();
            return Err(RariError::internal(error_msg));
        }

        Ok(())
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
                .send(crate::rsc::rendering::streaming::RscStreamChunk {
                    data: html.into_bytes(),
                    chunk_type: crate::rsc::rendering::streaming::RscChunkType::InitialShell,
                    row_id: 0,
                    is_final: true,
                    boundary_id: None,
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
            .send(crate::rsc::rendering::streaming::RscStreamChunk {
                data: html.into_bytes(),
                chunk_type: crate::rsc::rendering::streaming::RscChunkType::InitialShell,
                row_id: 0,
                is_final: true,
                boundary_id: None,
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
        let page_props = utils::create_page_props(route_match, context).map_err(|e| {
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

        let page_component_id = utils::create_component_id(&route_match.route.file_path);

        let page_render_script = if let Some(loading_id) = loading_component_id {
            let loading_file_path =
                route_match.loading.as_ref().map(|l| l.file_path.as_str()).unwrap_or("");

            JS_PAGE_RENDER_WITH_LOADING
                .cow_replace("{page_component_id}", &page_component_id)
                .cow_replace("{loading_id}", loading_id)
                .cow_replace("{page_props_json}", &page_props_json)
                .cow_replace("{use_suspense}", if use_suspense { "true" } else { "false" })
                .cow_replace("{route_file_path}", &route_match.route.file_path)
                .cow_replace("{loading_file_path}", loading_file_path)
                .into_owned()
        } else {
            JS_PAGE_RENDER_SIMPLE
                .cow_replace("{page_component_id}", &page_component_id)
                .cow_replace("{page_props_json}", &page_props_json)
                .into_owned()
        };

        let mut script = format!(
            r#"
            (async () => {{
                const timings = {{}};
                const startTotal = performance.now();

                const React = globalThis.React;
                const ReactDOMServer = globalThis.ReactDOMServer;

                if (!globalThis['~suspense']) globalThis['~suspense'] = {{}};
                globalThis['~suspense'].discoveredBoundaries = [];
                globalThis['~suspense'].pendingPromises = [];
                if (!globalThis['~render']) globalThis['~render'] = {{}};
                globalThis['~render'].deferredAsyncComponents = [];

                if (!globalThis['~react']) globalThis['~react'] = {{}};
                if (!globalThis['~react'].originalCreateElement) {{
                    globalThis['~react'].originalCreateElement = React.createElement;

                    React.createElement = function(type, props, ...children) {{
                        if (typeof type === 'function' &&
                            (type.constructor.name === 'AsyncFunction' ||
                             type.toString().trim().startsWith('async '))) {{

                            const AsyncComponentMarker = function(props) {{
                                return null;
                            }};

                            AsyncComponentMarker._isAsyncComponent = true;
                            AsyncComponentMarker._originalType = type;
                            AsyncComponentMarker.displayName = `AsyncWrapper(${{type.name || 'Anonymous'}})`;

                            return globalThis['~react'].originalCreateElement(AsyncComponentMarker, props, ...children);
                        }}

                        return globalThis['~react'].originalCreateElement(type, props, ...children);
                    }};
                }}

                const startPageRender = performance.now();
                {}
            "#,
            page_render_script
        );

        let mut current_element = "pageElement".to_string();

        for (i, layout) in route_match.layouts.iter().rev().enumerate() {
            let layout_component_id = utils::create_component_id(&layout.file_path);
            let layout_var = format!("layout{}", i);

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

            current_element = layout_var;
        }

        script.push_str("\n\n");
        script.push_str("                const startRSC = performance.now();\n");
        script.push_str(&format!("                const rscData = await globalThis.renderToRsc({}, globalThis['~rsc'].clientComponents || {{}});\n", current_element));
        script.push_str(r#"                timings.rscConversion = performance.now() - startRSC;

                timings.total = performance.now() - startTotal;

                const deferredComponents = globalThis['~render']?.deferredAsyncComponents || [];
                const hasAsync = deferredComponents.length > 0;
                const deferredCount = deferredComponents.length;

                const result = {
                    rsc_data: rscData,
                    boundaries: globalThis['~suspense']?.discoveredBoundaries || [],
                    pending_promises: globalThis['~suspense']?.pendingPromises || [],
                    has_suspense: (globalThis['~suspense']?.discoveredBoundaries && globalThis['~suspense'].discoveredBoundaries.length > 0) ||
                                 (globalThis['~suspense']?.pendingPromises && globalThis['~suspense'].pendingPromises.length > 0),
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
                    globalThis['~rsc'].renderResult = cleanResult;
                    return cleanResult;
                } catch (jsonError) {
                    globalThis['~rsc'].renderResult = result;
                    return result;
                }
            })()
            "#);

        Ok(script)
    }

    pub async fn render_loading(
        &self,
        loading_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(loading_path);

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, None).await
    }

    pub async fn render_error(
        &self,
        error_path: &str,
        error: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = utils::get_component_id(error_path);

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
        let component_id = utils::get_component_id(not_found_path);

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
