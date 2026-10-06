//! Streaming protocol between the host and a guest bundle.
//!
//! A guest render runs as a *streaming script* on a pooled isolate, exactly like
//! the host's own Fizz HTML streaming: the host registers a chunk sender for a
//! `stream_id`, the script pushes bytes through the runtime's stream ops, and
//! the script's returned promise settles only after the stream is done.
//!
//! Framing on top of that byte stream:
//!
//! 1. The **first** chunk is the header frame: UTF-8 JSON
//!    `{"status": 200, "headers": [["content-type", "text/html"], ...]}`. The
//!    guest emits it once it knows the status (after loaders/actions ran, before
//!    any HTML), via `op_fizz_chunk(streamId, json)`.
//! 2. Every following chunk is body bytes, via `op_fizz_chunk_bytes(streamId,
//!    uint8array)` (or `op_fizz_chunk` for text). Both have sync `_try`
//!    twins returning `0` sent / `1` full / `2` disconnected; a guest should
//!    try those first and only await the async op on `1`, so a render that
//!    never has to wait for the host completes without an event-loop turn per
//!    chunk (`@rari/core/guest`'s sink does this).
//! 3. The guest calls `op_fizz_done(streamId)` exactly once, then resolves the
//!    promise its handler returned. Settling before `done` makes the runtime fail
//!    the stream ("ended without completing"), which is what we want: a guest
//!    that forgets to close is a bug, not a truncated 200.
//!
//! Errors thrown by the script surface as an `Err` chunk; before the header
//! frame that becomes a 500, after it the body is cut short (and logged).

use std::sync::atomic::{AtomicU64, Ordering};

use rari_error::RariError;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::runtime::JsExecutionRuntime;

static NEXT_STREAM_ID: AtomicU64 = AtomicU64::new(1);

/// Body channel depth. Deep enough that a synchronous render burst rarely
/// awaits the async send path, small enough to bound memory per in-flight page.
const BODY_CHANNEL_CAPACITY: usize = 64;

/// Status + headers the guest reports before the body.
#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct HeaderFrame {
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

/// A guest response whose headers are known and whose body is still streaming
/// out of V8.
#[non_exhaustive]
pub struct GuestStream {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: mpsc::Receiver<Result<Vec<u8>, RariError>>,
}

/// Allocate a stream id for a guest render.
#[must_use]
pub fn next_stream_id() -> String {
    format!("guest-{}", NEXT_STREAM_ID.fetch_add(1, Ordering::Relaxed))
}

/// Run `script` (a JS expression evaluating to a promise, see module docs) as a
/// streaming script and return once the guest reported its header frame.
///
/// The render keeps running on its pool slot after this returns; the caller
/// drains `body` while V8 keeps producing it. Script failures after the header
/// frame close `body` with an `Err`.
///
/// # Errors
///
/// Fails when no pool slot can be acquired, when the script cannot be queued,
/// when the guest's first chunk is not a valid header frame, or when the
/// script fails or ends before reporting one.
pub async fn run_guest_script(
    runtime: &JsExecutionRuntime,
    stream_id: String,
    script_name: &str,
    script: String,
) -> Result<GuestStream, RariError> {
    let (handle, stream_lease) = runtime.pick_runtime_for_streaming().await?;
    let (tx, mut rx) = mpsc::channel::<Result<Vec<u8>, RariError>>(BODY_CHANNEL_CAPACITY);

    let completion = handle
        .queue_script_for_streaming(stream_id.clone(), script_name.to_string(), script, tx, None)
        .await?;

    // Hold the streaming slot for the lifetime of the render so the pool's
    // least-busy picking and health tracking see this request, then surface
    // any late failure in the logs (the body channel carries it to the client).
    tokio::spawn(async move {
        let _stream_lease = stream_lease;
        if let Err(err) = completion.await {
            tracing::error!(stream_id = %stream_id, "guest render failed: {err}");
        }
    });

    match rx.recv().await {
        Some(Ok(frame)) => {
            let frame: HeaderFrame = serde_json::from_slice(&frame).map_err(|err| {
                RariError::js_execution(format!(
                    "guest did not start its stream with a header frame: {err}"
                ))
            })?;
            Ok(GuestStream { status: frame.status, headers: frame.headers, body: rx })
        }
        Some(Err(err)) => Err(err),
        None => Err(RariError::js_execution(
            "guest stream ended before reporting a header frame".to_string(),
        )),
    }
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn header_frame_then_body_then_done() {
        let runtime = Arc::new(JsExecutionRuntime::with_pool_size(None, 1));
        let stream_id = next_stream_id();
        let script = format!(
            r#"(async function() {{
                const id = "{stream_id}";
                await Deno.core.ops.op_fizz_chunk(id, JSON.stringify({{
                    status: 201,
                    headers: [["content-type", "text/plain"], ["x-guest", "yes"]],
                }}));
                await Deno.core.ops.op_fizz_chunk_bytes(id, new TextEncoder().encode("hel"));
                await new Promise((resolve) => setTimeout(resolve, 20));
                await Deno.core.ops.op_fizz_chunk(id, "lo");
                Deno.core.ops.op_fizz_done(id);
            }})()"#
        );

        let mut stream = run_guest_script(&runtime, stream_id, "guest_test", script)
            .await
            .expect("header frame");

        assert_eq!(stream.status, 201);
        assert_eq!(
            stream.headers,
            vec![
                ("content-type".to_string(), "text/plain".to_string()),
                ("x-guest".to_string(), "yes".to_string())
            ]
        );

        let mut body = Vec::new();
        while let Some(chunk) = stream.body.recv().await {
            body.extend(chunk.expect("body chunk"));
        }
        assert_eq!(body, b"hello");
    }

    #[tokio::test]
    async fn script_error_before_headers_is_an_error() {
        let runtime = Arc::new(JsExecutionRuntime::with_pool_size(None, 1));
        let stream_id = next_stream_id();
        let script = r"(async function() { throw new Error('boom before headers') })()".to_string();

        let err = run_guest_script(&runtime, stream_id, "guest_test_err", script)
            .await
            .err()
            .expect("expected an error");
        assert!(err.to_string().contains("boom"), "{err}");
    }

    #[tokio::test]
    async fn settling_without_done_fails_the_body() {
        let runtime = Arc::new(JsExecutionRuntime::with_pool_size(None, 1));
        let stream_id = next_stream_id();
        let script = format!(
            r#"(async function() {{
                await Deno.core.ops.op_fizz_chunk("{stream_id}", JSON.stringify({{ status: 200, headers: [] }}));
                await Deno.core.ops.op_fizz_chunk("{stream_id}", "partial");
            }})()"#
        );

        let mut stream = run_guest_script(&runtime, stream_id, "guest_test_nodone", script)
            .await
            .expect("header frame");
        let mut saw_error = false;
        while let Some(chunk) = stream.body.recv().await {
            if chunk.is_err() {
                saw_error = true;
            }
        }
        assert!(saw_error, "body must end with an error when the guest never calls done");
    }
}
