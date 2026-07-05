use std::time::Duration;

pub mod hmr;

use axum::{
    body,
    body::Body,
    extract::{
        Request,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderName, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use futures::StreamExt as FuturesStreamExt;
use futures_util::SinkExt;
use http::uri::PathAndQuery;
use rari_error::RariError;
use reqwest::Client;
use tokio::time;
use tokio_tungstenite::tungstenite::Message;
use tungstenite::{client::IntoClientRequest, http::Request as HttpRequest};

use crate::server::config::Config;

const VITE_WS_PROTOCOL: &str = "vite-hmr";

fn create_client() -> Client {
    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
}

pub async fn vite_src_proxy(req: Request) -> impl IntoResponse {
    let Some(config) = Config::get() else {
        tracing::error!("Failed to get global configuration for Vite proxy");
        return create_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Configuration not available",
        );
    };

    let client = create_client();
    let vite_base_url = format!("http://{}", config.vite_address());

    let path_and_query =
        req.uri().path_and_query().map(PathAndQuery::as_str).unwrap_or(req.uri().path());

    let path_without_prefix = path_and_query.strip_prefix("/src").unwrap_or(path_and_query);
    let target_url = format!("{vite_base_url}/src{path_without_prefix}");

    let method = req.method().clone();
    let headers = req.headers().clone();

    let body_bytes = match body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read request body",
            );
        }
    };

    match client.request(method, &target_url).headers(headers).body(body_bytes).send().await {
        Ok(response) => {
            let status = response.status();
            let mut response_builder = Response::builder().status(status);

            if let Some(headers) = response_builder.headers_mut() {
                for (name, value) in response.headers() {
                    if let (Ok(name), Ok(value)) = (
                        HeaderName::from_bytes(name.as_ref()),
                        HeaderValue::from_bytes(value.as_ref()),
                    ) {
                        headers.insert(name, value);
                    }
                }
            }

            match response_builder.body(Body::from_stream(response.bytes_stream())) {
                Ok(response) => response,
                Err(e) => {
                    tracing::error!("Failed to build proxy response: {}", e);
                    create_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to build response",
                    )
                }
            }
        }
        Err(e) => {
            if e.is_connect() {
                create_error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!(
                        "Vite development server is not running on {vite_base_url}. Please start your Vite dev server."
                    ),
                )
            } else {
                create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Proxy error: {e}"),
                )
            }
        }
    }
}

pub async fn vite_reverse_proxy(req: Request) -> impl IntoResponse {
    let Some(config) = Config::get() else {
        tracing::error!("Failed to get global configuration for Vite proxy");
        return create_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Configuration not available",
        );
    };

    let client = create_client();
    let vite_base_url = format!("http://{}", config.vite_address());

    let path_and_query =
        req.uri().path_and_query().map(PathAndQuery::as_str).unwrap_or(req.uri().path());

    let path_without_prefix = path_and_query.strip_prefix("/vite-server").unwrap_or(path_and_query);
    let target_url = format!("{vite_base_url}/vite-server{path_without_prefix}");

    let method = req.method().clone();
    let headers = req.headers().clone();

    let body_bytes = match body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read request body",
            );
        }
    };

    match client.request(method, &target_url).headers(headers).body(body_bytes).send().await {
        Ok(response) => {
            let status = response.status();
            let mut response_builder = Response::builder().status(status);

            if let Some(headers) = response_builder.headers_mut() {
                for (name, value) in response.headers() {
                    if let (Ok(name), Ok(value)) = (
                        HeaderName::from_bytes(name.as_ref()),
                        HeaderValue::from_bytes(value.as_ref()),
                    ) {
                        headers.insert(name, value);
                    }
                }
            }

            match response_builder.body(Body::from_stream(response.bytes_stream())) {
                Ok(response) => response,
                Err(e) => {
                    tracing::error!("Failed to build proxy response: {}", e);
                    create_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to build response",
                    )
                }
            }
        }
        Err(e) => {
            if e.is_connect() {
                create_error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!(
                        "Vite development server is not running on {vite_base_url}. Please start your Vite dev server."
                    ),
                )
            } else {
                create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Proxy error: {e}"),
                )
            }
        }
    }
}

pub async fn vite_websocket_proxy(ws: WebSocketUpgrade, uri: Uri) -> impl IntoResponse {
    ws.protocols([VITE_WS_PROTOCOL]).on_upgrade(move |socket| handle_websocket(socket, uri))
}

