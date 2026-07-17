//! API route error types and response helpers.
//!
//! These types are framework-agnostic and reusable across backends.

use std::{error::Error, fmt, string::ToString};

use axum::{
    body::Body,
    http::{Response, StatusCode},
};
use rari_error::RariError;
use serde::{Deserialize, Serialize};

/// Structured error types for API route handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ApiRouteError {
    NotFound { path: String, message: String },
    MethodNotAllowed { path: String, method: String, allowed_methods: Vec<String>, message: String },
    HandlerError { path: String, method: String, message: String, stack: Option<String> },
    InvalidResponse { path: String, method: String, message: String, details: Option<String> },
    HandlerFileNotFound { path: String, file_path: String, message: String },
    HandlerLoadError { path: String, file_path: String, message: String },
    BodyParseError { path: String, method: String, message: String },
}

impl ApiRouteError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::MethodNotAllowed { .. } => StatusCode::METHOD_NOT_ALLOWED,
            Self::HandlerError { .. }
            | Self::InvalidResponse { .. }
            | Self::HandlerFileNotFound { .. }
            | Self::HandlerLoadError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BodyParseError { .. } => StatusCode::BAD_REQUEST,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "API_ROUTE_NOT_FOUND",
            Self::MethodNotAllowed { .. } => "METHOD_NOT_ALLOWED",
            Self::HandlerError { .. } => "HANDLER_EXECUTION_ERROR",
            Self::InvalidResponse { .. } => "INVALID_RESPONSE",
            Self::HandlerFileNotFound { .. } => "HANDLER_FILE_NOT_FOUND",
            Self::HandlerLoadError { .. } => "HANDLER_LOAD_ERROR",
            Self::BodyParseError { .. } => "BODY_PARSE_ERROR",
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Self::NotFound { path, .. }
            | Self::MethodNotAllowed { path, .. }
            | Self::HandlerError { path, .. }
            | Self::InvalidResponse { path, .. }
            | Self::HandlerFileNotFound { path, .. }
            | Self::HandlerLoadError { path, .. }
            | Self::BodyParseError { path, .. } => path,
        }
    }

    pub fn method(&self) -> Option<&str> {
        match self {
            Self::MethodNotAllowed { method, .. }
            | Self::HandlerError { method, .. }
            | Self::InvalidResponse { method, .. }
            | Self::BodyParseError { method, .. } => Some(method),
            _ => None,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::NotFound { message, .. }
            | Self::MethodNotAllowed { message, .. }
            | Self::HandlerError { message, .. }
            | Self::InvalidResponse { message, .. }
            | Self::HandlerFileNotFound { message, .. }
            | Self::HandlerLoadError { message, .. }
            | Self::BodyParseError { message, .. } => message,
        }
    }

    pub fn allowed_methods(&self) -> Option<&[String]> {
        match self {
            Self::MethodNotAllowed { allowed_methods, .. } => Some(allowed_methods),
            _ => None,
        }
    }

    pub fn stack(&self) -> Option<&str> {
        match self {
            Self::HandlerError { stack, .. } => stack.as_deref(),
            _ => None,
        }
    }
}

impl fmt::Display for ApiRouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path, message } => {
                write!(f, "API route not found at {path}: {message}")
            }
            Self::MethodNotAllowed { path, method, allowed_methods, message } => {
                write!(
                    f,
                    "Method {} not allowed for route {}. Allowed methods: {}. {}",
                    method,
                    path,
                    allowed_methods.join(", "),
                    message
                )
            }
            Self::HandlerError { path, method, message, .. } => {
                write!(f, "Handler error for {method} {path}: {message}")
            }
            Self::InvalidResponse { path, method, message, .. } => {
                write!(f, "Invalid response from {method} {path}: {message}")
            }
            Self::HandlerFileNotFound { path, file_path, message } => {
                write!(f, "Handler file not found for route {path} at {file_path}: {message}")
            }
            Self::HandlerLoadError { path, file_path, message } => {
                write!(f, "Failed to load handler for route {path} from {file_path}: {message}")
            }
            Self::BodyParseError { path, method, message } => {
                write!(f, "Failed to parse request body for {method} {path}: {message}")
            }
        }
    }
}

impl Error for ApiRouteError {}

