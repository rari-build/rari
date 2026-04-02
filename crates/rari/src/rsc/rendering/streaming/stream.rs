use futures::Stream;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::types::RscStreamChunk;

pub struct RscStream {
    receiver: mpsc::Receiver<RscStreamChunk>,
    _request_context_guard:
        Option<std::sync::Arc<crate::server::middleware::request_context::RequestContext>>,
    runtime_for_cleanup: Option<Arc<crate::runtime::JsExecutionRuntime>>,
}

impl RscStream {
    pub fn new(receiver: mpsc::Receiver<RscStreamChunk>) -> Self {
        Self { receiver, _request_context_guard: None, runtime_for_cleanup: None }
    }

    pub fn with_request_context(
        mut self,
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
        runtime: Arc<crate::runtime::JsExecutionRuntime>,
    ) -> Self {
        self._request_context_guard = Some(request_context);
        self.runtime_for_cleanup = Some(runtime);
        self
    }

    pub async fn next_chunk(&mut self) -> Option<RscStreamChunk> {
        self.receiver.recv().await
    }

    pub fn is_complete(&self) -> bool {
        self.receiver.is_closed()
    }
}

impl Stream for RscStream {
    type Item = Result<Vec<u8>, String>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(chunk.data))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for RscStream {
    fn drop(&mut self) {
        if let Some(runtime) = &self.runtime_for_cleanup {
            let runtime = Arc::clone(runtime);
            tokio::spawn(async move {
                if let Err(e) = runtime.clear_request_context().await {
                    tracing::error!("Failed to clear request context on stream drop: {}", e);
                }
            });
        }
    }
}