async fn handle_websocket(mut client_socket: WebSocket, uri: Uri) {
    if let Err(e) = client_socket.send(WsMessage::Ping("rari-vite-proxy".into())).await {
        tracing::error!("Failed to send initial ping to client: {}", e);
        return;
    }

    let Some(config) = Config::get() else {
        tracing::error!("Failed to get global configuration for WebSocket proxy");
        let _ = client_socket.send(WsMessage::Close(None)).await;
        return;
    };

    let path_and_query = uri.path_and_query().map(PathAndQuery::as_str).unwrap_or("/");
    let path_without_prefix = path_and_query.strip_prefix("/vite-server").unwrap_or(path_and_query);
    let vite_ws_url = format!("ws://{}/vite-server{}", config.vite_address(), path_without_prefix);

    #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
    let vite_ws_request = match HttpRequest::builder()
        .uri(&vite_ws_url)
        .header("Sec-WebSocket-Protocol", VITE_WS_PROTOCOL)
        .body(())
        .expect("Valid HTTP request builder")
        .into_client_request()
    {
        Ok(request) => request,
        Err(e) => {
            tracing::error!("Failed to create Vite WebSocket request: {}", e);
            let _ = client_socket.send(WsMessage::Close(None)).await;
            return;
        }
    };

    let vite_socket = match tokio_tungstenite::connect_async(vite_ws_request).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            tracing::error!("Failed to connect to Vite WebSocket server: {}", e);

            let error_msg = serde_json::json!({
                "type": "error",
                "message": format!("Failed to connect to Vite dev server: {}", e)
            });

            if let Ok(error_text) = serde_json::to_string(&error_msg) {
                let _ = client_socket.send(WsMessage::Text(error_text.into())).await;
            }

            let _ = client_socket.send(WsMessage::Close(None)).await;
            return;
        }
    };

    let (mut vite_sender, mut vite_receiver) = vite_socket.split();
    let (mut client_sender, mut client_receiver) = client_socket.split();

    let mut client_to_vite = tokio::spawn(async move {
        while let Some(msg) = client_receiver.next().await {
            let Ok(msg) = msg else {
                break;
            };

            let Some(vite_msg) = convert_axum_to_tungstenite_message(msg) else {
                continue;
            };

            if vite_sender.send(vite_msg).await.is_err() {
                break;
            }
        }
    });

    let mut vite_to_client = tokio::spawn(async move {
        while let Some(msg) = vite_receiver.next().await {
            let Ok(msg) = msg else {
                break;
            };

            let Some(client_msg) = convert_tungstenite_to_axum_message(msg) else {
                continue;
            };

            if client_sender.send(client_msg).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut client_to_vite => {
            vite_to_client.abort();
        }
        _ = &mut vite_to_client => {
            client_to_vite.abort();
        }
    }
}

#[expect(clippy::unnecessary_wraps, reason = "Option return type maintains API consistency")]
fn convert_axum_to_tungstenite_message(msg: WsMessage) -> Option<Message> {
    match msg {
        WsMessage::Text(text) => Some(Message::Text(text.to_string().into())),
        WsMessage::Binary(data) => Some(Message::Binary(data)),
        WsMessage::Ping(data) => Some(Message::Ping(data)),
        WsMessage::Pong(data) => Some(Message::Pong(data)),
        WsMessage::Close(_) => Some(Message::Close(None)),
    }
}

fn convert_tungstenite_to_axum_message(msg: Message) -> Option<WsMessage> {
    match msg {
        Message::Text(text) => Some(WsMessage::Text(text.to_string().into())),
        Message::Binary(data) => Some(WsMessage::Binary(data)),
        Message::Ping(data) => Some(WsMessage::Ping(data)),
        Message::Pong(data) => Some(WsMessage::Pong(data)),
        Message::Close(_) => Some(WsMessage::Close(None)),
        Message::Frame(_) => None,
    }
}

fn create_error_response(status: StatusCode, message: &str) -> Response<Body> {
    let error_body = serde_json::json!({
        "error": message,
        "status": status.as_u16()
    });

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(error_body.to_string()))
        .unwrap_or_else(|_| {
            #[expect(
                clippy::expect_used,
                reason = "Response::builder() with valid components never fails"
            )]
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal server error"))
                .expect("Valid fallback error response")
        })
}

#[expect(clippy::missing_errors_doc)]
pub async fn check_vite_server_health() -> Result<(), RariError> {
    let config = Config::get().ok_or_else(|| {
        RariError::configuration("Global configuration not available".to_string())
    })?;

    let client = Client::new();
    let health_url = format!("http://{}/vite-server/", config.vite_address());

    let mut last_error: Option<String> = None;
    let mut last_status: Option<reqwest::StatusCode> = None;

    for attempt in 1..=60 {
        match client.get(&health_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(());
                }
                last_status = Some(response.status());
                last_error = None;
            }
            Err(e) => {
                last_error = Some(e.to_string());
                last_status = None;
            }
        }

        if attempt < 60 {
            time::sleep(time::Duration::from_millis(100)).await;
        }
    }

    let error_detail = if let Some(status) = last_status {
        format!("health check failed with status {status}")
    } else if let Some(error) = last_error {
        format!("connection error: {error}")
    } else {
        "health check failed".to_string()
    };

    Err(RariError::network(format!(
        "Failed to connect to Vite server at {} after 60 attempts ({})",
        config.vite_address(),
        error_detail
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_conversion() {
        let axum_msg = WsMessage::Text("test".to_string().into());
        let tungstenite_msg = convert_axum_to_tungstenite_message(axum_msg);
        assert!(matches!(tungstenite_msg, Some(Message::Text(_))));

        let tungstenite_msg = Message::Text("test".to_string().into());
        let axum_msg = convert_tungstenite_to_axum_message(tungstenite_msg);
        assert!(matches!(axum_msg, Some(WsMessage::Text(_))));
    }

    #[test]
    fn test_error_response_creation() {
        let response = create_error_response(StatusCode::NOT_FOUND, "Test error");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
