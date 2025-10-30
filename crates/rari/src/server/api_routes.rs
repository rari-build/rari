use crate::error::RariError;
use crate::runtime::JsExecutionRuntime;
use crate::runtime::api_bridge::RequestBridge;
use axum::body::Body;
use axum::http::{HeaderMap, Request, Response};
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRouteEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub segments: Vec<RouteSegment>,
    pub params: Vec<String>,
    #[serde(rename = "isDynamic")]
    pub is_dynamic: bool,
    pub methods: Vec<String>,
}

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
pub struct ApiRouteManifest {
    #[serde(rename = "apiRoutes", default)]
    pub api_routes: Vec<ApiRouteEntry>,
}

#[derive(Debug, Clone)]
pub struct ApiRouteMatch {
    pub route: ApiRouteEntry,
    pub params: FxHashMap<String, String>,
    pub method: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledHandler {
    module_id: String,
    code: String,
    #[allow(dead_code)]
    methods: Vec<String>,
    last_modified: SystemTime,
}

pub struct ApiRouteHandler {
    runtime: Arc<JsExecutionRuntime>,
    manifest: Arc<ApiRouteManifest>,
    handler_cache: Arc<DashMap<String, CompiledHandler>>,
}

impl ApiRouteHandler {
    pub fn new(runtime: Arc<JsExecutionRuntime>, manifest: ApiRouteManifest) -> Self {
        info!(route_count = manifest.api_routes.len(), "Initializing API route handler");

        for route in &manifest.api_routes {
            info!(
                route_path = %route.path,
                file_path = %route.file_path,
                methods = ?route.methods,
                is_dynamic = route.is_dynamic,
                params = ?route.params,
                "Registered API route"
            );
        }

        Self { runtime, manifest: Arc::new(manifest), handler_cache: Arc::new(DashMap::new()) }
    }

    pub async fn from_file(
        runtime: Arc<JsExecutionRuntime>,
        manifest_path: &str,
    ) -> Result<Self, RariError> {
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(|e| RariError::io(format!("Failed to read API route manifest: {e}")))?;

        let manifest: ApiRouteManifest = serde_json::from_str(&content).map_err(|e| {
            RariError::configuration(format!("Failed to parse API route manifest: {e}"))
        })?;

        Ok(Self::new(runtime, manifest))
    }

    pub fn manifest(&self) -> &ApiRouteManifest {
        &self.manifest
    }

    pub fn runtime(&self) -> &Arc<JsExecutionRuntime> {
        &self.runtime
    }

    pub fn clear_cache(&self) {
        debug!("Clearing API route handler cache");
        self.handler_cache.clear();
    }

    pub fn invalidate_handler(&self, file_path: &str) {
        debug!("Invalidating handler cache for: {}", file_path);
        self.handler_cache.remove(file_path);
    }

    pub fn get_supported_methods(&self, path: &str) -> Option<Vec<String>> {
        let normalized_path = Self::normalize_path(path);

        for route in &self.manifest.api_routes {
            if self.match_route_pattern(route, &normalized_path).is_some() {
                return Some(route.methods.clone());
            }
        }

        None
    }

    pub fn match_route(&self, path: &str, method: &str) -> Result<ApiRouteMatch, RariError> {
        let normalized_path = Self::normalize_path(path);
        debug!(
            path = %normalized_path,
            method = %method,
            "Matching API route"
        );

        for route in &self.manifest.api_routes {
            if let Some(params) = self.match_route_pattern(route, &normalized_path) {
                if !route.methods.iter().any(|m| m.eq_ignore_ascii_case(method)) {
                    debug!(
                        route_path = %route.path,
                        requested_method = %method,
                        supported_methods = ?route.methods,
                        "Route matched path but method not supported"
                    );
                    return Err(RariError::bad_request(format!(
                        "Method {} not allowed for route {}. Supported methods: {}",
                        method,
                        route.path,
                        route.methods.join(", ")
                    ))
                    .with_property("error_type", "method_not_allowed")
                    .with_property("allowed_methods", &route.methods.join(",")));
                }

                debug!(
                    request_path = %normalized_path,
                    route_path = %route.path,
                    method = %method,
                    params = ?params,
                    "Successfully matched API route"
                );

                return Ok(ApiRouteMatch {
                    route: route.clone(),
                    params,
                    method: method.to_string(),
                });
            }
        }

        debug!(
            path = %path,
            method = %method,
            available_routes = ?self.manifest.api_routes.iter().map(|r| &r.path).collect::<Vec<_>>(),
            "No matching API route found"
        );

        Err(RariError::not_found(format!("No API route found for path: {}", path)))
    }

