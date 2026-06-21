use crate::runtime::JsExecutionRuntime;
use crate::server::routing::types::RouteSegment;
use axum::body::Body;
use axum::http::{HeaderMap, Request, Response, StatusCode};
use cow_utils::CowUtils;
use dashmap::DashMap;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::error;
use urlencoding::decode;

fn parse_decoded_path_segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| decode(s).unwrap_or_else(|_| s.to_string().into()).into_owned())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRouteEntry {
    pub path: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "componentId", default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    pub segments: Vec<RouteSegment>,
    pub params: Vec<String>,
    #[serde(rename = "isDynamic")]
    pub is_dynamic: bool,
    pub methods: Vec<String>,
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
    last_modified: SystemTime,
}

pub struct ApiRouteHandler {
    runtime: Arc<JsExecutionRuntime>,
    manifest: Arc<ApiRouteManifest>,
    handler_cache: Arc<DashMap<String, CompiledHandler>>,
}

impl ApiRouteHandler {
    pub fn new(runtime: Arc<JsExecutionRuntime>, manifest: ApiRouteManifest) -> Self {
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

    #[cfg(test)]
    pub fn clear_cache(&self) {
        self.handler_cache.clear();
    }

    pub fn invalidate_handler(&self, file_path: &str) {
        self.handler_cache.remove(file_path);
        // Handlers may be stored under component_id instead of file_path
        // (e.g. "app/api/hello/route_6634b3ed" vs "api/hello/route.ts").
        // Use a precise prefix match to avoid over-invalidating unrelated routes.
        let route_prefix = format!(
            "app/{}_",
            file_path
                .trim_end_matches(".tsx")
                .trim_end_matches(".ts")
                .trim_end_matches(".jsx")
                .trim_end_matches(".js")
        );
        self.handler_cache.retain(|key, _| key != file_path && !key.starts_with(&route_prefix));
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

        for route in &self.manifest.api_routes {
            if let Some(params) = self.match_route_pattern(route, &normalized_path) {
                if !route.methods.iter().any(|m| m.eq_ignore_ascii_case(method)) {
                    return Err(RariError::bad_request(format!(
                        "Method {} not allowed for route {}. Supported methods: {}",
                        method,
                        route.path,
                        route.methods.join(", ")
                    ))
                    .with_property("error_type", "method_not_allowed")
                    .with_property("allowed_methods", &route.methods.join(",")));
                }

                return Ok(ApiRouteMatch {
                    route: route.clone(),
                    params,
                    method: method.to_string(),
                });
            }
        }

        Err(RariError::not_found(format!("No API route found for path: {}", path)))
    }

    fn match_route_pattern(
        &self,
        route: &ApiRouteEntry,
        path: &str,
    ) -> Option<FxHashMap<String, String>> {
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
                    params.insert(param_name.to_string(), remaining.join("/"));
                }

                return Some(params);
            }

            if route_seg.starts_with("[...") && route_seg.ends_with(']') {
                let param_name = &route_seg[4..route_seg.len() - 1];

                if path_idx >= path_segments.len() {
                    return None;
                }

                let remaining: Vec<String> = path_segments[path_idx..].to_vec();
                params.insert(param_name.to_string(), remaining.join("/"));

                return Some(params);
            }

            if route_seg.starts_with('[') && route_seg.ends_with(']') {
                if path_idx >= path_segments.len() {
                    return None;
                }

                let param_name = &route_seg[1..route_seg.len() - 1];
                params.insert(param_name.to_string(), path_segments[path_idx].clone());

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
        let cache_key = route.component_id.as_deref().unwrap_or(file_path);

        if let Some(cached) = self.handler_cache.get(cache_key) {
            if is_development {
                let dist_path = Self::resolve_route_dist_path(route)?;
                if let Ok(metadata) = tokio::fs::metadata(&dist_path).await
                    && let Ok(modified) = metadata.modified()
                    && modified <= cached.last_modified
                {
                    return Ok(cached.clone());
                }
            } else {
                return Ok(cached.clone());
            }
        }

        let dist_path = Self::resolve_route_dist_path(route)?;

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

        let module_id = file_path
            .trim_start_matches("api/")
            .trim_end_matches(".ts")
            .trim_end_matches(".tsx")
            .trim_end_matches(".js")
            .trim_end_matches(".jsx")
            .cow_replace('/', "_");

        let compiled = CompiledHandler {
            module_id: module_id.into_owned(),
            code: code.clone(),
            last_modified,
        };

        self.handler_cache.insert(cache_key.to_string(), compiled.clone());

        Ok(compiled)
    }

