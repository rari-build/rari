use std::{
    env,
    error::Error,
    fs as std_fs, mem,
    path::PathBuf,
    sync::{Arc, OnceLock},
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::Response,
};
use futures_util::future::BoxFuture;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower::{Layer, Service};

use crate::{
    runtime::JsExecutionRuntime,
    server::core::{types::ServerState, utils::component::get_dist_path_for_component},
    utils::path::path_to_file_url,
};

async fn clone_renderer_runtime(state: &ServerState) -> Arc<JsExecutionRuntime> {
    let renderer = state.renderer.lock().await;
    Arc::clone(&renderer.runtime)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum JsonHeaderValue {
    Single(String),
    Multiple(Vec<String>),
}

impl JsonHeaderValue {
    fn as_strs(&self) -> Vec<&str> {
        match self {
            Self::Single(value) => vec![value.as_str()],
            Self::Multiple(values) => values.iter().map(String::as_str).collect(),
        }
    }
}

fn append_header_map(headers: &mut HeaderMap, map: FxHashMap<String, JsonHeaderValue>) {
    for (key, value) in map {
        let Ok(header_name) = key.parse::<HeaderName>() else {
            continue;
        };
        for value_str in value.as_strs() {
            let Ok(header_value) = value_str.parse::<HeaderValue>() else {
                continue;
            };
            headers.append(header_name.clone(), header_value);
        }
    }
}

fn apply_response_headers(headers: &mut HeaderMap, map: FxHashMap<String, JsonHeaderValue>) {
    let mut entries: Vec<(String, JsonHeaderValue)> = map.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut grouped: FxHashMap<HeaderName, Vec<String>> = FxHashMap::default();
    for (key, value) in entries {
        let Ok(header_name) = key.parse::<HeaderName>() else {
            continue;
        };
        let values: Vec<String> = value.as_strs().into_iter().map(str::to_owned).collect();
        if header_name == header::SET_COOKIE {
            grouped.entry(header_name).or_default().extend(values);
        } else {
            grouped.insert(header_name, values);
        }
    }

    for (header_name, values) in grouped {
        if header_name != header::SET_COOKIE {
            headers.remove(&header_name);
        }
        for value_str in values {
            let Ok(header_value) = HeaderValue::from_str(&value_str) else {
                continue;
            };
            headers.append(header_name.clone(), header_value);
        }
    }
}

fn apply_request_headers(headers: &mut HeaderMap, map: FxHashMap<String, JsonHeaderValue>) {
    for (key, value) in map {
        let Ok(header_name) = key.parse::<HeaderName>() else {
            continue;
        };
        headers.remove(&header_name);
        for value_str in value.as_strs() {
            let Ok(header_value) = value_str.parse::<HeaderValue>() else {
                continue;
            };
            headers.append(header_name.clone(), header_value);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ProxyResult {
    #[serde(rename = "continue")]
    continue_: bool,
    response: Option<ProxyResponse>,
    #[serde(rename = "requestHeaders")]
    request_headers: Option<FxHashMap<String, JsonHeaderValue>>,
    #[serde(rename = "responseHeaders")]
    response_headers: Option<FxHashMap<String, JsonHeaderValue>>,
    rewrite: Option<String>,
    redirect: Option<RedirectInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProxyResponse {
    status: u16,
    headers: FxHashMap<String, JsonHeaderValue>,
    body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RedirectInfo {
    destination: String,
    permanent: bool,
}

static PROXY_DIST_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

fn resolve_proxy_dist_path() -> Option<PathBuf> {
    PROXY_DIST_PATH
        .get_or_init(|| {
            let hashed = get_dist_path_for_component("src/proxy.ts").ok()?;
            std_fs::metadata(&hashed).ok()?;
            Some(hashed)
        })
        .clone()
}

fn is_proxy_enabled() -> bool {
    resolve_proxy_dist_path().is_some()
}

async fn execute_proxy(
    state: &ServerState,
    method: String,
    uri: String,
    headers: FxHashMap<String, String>,
) -> Result<ProxyResult, RariError> {
    let scheme = headers.get("x-forwarded-proto").cloned().unwrap_or_else(|| "http".to_string());
    let host = headers.get("host").cloned().unwrap_or_else(|| "localhost".to_string());
    let url = format!("{scheme}://{host}{uri}");

    let request_data = serde_json::json!({
        "url": url,
        "method": method,
        "headers": headers,
    });

    let runtime = clone_renderer_runtime(state).await;

    let result_json = runtime.execute_function("~rariExecuteProxy", vec![request_data]).await?;

    let proxy_result: ProxyResult = serde_json::from_value(result_json)
        .map_err(|e| RariError::deserialization(format!("Invalid proxy result: {e}")))?;

    Ok(proxy_result)
}

#[derive(Clone)]
pub struct ProxyLayer {
    state: ServerState,
}

impl ProxyLayer {
    pub fn new(state: ServerState) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for ProxyLayer {
    type Service = ProxyMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ProxyMiddleware { inner, state: self.state.clone() }
    }
}

#[derive(Clone)]
pub struct ProxyMiddleware<S> {
    inner: S,
    state: ServerState,
}

impl<S> Service<Request> for ProxyMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static + Clone,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn Error + Send + Sync>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        let state = self.state.clone();
        let inner = self.inner.clone();
        let mut inner = mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            if !is_proxy_enabled() {
                return inner.call(request).await;
            }

            let path = request.uri().path();
            if path.starts_with("/_rari/") || path.starts_with("/vite-server/") {
                return inner.call(request).await;
            }

            let method = request.method().to_string();
            let uri = request.uri().to_string();
            let headers: FxHashMap<String, String> = request
                .headers()
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                .collect();

            match execute_proxy(&state, method, uri, headers).await {
                Ok(result) => {
                    if let Some(redirect) = result.redirect {
                        let status = if redirect.permanent {
                            StatusCode::MOVED_PERMANENTLY
                        } else {
                            StatusCode::TEMPORARY_REDIRECT
                        };

                        return match Response::builder()
                            .status(status)
                            .header("Location", redirect.destination)
                            .body(Body::empty())
                        {
                            Ok(response) => Ok(response),
                            Err(_) => inner.call(request).await,
                        };
                    }

                    if let Some(rewrite_path) = result.rewrite {
                        match rewrite_path.parse() {
                            Ok(uri) => {
                                *request.uri_mut() = uri;
                            }
                            Err(e) => {
                                tracing::error!("Failed to parse rewrite path: {}", e);
                                return inner.call(request).await;
                            }
                        }
                    }

                    if let Some(headers) = result.request_headers {
                        apply_request_headers(request.headers_mut(), headers);
                    }

                    if let Some(proxy_response) = result.response {
                        let Ok(mut response) = Response::builder()
                            .status(proxy_response.status)
                            .body(Body::from(proxy_response.body.unwrap_or_default()))
                        else {
                            return inner.call(request).await;
                        };

                        append_header_map(response.headers_mut(), proxy_response.headers);
                        return Ok(response);
                    }

                    if result.continue_ {
                        let mut response = inner.call(request).await?;

                        if let Some(headers) = result.response_headers {
                            apply_response_headers(response.headers_mut(), headers);
                        }

                        return Ok(response);
                    }

                    inner.call(request).await
                }
                Err(e) => {
                    tracing::error!("Proxy execution failed: {}", e);
                    inner.call(request).await
                }
            }
        })
    }
}

async fn resolve_rari_package_dir() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    let mut search_dir = cwd.as_path();

    loop {
        let candidate = search_dir.join("node_modules").join("rari");
        if fs::try_exists(&candidate).await.unwrap_or(false) {
            return Some(candidate);
        }
        search_dir = search_dir.parent()?;
    }
}

#[expect(clippy::missing_errors_doc)]
pub async fn initialize_proxy(state: &ServerState) -> Result<(), RariError> {
    if !is_proxy_enabled() {
        return Ok(());
    }

    let Some(rari_pkg_dir) = resolve_rari_package_dir().await else {
        tracing::debug!("Proxy: rari package directory not found in node_modules");
        return Ok(());
    };

    let executor_path = rari_pkg_dir.join("dist/proxy/runtime-executor.mjs");

    if !fs::try_exists(&executor_path).await.unwrap_or(false) {
        tracing::debug!(
            "Proxy: executor not found at {}, skipping proxy setup",
            executor_path.display()
        );
        return Ok(());
    }

    let executor_absolute = fs::canonicalize(&executor_path).await.unwrap_or(executor_path);
    let executor_specifier = path_to_file_url(&executor_absolute);

    let rari_request_path = rari_pkg_dir.join("dist/proxy/RariRequest.mjs");
    let rari_request_absolute =
        fs::canonicalize(&rari_request_path).await.unwrap_or(rari_request_path);
    let rari_request_specifier = path_to_file_url(&rari_request_absolute);

    let Some(proxy_file_path) = resolve_proxy_dist_path() else {
        return Ok(());
    };
    let proxy_absolute = match fs::canonicalize(&proxy_file_path).await {
        Ok(canonical) => canonical,
        Err(_) => env::current_dir()
            .map_err(|e| RariError::io(format!("Failed to get current directory: {e}")))?
            .join(&proxy_file_path),
    };
    let proxy_specifier = path_to_file_url(&proxy_absolute);

    let runtime = clone_renderer_runtime(state).await;

    let init_script = format!(
        r#"(async function() {{
            try {{
                const {{ initializeProxyExecutor }} = await import("{executor_specifier}");
                const success = await initializeProxyExecutor("{proxy_specifier}", "{rari_request_specifier}");
                if (!success) {{
                    throw new Error("initializeProxyExecutor returned false");
                }}
                return {{ success: true }};
            }} catch (error) {{
                console.error("[rari] Proxy: Failed to initialize:", error);
                throw error;
            }}
        }})()"#
    );

    runtime.broadcast_script("initialize_proxy_executor", &init_script).await.map_err(|e| {
        tracing::error!("Failed to register proxy function: {}", e);
        e
    })
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn apply_request_headers_replaces_existing_authorization() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer old"),
        );
        headers.insert(HeaderName::from_static("x-keep"), HeaderValue::from_static("1"));

        let mut map = FxHashMap::default();
        map.insert(
            "authorization".to_string(),
            JsonHeaderValue::Single("Bearer proxy".to_string()),
        );

        apply_request_headers(&mut headers, map);

        let auth: Vec<_> =
            headers.get_all("authorization").iter().map(|v| v.to_str().unwrap()).collect();
        assert_eq!(auth, vec!["Bearer proxy"]);
        assert_eq!(headers.get("x-keep").and_then(|v| v.to_str().ok()), Some("1"));
    }

    #[test]
    fn apply_response_headers_replaces_content_type_but_appends_set_cookie() {
        let mut headers = HeaderMap::new();
        headers
            .insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/html"));
        headers.append(header::SET_COOKIE, HeaderValue::from_static("a=1"));

        let mut map = FxHashMap::default();
        map.insert(
            "content-type".to_string(),
            JsonHeaderValue::Single("application/json".to_string()),
        );
        map.insert(
            "set-cookie".to_string(),
            JsonHeaderValue::Multiple(vec!["b=2".to_string(), "c=3".to_string()]),
        );

        apply_response_headers(&mut headers, map);

        let content_types: Vec<_> =
            headers.get_all("content-type").iter().map(|v| v.to_str().unwrap()).collect();
        assert_eq!(content_types, vec!["application/json"]);

        let set_cookies: Vec<_> =
            headers.get_all(header::SET_COOKIE).iter().map(|v| v.to_str().unwrap()).collect();
        assert_eq!(set_cookies, vec!["a=1", "b=2", "c=3"]);
    }

    #[test]
    fn apply_response_headers_case_variants_last_win_deterministically() {
        let mut headers = HeaderMap::new();
        headers
            .insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/html"));

        let mut map = FxHashMap::default();
        map.insert("Content-Type".to_string(), JsonHeaderValue::Single("text/plain".to_string()));
        map.insert(
            "content-type".to_string(),
            JsonHeaderValue::Single("application/json".to_string()),
        );

        apply_response_headers(&mut headers, map);

        let content_types: Vec<_> =
            headers.get_all("content-type").iter().map(|v| v.to_str().unwrap()).collect();
        assert_eq!(content_types, vec!["application/json"]);
    }

    #[test]
    fn apply_response_headers_set_cookie_case_variants_concatenate_in_key_order() {
        let mut headers = HeaderMap::new();
        let mut map = FxHashMap::default();
        map.insert("Set-Cookie".to_string(), JsonHeaderValue::Single("a=1".to_string()));
        map.insert("set-cookie".to_string(), JsonHeaderValue::Single("b=2".to_string()));

        apply_response_headers(&mut headers, map);

        let set_cookies: Vec<_> =
            headers.get_all(header::SET_COOKIE).iter().map(|v| v.to_str().unwrap()).collect();
        assert_eq!(set_cookies, vec!["a=1", "b=2"]);
    }
}
