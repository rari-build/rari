use std::{
    borrow::Cow,
    env,
    error::Error,
    fs as std_fs,
    io::ErrorKind,
    mem,
    path::{Path, PathBuf},
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
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower::{Layer, Service};

use crate::{
    runtime::JsExecutionRuntime,
    server::{config::Config, core::types::ServerState},
    utils::path::path_to_file_url,
};

async fn clone_renderer_runtime(state: &ServerState) -> Arc<JsExecutionRuntime> {
    let renderer = state.renderer.lock().await;
    Arc::clone(&renderer.runtime)
}

const PROXY_MANIFEST_PATH: &str = "dist/server/proxy.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProxyManifestFile {
    enabled: bool,
    #[serde(default)]
    rules: Vec<ProxyRule>,
    #[serde(rename = "requiresRuntime", default)]
    requires_runtime: bool,
    #[serde(rename = "bundlePath")]
    bundle_path: Option<String>,
    #[serde(default)]
    matcher: Option<ProxyMatcherField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ProxyMatcherField {
    Pattern(String),
    Patterns(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProxyRule {
    source: String,
    #[serde(rename = "type")]
    rule_type: String,
    destination: Option<String>,
    permanent: Option<bool>,
    headers: Option<FxHashMap<String, String>>,
}

#[derive(Debug)]
enum AppliedProxyRule {
    Redirect { destination: String, permanent: bool },
    Rewrite(String),
    Headers(FxHashMap<String, String>),
    Block,
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

fn apply_string_headers(headers: &mut HeaderMap, map: FxHashMap<String, String>) {
    let json_map =
        map.into_iter().map(|(key, value)| (key, JsonHeaderValue::Single(value))).collect();
    apply_response_headers(headers, json_map);
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

static PROXY_MANIFEST: OnceLock<Result<Option<ProxyManifestFile>, RariError>> = OnceLock::new();

fn load_proxy_manifest() -> Result<Option<&'static ProxyManifestFile>, RariError> {
    match PROXY_MANIFEST.get_or_init(|| {
        let path = Path::new(PROXY_MANIFEST_PATH);
        let content = match std_fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(RariError::configuration(format!(
                    "Failed to read {PROXY_MANIFEST_PATH}: {error}"
                )));
            }
        };

        match serde_json::from_str::<ProxyManifestFile>(&content) {
            Ok(manifest) if manifest.enabled => Ok(Some(manifest)),
            Ok(_) => Ok(None),
            Err(error) => Err(RariError::configuration(format!(
                "Failed to parse {PROXY_MANIFEST_PATH}: {error}"
            ))),
        }
    }) {
        Ok(Some(manifest)) => Ok(Some(manifest)),
        Ok(None) => Ok(None),
        Err(error) => Err(error.clone()),
    }
}

fn requires_proxy_runtime() -> Result<bool, RariError> {
    Ok(load_proxy_manifest()?.is_some_and(|manifest| manifest.requires_runtime))
}

fn resolve_proxy_dist_path() -> Result<Option<PathBuf>, RariError> {
    let Some(manifest) = load_proxy_manifest()? else {
        return Ok(None);
    };
    if !manifest.requires_runtime {
        return Ok(None);
    }
    let Some(bundle_path) = manifest.bundle_path.as_deref() else {
        return Ok(None);
    };
    let path = PathBuf::from("dist").join(bundle_path);
    if std_fs::metadata(&path).is_err() {
        return Ok(None);
    }
    Ok(Some(path))
}

fn normalize_proxy_path(path: &str) -> Cow<'_, str> {
    let needs_collapse = path.contains("//");
    let needs_trim = path.len() > 1 && path.ends_with('/');

    if !needs_collapse && !needs_trim {
        return Cow::Borrowed(path);
    }

    let mut normalized = if needs_collapse {
        let mut collapsed = String::with_capacity(path.len());
        let mut prev_slash = false;
        for ch in path.chars() {
            if ch == '/' {
                if !prev_slash {
                    collapsed.push('/');
                }
                prev_slash = true;
            } else {
                collapsed.push(ch);
                prev_slash = false;
            }
        }
        collapsed
    } else {
        path.to_owned()
    };

    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    Cow::Owned(normalized)
}

fn path_matches_source(pathname: &str, source: &str) -> bool {
    normalize_proxy_path(pathname) == normalize_proxy_path(source)
}

fn path_matches_pattern(pathname: &str, pattern: &str) -> bool {
    let normalized_path = normalize_proxy_path(pathname);
    let normalized_pattern = normalize_proxy_path(pattern);

    if !normalized_pattern.contains('*') && !normalized_pattern.contains(':') {
        return normalized_path == normalized_pattern;
    }

    let chars: Vec<char> = normalized_pattern.chars().collect();
    let mut rebuilt = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            if end > start {
                let suffix = chars.get(end).copied();
                let preceded_by_slash = rebuilt.ends_with('/');
                let token = match suffix {
                    Some('*') => {
                        i = end + 1;
                        if preceded_by_slash {
                            rebuilt.pop();
                            "___PARAM_DOTSTAR_SLASH___"
                        } else {
                            "___PARAM_DOTSTAR___"
                        }
                    }
                    Some('+') => {
                        i = end + 1;
                        "___PARAM_DOTPLUS___"
                    }
                    Some('?') => {
                        i = end + 1;
                        if preceded_by_slash {
                            rebuilt.pop();
                            "___PARAM_OPT_SLASH___"
                        } else {
                            "___PARAM_OPT___"
                        }
                    }
                    _ => {
                        i = end;
                        "___PARAM_SEG___"
                    }
                };
                rebuilt.push_str(token);
                continue;
            }
        }
        if chars[i] == '*' {
            rebuilt.push_str("___STAR___");
            i += 1;
            continue;
        }
        rebuilt.push(chars[i]);
        i += 1;
    }

    let mut escaped = String::new();
    for ch in rebuilt.chars() {
        match ch {
            '.' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }

    let regex_body = escaped
        .replace("___PARAM_DOTSTAR_SLASH___", "(?:/(.*))?")
        .replace("___PARAM_OPT_SLASH___", "(?:/([^/]*))?")
        .replace("___PARAM_DOTSTAR___", "(.*)")
        .replace("___PARAM_DOTPLUS___", "(.+)")
        .replace("___PARAM_OPT___", "([^/]*)")
        .replace("___PARAM_SEG___", "([^/]+)")
        .replace("___STAR___", ".*");

    let Ok(regex) = Regex::new(&format!("^{regex_body}$")) else {
        return false;
    };
    regex.is_match(&normalized_path)
}