    fn match_route_pattern(
        &self,
        route: &ApiRouteEntry,
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

    pub(crate) async fn load_handler(
        &self,
        route: &ApiRouteEntry,
        is_development: bool,
    ) -> Result<CompiledHandler, RariError> {
        let file_path = &route.file_path;

        if let Some(cached) = self.handler_cache.get(file_path) {
            if is_development {
                let dist_path = Self::resolve_dist_path(file_path)?;
                if let Ok(metadata) = tokio::fs::metadata(&dist_path).await
                    && let Ok(modified) = metadata.modified()
                {
                    if modified <= cached.last_modified {
                        debug!(
                            file_path = %file_path,
                            module_id = %cached.module_id,
                            "Using cached handler"
                        );
                        return Ok(cached.clone());
                    } else {
                        debug!(
                            file_path = %file_path,
                            "Handler file modified, reloading"
                        );
                    }
                }
            } else {
                debug!(
                    file_path = %file_path,
                    module_id = %cached.module_id,
                    "Using cached handler (production mode)"
                );
                return Ok(cached.clone());
            }
        }

        info!(
            file_path = %file_path,
            route_path = %route.path,
            "Loading API route handler"
        );

        let dist_path = Self::resolve_dist_path(file_path)?;

        if !dist_path.exists() {
            error!(
                file_path = %file_path,
                dist_path = %dist_path.display(),
                route_path = %route.path,
                "Handler file not found"
            );
            return Err(RariError::not_found(format!(
                "Handler file not found: {}",
                dist_path.display()
            )));
        }

        let code = tokio::fs::read_to_string(&dist_path).await.map_err(|e| {
            error!(
                file_path = %file_path,
                dist_path = %dist_path.display(),
                error = %e,
                "Failed to read handler file"
            );
            RariError::io(format!("Failed to read handler file: {e}"))
        })?;

        let last_modified = tokio::fs::metadata(&dist_path)
            .await
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());

        let methods = Self::detect_http_methods(&code);

        if methods.is_empty() {
            warn!(
                file_path = %file_path,
                route_path = %route.path,
                "No HTTP method handlers found in file"
            );
        }

        let module_id = file_path
            .trim_start_matches("api/")
            .trim_end_matches(".ts")
            .trim_end_matches(".tsx")
            .trim_end_matches(".js")
            .trim_end_matches(".jsx")
            .replace('/', "_");

        let compiled = CompiledHandler {
            module_id: module_id.clone(),
            code: code.clone(),
            methods: methods.clone(),
            last_modified,
        };

        self.handler_cache.insert(file_path.clone(), compiled.clone());

        info!(
            file_path = %file_path,
            route_path = %route.path,
            module_id = %module_id,
            methods = ?methods,
            code_size = code.len(),
            "Successfully loaded and cached API route handler"
        );

        Ok(compiled)
    }

