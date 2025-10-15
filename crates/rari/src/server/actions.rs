use crate::error::RariError;
use crate::server::ServerState;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{debug, error};

#[derive(Debug, Deserialize)]
pub struct ServerActionRequest {
    pub id: String,
    pub export_name: String,
    pub args: Vec<JsonValue>,
}

#[derive(Debug, Serialize)]
pub struct ServerActionResponse {
    pub success: bool,
    pub result: Option<JsonValue>,
    pub error: Option<String>,
    pub redirect: Option<String>,
}

pub async fn handle_server_action(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    body: Bytes,
) -> Result<Response, StatusCode> {
    debug!("Handling server action request");

    let request: ServerActionRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse server action request: {}", e);
            let mut response = Json(ServerActionResponse {
                success: false,
                result: None,
                error: Some("Invalid request format".to_string()),
                redirect: None,
            })
            .into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                "no-store, no-cache, must-revalidate, private"
                    .parse()
                    .expect("Valid cache-control header"),
            );
            return Ok(response);
        }
    };

    debug!("Executing server action: {} (export: {})", request.id, request.export_name);

    let renderer = state.renderer.lock().await;
    let result =
        renderer.execute_server_function(&request.id, &request.export_name, &request.args).await;

    match result {
        Ok(value) => {
            debug!("Server action executed successfully, result: {:?}", value);

            let redirect = extract_redirect_from_result(&value);

            let response =
                ServerActionResponse { success: true, result: Some(value), error: None, redirect };

            debug!("Sending response: {:?}", response);

            let mut response = Json(response).into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                "no-store, no-cache, must-revalidate, private"
                    .parse()
                    .expect("Valid cache-control header"),
            );
            Ok(response)
        }
        Err(e) => {
            error!("Server action execution failed: {}", e);
            let mut response = Json(ServerActionResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
                redirect: None,
            })
            .into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                "no-store, no-cache, must-revalidate, private"
                    .parse()
                    .expect("Valid cache-control header"),
            );
            Ok(response)
        }
    }
}

pub async fn handle_form_action(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, StatusCode> {
    debug!("Handling form-based server action");

    let form_data = match parse_form_data(&body) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse form data: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let action_id = form_data.get("__action_id").ok_or(StatusCode::BAD_REQUEST)?;
    let export_name = form_data.get("__export_name").ok_or(StatusCode::BAD_REQUEST)?;

    let args = convert_form_data_to_args(&form_data);

    debug!("Executing form action: {} (export: {})", action_id, export_name);

    let renderer = state.renderer.lock().await;
    let result = renderer.execute_server_function(action_id, export_name, &args).await;

    match result {
        Ok(value) => {
            if let Some(redirect_url) = extract_redirect_from_result(&value) {
                return Response::builder()
                    .status(StatusCode::SEE_OTHER)
                    .header("Location", redirect_url)
                    .header("Cache-Control", "no-store, no-cache, must-revalidate")
                    .body("".into())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
            }

            let redirect_url = headers.get("referer").and_then(|h| h.to_str().ok()).unwrap_or("/");

            Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header("Location", redirect_url)
                .header("Cache-Control", "no-store, no-cache, must-revalidate")
                .body("".into())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(e) => {
            error!("Form action execution failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn extract_redirect_from_result(result: &JsonValue) -> Option<String> {
    if let Some(redirect) = result.get("redirect") {
        if let Some(url) = redirect.as_str() {
            return Some(url.to_string());
        }
        if let Some(obj) = redirect.as_object()
            && let Some(destination) = obj.get("destination").and_then(|d| d.as_str())
        {
            return Some(destination.to_string());
        }
    }
    None
}

fn parse_form_data(body: &Bytes) -> Result<FxHashMap<String, String>, RariError> {
    let body_str = std::str::from_utf8(body)
        .map_err(|_| RariError::bad_request("Invalid UTF-8 in form data"))?;

    let mut form_data = FxHashMap::default();

    for pair in body_str.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = percent_decode(key)
                .map_err(|_| RariError::bad_request("Invalid URL encoding in form key"))?;
            let value = percent_decode(value)
                .map_err(|_| RariError::bad_request("Invalid URL encoding in form value"))?;
            form_data.insert(key, value);
        }
    }

    Ok(form_data)
}

fn convert_form_data_to_args(form_data: &FxHashMap<String, String>) -> Vec<JsonValue> {
    let mut form_entries = serde_json::Map::new();

    for (key, value) in form_data {
        if key.starts_with("__") {
            continue;
        }
        form_entries.insert(key.clone(), JsonValue::String(value.clone()));
    }

    let form_data_object = JsonValue::Object(form_entries);

    vec![JsonValue::Null, form_data_object]
}

fn percent_decode(input: &str) -> Result<String, RariError> {
    let mut result = String::new();
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex1 =
                chars.next().ok_or_else(|| RariError::bad_request("Invalid percent encoding"))?;
            let hex2 =
                chars.next().ok_or_else(|| RariError::bad_request("Invalid percent encoding"))?;

            let hex_str = format!("{}{}", hex1, hex2);
            let byte = u8::from_str_radix(&hex_str, 16)
                .map_err(|_| RariError::bad_request("Invalid hex in percent encoding"))?;

            result.push(byte as char);
        } else if ch == '+' {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}
