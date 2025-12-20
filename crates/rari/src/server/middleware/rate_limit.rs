use axum::{body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response};
use governor::middleware::StateInformationMiddleware;
use std::net::SocketAddr;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor,
};

use crate::server::config::Config;

pub fn create_rate_limit_layer(
    config: &Config,
) -> Option<GovernorLayer<PeerIpKeyExtractor, StateInformationMiddleware, Body>> {
    if !config.rate_limit.enabled {
        return None;
    }

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(config.rate_limit.requests_per_second as u64)
        .burst_size(config.rate_limit.burst_size)
        .use_headers()
        .finish()
        .expect("Failed to create rate limit configuration");

    Some(GovernorLayer::new(governor_conf))
}

pub async fn rate_limit_logger(
    ConnectInfo(_): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    next.run(request).await
}

#[cfg(test)]
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
        assert!(config.enabled);
        assert_eq!(config.requests_per_second, 100);
        assert_eq!(config.burst_size, 200);
    }

    #[test]
    fn test_rate_limit_config_custom() {
        let config =
            RateLimitConfig { enabled: false, requests_per_second: 1000, burst_size: 2000 };
        assert!(!config.enabled);
        assert_eq!(config.requests_per_second, 1000);
        assert_eq!(config.burst_size, 2000);
    }
}