    fn resolve_route_dist_path(route: &ApiRouteEntry) -> Result<std::path::PathBuf, RariError> {
        if let Some(component_id) = &route.component_id {
            return Ok(Path::new("dist").join("server").join(format!("{component_id}.js")));
        }

        Self::resolve_dist_path(&route.file_path)
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

    pub async fn execute_handler(
        &self,
        route_match: &ApiRouteMatch,
        request: Request<Body>,
        is_development: bool,
    ) -> Result<Response<Body>, RariError> {
        const MAX_API_BODY_SIZE: usize = 10 * 1024 * 1024;

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

        let body_bytes = axum::body::to_bytes(body, MAX_API_BODY_SIZE).await.map_err(|e| {
            error!(
                route_path = %route_match.route.path,
                method = %route_match.method,
                error = %e,
                "Failed to read request body (may exceed size limit)"
            );
            RariError::bad_request(format!("Failed to read request body: {e}"))
        })?;

        let body_string = String::from_utf8_lossy(&body_bytes).to_string();

        let request_obj = self.create_request_object(
            parts.method.as_ref(),
            &parts.uri.to_string(),
            &parts.headers,
            &body_string,
            &route_match.params,
        )?;

        let request_context =
            std::sync::Arc::new(crate::server::middleware::request_context::RequestContext::new(
                route_match.route.path.clone(),
            ));

        self.runtime
            .execute_with_request_context(request_context, async {
                let dist_path = Self::resolve_route_dist_path(&route_match.route)?;
                let canonical_path = dist_path.canonicalize().map_err(|e| {
                    RariError::io(format!(
                        "Failed to canonicalize API route path {}: {e}",
                        dist_path.display()
                    ))
                })?;
                let module_specifier = url::Url::from_file_path(&canonical_path)
                    .map_err(|_| {
                        RariError::configuration(format!(
                            "Failed to create file URL from path: {}",
                            canonical_path.display()
                        ))
                    })?
                    .to_string();

                if let Err(e) = self
                    .runtime
                    .add_module_to_loader_only(&module_specifier, handler.code.clone())
                    .await
                {
                    error!(
                        route_path = %route_match.route.path,
                        method = %route_match.method,
                        module_id = %handler.module_id,
                        error = %e,
                        "Failed to add API route module to loader"
                    );
                    return Err(RariError::js_execution(format!(
                        "Failed to add module to loader: {e}"
                    )));
                }

                let component_id = route_match.route.component_id.clone().unwrap_or_else(|| {
                    dist_path
                        .strip_prefix(Path::new("dist").join("server"))
                        .map(|p| {
                            p.with_extension("")
                                .to_string_lossy()
                                .cow_replace('\\', "/")
                                .into_owned()
                        })
                        .unwrap_or_else(|_| {
                            dist_path
                                .with_extension("")
                                .to_string_lossy()
                                .cow_replace('\\', "/")
                                .into_owned()
                        })
                });

                let module_id = self.runtime.load_es_module(&component_id).await.map_err(|e| {
                    error!(
                        route_path = %route_match.route.path,
                        method = %route_match.method,
                        component_id = %component_id,
                        error = %e,
                        "Failed to load API route as ES module"
                    );
                    RariError::js_execution(format!("Failed to load ES module: {e}"))
                })?;

                if let Err(e) = self.runtime.evaluate_module(module_id).await {
                    error!(
                        route_path = %route_match.route.path,
                        method = %route_match.method,
                        module_id = module_id,
                        error = %e,
                        "Failed to evaluate API route module"
                    );
                    return Err(RariError::js_execution(format!("Failed to evaluate module: {e}")));
                }

                let result = self
                    .execute_handler_from_namespace(
                        &route_match.method,
                        &request_obj,
                        &module_specifier,
                    )
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

                Ok(response)
            })
            .await
    }

    fn create_request_object(
        &self,
        method: &str,
        uri: &str,
        headers: &HeaderMap,
        body: &str,
        params: &FxHashMap<String, String>,
    ) -> Result<JsonValue, RariError> {
        let mut headers_map = FxHashMap::default();
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                headers_map.insert(name.to_string(), value_str.to_string());
            }
        }

