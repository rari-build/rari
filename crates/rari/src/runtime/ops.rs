use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::http::HeaderMap;
use deno_core::{ModuleSpecifier, OpDecl, OpState, op2};
use deno_error::JsErrorBox;
use deno_runtime::BootstrapOptions;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::{
    rendering::base,
    server::{
        actions,
        core::utils::client,
        middleware::request_context::{PendingCookie, PendingCookieKey, RequestContext},
    },
};

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
        #[expect(unused)]
        final_row_id: Option<String>,
    },
}

#[derive(Default)]
#[non_exhaustive]
pub struct StreamOpState {
    /// Per-stream chunk senders so concurrent streams on one isolate don't clobber each other.
    pub chunk_senders: FxHashMap<String, mpsc::Sender<Result<Vec<u8>, RariError>>>,
    pub row_counters: FxHashMap<String, u32>,
    /// Filled by `op_stream_promise_settled` so the isolate worker can complete
    /// pending streams without polling V8 via `execute_script` every pump tick.
    pub settled: FxHashMap<String, Result<(), String>>,
}

impl StreamOpState {
    pub fn register_sender(
        &mut self,
        stream_id: String,
        sender: mpsc::Sender<Result<Vec<u8>, RariError>>,
    ) {
        self.chunk_senders.insert(stream_id.clone(), sender);
        self.row_counters.entry(stream_id).or_insert(0);
    }

    pub fn take_sender(
        &mut self,
        stream_id: &str,
    ) -> Option<mpsc::Sender<Result<Vec<u8>, RariError>>> {
        self.row_counters.remove(stream_id);
        self.chunk_senders.remove(stream_id)
    }

    pub fn get_sender(&self, stream_id: &str) -> Option<mpsc::Sender<Result<Vec<u8>, RariError>>> {
        self.chunk_senders.get(stream_id).cloned()
    }

    pub fn take_settled(&mut self, stream_id: &str) -> Option<Result<(), String>> {
        self.settled.remove(stream_id)
    }

    pub fn mark_settled(&mut self, stream_id: String, result: Result<(), String>) {
        self.settled.insert(stream_id, result);
    }

    pub fn get_next_row_id(&mut self, stream_id: &str) -> String {
        let counter = self.row_counters.entry(stream_id.to_string()).or_insert(0);
        let id = format!("{:x}", *counter);
        *counter += 1;
        id
    }
}

/// Multiplexed request contexts keyed by `RequestContext::request_id`.
#[derive(Default)]
#[non_exhaustive]
pub struct RequestContextStore {
    pub by_id: FxHashMap<String, Arc<RequestContext>>,
}

impl RequestContextStore {
    pub fn insert(&mut self, ctx: Arc<RequestContext>) {
        self.by_id.insert(ctx.request_id().to_string(), ctx);
    }

    pub fn remove(&mut self, request_id: &str) -> Option<Arc<RequestContext>> {
        self.by_id.remove(request_id)
    }

    pub fn get(&self, request_id: &str) -> Option<Arc<RequestContext>> {
        self.by_id.get(request_id).cloned()
    }
}

fn resolve_request_context(
    op_state: &OpState,
    request_id: Option<&str>,
) -> Option<Arc<RequestContext>> {
    if let Some(id) = request_id.filter(|id| !id.is_empty()) {
        return op_state.try_borrow::<RequestContextStore>().and_then(|store| store.get(id));
    }
    op_state.try_borrow::<Arc<RequestContext>>().cloned()
}

fn parse_hex_row_id(row_id: &str, context: &str) -> Result<u32, JsErrorBox> {
    if !row_id.chars().all(|c| c.is_ascii_hexdigit()) {
        tracing::error!("op_send_chunk_to_rust: invalid row_id '{}' for {}", row_id, context);
        return Err(JsErrorBox::generic(format!("Invalid row_id: {row_id}")));
    }

    u32::from_str_radix(row_id, 16).map_err(|e| {
        tracing::error!(
            "op_send_chunk_to_rust: invalid row_id '{}' for {}: {}",
            row_id,
            context,
            e
        );
        JsErrorBox::generic(format!("Invalid row_id: {row_id}"))
    })
}

