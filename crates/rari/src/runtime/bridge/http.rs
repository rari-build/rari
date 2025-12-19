use crate::error::RariError;
use axum::body::Body;
use axum::http::{HeaderMap, Response, StatusCode};
use rustc_hash::FxHashMap;
use serde_json::{Value as JsonValue, json};
use tracing::warn;

pub struct RequestBridge;

impl RequestBridge {
    pub fn to_json(
        method: &str,
        uri: &str,
        headers: &HeaderMap,
        body: &str,
        params: &FxHashMap<String, String>,
    ) -> Result<JsonValue, RariError> {
        let mut headers_map = FxHashMap::default();
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                headers_map.insert(name.to_string(), value_str.to_string());
            } else {
                warn!("Skipping non-UTF8 header: {}", name);
            }
        }

        #[allow(clippy::disallowed_methods)]
        let request_obj = json!({
            "method": method,
            "url": uri,
            "headers": headers_map,
            "body": body,
            "params": params,
        });

        Ok(request_obj)
    }

    pub fn from_json(result: JsonValue) -> Result<Response<Body>, RariError> {
        if result.is_object() && result.get("status").is_some() {
            Self::from_response_object(result)
        } else {
            Self::from_plain_value(result)
        }
    }

    fn from_response_object(result: JsonValue) -> Result<Response<Body>, RariError> {
        let status = result.get("status").and_then(|v| v.as_u64()).unwrap_or(200) as u16;

        let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let body_str = result.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let mut response = Response::builder().status(status_code);

        if let Some(headers_obj) = result.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in headers_obj {
                if let Some(value_str) = value.as_str() {
                    response = response.header(key, value_str);
                }
            }
        }

        let response = response
            .body(Body::from(body_str))
            .map_err(|e| RariError::internal(format!("Failed to build response: {e}")))?;

        Ok(response)
    }

    fn from_plain_value(result: JsonValue) -> Result<Response<Body>, RariError> {
        let body = serde_json::to_string(&result)
            .map_err(|e| RariError::serialization(format!("Failed to serialize response: {e}")))?;

        let response = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .map_err(|e| RariError::internal(format!("Failed to build response: {e}")))?;

        Ok(response)
    }

    pub fn headers_to_json(headers: &HeaderMap) -> FxHashMap<String, String> {
        let mut headers_map = FxHashMap::default();
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                headers_map.insert(name.to_string(), value_str.to_string());
            }
        }
        headers_map
    }

    pub fn get_content_type(headers: &HeaderMap) -> Option<String> {
        headers.get("content-type").and_then(|v| v.to_str().ok()).map(|s| s.to_string())
    }

    pub fn is_json_content_type(headers: &HeaderMap) -> bool {
        Self::get_content_type(headers).map(|ct| ct.contains("application/json")).unwrap_or(false)
    }

    pub fn is_form_content_type(headers: &HeaderMap) -> bool {
        Self::get_content_type(headers)
            .map(|ct| {
                ct.contains("application/x-www-form-urlencoded")
                    || ct.contains("multipart/form-data")
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_to_json_basic() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("user-agent", HeaderValue::from_static("test-agent"));

        let params = FxHashMap::default();

        let result = RequestBridge::to_json("GET", "/api/test", &headers, "", &params).unwrap();

        assert_eq!(result["method"], "GET");
        assert_eq!(result["url"], "/api/test");
        assert_eq!(result["headers"]["content-type"], "application/json");
        assert_eq!(result["headers"]["user-agent"], "test-agent");
    }

    #[test]
    fn test_to_json_with_params() {
        let headers = HeaderMap::new();
        let mut params = FxHashMap::default();
        params.insert("id".to_string(), "123".to_string());
        params.insert("name".to_string(), "test".to_string());

        let result =
            RequestBridge::to_json("GET", "/api/users/123", &headers, "", &params).unwrap();

        assert_eq!(result["params"]["id"], "123");
        assert_eq!(result["params"]["name"], "test");
    }

    #[test]
    fn test_from_response_object() {
        let response_json = json!({
            "status": 201,
            "headers": {
                "content-type": "application/json",
                "x-custom": "value"
            },
            "body": r#"{"success":true}"#
        });

        let response = RequestBridge::from_json(response_json).unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
        assert_eq!(response.headers().get("x-custom").unwrap(), "value");
    }

    #[test]
    fn test_from_plain_value() {
        let plain_json = json!({
            "message": "Hello",
            "count": 42
        });

        let response = RequestBridge::from_json(plain_json).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_is_json_content_type() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        assert!(RequestBridge::is_json_content_type(&headers));

        headers.insert("content-type", HeaderValue::from_static("application/json; charset=utf-8"));
        assert!(RequestBridge::is_json_content_type(&headers));

        headers.insert("content-type", HeaderValue::from_static("text/plain"));
        assert!(!RequestBridge::is_json_content_type(&headers));
    }

    #[test]
    fn test_is_form_content_type() {
        let mut headers = HeaderMap::new();
        headers
            .insert("content-type", HeaderValue::from_static("application/x-www-form-urlencoded"));
        assert!(RequestBridge::is_form_content_type(&headers));

        headers.insert("content-type", HeaderValue::from_static("multipart/form-data"));
        assert!(RequestBridge::is_form_content_type(&headers));

        headers.insert("content-type", HeaderValue::from_static("application/json"));
        assert!(!RequestBridge::is_form_content_type(&headers));
    }
}
