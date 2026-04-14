use crate::server::http_client::get_http_client;
use deno_core::{OpDecl, OpState, op2};
use deno_error::JsErrorBox;
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum RscStreamOperation {
    #[serde(rename = "module")]
    ModuleReference {
        row_id: String,
        module_id: String,
        chunks: Vec<String>,
        name: String,
        #[serde(default)]
        async_module: bool,
    },
    #[serde(rename = "element")]
    ReactElement { row_id: String, element: serde_json::Value },
    #[serde(rename = "symbol")]
    Symbol { row_id: String, symbol_ref: String },
    #[serde(rename = "error")]
    Error {
        row_id: String,
        message: String,
        #[serde(default)]
        stack: Option<String>,
        #[serde(default)]
        phase: Option<String>,
        #[serde(default)]
        digest: Option<String>,
    },
    #[serde(rename = "complete")]
    Complete {
        #[serde(default)]
        #[allow(unused)]
        final_row_id: Option<String>,
    },
}

#[derive(Default)]
pub struct StreamOpState {
    pub chunk_sender: Option<mpsc::Sender<Result<Vec<u8>, String>>>,
    pub current_stream_id: Option<String>,
    pub row_counter: u32,
}

impl StreamOpState {
    pub fn get_next_row_id(&mut self) -> String {
        let id = self.row_counter.to_string();
        self.row_counter += 1;
        id
    }
}

#[allow(clippy::disallowed_methods)]
#[op2]
pub async fn op_send_chunk_to_rust(
    state: Rc<RefCell<OpState>>,
    #[string] operation_json: String,
) -> Result<(), JsErrorBox> {
    let operation: RscStreamOperation = match serde_json::from_str(&operation_json) {
        Ok(op) => op,
        Err(e) => {
            let err_msg = format!(
                "Invalid JSON for RSC operation: {e}. JSON length: {}",
                operation_json.len()
            );
            error!("{err_msg}");
            return Err(JsErrorBox::generic(err_msg));
        }
    };

    let sender_option = {
        let mut op_state_ref = state.borrow_mut();
        let stream_op_state = match op_state_ref.try_borrow_mut::<StreamOpState>() {
            Some(sos) => sos,
            None => return Err(JsErrorBox::generic("StreamOpState not found.")),
        };

        match &operation {
            RscStreamOperation::Complete { .. } | RscStreamOperation::Error { .. } => {
                stream_op_state.chunk_sender.take()
            }
            _ => stream_op_state.chunk_sender.as_ref().cloned(),
        }
    };

    match (sender_option, operation) {
        (
            Some(sender),
            RscStreamOperation::ModuleReference { row_id, module_id, chunks, name, async_module },
        ) => {
            let module_data = serde_json::json!({
                "id": module_id,
                "chunks": chunks,
                "name": name,
                "async": async_module
            });

            let row_id_num: u32 = match row_id.parse() {
                Ok(num) => num,
                Err(e) => {
                    error!(
                        "op_send_chunk_to_rust: invalid row_id '{}' for module reference: {}",
                        row_id, e
                    );
                    return Err(JsErrorBox::generic(format!("Invalid row_id: {}", row_id)));
                }
            };
            let rsc_row = format!("{:x}:M{}", row_id_num, module_data);

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                error!("op_send_chunk_to_rust: receiver dropped for module reference.");
            }
        }
        (Some(sender), RscStreamOperation::ReactElement { row_id, element }) => {
            let row_id_num: u32 = match row_id.parse() {
                Ok(num) => num,
                Err(e) => {
                    error!(
                        "op_send_chunk_to_rust: invalid row_id '{}' for React element: {}",
                        row_id, e
                    );
                    return Err(JsErrorBox::generic(format!("Invalid row_id: {}", row_id)));
                }
            };
            let rsc_row = format!("{:x}:J{}", row_id_num, element);

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                error!("op_send_chunk_to_rust: receiver dropped for React element.");
            }
        }
        (Some(sender), RscStreamOperation::Symbol { row_id, symbol_ref }) => {
            let row_id_num: u32 = match row_id.parse() {
                Ok(num) => num,
                Err(e) => {
                    error!(
                        "op_send_chunk_to_rust: invalid row_id '{}' for symbol reference: {}",
                        row_id, e
                    );
                    return Err(JsErrorBox::generic(format!("Invalid row_id: {}", row_id)));
                }
            };
            let rsc_row = format!("{:x}:S\"{}\"", row_id_num, symbol_ref);

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                error!("op_send_chunk_to_rust: receiver dropped for symbol reference.");
            }
        }
        (Some(sender), RscStreamOperation::Error { row_id, message, stack, phase, digest }) => {
            error!("Streaming error in row {row_id}: {message}");
            if let Some(stack_trace) = &stack {
                error!("Stack trace: {stack_trace}");
            }

            let error_data = serde_json::json!({
                "message": message,
                "stack": stack,
                "phase": phase,
                "digest": digest
            });

            let row_id_num: u32 = match row_id.parse() {
                Ok(num) => num,
                Err(e) => {
                    error!(
                        "op_send_chunk_to_rust: invalid row_id '{}' for error message: {}",
                        row_id, e
                    );
                    return Err(JsErrorBox::generic(format!("Invalid row_id: {}", row_id)));
                }
            };
            let rsc_row = format!("{:x}:E{}", row_id_num, error_data);

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                error!("op_send_chunk_to_rust: receiver dropped for error message.");
            }
        }
        (Some(_sender), RscStreamOperation::Complete { final_row_id: _ }) => {}
        (None, operation) => {
            error!("No sender available for operation: {operation:?}");
            return Err(JsErrorBox::generic("No chunk sender available"));
        }
    }

    Ok(())
}

