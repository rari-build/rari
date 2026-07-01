use std::{
    panic,
    pin::Pin,
    string::String,
    sync::Arc,
    task::{Context, Poll},
};

use futures::Stream;
use tokio::sync::mpsc;

use super::types::RscStreamChunk;
use crate::server::middleware::request_context::RequestContext;

type CleanupCallback = Box<dyn FnOnce() + Send + 'static>;

pub struct RscStream {
    receiver: mpsc::Receiver<RscStreamChunk>,
    request_context_guard: Option<Arc<RequestContext>>,
    cleanup: Option<CleanupCallback>,
}

impl RscStream {
    pub fn new(receiver: mpsc::Receiver<RscStreamChunk>) -> Self {
        Self { receiver, request_context_guard: None, cleanup: None }
    }

    #[must_use]
    pub fn with_request_context(mut self, request_context: Arc<RequestContext>) -> Self {
        self.request_context_guard = Some(request_context);
        self
    }

    #[must_use]
    pub fn with_cleanup<F>(mut self, cleanup: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(existing_cleanup) = self.cleanup.take() {
            self.cleanup = Some(Box::new(move || {
                if let Err(e) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    existing_cleanup();
                })) {
                    tracing::error!(
                        "Panic in existing cleanup handler: {:?}",
                        e.downcast_ref::<&str>()
                            .copied()
                            .or_else(|| e.downcast_ref::<String>().map(String::as_str))
                            .unwrap_or("unknown panic")
                    );
                }

                if let Err(e) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    cleanup();
                })) {
                    tracing::error!(
                        "Panic in cleanup handler: {:?}",
                        e.downcast_ref::<&str>()
                            .copied()
                            .or_else(|| e.downcast_ref::<String>().map(String::as_str))
                            .unwrap_or("unknown panic")
                    );
                }
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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
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
            && let Err(e) = panic::catch_unwind(panic::AssertUnwindSafe(cleanup))
        {
            tracing::error!("RscStream cleanup callback panicked: {:?}", e);
        }
    }
}
