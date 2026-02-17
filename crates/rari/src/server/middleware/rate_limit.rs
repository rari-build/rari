use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::middleware::StateInformationMiddleware;
use std::net::SocketAddr;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::server::config::Config;
use crate::server::utils::ip_extractor::{RariIpKeyExtractor, extract_client_ip};

pub fn create_rate_limit_layer(
    config: &Config,
) -> Option<GovernorLayer<RariIpKeyExtractor, StateInformationMiddleware, Body>> {
    if !config.rate_limit.enabled {
        return None;
    }

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(config.rate_limit.requests_per_second as u64)
        .burst_size(config.rate_limit.burst_size)
        .use_headers()
        .key_extractor(RariIpKeyExtractor)
        .finish()
        .expect("Failed to create rate limit configuration");

    Some(GovernorLayer::new(governor_conf))
}

pub fn create_strict_rate_limit_layer(
    requests_per_minute: Option<u32>,
) -> GovernorLayer<RariIpKeyExtractor, StateInformationMiddleware, Body> {
    let rpm = requests_per_minute.unwrap_or(10);

    let period_secs = if rpm > 0 { 60 / rpm } else { 60 };
    let period_secs = period_secs.max(1);

    let burst_size = (rpm / 2).max(1);

    let governor_conf = GovernorConfigBuilder::default()
        .period(std::time::Duration::from_secs(period_secs as u64))
        .burst_size(burst_size)
        .use_headers()
        .key_extractor(RariIpKeyExtractor)
        .finish()
        .expect("Failed to create strict rate limit configuration");

    GovernorLayer::new(governor_conf)
}

pub async fn rate_limit_logger(
    ConnectInfo(socket_addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let client_ip = extract_client_ip(request.headers(), &socket_addr);

    let response = next.run(request).await;

    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown");

        tracing::warn!(
            method = %method,
            uri = %uri,
            client_ip = %client_ip,
            status = %status,
            retry_after = %retry_after,
            "Rate limit exceeded"
        );
    } else if response.headers().contains_key("x-ratelimit-remaining") {
        let remaining = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown");

        tracing::debug!(
            method = %method,
            uri = %uri,
            client_ip = %client_ip,
            remaining = %remaining,
            "Rate limit check"
        );
    }

    response
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::server::config::{Mode, RateLimitConfig};

    #[test]
    fn test_rate_limit_layer_disabled() {
        let mut config = Config::new(Mode::Development);
        config.rate_limit.enabled = false;

        let layer = create_rate_limit_layer(&config);
        assert!(layer.is_none());
    }

    #[test]
    fn test_rate_limit_layer_enabled() {
        let mut config = Config::new(Mode::Development);
        config.rate_limit.enabled = true;
        config.rate_limit.requests_per_second = 50;
        config.rate_limit.burst_size = 100;

        let layer = create_rate_limit_layer(&config);
        assert!(layer.is_some());
    }

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = RateLimitConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.requests_per_second, 100);
        assert_eq!(config.burst_size, 200);
        assert_eq!(config.revalidate_requests_per_minute, 10);
    }

    #[test]
    fn test_rate_limit_config_custom() {
        let config = RateLimitConfig {
            enabled: false,
            requests_per_second: 1000,
            burst_size: 2000,
            revalidate_requests_per_minute: 20,
        };
        assert!(!config.enabled);
        assert_eq!(config.requests_per_second, 1000);
        assert_eq!(config.burst_size, 2000);
        assert_eq!(config.revalidate_requests_per_minute, 20);
    }
}