#[op2]
pub async fn op_send_chunk_to_rust(
    state: Rc<RefCell<OpState>>,
    #[string] stream_id: String,
    #[string] operation_json: String,
) -> Result<(), JsErrorBox> {
    let operation: RscStreamOperation = match serde_json::from_str(&operation_json) {
        Ok(op) => op,
        Err(e) => {
            let err_msg = format!(
                "Invalid JSON for RSC operation: {e}. JSON length: {}",
                operation_json.len()
            );
            tracing::error!("{err_msg}");
            return Err(JsErrorBox::generic(err_msg));
        }
    };

    let sender_option = {
        let mut op_state_ref = state.borrow_mut();
        let Some(stream_op_state) = op_state_ref.try_borrow_mut::<StreamOpState>() else {
            return Err(JsErrorBox::generic("StreamOpState not found."));
        };

        match &operation {
            RscStreamOperation::Complete { .. } | RscStreamOperation::Error { .. } => {
                stream_op_state.take_sender(&stream_id)
            }
            _ => stream_op_state.get_sender(&stream_id),
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

            let row_id_num = parse_hex_row_id(&row_id, "module reference")?;
            let rsc_row = format!("{row_id_num:x}:M{module_data}");

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                tracing::error!("op_send_chunk_to_rust: receiver dropped for module reference.");
            }
        }
        (Some(sender), RscStreamOperation::ReactElement { row_id, element }) => {
            let row_id_num = parse_hex_row_id(&row_id, "React element")?;
            let rsc_row = format!("{row_id_num:x}:J{element}");

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                tracing::error!("op_send_chunk_to_rust: receiver dropped for React element.");
            }
        }
        (Some(sender), RscStreamOperation::Symbol { row_id, symbol_ref }) => {
            let row_id_num = parse_hex_row_id(&row_id, "symbol reference")?;
            let rsc_row = format!("{row_id_num:x}:S\"{symbol_ref}\"");

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                tracing::error!("op_send_chunk_to_rust: receiver dropped for symbol reference.");
            }
        }
        (Some(sender), RscStreamOperation::Error { row_id, message, stack, phase, digest }) => {
            tracing::error!("Streaming error in row {row_id}: {message}");
            if let Some(stack_trace) = &stack {
                tracing::error!("Stack trace: {stack_trace}");
            }

            let error_data = serde_json::json!({
                "message": message,
                "stack": stack,
                "phase": phase,
                "digest": digest
            });

            let row_id_num = parse_hex_row_id(&row_id, "error message")?;
            let rsc_row = format!("{row_id_num:x}:E{error_data}");

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                tracing::error!("op_send_chunk_to_rust: receiver dropped for error message.");
            }
        }
        (Some(_sender), RscStreamOperation::Complete { final_row_id: _ }) => {}
        (None, operation) => {
            tracing::error!("No sender available for operation: {operation:?}");
            return Err(JsErrorBox::generic("No chunk sender available"));
        }
    }

    Ok(())
}

#[cfg(test)]
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

#[cfg(test)]
pub fn create_element_operation(row_id: &str, element: &serde_json::Value) -> String {
    serde_json::json!({
        "type": "element",
        "row_id": row_id,
        "element": element
    })
    .to_string()
}

#[cfg(test)]
pub fn create_symbol_operation(row_id: &str, symbol_ref: &str) -> String {
    serde_json::json!({
        "type": "symbol",
        "row_id": row_id,
        "symbol_ref": symbol_ref
    })
    .to_string()
}

#[cfg(test)]
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

#[op2(fast)]
pub fn op_rari_has_node_modules_dir(state: &OpState) -> bool {
    state.try_borrow::<BootstrapOptions>().is_some_and(|options| options.has_node_modules_dir)
}

#[op2]
#[string]
pub fn op_main_module(state: &OpState) -> String {
    state.borrow::<ModuleSpecifier>().to_string()
}