impl From<ApiRouteError> for RariError {
    fn from(error: ApiRouteError) -> Self {
        match error {
            ApiRouteError::NotFound { message, .. } => {
                Self::not_found(message).with_property("error_type", "api_route_not_found")
            }
            ApiRouteError::MethodNotAllowed { message, allowed_methods, .. } => {
                Self::bad_request(message)
                    .with_property("error_type", "method_not_allowed")
                    .with_property("allowed_methods", &allowed_methods.join(","))
            }
            ApiRouteError::HandlerError { message, .. } => {
                Self::js_execution(message).with_property("error_type", "handler_error")
            }
            ApiRouteError::InvalidResponse { message, .. } => {
                Self::internal(message).with_property("error_type", "invalid_response")
            }
            ApiRouteError::HandlerFileNotFound { message, .. } => {
                Self::not_found(message).with_property("error_type", "handler_file_not_found")
            }
            ApiRouteError::HandlerLoadError { message, .. } => {
                Self::internal(message).with_property("error_type", "handler_load_error")
            }
            ApiRouteError::BodyParseError { message, .. } => {
                Self::bad_request(message).with_property("error_type", "body_parse_error")
            }
        }
    }
}

/// JSON body for error responses returned to clients.
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiRouteError {
    pub fn to_json_response(&self, is_development: bool) -> ErrorResponse {
        if is_development {
            ErrorResponse {
                error: self.status_code().canonical_reason().unwrap_or("Error").to_string(),
                message: self.message().to_string(),
                code: Some(self.error_code().to_string()),
                route: Some(self.path().to_string()),
                method: self.method().map(ToString::to_string),
                stack: self.stack().map(ToString::to_string),
                details: self.get_error_details(),
            }
        } else {
            let generic_message = match self {
                Self::NotFound { .. } => "The requested resource was not found",
                Self::MethodNotAllowed { .. } => "Method not allowed for this resource",
                Self::BodyParseError { .. } => "Invalid request body",
                _ => "An error occurred processing your request",
            };

            ErrorResponse {
                error: self.status_code().canonical_reason().unwrap_or("Error").to_string(),
                message: generic_message.to_string(),
                code: None,
                route: None,
                method: None,
                stack: None,
                details: None,
            }
        }
    }

    fn get_error_details(&self) -> Option<serde_json::Value> {
        match self {
            Self::MethodNotAllowed { allowed_methods, .. } => Some(serde_json::json!({
                "allowed_methods": allowed_methods
            })),
            Self::InvalidResponse { details, .. } => {
                details.as_ref().and_then(|d| serde_json::from_str(d).ok())
            }
            Self::HandlerFileNotFound { file_path, .. }
            | Self::HandlerLoadError { file_path, .. } => Some(serde_json::json!({
                "file_path": file_path
            })),
            _ => None,
        }
    }

    pub fn to_json_string(&self, is_development: bool) -> String {
        let response = self.to_json_response(is_development);
        serde_json::to_string(&response).unwrap_or_else(|_| {
            r#"{"error":"Internal Server Error","message":"Failed to serialize error response"}"#
                .to_string()
        })
    }

    pub fn to_http_response(&self, is_development: bool) -> Response<Body> {
        let status = self.status_code();
        let body = self.to_json_string(is_development);

        let mut builder =
            Response::builder().status(status).header("content-type", "application/json");

        if let Self::MethodNotAllowed { allowed_methods, .. } = self {
            let allow_header = allowed_methods.join(", ");
            builder = builder.header("allow", allow_header);
        }

        #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
        builder.body(Body::from(body)).expect("Valid error response")
    }
}

/// Build a generic HTTP error response (no structured error type).
pub fn create_generic_error_response(
    status: StatusCode,
    message: &str,
    is_development: bool,
) -> Response<Body> {
    let response = if is_development {
        ErrorResponse {
            error: status.canonical_reason().unwrap_or("Error").to_string(),
            message: message.to_string(),
            code: None,
            route: None,
            method: None,
            stack: None,
            details: None,
        }
    } else {
        ErrorResponse {
            error: status.canonical_reason().unwrap_or("Error").to_string(),
            message: "An error occurred processing your request".to_string(),
            code: None,
            route: None,
            method: None,
            stack: None,
            details: None,
        }
    };

    let body = serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"error":"Internal Server Error","message":"Failed to serialize error response"}"#
            .to_string()
    });

    #[expect(clippy::expect_used, reason = "Response::builder() with valid components never fails")]
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .expect("Valid error response")
}
