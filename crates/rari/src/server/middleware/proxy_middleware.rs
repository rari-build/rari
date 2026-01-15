use axum::{body::Body, extract::Request, http::StatusCode, response::Response};
use futures_util::future::BoxFuture;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::error;

use crate::server::types::ServerState;

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
    *PROXY_ENABLED.get_or_init(|| std::fs::metadata("dist/server/proxy.js").is_ok())
}

async fn execute_proxy(
    state: &ServerState,
    method: String,
    uri: String,
    headers: FxHashMap<String, String>,
) -> Result<ProxyResult, Box<dyn std::error::Error + Send + Sync>> {
    let renderer = state.renderer.lock().await;
    let runtime = &renderer.runtime;

    let scheme = headers.get("x-forwarded-proto").cloned().unwrap_or_else(|| "http".to_string());

    let host = headers.get("host").cloned().unwrap_or_else(|| "localhost".to_string());

    let url = format!("{}://{}{}", scheme, host, uri);

    #[allow(clippy::disallowed_methods)]
    let request_data = serde_json::json!({
        "url": url,
        "method": method,
        "headers": headers,
    });

    let result_json = runtime.execute_function("__rariExecuteProxy", vec![request_data]).await?;

    let proxy_result: ProxyResult = serde_json::from_value(result_json)?;

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
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
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
        let mut inner = std::mem::replace(&mut self.inner, inner);

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
                                error!("Failed to parse rewrite path: {}", e);
                                return inner.call(request).await;
                            }
                        }
                    }

                    if let Some(headers) = result.request_headers {
                        for (key, value) in headers {
                            if let Ok(header_name) = key.parse::<axum::http::HeaderName>()
                                && let Ok(header_value) = value.parse::<axum::http::HeaderValue>()
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
                                if let Ok(header_name) = key.parse::<axum::http::HeaderName>()
                                    && let Ok(header_value) =
                                        value.parse::<axum::http::HeaderValue>()
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
                    error!("Proxy execution failed: {}", e);
                    inner.call(request).await
                }
            }
        })
    }
}

pub async fn initialize_proxy(state: &ServerState) -> Result<(), Box<dyn std::error::Error>> {
    if !is_proxy_enabled() {
        return Ok(());
    }

    let renderer = state.renderer.lock().await;
    let runtime = &renderer.runtime;

    let executor_path = std::path::Path::new("node_modules/rari/dist/proxy/runtime-executor.mjs");

    if !executor_path.exists() {
        return Ok(());
    }

    let executor_code = tokio::fs::read_to_string(executor_path).await?;

    let executor_specifier = format!(
        "file://{}",
        executor_path.canonicalize().unwrap_or_else(|_| executor_path.to_path_buf()).display()
    );

    runtime.add_module_to_loader_only(&executor_specifier, executor_code).await?;

    let proxy_file_path = std::path::Path::new("dist/server/proxy.js");
    let proxy_specifier = format!(
        "file://{}",
        proxy_file_path.canonicalize().unwrap_or_else(|_| proxy_file_path.to_path_buf()).display()
    );

    let init_script = format!(
        r#"(async function() {{
            try {{
                const {{ initializeProxyExecutor }} = await import("{}");
                const success = await initializeProxyExecutor("{}");
                return {{ success }};
            }} catch (error) {{
                console.error("[rari:proxy] Failed to initialize:", error);
                return {{ success: false, error: error.message }};
            }}
        }})()"#,
        executor_specifier, proxy_specifier
    );

    match runtime.execute_script("initialize_proxy_executor".to_string(), init_script).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to register proxy function: {}", e);
            Err(e.into())
        }
    }
}
