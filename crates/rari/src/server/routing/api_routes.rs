use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use axum::{
    body,
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode},
};
use cow_utils::CowUtils;
use dashmap::DashMap;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::fs;

use crate::{
    rendering::layout::{component_dist_path, create_component_id},
    runtime::JsExecutionRuntime,
    server::{
        core::utils::http::extract_headers, middleware::request_context::RequestContext,
        routing::types::RouteSegment,
    },
};

fn parse_decoded_path_segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| urlencoding::decode(s).unwrap_or_else(|_| s.to_string().into()).into_owned())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct ApiRouteManifest {
    #[serde(rename = "apiRoutes", default)]
    pub api_routes: Vec<ApiRouteEntry>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ApiRouteMatch {
    pub route: ApiRouteEntry,
    pub params: FxHashMap<String, String>,
    pub method: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledHandler {
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

    #[expect(clippy::missing_errors_doc)]
    pub async fn from_file(
        runtime: Arc<JsExecutionRuntime>,
        manifest_path: &str,
    ) -> Result<Self, RariError> {
        let content = fs::read_to_string(manifest_path)
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
        self.handler_cache.remove(&create_component_id(file_path));
    }

    pub fn get_supported_methods(&self, path: &str) -> Option<Vec<String>> {
        let normalized_path = Self::normalize_path(path);

        for route in &self.manifest.api_routes {
            if Self::match_route_pattern(route, &normalized_path).is_some() {
                return Some(route.methods.clone());
            }
        }

        None
    }

    #[expect(clippy::missing_errors_doc)]
    pub fn match_route(&self, path: &str, method: &str) -> Result<ApiRouteMatch, RariError> {
        let normalized_path = Self::normalize_path(path);

        for route in &self.manifest.api_routes {
            if let Some(params) = Self::match_route_pattern(route, &normalized_path) {
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

        Err(RariError::not_found(format!("No API route found for path: {path}")))
    }

    fn match_route_pattern(route: &ApiRouteEntry, path: &str) -> Option<FxHashMap<String, String>> {
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
            format!("/{path}")
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
        let cache_key = create_component_id(file_path);

        if let Some(cached) = self.handler_cache.get(&cache_key) {
            if is_development {
                let dist_path = Self::resolve_route_dist_path(route);
                if let Ok(metadata) = fs::metadata(&dist_path).await
                    && let Ok(modified) = metadata.modified()
                    && modified <= cached.last_modified
                {
                    return Ok(cached.clone());
                }
            } else {
                return Ok(cached.clone());
            }
        }

        let dist_path = Self::resolve_route_dist_path(route);

        if !fs::try_exists(&dist_path).await.unwrap_or(false) {
            tracing::error!(
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

        let code = fs::read_to_string(&dist_path).await.map_err(|e| {
            tracing::error!(
                file_path = %file_path,
                dist_path = %dist_path.display(),
                error = %e,
                "Failed to read handler file"
            );
            RariError::io(format!("Failed to read handler file: {e}"))
        })?;

        let last_modified = fs::metadata(&dist_path)
            .await
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());

        let compiled = CompiledHandler { code, last_modified };

        self.handler_cache.insert(cache_key, compiled.clone());

        Ok(compiled)
    }

    fn resolve_route_dist_path(route: &ApiRouteEntry) -> PathBuf {
        component_dist_path(Path::new("dist/server"), &route.file_path)
    }

    #[expect(clippy::missing_errors_doc, clippy::too_many_lines)]
    pub async fn execute_handler(
        &self,
        route_match: &ApiRouteMatch,
        request: Request<Body>,
        is_development: bool,
    ) -> Result<Response<Body>, RariError> {
        const MAX_API_BODY_SIZE: usize = 10 * 1024 * 1024;

        let handler = self.load_handler(&route_match.route, is_development).await.map_err(|e| {
            tracing::error!(
                route_path = %route_match.route.path,
                method = %route_match.method,
                error = %e,
                "Failed to load handler"
            );
            e
        })?;

        let (parts, body) = request.into_parts();

        let body_bytes = body::to_bytes(body, MAX_API_BODY_SIZE).await.map_err(|e| {
            tracing::error!(
                route_path = %route_match.route.path,
                method = %route_match.method,
                error = %e,
                "Failed to read request body (may exceed size limit)"
            );
            RariError::bad_request(format!("Failed to read request body: {e}"))
        })?;

        let body_string = String::from_utf8_lossy(&body_bytes).to_string();

        let request_obj = Self::create_request_object(
            parts.method.as_ref(),
            &parts.uri.to_string(),
            &parts.headers,
            &body_string,
            &route_match.params,
        )?;

        let request_context = Arc::new(
            RequestContext::new(route_match.route.path.clone())
                .with_http_headers(extract_headers(&parts.headers)),
        );

        let dist_path = Self::resolve_route_dist_path(&route_match.route);
        let canonical_path = fs::canonicalize(&dist_path).await.map_err(|e| {
            RariError::io(format!(
                "Failed to canonicalize API route path {}: {e}",
                dist_path.display()
            ))
        })?;
        let module_specifier = url::Url::from_file_path(&canonical_path)
            .map_err(|()| {
                RariError::configuration(format!(
                    "Failed to create file URL from path: {}",
                    canonical_path.display()
                ))
            })?
            .to_string();

        let component_id = create_component_id(&route_match.route.file_path);

        if let Err(e) =
            self.runtime.add_module_to_loader(&module_specifier, handler.code.clone()).await
        {
            tracing::error!(
                route_path = %route_match.route.path,
                method = %route_match.method,
                component_id = %component_id,
                error = %e,
                "Failed to add API route module to loader"
            );
            return Err(RariError::js_execution(format!("Failed to add module to loader: {e}")));
        }

        // Ensure the module is evaluated on every isolate before the sticky request path runs.
        if let Err(e) = self.runtime.load_and_evaluate_module(&component_id).await {
            tracing::error!(
                route_path = %route_match.route.path,
                method = %route_match.method,
                component_id = %component_id,
                error = %e,
                "Failed to load/evaluate API route module"
            );
            return Err(RariError::js_execution(format!("Failed to load/evaluate ES module: {e}")));
        }

        let method = route_match.method.clone();
        let route_path = route_match.route.path.clone();
        let runtime = Arc::clone(&self.runtime);
        let request_json = serde_json::to_string(&request_obj)
            .map_err(|e| RariError::serialization(format!("Failed to serialize request: {e}")))?;
        let script = format!(
            r#"globalThis['~rari'].apiHandler.callHandler({}, "{}", "{}")"#,
            request_json,
            module_specifier.cow_replace('\\', "\\\\").cow_replace('"', "\\\""),
            method.cow_replace('\\', "\\\\").cow_replace('"', "\\\"")
        );
        let trimmed = module_specifier.trim_start_matches("file://");
        let with_underscores = trimmed.cow_replace('/', "_");
        let script_name = format!("api_route_call_{}", with_underscores.cow_replace(':', "_"));

        let result = runtime
            .with_request_context(request_context, move |rt| async move {
                rt.execute_script(script_name, script)
                    .await
                    .map_err(|e| RariError::js_execution(format!("Failed to execute handler: {e}")))
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    route_path = %route_path,
                    method = %method,
                    component_id = %component_id,
                    error = %e,
                    "Handler execution failed"
                );
                RariError::js_execution(format!("Handler execution failed: {e}"))
            })?;

        Self::create_response(&result).map_err(|e| {
            tracing::error!(
                route_path = %route_path,
                method = %method,
                error = %e,
                "Failed to create response from handler result"
            );
            e
        })
    }
    #[expect(clippy::unnecessary_wraps, reason = "Result return type maintains API consistency")]
    fn create_request_object(
        method: &str,
        uri: &str,
        headers: &HeaderMap,
        body: &str,
        params: &FxHashMap<String, String>,
    ) -> Result<Value, RariError> {
        let mut headers_map = FxHashMap::default();
        for (name, value) in headers {
            if let Ok(value_str) = value.to_str() {
                headers_map.insert(name.to_string(), value_str.to_string());
            }
        }

        let request_obj = json!({
            "method": method,
            "url": uri,
            "headers": headers_map,
            "body": body,
            "params": params,
        });

        Ok(request_obj)
    }

    fn create_response(result: &Value) -> Result<Response<Body>, RariError> {
        let is_http_envelope = if let Some(status_val) = result.get("status") {
            if let Some(status_num) = status_val.as_u64() {
                (100..=599).contains(&status_num)
            } else {
                false
            }
        } else {
            false
        };

        if is_http_envelope {
            let status = result
                .get("status")
                .and_then(serde_json::Value::as_u64)
                .and_then(|n| u16::try_from(n).ok())
                .unwrap_or(500);

            let status_code =
                StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body_str = result.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let mut response =
                Response::builder()
                    .status(status_code)
                    .body(Body::from(body_str))
                    .map_err(|e| RariError::internal(format!("Failed to build response: {e}")))?;

            if let Some(headers_obj) = result.get("headers").and_then(|v| v.as_object()) {
                append_json_headers(response.headers_mut(), headers_obj);
            }

            Ok(response)
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

fn append_json_headers(headers: &mut HeaderMap, headers_obj: &serde_json::Map<String, Value>) {
    for (key, value) in headers_obj {
        let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) else {
            continue;
        };
        let values: Vec<&str> = match value {
            Value::String(s) => vec![s.as_str()],
            Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
            _ => continue,
        };
        for value_str in values {
            let Ok(header_value) = HeaderValue::from_str(value_str) else {
                continue;
            };
            headers.append(header_name.clone(), header_value);
        }
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn test_create_request_object_basic() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("user-agent", HeaderValue::from_static("test-agent"));

        let params = FxHashMap::default();

        let result =
            ApiRouteHandler::create_request_object("GET", "/api/test", &headers, "", &params)
                .unwrap();

        assert_eq!(result["method"], "GET");
        assert_eq!(result["url"], "/api/test");
        assert_eq!(result["headers"]["content-type"], "application/json");
        assert_eq!(result["headers"]["user-agent"], "test-agent");
    }

    #[test]
    fn test_create_request_object_with_params() {
        let headers = HeaderMap::new();
        let mut params = FxHashMap::default();
        params.insert("id".to_string(), "123".to_string());
        params.insert("name".to_string(), "test".to_string());

        let result =
            ApiRouteHandler::create_request_object("GET", "/api/users/123", &headers, "", &params)
                .unwrap();

        assert_eq!(result["params"]["id"], "123");
        assert_eq!(result["params"]["name"], "test");
    }

    #[tokio::test]
    async fn test_create_response_with_status() {
        let response_json = json!({
            "status": 201,
            "headers": {
                "content-type": "application/json",
                "x-custom": "value"
            },
            "body": r#"{"success":true}"#
        });

        let response = ApiRouteHandler::create_response(&response_json).unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
        assert_eq!(response.headers().get("x-custom").unwrap(), "value");
    }

    #[tokio::test]
    async fn test_create_response_preserves_multiple_set_cookie_headers() {
        let response_json = json!({
            "status": 200,
            "headers": {
                "content-type": "application/json",
                "set-cookie": ["foo=bar; Path=/", "hello=world; Path=/"]
            },
            "body": r#"{"ok":true}"#
        });

        let response = ApiRouteHandler::create_response(&response_json).unwrap();
        let set_cookies: Vec<_> =
            response.headers().get_all("set-cookie").iter().map(|v| v.to_str().unwrap()).collect();

        assert_eq!(set_cookies.len(), 2);
        assert!(set_cookies.iter().any(|v| v.starts_with("foo=bar")));
        assert!(set_cookies.iter().any(|v| v.starts_with("hello=world")));
    }

    #[tokio::test]
    async fn test_create_response_plain_json() {
        let plain_json = json!({
            "message": "Hello",
            "count": 42
        });

        let response = ApiRouteHandler::create_response(&plain_json).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    }
}
