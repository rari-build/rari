use axum::body::Body;
use axum::response::{IntoResponse, Response};
use futures::{Stream, StreamExt};
use std::pin::Pin;

use crate::error::RariError;
use tracing::warn;

pub struct StreamingHtmlResponse {
    stream: Pin<Box<dyn Stream<Item = Result<Vec<u8>, RariError>> + Send>>,
}

impl StreamingHtmlResponse {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Vec<u8>, RariError>> + Send + 'static,
    {
        Self { stream: Box::pin(stream) }
    }
}

impl IntoResponse for StreamingHtmlResponse {
    fn into_response(self) -> Response {
        let client_connected = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let client_connected_clone = client_connected.clone();

        let stream = self.stream.map(move |chunk| {
            if !client_connected_clone.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Client disconnected",
                ));
            }

            match chunk {
                Ok(bytes) => Ok(bytes::Bytes::from(bytes)),
                Err(e) => {
                    if e.to_string().contains("disconnected")
                        || e.to_string().contains("broken pipe")
                    {
                        warn!("Client disconnected during streaming: {}", e);
                        client_connected_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                    }

                    Err(std::io::Error::other(e.to_string()))
                }
            }
        });

        Response::builder()
            .status(200)
            .header("content-type", "text/html; charset=utf-8")
            .header("transfer-encoding", "chunked")
            .header("x-content-type-options", "nosniff")
            .header("x-accel-buffering", "no")
            .header("cache-control", "no-cache")
            .body(Body::from_stream(stream))
            .expect("Failed to build streaming response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_stream::stream;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn test_streaming_response_basic() {
        let html_stream = stream! {
            yield Ok(b"<!DOCTYPE html>".to_vec());
            yield Ok(b"<html><body>".to_vec());
            yield Ok(b"<h1>Hello</h1>".to_vec());
            yield Ok(b"</body></html>".to_vec());
        };

        let response = StreamingHtmlResponse::new(html_stream).into_response();

        assert_eq!(response.status(), 200);

        assert_eq!(response.headers().get("content-type").unwrap(), "text/html; charset=utf-8");
        assert_eq!(response.headers().get("transfer-encoding").unwrap(), "chunked");
        assert_eq!(response.headers().get("x-content-type-options").unwrap(), "nosniff");
        assert_eq!(response.headers().get("x-accel-buffering").unwrap(), "no");
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-cache");

        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body_str, "<!DOCTYPE html><html><body><h1>Hello</h1></body></html>");
    }

    #[tokio::test]
    async fn test_streaming_response_with_error() {
        let html_stream = stream! {
            yield Ok(b"<!DOCTYPE html>".to_vec());
            yield Err(RariError::internal("Stream error"));
        };

        let response = StreamingHtmlResponse::new(html_stream).into_response();

        assert_eq!(response.status(), 200);

        let result = to_bytes(response.into_body(), usize::MAX).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_streaming_response_empty() {
        let html_stream = stream! {
            if false {
                yield Ok::<Vec<u8>, RariError>(vec![]);
            }
        };

        let response = StreamingHtmlResponse::new(html_stream).into_response();

        assert_eq!(response.status(), 200);

        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body_bytes.len(), 0);
    }

    #[tokio::test]
    async fn test_streaming_response_headers() {
        let html_stream = stream! {
            yield Ok(b"<html></html>".to_vec());
        };

        let response = StreamingHtmlResponse::new(html_stream).into_response();

        let headers = response.headers();
        assert!(headers.contains_key("content-type"));
        assert!(headers.contains_key("transfer-encoding"));
        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-accel-buffering"));
        assert!(headers.contains_key("cache-control"));
    }
}