/// Expensive on Windows; mirrors `deno_runtime::ops::runtime::op_ppid`.
#[op2(fast)]
#[number]
pub fn op_ppid() -> i64 {
    #[cfg(windows)]
    {
        use std::mem;

        use windows_sys::Win32::{
            Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
            System::{
                Diagnostics::ToolHelp::{
                    CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next,
                    TH32CS_SNAPPROCESS,
                },
                Threading::GetCurrentProcessId,
            },
        };

        // SAFETY: Win32 calls
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return -1;
            }

            let mut entry: PROCESSENTRY32 = mem::zeroed();
            entry.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;

            let success = Process32First(snapshot, &mut entry);
            if success == 0 {
                CloseHandle(snapshot);
                return -1;
            }

            let this_pid = GetCurrentProcessId();
            while entry.th32ProcessID != this_pid {
                let success = Process32Next(snapshot, &mut entry);
                if success == 0 {
                    CloseHandle(snapshot);
                    return -1;
                }
            }
            CloseHandle(snapshot);

            entry.th32ParentProcessID.into()
        }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::process::parent_id;
        parent_id().into()
    }
}

pub fn rari_main_module() -> ModuleSpecifier {
    static MAIN: OnceLock<ModuleSpecifier> = OnceLock::new();
    #[expect(clippy::expect_used, reason = "Static main module URL is valid")]
    MAIN.get_or_init(|| ModuleSpecifier::parse("file:///rari").expect("valid rari main module url"))
        .clone()
}

pub fn get_streaming_ops() -> Vec<OpDecl> {
    vec![
        op_rari_has_node_modules_dir(),
        op_main_module(),
        op_ppid(),
        op_send_chunk_to_rust(),
        op_fizz_chunk_try(),
        op_fizz_chunk(),
        op_fizz_done(),
        op_stream_promise_settled(),
        op_internal_log(),
        op_sanitize_html(),
        op_get_cookies(),
        op_get_request_headers(),
        op_set_cookie(),
        op_delete_cookie(),
    ]
}

/// Sync try-send for Fizz chunks. Returns: `0` sent, `1` full (use async op), `2` disconnected.
#[op2(fast)]
pub fn op_fizz_chunk_try(state: &OpState, #[string] stream_id: &str, #[string] html: &str) -> u8 {
    let Some(stream_op_state) = state.try_borrow::<StreamOpState>() else {
        return 2;
    };
    let Some(sender) = stream_op_state.get_sender(stream_id) else {
        return 2;
    };
    match sender.try_send(Ok(html.as_bytes().to_vec())) {
        Ok(()) => 0,
        Err(mpsc::error::TrySendError::Full(_)) => 1,
        Err(mpsc::error::TrySendError::Closed(_)) => 2,
    }
}

#[op2]
pub async fn op_fizz_chunk(
    state: Rc<RefCell<OpState>>,
    #[string] stream_id: String,
    #[string] html: String,
) -> Result<(), JsErrorBox> {
    let sender = {
        let op_state_ref = state.borrow();
        let Some(stream_op_state) = op_state_ref.try_borrow::<StreamOpState>() else {
            return Err(JsErrorBox::generic("StreamOpState not found."));
        };
        stream_op_state.get_sender(&stream_id)
    };

    match sender {
        Some(sender) => {
            let bytes = html.into_bytes();
            // Prefer try_send so the common path doesn't await the channel. Fall back to
            // async send only under backpressure -- never blocking_send (panics in Tokio).
            match sender.try_send(Ok(bytes)) {
                Ok(()) => Ok(()),
                Err(mpsc::error::TrySendError::Full(msg)) => {
                    if sender.send(msg).await.is_err() {
                        tracing::debug!("Fizz stream client disconnected before chunk was sent");
                        return Err(JsErrorBox::generic("Fizz stream receiver disconnected"));
                    }
                    Ok(())
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    // Leave the map entry for op_fizz_done / settle cleanup; just
                    // signal disconnect so JS stops pumping this stream.
                    tracing::debug!("Fizz stream client disconnected before chunk was sent");
                    Err(JsErrorBox::generic("Fizz stream receiver disconnected"))
                }
            }
        }
        None => Err(JsErrorBox::generic("No chunk sender available for Fizz chunk")),
    }
}

