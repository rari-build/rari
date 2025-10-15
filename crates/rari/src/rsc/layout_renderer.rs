use crate::error::RariError;
use crate::rsc::renderer::RscRenderer;
use crate::rsc::streaming::RscStream;
use crate::server::app_router::AppRouteMatch;
use crate::server::request_type::RenderMode;
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct LayoutRenderContext {
    pub params: FxHashMap<String, String>,
    pub search_params: FxHashMap<String, Vec<String>>,
    pub headers: FxHashMap<String, String>,
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
        debug!(
            "Rendering route {} with {} layouts (RSC wire format path for client navigation)",
            route_match.route.path,
            route_match.layouts.len()
        );

        let composition_script = self.build_composition_script(route_match, context)?;

        debug!("Executing composition script to render composed component tree");

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

        let result = if promise_result.is_object() && promise_result.get("rsc").is_some() {
            promise_result
        } else {
            let get_result_script = r#"globalThis.__rsc_render_result"#.to_string();
            renderer.runtime.execute_script("get_result".to_string(), get_result_script).await?
        };

        let rsc_data = result.get("rsc").ok_or_else(|| {
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

        debug!("RSC wire format generation completed for {}", route_match.route.path);

        Ok(rsc_wire_format)
    }

    pub async fn render_route_optimized(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        mode: RenderMode,
        request_context: Option<std::sync::Arc<crate::server::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        debug!("Rendering route {} in {:?} mode", route_match.route.path, mode);

        let result = match mode {
            RenderMode::Ssr => {
                debug!("Using direct HTML rendering path for SSR");
                self.render_route_to_html_direct(route_match, context, request_context).await
            }
            RenderMode::RscNavigation => {
                debug!("Using RSC wire format path for client navigation");
                self.render_route(route_match, context, request_context).await
            }
        };

        match mode {
            RenderMode::Ssr => {
                debug!("SSR direct HTML path completed for {}", route_match.route.path);
            }
            RenderMode::RscNavigation => {
                debug!("RSC navigation path completed for {}", route_match.route.path);
            }
        }

        result
    }

    pub async fn render_route_to_html_direct(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
        _request_context: Option<std::sync::Arc<crate::server::request_context::RequestContext>>,
    ) -> Result<String, RariError> {
        let cache_key = self.generate_cache_key(route_match, context);

        let should_skip_cache = route_match.route.path.contains("/actions")
            || route_match.route.path.contains("/interactive");

        if !should_skip_cache && let Some(cached_html) = self.html_cache.get(cache_key) {
            debug!("Cache HIT for route {} (key: {})", route_match.route.path, cache_key);
            return Ok(cached_html);
        }

        debug!("Cache MISS for route {} (key: {})", route_match.route.path, cache_key);

        debug!(
            "Direct HTML rendering START for {} (SSR path - bypasses RSC wire format)",
            route_match.route.path
        );

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

        let renderer = self.renderer.lock().await;

        if let Some(ref ctx) = _request_context
            && let Err(e) = renderer.runtime.set_request_context(ctx.clone()).await
        {
            tracing::warn!("Failed to set request context in runtime: {}", e);
        }

        debug!("Calling renderRouteToHtmlDirect...");
        let result = renderer
            .runtime
            .execute_function(
                "renderRouteToHtmlDirect",
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

        debug!("Direct HTML rendering completed for {}", route_match.route.path);

        use crate::server::html_diagnostics::HtmlDiagnostics;
        HtmlDiagnostics::log_html_snippet(&html, "After renderRouteToHtmlDirect", 300);
        HtmlDiagnostics::check_root_element(&html, "After renderRouteToHtmlDirect");

        if !should_skip_cache {
            self.html_cache.insert(cache_key, html.clone());
        }

        Ok(html)
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
        debug!(
            "Streaming route {} with {} layouts",
            route_match.route.path,
            route_match.layouts.len()
        );

        let has_root_layout = route_match.layouts.iter().any(|l| l.is_root);

        if has_root_layout {
            debug!("Route has root layout - streaming entire document as single unit");

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
            // TODO: Implement true progressive streaming for nested layouts
            debug!("Route has no root layout - using standard streaming");

            self.render_route_streaming_progressive(route_match, context).await
        }
    }

    async fn render_route_streaming_progressive(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<RscStream, RariError> {
        debug!("Progressive streaming not yet implemented - falling back to synchronous render");

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

    fn build_composition_script(
        &self,
        route_match: &AppRouteMatch,
        context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let page_props = self.create_page_props(route_match, context)?;
        let page_props_json = serde_json::to_string(&page_props)
            .map_err(|e| RariError::internal(format!("Failed to serialize page props: {e}")))?;

        let page_component_id = self.create_component_id(&route_match.route.file_path);

        let mut script = format!(
            r#"
            (async () => {{
                const timings = {{}};
                const startTotal = performance.now();

                const React = globalThis.React || require('react');
                const ReactDOMServer = globalThis.ReactDOMServer || require('react-dom/server');

                const startPage = performance.now();
                const PageComponent = globalThis["{}"];
                if (!PageComponent || typeof PageComponent !== 'function') {{
                    throw new Error('Page component {} not found');
                }}

                const pageProps = {};
                const pageResult = PageComponent(pageProps);
                const pageElement = pageResult && typeof pageResult.then === 'function'
                    ? await pageResult
                    : pageResult;
                timings.pageRender = performance.now() - startPage;
            "#,
            page_component_id, page_component_id, page_props_json
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

                const {} = LayoutComponent{}({{ children: {} }});
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

                const {} = LayoutComponent{}({{ children: {} }});
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
                    i,
                    i
                ));
            }

            current_element = layout_var;
        }

        script.push_str(&format!(
            r#"
                globalThis.__rsc_render_result = null;

                const traverseToRSC = globalThis.traverseToRSC;
                if (!traverseToRSC) {{
                    throw new Error('traverseToRSC not available - RSC runtime not initialized');
                }}

                if (!globalThis.renderToRSC) {{
                    globalThis.renderToRSC = async function(element, clientComponents = {{}}) {{
                        if (!element) return null;

                        if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {{
                            return element;
                        }}

                        if (Array.isArray(element)) {{
                            const results = [];
                            for (const child of element) {{
                                results.push(await globalThis.renderToRSC(child, clientComponents));
                            }}
                            return results;
                        }}

                        if (element && typeof element === 'object') {{
                            const uniqueKey = element.key || null;

                            if (element.type) {{
                                if (typeof element.type === 'string') {{
                                    const props = element.props || {{}};
                                    const {{ children: propsChildren, ...otherProps }} = props;

                                    const actualChildren = element.children || propsChildren;

                                    const rscProps = {{
                                        ...otherProps,
                                        children: actualChildren ? await globalThis.renderToRSC(actualChildren, clientComponents) : undefined
                                    }};
                                    if (rscProps.children === undefined) {{
                                        delete rscProps.children;
                                    }}
                                    return ["$", element.type, uniqueKey, rscProps];
                                }} else if (typeof element.type === 'function') {{
                                    try {{
                                        const props = element.props || {{}};
                                        let result = element.type(props);

                                        if (result && typeof result.then === 'function') {{
                                            result = await result;
                                        }}

                                        return await globalThis.renderToRSC(result, clientComponents);
                                    }} catch (error) {{
                                        console.error('Error rendering function component:', error);
                                        return ["$", "div", uniqueKey, {{
                                            children: `Error: ${{error.message}}`,
                                            style: {{ color: 'red', border: '1px solid red', padding: '10px' }}
                                        }}];
                                    }}
                                }}
                            }}

                            return ["$", "div", uniqueKey, {{
                                className: "rsc-unknown",
                                children: "Unknown element type"
                            }}];
                        }}

                        return element;
                    }};
                }}

                const startRSC = performance.now();
                const rscData = await traverseToRSC({});
                timings.rscConversion = performance.now() - startRSC;

                timings.total = performance.now() - startTotal;

                globalThis.__rsc_render_result = {{ rsc: rscData, timings }};
                return globalThis.__rsc_render_result;
            }})();
            "#,
            current_element
        ));

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

    pub async fn render_loading(
        &self,
        loading_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = self.get_component_id(loading_path);

        debug!("Rendering loading component: {}", component_id);

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

        debug!("Rendering error component: {} with error: {}", component_id, error);

        let mut renderer = self.renderer.lock().await;
        renderer.render_to_string(&component_id, Some(&props_json)).await
    }

    pub async fn render_not_found(
        &self,
        not_found_path: &str,
        _context: &LayoutRenderContext,
    ) -> Result<String, RariError> {
        let component_id = self.get_component_id(not_found_path);

        debug!("Rendering not-found component: {}", component_id);

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
) -> LayoutRenderContext {
    LayoutRenderContext { params, search_params, headers }
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
}