        #[allow(clippy::disallowed_methods)]
        let request_obj = json!({
            "method": method,
            "url": uri,
            "headers": headers_map,
            "body": body,
            "params": params,
        });

        Ok(request_obj)
    }

    async fn execute_handler_from_namespace(
        &self,
        method: &str,
        request_obj: &JsonValue,
        module_specifier: &str,
    ) -> Result<JsonValue, RariError> {
        let request_json = serde_json::to_string(request_obj)
            .map_err(|e| RariError::serialization(format!("Failed to serialize request: {e}")))?;

        let script = format!(
            r#"globalThis['~rari'].apiHandler.callHandler({}, "{}", "{}")"#,
            request_json,
            module_specifier.cow_replace('\\', "\\\\").cow_replace('"', "\\\""),
            method.cow_replace('\\', "\\\\").cow_replace('"', "\\\"")
        );

        let trimmed = module_specifier.trim_start_matches("file://");
        let with_underscores = trimmed.cow_replace('/', "_");
        let script_name = with_underscores.cow_replace(':', "_");

        self.runtime
            .execute_script(format!("api_route_call_{}", script_name), script)
            .await
            .map_err(|e| RariError::js_execution(format!("Failed to execute handler: {e}")))
    }

    async fn create_response(&self, result: JsonValue) -> Result<Response<Body>, RariError> {
        if result.is_object() && result.get("status").is_some() {
            let status = result.get("status").and_then(|v| v.as_u64()).unwrap_or(200) as u16;
            let status_code =
                StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body_str = result.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let mut response = Response::builder().status(status_code);

            if let Some(headers_obj) = result.get("headers").and_then(|v| v.as_object()) {
                for (key, value) in headers_obj {
                    if let Some(value_str) = value.as_str() {
                        response = response.header(key, value_str);
                    }
                }
            }

            response
                .body(Body::from(body_str))
                .map_err(|e| RariError::internal(format!("Failed to build response: {e}")))
        } else {
            let body = serde_json::to_string(&result).map_err(|e| {
                RariError::serialization(format!("Failed to serialize response: {e}"))
            })?;

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .map_err(|e| RariError::internal(format!("Failed to build response: {e}")))
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_create_request_object_basic() {
        let runtime = Arc::new(JsExecutionRuntime::default());
        let manifest = ApiRouteManifest { api_routes: vec![] };
        let handler = ApiRouteHandler::new(runtime, manifest);

        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("user-agent", HeaderValue::from_static("test-agent"));

        let params = FxHashMap::default();

        let result =
            handler.create_request_object("GET", "/api/test", &headers, "", &params).unwrap();

        assert_eq!(result["method"], "GET");
        assert_eq!(result["url"], "/api/test");
        assert_eq!(result["headers"]["content-type"], "application/json");
        assert_eq!(result["headers"]["user-agent"], "test-agent");
    }

    #[test]
    fn test_create_request_object_with_params() {
        let runtime = Arc::new(JsExecutionRuntime::default());
        let manifest = ApiRouteManifest { api_routes: vec![] };
        let handler = ApiRouteHandler::new(runtime, manifest);

        let headers = HeaderMap::new();
        let mut params = FxHashMap::default();
        params.insert("id".to_string(), "123".to_string());
        params.insert("name".to_string(), "test".to_string());

        let result =
            handler.create_request_object("GET", "/api/users/123", &headers, "", &params).unwrap();

        assert_eq!(result["params"]["id"], "123");
        assert_eq!(result["params"]["name"], "test");
    }

    #[tokio::test]
    async fn test_create_response_with_status() {
        let runtime = Arc::new(JsExecutionRuntime::default());
        let manifest = ApiRouteManifest { api_routes: vec![] };
        let handler = ApiRouteHandler::new(runtime, manifest);

        let response_json = json!({
            "status": 201,
            "headers": {
                "content-type": "application/json",
                "x-custom": "value"
            },
            "body": r#"{"success":true}"#
        });

        let response = handler.create_response(response_json).await.unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
        assert_eq!(response.headers().get("x-custom").unwrap(), "value");
    }

    #[tokio::test]
    async fn test_create_response_plain_json() {
        let runtime = Arc::new(JsExecutionRuntime::default());
        let manifest = ApiRouteManifest { api_routes: vec![] };
        let handler = ApiRouteHandler::new(runtime, manifest);

        let plain_json = json!({
            "message": "Hello",
            "count": 42
        });

        let response = handler.create_response(plain_json).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    }
}
