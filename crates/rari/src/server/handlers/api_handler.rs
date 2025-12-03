use crate::server::ServerState;
use crate::server::handlers::static_handlers::cors_preflight_response;
use crate::server::utils::http_utils::{add_api_cors_headers, add_api_security_headers};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, StatusCode},
    response::Response,
};
use tracing::{debug, error, info};

#[axum::debug_handler]
pub async fn api_cors_preflight(
    State(state): State<ServerState>,
    req: axum::http::Request<Body>,
) -> Response {
    let path = req.uri().path();

    if let Some(api_handler) = &state.api_route_handler
        && let Some(methods) = api_handler.get_supported_methods(path)
    {
        let mut builder = Response::builder().status(StatusCode::NO_CONTENT);
        let headers = builder.headers_mut().expect("Response builder should have headers");

        headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));

        let mut all_methods = methods.clone();
        if !all_methods.contains(&"OPTIONS".to_string()) {
            all_methods.push("OPTIONS".to_string());
        }
        let methods_str = all_methods.join(", ");

        if let Ok(methods_value) = HeaderValue::from_str(&methods_str) {
            headers.insert("Access-Control-Allow-Methods", methods_value);
        }

        headers.insert(
                "Access-Control-Allow-Headers",
                HeaderValue::from_static(
                    "Content-Type, Authorization, Accept, Origin, X-Requested-With, Cache-Control, X-RSC-Streaming",
                ),
            );
        headers.insert("Access-Control-Max-Age", HeaderValue::from_static("86400"));

        debug!(
            path = %path,
            allowed_methods = %methods_str,
            "Returning CORS preflight response for API route"
        );

        return builder.body(Body::empty()).expect("Valid preflight response");
    }

    cors_preflight_response()
}

#[axum::debug_handler]
pub async fn handle_api_route(
    State(state): State<ServerState>,
    req: axum::http::Request<Body>,
) -> Result<axum::http::Response<Body>, StatusCode> {
    use crate::server::api_error::{ApiRouteError, create_generic_error_response};

    let path = req.uri().path().to_string();
    let method = req.method().to_string();
    let is_development = state.config.is_development();

    debug!(
        path = %path,
        method = %method,
        "Received API route request"
    );

    let api_handler = match &state.api_route_handler {
        Some(handler) => handler,
        None => {
            debug!("No API route handler available");
            return Ok(create_generic_error_response(
                StatusCode::NOT_FOUND,
                "API routes not configured",
                is_development,
            ));
        }
    };

    let route_match = match api_handler.match_route(&path, &method) {
        Ok(m) => m,
        Err(e) => {
            if let Some(error_type) = e.get_property("error_type")
                && error_type == "method_not_allowed"
            {
                let allowed_methods = e
                    .get_property("allowed_methods")
                    .unwrap_or("")
                    .split(',')
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();

                let api_error = ApiRouteError::MethodNotAllowed {
                    path: path.to_string(),
                    method: method.to_string(),
                    allowed_methods: allowed_methods.clone(),
                    message: e.message(),
                };

                info!(
                    path = %path,
                    method = %method,
                    allowed_methods = ?allowed_methods,
                    "Method not allowed for API route"
                );

                let mut response = api_error.to_http_response(is_development);
                if is_development {
                    add_api_cors_headers(response.headers_mut());
                }
                return Ok(response);
            }

            debug!(
                path = %path,
                method = %method,
                error = %e,
                "No matching API route found"
            );

            let api_error = ApiRouteError::NotFound {
                path: path.to_string(),
                message: format!("No API route found for path: {}", path),
            };

            let mut response = api_error.to_http_response(is_development);
            if is_development {
                add_api_cors_headers(response.headers_mut());
            }
            return Ok(response);
        }
    };

    debug!(
        request_path = %path,
        route_path = %route_match.route.path,
        method = %method,
        "Matched API route"
    );

    match api_handler.execute_handler(&route_match, req, is_development).await {
        Ok(mut response) => {
            debug!(
                route_path = %route_match.route.path,
                method = %method,
                status = response.status().as_u16(),
                "API route handler executed successfully"
            );

            let headers = response.headers_mut();
            if is_development {
                add_api_cors_headers(headers);
            } else {
                add_api_security_headers(headers);
            }

            Ok(response)
        }
        Err(e) => {
            error!(
                route_path = %route_match.route.path,
                method = %method,
                error = %e,
                error_code = %e.code(),
                "API route handler execution failed"
            );

            let api_error = if e.code() == "JS_EXECUTION_ERROR" {
                ApiRouteError::HandlerError {
                    path: route_match.route.path.clone(),
                    method: method.to_string(),
                    message: e.message(),
                    stack: None,
                }
            } else if e.code() == "BAD_REQUEST" {
                ApiRouteError::BodyParseError {
                    path: route_match.route.path.clone(),
                    method: method.to_string(),
                    message: e.message(),
                }
            } else {
                ApiRouteError::HandlerError {
                    path: route_match.route.path.clone(),
                    method: method.to_string(),
                    message: e.message(),
                    stack: None,
                }
            };

            let mut response = api_error.to_http_response(is_development);
            let headers = response.headers_mut();
            if is_development {
                add_api_cors_headers(headers);
            } else {
                add_api_security_headers(headers);
            }
            Ok(response)
        }
    }
}