#[allow(clippy::disallowed_methods)]
pub fn create_module_operation(
    row_id: &str,
    module_id: &str,
    chunks: &[&str],
    name: &str,
) -> String {
    serde_json::json!({
        "type": "module",
        "row_id": row_id,
        "module_id": module_id,
        "chunks": chunks,
        "name": name,
        "async_module": false
    })
    .to_string()
}

#[allow(clippy::disallowed_methods)]
pub fn create_element_operation(row_id: &str, element: &serde_json::Value) -> String {
    serde_json::json!({
        "type": "element",
        "row_id": row_id,
        "element": element
    })
    .to_string()
}

#[allow(clippy::disallowed_methods)]
pub fn create_symbol_operation(row_id: &str, symbol_ref: &str) -> String {
    serde_json::json!({
        "type": "symbol",
        "row_id": row_id,
        "symbol_ref": symbol_ref
    })
    .to_string()
}

#[allow(clippy::disallowed_methods)]
pub fn create_error_operation(
    row_id: &str,
    message: &str,
    stack: Option<&str>,
    phase: Option<&str>,
    digest: Option<&str>,
) -> String {
    serde_json::json!({
        "type": "error",
        "row_id": row_id,
        "message": message,
        "stack": stack,
        "phase": phase,
        "digest": digest
    })
    .to_string()
}

pub fn get_streaming_ops() -> Vec<OpDecl> {
    vec![
        op_send_chunk_to_rust(),
        op_internal_log(),
        op_sanitize_html(),
        op_get_cookies(),
        op_set_cookie(),
        op_delete_cookie(),
    ]
}

#[op2(fast)]
pub fn op_internal_log(#[string] message: &str) {
    error!("[rari] {message}");
}

#[op2]
#[string]
pub fn op_sanitize_html(#[string] html: &str, #[string] _component_id: &str) -> String {
    crate::rsc::rendering::sanitizer::sanitize_component_output(html)
}

fn http_status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