fn matcher_allows_path(matcher: Option<&ProxyMatcherField>, pathname: &str) -> bool {
    let Some(matcher) = matcher else {
        return true;
    };

    match matcher {
        ProxyMatcherField::Pattern(pattern) => {
            pattern.is_empty() || path_matches_pattern(pathname, pattern)
        }
        ProxyMatcherField::Patterns(patterns) => {
            patterns.is_empty()
                || patterns.iter().any(|pattern| path_matches_pattern(pathname, pattern))
        }
    }
}

fn resolve_redirect_destination(destination: &str) -> String {
    if destination.starts_with("http://") || destination.starts_with("https://") {
        return destination.to_owned();
    }

    let path = if destination.starts_with('/') {
        destination.to_owned()
    } else {
        format!("/{destination}")
    };

    let Some(origin) = Config::get()
        .and_then(|config| config.server.origin.as_deref())
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
    else {
        return path;
    };

    format!("{}{path}", origin.trim_end_matches('/'))
}

fn find_matching_rule<'a>(rules: &'a [ProxyRule], pathname: &str) -> Option<&'a ProxyRule> {
    rules.iter().find(|rule| path_matches_source(pathname, &rule.source))
}

fn apply_proxy_rule(rule: &ProxyRule) -> Option<AppliedProxyRule> {
    match rule.rule_type.as_str() {
        "redirect" => {
            let destination = rule.destination.as_deref()?.to_owned();
            Some(AppliedProxyRule::Redirect {
                destination,
                permanent: rule.permanent.unwrap_or(false),
            })
        }
        "rewrite" => Some(AppliedProxyRule::Rewrite(rule.destination.as_deref()?.to_owned())),
        "header" => Some(AppliedProxyRule::Headers(rule.headers.clone().unwrap_or_default())),
        "block" => Some(AppliedProxyRule::Block),
        _ => None,
    }
}

