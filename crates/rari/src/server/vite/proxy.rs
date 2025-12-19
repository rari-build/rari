use crate::error::RariError;
use crate::server::config::Config;
use axum::{
    body::Body,
    extract::{
        Path, Query,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use futures::StreamExt as FuturesStreamExt;
use futures_util::SinkExt;
use reqwest::Client;
use rustc_hash::FxHashMap;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, warn};
use tungstenite::{client::IntoClientRequest, http::Request as HttpRequest};

const VITE_WS_PROTOCOL: &str = "vite-hmr";

pub async fn vite_src_proxy(
    Path(path): Path<String>,
    query: Query<FxHashMap<String, String>>,
) -> impl IntoResponse {
    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for Vite proxy");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration not available",
            );
        }
    };

    let client = Client::new();
    let vite_base_url = format!("http://{}", config.vite_address());

    let query_string = if query.0.is_empty() {
        String::new()
    } else {
        let query_params = query
            .0
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        format!("?{query_params}")
    };

    let target_url = format!("{vite_base_url}/src/{path}{query_string}");

    match client.get(&target_url).send().await {
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
                    error!("Failed to build proxy response: {}", e);
                    create_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to build response",
                    )
                }
            }
        }
        Err(e) => {
            warn!("Failed to proxy src request to Vite server: {}", e);

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

pub async fn vite_reverse_proxy(
    Path(path): Path<String>,
    query: Query<FxHashMap<String, String>>,
) -> impl IntoResponse {
    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for Vite proxy");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration not available",
            );
        }
    };

    let client = Client::new();
    let vite_base_url = format!("http://{}", config.vite_address());

    let query_string = if query.0.is_empty() {
        String::new()
    } else {
        let query_params = query
            .0
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        format!("?{query_params}")
    };

    let target_url = format!("{vite_base_url}/vite-server/{path}{query_string}");

    match client.get(&target_url).send().await {
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
                    error!("Failed to build proxy response: {}", e);
                    create_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to build response",
                    )
                }
            }
        }
        Err(e) => {
            warn!("Failed to proxy request to Vite server: {}", e);

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

pub async fn vite_websocket_proxy(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.protocols([VITE_WS_PROTOCOL]).on_upgrade(handle_websocket)
}

async fn handle_websocket(mut client_socket: WebSocket) {
    if let Err(e) = client_socket.send(WsMessage::Ping("rari-vite-proxy".into())).await {
        error!("Failed to send initial ping to client: {}", e);
        return;
    }

    let config = match Config::get() {
        Some(config) => config,
        None => {
            error!("Failed to get global configuration for WebSocket proxy");
            let _ = client_socket.send(WsMessage::Close(None)).await;
            return;
        }
    };

    let vite_ws_url = format!("ws://{}/vite-server/", config.vite_address());

    let vite_ws_request = match HttpRequest::builder()
        .uri(&vite_ws_url)
        .header("Sec-WebSocket-Protocol", VITE_WS_PROTOCOL)
        .body(())
        .expect("Valid HTTP request builder")
        .into_client_request()
    {
        Ok(request) => request,
        Err(e) => {
            error!("Failed to create Vite WebSocket request: {}", e);
            let _ = client_socket.send(WsMessage::Close(None)).await;
            return;
        }
    };

    let vite_socket = match connect_async(vite_ws_request).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            error!("Failed to connect to Vite WebSocket server: {}", e);

            #[allow(clippy::disallowed_methods)]
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

    let client_to_vite = tokio::spawn(async move {
        while let Some(msg) = client_receiver.next().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(_) => {
                    break;
                }
            };

            let vite_msg = match convert_axum_to_tungstenite_message(msg) {
                Some(msg) => msg,
                None => continue,
            };

            if vite_sender.send(vite_msg).await.is_err() {
                break;
            }
        }
    });

    let vite_to_client = tokio::spawn(async move {
        while let Some(msg) = vite_receiver.next().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(_) => {
                    break;
                }
            };

            let client_msg = match convert_tungstenite_to_axum_message(msg) {
                Some(msg) => msg,
                None => continue,
            };

            if client_sender.send(client_msg).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = client_to_vite => {}
        _ = vite_to_client => {}
    }
}

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
    #[allow(clippy::disallowed_methods)]
    let error_body = serde_json::json!({
        "error": message,
        "status": status.as_u16()
    });

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(error_body.to_string()))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal server error"))
                .expect("Valid fallback error response")
        })
}

pub async fn check_vite_server_health() -> Result<(), RariError> {
    let config = Config::get().ok_or_else(|| {
        RariError::configuration("Global configuration not available".to_string())
    })?;

    let client = Client::new();
    let health_url = format!("http://{}/vite-server/", config.vite_address());

    match client.get(&health_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                Ok(())
            } else {
                Err(RariError::network(format!(
                    "Vite server returned status: {}",
                    response.status()
                )))
            }
        }
        Err(e) => Err(RariError::network(format!(
            "Failed to connect to Vite server at {}: {}",
            config.vite_address(),
            e
        ))),
    }
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
