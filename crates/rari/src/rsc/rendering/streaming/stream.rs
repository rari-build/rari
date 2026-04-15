use futures::Stream;
use tokio::sync::mpsc;

use super::types::RscStreamChunk;

type CleanupCallback = Box<dyn FnOnce() + Send + 'static>;

pub struct RscStream {
    receiver: mpsc::Receiver<RscStreamChunk>,
    _request_context_guard:
        Option<std::sync::Arc<crate::server::middleware::request_context::RequestContext>>,
    cleanup: Option<CleanupCallback>,
}

impl RscStream {
    pub fn new(receiver: mpsc::Receiver<RscStreamChunk>) -> Self {
        Self { receiver, _request_context_guard: None, cleanup: None }
    }

    pub fn with_request_context(
        mut self,
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
    ) -> Self {
        self._request_context_guard = Some(request_context);
        self
    }

    pub fn with_cleanup<F>(mut self, cleanup: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(existing_cleanup) = self.cleanup.take() {
            self.cleanup = Some(Box::new(move || {
                existing_cleanup();
                cleanup();
            }));
        } else {
            self.cleanup = Some(Box::new(cleanup));
        }
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
        if let Some(cleanup) = self.cleanup.take()
            && let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(cleanup))
        {
            tracing::error!("RscStream cleanup callback panicked: {:?}", e);
        }
    }
}