fn redirect_response(destination: String, permanent: bool) -> Option<Response> {
    let status =
        if permanent { StatusCode::PERMANENT_REDIRECT } else { StatusCode::TEMPORARY_REDIRECT };
    Response::builder().status(status).header("Location", destination).body(Body::empty()).ok()
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

    #[expect(clippy::too_many_lines)]
    fn call(&mut self, mut request: Request) -> Self::Future {
        let state = self.state.clone();
        let inner = self.inner.clone();
        let mut inner = mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            let manifest = match load_proxy_manifest() {
                Ok(Some(manifest)) => manifest,
                Ok(None) => return inner.call(request).await,
                Err(error) => {
                    tracing::error!("Invalid proxy configuration: {}", error);
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Invalid proxy configuration"))
                        .unwrap_or_else(|_| Response::new(Body::empty())));
                }
            };

            let path = request.uri().path();
            if path.starts_with("/_rari/") || path.starts_with("/vite-server/") {
                return inner.call(request).await;
            }

            if !matcher_allows_path(manifest.matcher.as_ref(), path) {
                return inner.call(request).await;
            }

            if let Some(rule) = find_matching_rule(&manifest.rules, path)
                && let Some(applied) = apply_proxy_rule(rule)
            {
                match applied {
                    AppliedProxyRule::Redirect { destination, permanent } => {
                        let location = resolve_redirect_destination(&destination);
                        if let Some(response) = redirect_response(location, permanent) {
                            return Ok(response);
                        }
                    }
                    AppliedProxyRule::Rewrite(rewrite_path) => match rewrite_path.parse() {
                        Ok(uri) => {
                            *request.uri_mut() = uri;
                        }
                        Err(e) => {
                            tracing::error!("Failed to parse rewrite path: {}", e);
                        }
                    },
                    AppliedProxyRule::Headers(headers) => {
                        let mut response = inner.call(request).await?;
                        apply_string_headers(response.headers_mut(), headers);
                        return Ok(response);
                    }
                    AppliedProxyRule::Block => {
                        return Ok(Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Body::empty())
                            .unwrap_or_else(|_| Response::new(Body::empty())));
                    }
                }
            }

            if !manifest.requires_runtime {
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
                        let location = resolve_redirect_destination(&redirect.destination);
                        if let Some(response) = redirect_response(location, redirect.permanent) {
                            return Ok(response);
                        }
                        return inner.call(request).await;
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
    if !requires_proxy_runtime()? {
        return Ok(());
    }

    let Some(rari_pkg_dir) = resolve_rari_package_dir().await else {
        tracing::debug!("Proxy: rari package directory not found in node_modules");
        return Err(RariError::configuration(
            "Proxy requiresRuntime is true but the rari package was not found in node_modules",
        ));
    };

    let executor_path = rari_pkg_dir.join("dist/proxy/runtime-executor.mjs");

    if !fs::try_exists(&executor_path).await.unwrap_or(false) {
        tracing::debug!(
            "Proxy: executor not found at {}, skipping proxy setup",
            executor_path.display()
        );
        return Err(RariError::configuration(format!(
            "Proxy requiresRuntime is true but runtime executor was not found at {}",
            executor_path.display()
        )));
    }

    let executor_absolute = fs::canonicalize(&executor_path).await.unwrap_or(executor_path);
    let executor_specifier = path_to_file_url(&executor_absolute);

    let rari_request_path = rari_pkg_dir.join("dist/proxy/RariRequest.mjs");
    let rari_request_absolute =
        fs::canonicalize(&rari_request_path).await.unwrap_or(rari_request_path);
    let rari_request_specifier = path_to_file_url(&rari_request_absolute);

    let Some(proxy_file_path) = resolve_proxy_dist_path()? else {
        return Err(RariError::configuration(
            "Proxy requiresRuntime is true but bundlePath is missing or the proxy bundle was not found",
        ));
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

    #[test]
    fn path_matches_source_ignores_trailing_slash() {
        assert!(path_matches_source("/docs/", "/docs"));
        assert!(path_matches_source("/docs", "/docs/"));
        assert!(!path_matches_source("/docs", "/sponsors"));
    }

    #[test]
    fn find_matching_rule_returns_first_source_hit() {
        let rules = vec![
            ProxyRule {
                source: "/docs".to_string(),
                rule_type: "redirect".to_string(),
                destination: Some("/docs/getting-started".to_string()),
                permanent: Some(true),
                headers: None,
            },
            ProxyRule {
                source: "/sponsors".to_string(),
                rule_type: "redirect".to_string(),
                destination: Some("/enterprise/sponsors".to_string()),
                permanent: Some(true),
                headers: None,
            },
        ];

        let matched = find_matching_rule(&rules, "/sponsors/").unwrap();
        assert_eq!(matched.destination.as_deref(), Some("/enterprise/sponsors"));
    }

    #[test]
    fn path_matches_pattern_collapses_consecutive_slashes() {
        assert!(path_matches_pattern("//admin", "/admin"));
        assert!(path_matches_pattern("/admin//", "/admin"));
        assert!(path_matches_pattern("//api//users", "/api/*"));
    }

    #[test]
    fn path_matches_pattern_supports_wildcards_and_params() {
        assert!(path_matches_pattern("/api/users", "/api/*"));
        assert!(path_matches_pattern("/users/123", "/users/:id"));
        assert!(!path_matches_pattern("/blog/post", "/api/*"));
    }

    #[test]
    fn path_matches_pattern_optional_catch_all_includes_base_path() {
        assert!(path_matches_pattern("/dashboard", "/dashboard/:path*"));
        assert!(path_matches_pattern("/dashboard/settings", "/dashboard/:path*"));
        assert!(path_matches_pattern("/docs", "/docs/:slug?"));
        assert!(path_matches_pattern("/docs/intro", "/docs/:slug?"));
        assert!(!path_matches_pattern("/dashboard", "/dashboard/:id"));
    }

    #[test]
    fn matcher_allows_path_defaults_to_true_without_matcher() {
        assert!(matcher_allows_path(None, "/anything"));
        assert!(!matcher_allows_path(
            Some(&ProxyMatcherField::Pattern("/dashboard/:path*".to_string())),
            "/about",
        ));
        assert!(matcher_allows_path(
            Some(&ProxyMatcherField::Patterns(vec!["/api/*".to_string(), "/admin".to_string()])),
            "/api/v1",
        ));
    }

    #[test]
    fn redirect_response_uses_308_for_permanent() {
        let response = redirect_response("https://example.com/docs".to_string(), true).unwrap();
        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(
            response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()),
            Some("https://example.com/docs")
        );
    }

    #[test]
    fn resolve_redirect_destination_keeps_absolute_urls() {
        assert_eq!(
            resolve_redirect_destination("https://cdn.example/docs"),
            "https://cdn.example/docs"
        );
    }

    #[test]
    fn resolve_redirect_destination_uses_relative_location_without_origin() {
        assert_eq!(resolve_redirect_destination("/docs/getting-started"), "/docs/getting-started");
        assert_eq!(resolve_redirect_destination("relative"), "/relative");
    }
}