fn headers_to_json(headers: &axum::http::HeaderMap) -> serde_json::Map<String, serde_json::Value> {
    let mut headers_obj = serde_json::Map::new();
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            let key = name.as_str().to_string();
            if let Some(existing) = headers_obj.get_mut(&key) {
                if let Some(s) = existing.as_str() {
                    *existing = serde_json::Value::String(format!("{}, {}", s, value_str));
                }
            } else {
                headers_obj.insert(key, serde_json::Value::String(value_str.to_string()));
            }
        }
    }
    headers_obj
}

#[allow(clippy::disallowed_methods)]
#[op2]
#[serde]
pub async fn op_fetch_with_cache(
    state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[string] options_json: String,
) -> Result<serde_json::Value, JsErrorBox> {
    let options: rustc_hash::FxHashMap<String, String> = serde_json::from_str(&options_json)
        .map_err(|e| JsErrorBox::generic(format!("Invalid options JSON: {}", e)))?;

    let request_context = {
        let op_state_ref = state.borrow();
        op_state_ref
            .try_borrow::<Arc<crate::server::middleware::request_context::RequestContext>>()
            .cloned()
    };

    if let Some(ctx) = request_context {
        match ctx.fetch_with_cache(&url, options).await {
            Ok(result) => {
                let body_str = String::from_utf8_lossy(&result.body).to_string();
                let headers_obj = headers_to_json(&result.headers);

                Ok(serde_json::json!({
                    "ok": true,
                    "status": result.status,
                    "statusText": http_status_text(result.status),
                    "body": body_str,
                    "headers": headers_obj,
                    "cached": result.was_cached,
                    "tags": result.tags
                }))
            }
            Err(e) => {
                error!("Fetch failed for {}: {}", url, e);
                Ok(serde_json::json!({
                    "ok": false,
                    "status": 500,
                    "statusText": "Internal Server Error",
                    "error": e.to_string(),
                    "cached": false,
                    "tags": Vec::<String>::new()
                }))
            }
        }
    } else {
        match perform_simple_fetch(&url, &options).await {
            Ok((status, body, headers)) => Ok(serde_json::json!({
                "ok": (200..300).contains(&status),
                "status": status,
                "statusText": http_status_text(status),
                "body": body,
                "headers": headers,
                "cached": false,
                "tags": Vec::<String>::new()
            })),
            Err(e) => {
                error!("Fetch failed for {}: {}", url, e);
                Ok(serde_json::json!({
                    "ok": false,
                    "status": 500,
                    "statusText": "Internal Server Error",
                    "error": e,
                    "cached": false,
                    "tags": Vec::<String>::new()
                }))
            }
        }
    }
}

async fn perform_simple_fetch(
    url: &str,
    options: &rustc_hash::FxHashMap<String, String>,
) -> Result<(u16, String, serde_json::Map<String, serde_json::Value>), String> {
    let client = get_http_client()?;
    let mut request = client.get(url);

    if let Some(headers_str) = options.get("headers")
        && let Ok(pairs) = serde_json::from_str::<Vec<(String, String)>>(headers_str)
    {
        for (key, value) in pairs {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    let timeout = options.get("timeout").and_then(|t| t.parse::<u64>().ok()).unwrap_or(5000);

    request = request.timeout(std::time::Duration::from_millis(timeout));

    let response = request.send().await.map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let headers_obj = headers_to_json(&headers);

    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    Ok((status, body, headers_obj))
}

#[op2]
#[string]
pub fn op_get_cookies(state: Rc<RefCell<OpState>>) -> String {
    let op_state_ref = state.borrow();
    let Some(ctx) = op_state_ref
        .try_borrow::<Arc<crate::server::middleware::request_context::RequestContext>>()
    else {
        return String::new();
    };

    let mut cookies: std::collections::BTreeMap<String, String> = ctx
        .cookie_header
        .as_deref()
        .unwrap_or("")
        .split(';')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let name = parts.next()?.trim().to_string();
            let value = parts.next().unwrap_or("").trim().to_string();
            if name.is_empty() { None } else { Some((name, value)) }
        })
        .collect();

    let mut pending: Vec<_> =
        ctx.pending_cookies.iter().map(|e| (e.key().clone(), e.value().clone())).collect();
    pending.sort_by(|(a_key, a_cookie), (b_key, b_cookie)| {
        let a_is_delete = a_cookie.max_age == Some(0);
        let b_is_delete = b_cookie.max_age == Some(0);
        match (a_is_delete, b_is_delete) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_path_len = a_key.path.as_deref().unwrap_or("").len();
                let b_path_len = b_key.path.as_deref().unwrap_or("").len();
                a_path_len
                    .cmp(&b_path_len)
                    .then_with(|| a_key.name.cmp(&b_key.name))
                    .then_with(|| a_key.domain.cmp(&b_key.domain))
            }
        }
    });

    for (key, cookie) in &pending {
        if cookie.max_age == Some(0) {
            cookies.remove(&key.name);
        } else {
            cookies.insert(key.name.clone(), cookie.value.clone());
        }
    }

    cookies.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("; ")
}