    fn file_path_to_module_key(file_path: &str) -> String {
        let mut module_key = format!("app/{}", file_path);

        if module_key.ends_with(".ts") {
            module_key = module_key[..module_key.len() - 3].to_string();
        } else if module_key.ends_with(".tsx") {
            module_key = module_key[..module_key.len() - 4].to_string();
        } else if module_key.ends_with(".js") {
            module_key = module_key[..module_key.len() - 3].to_string();
        } else if module_key.ends_with(".jsx") {
            module_key = module_key[..module_key.len() - 4].to_string();
        }

        let mut result = String::new();
        let mut chars = module_key.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '[' {
                if chars.peek() == Some(&'[') {
                    chars.next();

                    if chars.peek() == Some(&'.') {
                        chars.next();
                        if chars.peek() == Some(&'.') {
                            chars.next();
                            if chars.peek() == Some(&'.') {
                                chars.next();
                                result.push_str("_____");

                                while let Some(ch) = chars.next() {
                                    if ch == ']' && chars.peek() == Some(&']') {
                                        chars.next();
                                        result.push_str("__");
                                        break;
                                    } else {
                                        result.push(ch);
                                    }
                                }
                            }
                        }
                    }
                } else if chars.peek() == Some(&'.') {
                    chars.next();
                    if chars.peek() == Some(&'.') {
                        chars.next();
                        if chars.peek() == Some(&'.') {
                            chars.next();
                            result.push_str("____");

                            for ch in chars.by_ref() {
                                if ch == ']' {
                                    result.push('_');
                                    break;
                                } else {
                                    result.push(ch);
                                }
                            }
                        }
                    }
                } else {
                    result.push('_');

                    for ch in chars.by_ref() {
                        if ch == ']' {
                            result.push('_');
                            break;
                        } else {
                            result.push(ch);
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn resolve_dist_path(file_path: &str) -> Result<std::path::PathBuf, RariError> {
        let mut normalized_path = String::new();
        let mut chars = file_path.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '[' {
                if chars.peek() == Some(&'[') {
                    chars.next();

                    if chars.peek() == Some(&'.') {
                        chars.next();
                        if chars.peek() == Some(&'.') {
                            chars.next();
                            if chars.peek() == Some(&'.') {
                                chars.next();
                                normalized_path.push_str("_____");

                                while let Some(ch) = chars.next() {
                                    if ch == ']' && chars.peek() == Some(&']') {
                                        chars.next();
                                        normalized_path.push_str("__");
                                        break;
                                    } else {
                                        normalized_path.push(ch);
                                    }
                                }
                            }
                        }
                    }
                } else if chars.peek() == Some(&'.') {
                    chars.next();
                    if chars.peek() == Some(&'.') {
                        chars.next();
                        if chars.peek() == Some(&'.') {
                            chars.next();
                            normalized_path.push_str("____");
                            for ch in chars.by_ref() {
                                if ch == ']' {
                                    normalized_path.push('_');
                                    break;
                                } else {
                                    normalized_path.push(ch);
                                }
                            }
                        }
                    }
                } else {
                    normalized_path.push('_');
                    for ch in chars.by_ref() {
                        if ch == ']' {
                            normalized_path.push('_');
                            break;
                        } else {
                            normalized_path.push(ch);
                        }
                    }
                }
            } else {
                normalized_path.push(ch);
            }
        }

        let dist_path =
            Path::new("dist").join("server").join("app").join(normalized_path).with_extension("js");

        Ok(dist_path)
    }

    fn detect_http_methods(code: &str) -> Vec<String> {
        let http_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
        let mut detected = Vec::new();

        for method in http_methods {
            let patterns = [
                format!(r"export\s+function\s+{}\s*\(", method),
                format!(r"export\s+async\s+function\s+{}\s*\(", method),
                format!(r"export\s+const\s+{}\s*=", method),
                format!(r"export\s+const\s+{}\s*:", method),
            ];

            for pattern in &patterns {
                if let Ok(re) = regex::Regex::new(pattern)
                    && re.is_match(code)
                {
                    detected.push(method.to_string());
                    break;
                }
            }
        }

        detected
    }

    pub async fn execute_handler(
        &self,
        route_match: &ApiRouteMatch,
        request: Request<Body>,
        is_development: bool,
    ) -> Result<Response<Body>, RariError> {
        let start_time = std::time::Instant::now();

        debug!(
            route_path = %route_match.route.path,
            method = %route_match.method,
            params = ?route_match.params,
            "Executing API route handler"
        );

        let handler = self.load_handler(&route_match.route, is_development).await.map_err(|e| {
            error!(
                route_path = %route_match.route.path,
                method = %route_match.method,
                error = %e,
                "Failed to load handler"
            );
            e
        })?;

        let (parts, body) = request.into_parts();

        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(|e| {
            error!(
                route_path = %route_match.route.path,
                method = %route_match.method,
                error = %e,
                "Failed to read request body"
            );
            RariError::bad_request(format!("Failed to read request body: {e}"))
        })?;

        let body_string = String::from_utf8_lossy(&body_bytes).to_string();

        debug!(
            route_path = %route_match.route.path,
            method = %route_match.method,
            body_size = body_bytes.len(),
            "Request body read successfully"
        );

        let request_obj = self.create_request_object(
            parts.method.as_ref(),
            &parts.uri.to_string(),
            &parts.headers,
            &body_string,
            &route_match.params,
        )?;

        let script = self.create_handler_execution_script(
            &handler,
            &route_match.method,
            &request_obj,
            &route_match.route.file_path,
        )?;

        debug!(
            route_path = %route_match.route.path,
            method = %route_match.method,
            module_id = %handler.module_id,
            "Executing handler in JavaScript runtime"
        );

        let result = self
            .runtime
            .execute_script(format!("api_route_{}", handler.module_id), script)
            .await
            .map_err(|e| {
                error!(
                    route_path = %route_match.route.path,
                    method = %route_match.method,
                    module_id = %handler.module_id,
                    error = %e,
                    "Handler execution failed"
                );
                RariError::js_execution(format!("Handler execution failed: {e}"))
            })?;

        let response = self.create_response(result).await.map_err(|e| {
            error!(
                route_path = %route_match.route.path,
                method = %route_match.method,
                error = %e,
                "Failed to create response from handler result"
            );
            e
        })?;

        let elapsed = start_time.elapsed();
        info!(
            route_path = %route_match.route.path,
            method = %route_match.method,
            status = response.status().as_u16(),
            duration_ms = elapsed.as_millis(),
            "API route handler executed successfully"
        );

        Ok(response)
    }

    fn create_request_object(
        &self,
        method: &str,
        uri: &str,
        headers: &HeaderMap,
        body: &str,
        params: &FxHashMap<String, String>,
    ) -> Result<JsonValue, RariError> {
        RequestBridge::to_json(method, uri, headers, body, params)
    }

    fn create_handler_execution_script(
        &self,
        handler: &CompiledHandler,
        method: &str,
        request_obj: &JsonValue,
        file_path: &str,
    ) -> Result<String, RariError> {
        let request_json = serde_json::to_string(request_obj)
            .map_err(|e| RariError::serialization(format!("Failed to serialize request: {e}")))?;

        let module_key = Self::file_path_to_module_key(file_path);

        let script = format!(
            r#"
(async function() {{
    try {{
        {handler_code}

        const requestData = {request_json};

        const url = new URL(requestData.url, 'http://localhost');

        if (requestData.params) {{
            for (const [key, value] of Object.entries(requestData.params)) {{
                url.searchParams.set(key, value);
            }}
        }}

        const headers = new Headers(requestData.headers || {{}});

        const request = new Request(url.toString(), {{
            method: requestData.method,
            headers: headers,
            body: requestData.body || undefined,
        }});

        const context = {{
            params: requestData.params || {{}}
        }};

        const moduleKey = '{module_key}';

        const moduleExports = globalThis[moduleKey];

        let handler;
        if (typeof moduleExports === 'function') {{
            handler = moduleExports;
        }} else if (moduleExports && typeof moduleExports === 'object') {{
            handler = moduleExports['{method}'];
        }}

        if (typeof handler !== 'function') {{
            console.error('Module key:', moduleKey);
            console.error('Module exports:', moduleExports);
            console.error('Module exports type:', typeof moduleExports);
            console.error('Available methods:', moduleExports && typeof moduleExports === 'object' ? Object.keys(moduleExports) : 'none');
            throw new Error('Handler {method} is not a function in module ' + moduleKey);
        }}

        const result = await handler(request, context);

        if (result instanceof Response) {{
            const body = await result.text();
            const headers = {{}};
            result.headers.forEach((value, key) => {{
                headers[key] = value;
            }});

            return {{
                status: result.status,
                statusText: result.statusText,
                headers: headers,
                body: body,
            }};
        }} else {{
            console.error('Result is not a Response instance:', result, typeof result);
            return {{
                status: 200,
                headers: {{ 'content-type': 'application/json' }},
                body: JSON.stringify(result),
            }};
        }}
    }} catch (error) {{
        console.error('API route handler error:', error);
        return {{
            status: 500,
            statusText: 'Internal Server Error',
            headers: {{ 'content-type': 'application/json' }},
            body: JSON.stringify({{
                error: 'Internal Server Error',
                message: error.message || String(error),
                stack: error.stack
            }}),
        }};
    }}
}})()
"#,
            handler_code = handler.code,
            request_json = request_json,
            method = method,
            module_key = module_key,
        );

        Ok(script)
    }

    async fn create_response(&self, result: JsonValue) -> Result<Response<Body>, RariError> {
        RequestBridge::from_json(result)
    }
}