#[op2(fast)]
pub fn op_fizz_done(state: &mut OpState, #[string] stream_id: &str) {
    if let Some(stream_op_state) = state.try_borrow_mut::<StreamOpState>() {
        stream_op_state.take_sender(stream_id);
    }
}

#[op2(fast)]
pub fn op_stream_promise_settled(
    state: &mut OpState,
    #[string] stream_id: &str,
    ok: bool,
    #[string] error: &str,
) {
    if let Some(stream_op_state) = state.try_borrow_mut::<StreamOpState>() {
        let result = if ok { Ok(()) } else { Err(error.to_string()) };
        stream_op_state.mark_settled(stream_id.to_string(), result);
    }
}

#[op2(fast)]
pub fn op_internal_log(#[string] message: &str) {
    tracing::debug!("[rari] {message}");
}

#[op2]
#[string]
pub fn op_sanitize_html(#[string] html: &str, #[string] _component_id: &str) -> String {
    base::sanitize_html_output(html)
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

fn headers_to_json(headers: &HeaderMap) -> serde_json::Map<String, serde_json::Value> {
    let mut headers_obj = serde_json::Map::new();
    for (name, value) in headers {
        if let Ok(value_str) = value.to_str() {
            let key = name.as_str().to_string();
            if let Some(existing) = headers_obj.get_mut(&key) {
                if let Some(s) = existing.as_str() {
                    *existing = serde_json::Value::String(format!("{s}, {value_str}"));
                }
            } else {
                headers_obj.insert(key, serde_json::Value::String(value_str.to_string()));
            }
        }
    }
    headers_obj
}

#[op2]
#[serde]
pub async fn op_fetch_with_cache(
    state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[string] options_json: String,
    #[string] request_id: String,
) -> Result<serde_json::Value, JsErrorBox> {
    let options: rustc_hash::FxHashMap<String, String> = serde_json::from_str(&options_json)
        .map_err(|e| JsErrorBox::generic(format!("Invalid options JSON: {e}")))?;

    let request_context = {
        let op_state_ref = state.borrow();
        resolve_request_context(&op_state_ref, Some(request_id.as_str()))
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
                tracing::error!("Fetch failed for {}: {}", url, e);
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
                // Any completed HTTP exchange is ok -- a 404/500 response is
                // data for the caller, not a fetch failure. Gating ok on 2xx
                // made the JS wrapper throw bare "Fetch failed" for every
                // non-2xx and re-fetch the same URL through the fallback path.
                // Matches the request-context branch above.
                "ok": true,
                "status": status,
                "statusText": http_status_text(status),
                "body": body,
                "headers": headers,
                "cached": false,
                "tags": Vec::<String>::new()
            })),
            Err(e) => {
                tracing::error!("Fetch failed for {}: {}", url, e);
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
    }
}