#[derive(serde::Deserialize)]
pub struct SetCookieArgs {
    name: String,
    value: String,
    path: Option<String>,
    domain: Option<String>,
    expires: Option<String>,
    #[serde(rename = "maxAge")]
    max_age: Option<i64>,
    #[serde(rename = "httpOnly", default)]
    http_only: bool,
    #[serde(default)]
    secure: bool,
    #[serde(rename = "sameSite")]
    same_site: Option<String>,
    priority: Option<String>,
    #[serde(default)]
    partitioned: bool,
}

#[op2]
#[serde]
pub fn op_set_cookie(
    state: Rc<RefCell<OpState>>,
    #[serde] args: SetCookieArgs,
) -> Result<(), JsErrorBox> {
    use crate::server::middleware::request_context::{PendingCookie, PendingCookieKey};

    if !crate::server::actions::is_valid_cookie_name(&args.name) {
        return Err(JsErrorBox::type_error(format!("Invalid cookie name: '{}'", args.name)));
    }

    if !crate::server::actions::is_valid_cookie_value(&args.value) {
        return Err(JsErrorBox::type_error(format!(
            "Invalid cookie value for '{}': contains invalid characters",
            args.name
        )));
    }

    if let Some(ref path) = args.path
        && !crate::server::actions::is_valid_attr_value(path)
    {
        return Err(JsErrorBox::type_error(format!(
            "Invalid cookie path for '{}': '{}'",
            args.name, path
        )));
    }

    if let Some(ref domain) = args.domain
        && !crate::server::actions::is_valid_attr_value(domain)
    {
        return Err(JsErrorBox::type_error(format!(
            "Invalid cookie domain for '{}': '{}'",
            args.name, domain
        )));
    }

    let op_state_ref = state.borrow();
    if let Some(ctx) =
        op_state_ref.try_borrow::<Arc<crate::server::middleware::request_context::RequestContext>>()
    {
        let path = args.path.or_else(|| Some("/".to_string()));
        ctx.pending_cookies.insert(
            PendingCookieKey::new(&args.name, path.as_deref(), args.domain.as_deref()),
            PendingCookie {
                name: args.name,
                value: args.value,
                path,
                domain: args.domain,
                expires: args.expires,
                max_age: args.max_age,
                http_only: args.http_only,
                secure: args.secure,
                same_site: args.same_site,
                priority: args.priority,
                partitioned: args.partitioned,
            },
        );
    }

    Ok(())
}

