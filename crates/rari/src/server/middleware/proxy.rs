use std::{
    env,
    error::Error,
    fs as std_fs, mem,
    path::{Path, PathBuf},
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderName, HeaderValue, StatusCode},
    response::Response,
};
use futures_util::future::BoxFuture;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower::{Layer, Service};

use crate::{
    runtime::JsExecutionRuntime, server::core::types::ServerState, utils::path::path_to_file_url,
};

async fn clone_renderer_runtime(state: &ServerState) -> Arc<JsExecutionRuntime> {
    let renderer = state.renderer.lock().await;
    Arc::clone(&renderer.runtime)
}

#[derive(Debug, Serialize, Deserialize)]
struct ProxyResult {
    #[serde(rename = "continue")]
    continue_: bool,
    response: Option<ProxyResponse>,
    #[serde(rename = "requestHeaders")]
    request_headers: Option<FxHashMap<String, String>>,
    #[serde(rename = "responseHeaders")]
    response_headers: Option<FxHashMap<String, String>>,
    rewrite: Option<String>,
    redirect: Option<RedirectInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProxyResponse {
    status: u16,
    headers: FxHashMap<String, String>,
    body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RedirectInfo {
    destination: String,
    permanent: bool,
}

use std::sync::OnceLock;

static PROXY_ENABLED: OnceLock<bool> = OnceLock::new();

fn is_proxy_enabled() -> bool {
    *PROXY_ENABLED.get_or_init(|| std_fs::metadata("dist/server/proxy.js").is_ok())
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

    let result_json = runtime
        .execute_function("~rariExecuteProxy", vec![request_data])
        .await
        .map_err(|e| RariError::js_execution(format!("Proxy execution failed: {e}")))?;

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
                        for (key, value) in headers {
                            if let Ok(header_name) = key.parse::<HeaderName>()
                                && let Ok(header_value) = value.parse::<HeaderValue>()
                            {
                                request.headers_mut().insert(header_name, header_value);
                            }
                        }
                    }

                    if let Some(proxy_response) = result.response {
                        let mut response_builder =
                            Response::builder().status(proxy_response.status);

                        for (key, value) in proxy_response.headers {
                            response_builder = response_builder.header(key, value);
                        }

                        let body = proxy_response.body.unwrap_or_default();
                        return match response_builder.body(Body::from(body)) {
                            Ok(response) => Ok(response),
                            Err(_) => inner.call(request).await,
                        };
                    }

                    if result.continue_ {
                        let mut response = inner.call(request).await?;

                        if let Some(headers) = result.response_headers {
                            for (key, value) in headers {
                                if let Ok(header_name) = key.parse::<HeaderName>()
                                    && let Ok(header_value) = value.parse::<HeaderValue>()
                                {
                                    response.headers_mut().insert(header_name, header_value);
                                }
                            }
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

    let proxy_file_path = Path::new("dist/server/proxy.js");
    let proxy_absolute = match fs::canonicalize(proxy_file_path).await {
        Ok(canonical) => canonical,
        Err(_) => env::current_dir()
            .map_err(|e| RariError::io(format!("Failed to get current directory: {e}")))?
            .join(proxy_file_path),
    };
    let proxy_specifier = path_to_file_url(&proxy_absolute);

    let runtime = clone_renderer_runtime(state).await;

    let init_script = format!(
        r#"(async function() {{
            try {{
                const {{ initializeProxyExecutor }} = await import("{executor_specifier}");
                const success = await initializeProxyExecutor("{proxy_specifier}", "{rari_request_specifier}");
                return {{ success }};
            }} catch (error) {{
                console.error("[rari] Proxy: Failed to initialize:", error);
                return {{ success: false, error: error.message }};
            }}
        }})()"#
    );

    match runtime.execute_script("initialize_proxy_executor".to_string(), init_script).await {
        Ok(result) => {
            if let Some(success) = result.get("success").and_then(serde_json::Value::as_bool) {
                if success {
                    Ok(())
                } else {
                    let error_msg = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error during proxy initialization");
                    tracing::error!("Proxy initialization failed: {error_msg}");
                    Err(RariError::js_runtime(format!("Proxy initialization failed: {error_msg}")))
                }
            } else {
                tracing::error!("Proxy initialization returned invalid result format");
                Err(RariError::js_runtime("Proxy initialization returned invalid result format"))
            }
        }
        Err(e) => {
            tracing::error!("Failed to register proxy function: {}", e);
            Err(e)
        }
    }
}