async fn perform_simple_fetch(
    url: &str,
    options: &rustc_hash::FxHashMap<String, String>,
) -> Result<(u16, String, serde_json::Map<String, serde_json::Value>), RariError> {
    let client = client::get_http_client()?;
    let mut request = client.get(url);

    if let Some(headers_str) = options.get("headers")
        && let Ok(pairs) = serde_json::from_str::<Vec<(String, String)>>(headers_str)
    {
        for (key, value) in pairs {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    let timeout = options.get("timeout").and_then(|t| t.parse::<u64>().ok()).unwrap_or(5000);

    request = request.timeout(Duration::from_millis(timeout));

    let response =
        request.send().await.map_err(|e| RariError::network(format!("Request failed: {e}")))?;

    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let headers_obj = headers_to_json(&headers);

    let body = response
        .text()
        .await
        .map_err(|e| RariError::network(format!("Failed to read response: {e}")))?;

    Ok((status, body, headers_obj))
}

#[allow(clippy::allow_attributes, clippy::needless_pass_by_value)]
#[op2]
#[string]
pub fn op_get_cookies(state: Rc<RefCell<OpState>>, #[string] request_id: String) -> String {
    let op_state_ref = state.borrow();
    let Some(ctx) = resolve_request_context(&op_state_ref, Some(request_id.as_str())) else {
        return String::new();
    };

    let mut cookies: BTreeMap<String, String> = ctx
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
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
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

    cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ")
}

#[allow(clippy::allow_attributes, clippy::needless_pass_by_value)]
#[op2]
#[string]
pub fn op_get_request_headers(state: Rc<RefCell<OpState>>, #[string] request_id: String) -> String {
    let op_state_ref = state.borrow();
    let Some(ctx) = resolve_request_context(&op_state_ref, Some(request_id.as_str())) else {
        return "{}".to_string();
    };

    serde_json::to_string(&ctx.request_headers).unwrap_or_else(|_| "{}".to_string())
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
    #[serde(rename = "requestId", default)]
    request_id: Option<String>,
}

#[allow(clippy::allow_attributes, clippy::needless_pass_by_value)]
#[op2]
#[serde]
pub fn op_set_cookie(
    state: Rc<RefCell<OpState>>,
    #[serde] args: SetCookieArgs,
) -> Result<(), JsErrorBox> {
    if !actions::is_valid_cookie_name(&args.name) {
        return Err(JsErrorBox::type_error(format!("Invalid cookie name: '{}'", args.name)));
    }

    if !actions::is_valid_cookie_value(&args.value) {
        return Err(JsErrorBox::type_error(format!(
            "Invalid cookie value for '{}': contains invalid characters",
            args.name
        )));
    }

    if let Some(ref path) = args.path
        && !actions::is_valid_attr_value(path)
    {
        return Err(JsErrorBox::type_error(format!(
            "Invalid cookie path for '{}': '{}'",
            args.name, path
        )));
    }

    if let Some(ref domain) = args.domain
        && !actions::is_valid_attr_value(domain)
    {
        return Err(JsErrorBox::type_error(format!(
            "Invalid cookie domain for '{}': '{}'",
            args.name, domain
        )));
    }

    let op_state_ref = state.borrow();
    if let Some(ctx) = resolve_request_context(&op_state_ref, args.request_id.as_deref()) {
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

#[allow(clippy::allow_attributes, clippy::needless_pass_by_value)]
#[op2(fast)]
pub fn op_delete_cookie(
    state: Rc<RefCell<OpState>>,
    #[string] name: String,
    #[string] request_id: String,
) {
    let op_state_ref = state.borrow();
    if let Some(ctx) = resolve_request_context(&op_state_ref, Some(request_id.as_str())) {
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

#[allow(clippy::allow_attributes, clippy::needless_pass_by_value)]
#[op2]
#[serde]
pub fn op_cache_get(
    state: Rc<RefCell<OpState>>,
    #[string] cache_key: &str,
    #[string] request_id: &str,
) -> Option<serde_json::Value> {
    let op_state_ref = state.borrow();
    if let Some(ctx) = resolve_request_context(&op_state_ref, Some(request_id)) {
        ctx.function_cache.get(cache_key).map(|entry| entry.value().clone())
    } else {
        None
    }
}

#[allow(clippy::allow_attributes, clippy::needless_pass_by_value)]
#[op2]
pub fn op_cache_set(
    state: Rc<RefCell<OpState>>,
    #[string] cache_key: String,
    #[serde] value: serde_json::Value,
    #[string] request_id: &str,
) {
    let op_state_ref = state.borrow();
    if let Some(ctx) = resolve_request_context(&op_state_ref, Some(request_id)) {
        ctx.function_cache.insert(cache_key, value);
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::*;

    #[test]
    fn test_stream_op_state_operations() {
        let mut stream_state = StreamOpState::default();
        let stream_id = "s1";

        let row_id_1 = stream_state.get_next_row_id(stream_id);
        let row_id_2 = stream_state.get_next_row_id(stream_id);

        assert_eq!(row_id_1, "0");
        assert_eq!(row_id_2, "1");
        assert_eq!(stream_state.row_counters.get(stream_id), Some(&2));

        let (sender, _receiver) = mpsc::channel::<Result<Vec<u8>, RariError>>(32);
        stream_state.register_sender(stream_id.to_string(), sender);

        assert!(stream_state.get_sender(stream_id).is_some());
        assert!(stream_state.take_sender(stream_id).is_some());
        assert!(stream_state.get_sender(stream_id).is_none());
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