#[op2(fast)]
pub fn op_delete_cookie(state: Rc<RefCell<OpState>>, #[string] name: String) {
    use crate::server::middleware::request_context::{PendingCookie, PendingCookieKey};
    let op_state_ref = state.borrow();
    if let Some(ctx) =
        op_state_ref.try_borrow::<Arc<crate::server::middleware::request_context::RequestContext>>()
    {
        let cookies_to_delete: Vec<(Option<String>, Option<String>)> = ctx
            .pending_cookies
            .iter()
            .filter(|entry| entry.key().name == name)
            .map(|entry| (entry.key().path.clone(), entry.key().domain.clone()))
            .collect();

        ctx.pending_cookies.retain(|k, _| k.name != name);

        if cookies_to_delete.is_empty() {
            ctx.pending_cookies.insert(
                PendingCookieKey::new(&name, Some("/"), None),
                PendingCookie {
                    name,
                    value: String::new(),
                    path: Some("/".to_string()),
                    domain: None,
                    expires: None,
                    max_age: Some(0),
                    http_only: false,
                    secure: false,
                    same_site: None,
                    priority: None,
                    partitioned: false,
                },
            );
        } else {
            for (path, domain) in cookies_to_delete {
                let deletion_path = path.clone().unwrap_or_else(|| "/".to_string());
                ctx.pending_cookies.insert(
                    PendingCookieKey::new(&name, Some(&deletion_path), domain.as_deref()),
                    PendingCookie {
                        name: name.clone(),
                        value: String::new(),
                        path: Some(deletion_path),
                        domain,
                        expires: None,
                        max_age: Some(0),
                        http_only: false,
                        secure: false,
                        same_site: None,
                        priority: None,
                        partitioned: false,
                    },
                );
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_stream_op_state_operations() {
        let mut stream_state = StreamOpState::default();

        let row_id_1 = stream_state.get_next_row_id();
        let row_id_2 = stream_state.get_next_row_id();

        assert_eq!(row_id_1, "0");
        assert_eq!(row_id_2, "1");
        assert_eq!(stream_state.row_counter, 2);

        let (sender, _receiver) = mpsc::channel::<Result<Vec<u8>, String>>(32);
        stream_state.chunk_sender = Some(sender);

        assert!(stream_state.chunk_sender.is_some());
    }

    #[test]
    fn test_operation_creation() {
        let module_op = create_module_operation("0", "Button", &["chunk1", "chunk2"], "default");
        assert!(module_op.contains("\"type\":\"module\""));
        assert!(module_op.contains("\"row_id\":\"0\""));
        assert!(module_op.contains("\"module_id\":\"Button\""));

        let element_op = create_element_operation("1", &serde_json::json!({"type": "div"}));
        assert!(element_op.contains("\"type\":\"element\""));
        assert!(element_op.contains("\"row_id\":\"1\""));

        let symbol_op = create_symbol_operation("2", "Symbol.for('react.transitional.element')");
        assert!(symbol_op.contains("\"type\":\"symbol\""));
        assert!(symbol_op.contains("\"row_id\":\"2\""));

        let error_op = create_error_operation("3", "Test error", Some("stack trace"), None, None);
        assert!(error_op.contains("\"type\":\"error\""));
        assert!(error_op.contains("\"message\":\"Test error\""));
        assert!(error_op.contains("\"stack\":\"stack trace\""));
    }
}

#[op2]
#[serde]
pub fn op_cache_get(
    state: Rc<RefCell<OpState>>,
    #[string] cache_key: String,
) -> Option<serde_json::Value> {
    let op_state_ref = state.borrow();
    if let Some(ctx) =
        op_state_ref.try_borrow::<Arc<crate::server::middleware::request_context::RequestContext>>()
    {
        ctx.function_cache.get(&cache_key).map(|entry| entry.value().clone())
    } else {
        None
    }
}

#[op2]
pub fn op_cache_set(
    state: Rc<RefCell<OpState>>,
    #[string] cache_key: String,
    #[serde] value: serde_json::Value,
) {
    let op_state_ref = state.borrow();
    if let Some(ctx) =
        op_state_ref.try_borrow::<Arc<crate::server::middleware::request_context::RequestContext>>()
    {
        ctx.function_cache.insert(cache_key, value);
    }
}
