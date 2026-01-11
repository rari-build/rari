use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct IpRateLimiter {
    requests: Arc<RwLock<FxHashMap<String, (u32, Instant)>>>,
    max_requests: u32,
    window_duration: Duration,
}

impl IpRateLimiter {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(FxHashMap::default())),
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }

    pub fn check(&self, ip: &str) -> Result<(), u64> {
        let now = Instant::now();
        let mut requests = self.requests.write();

        if let Some((count, window_start)) = requests.get_mut(ip) {
            let elapsed = now.duration_since(*window_start);

            if elapsed >= self.window_duration {
                *count = 1;
                *window_start = now;
                Ok(())
            } else if *count >= self.max_requests {
                let remaining = self.window_duration.saturating_sub(elapsed);
                Err(remaining.as_secs() + 1)
            } else {
                *count += 1;
                Ok(())
            }
        } else {
            requests.insert(ip.to_string(), (1, now));
            Ok(())
        }
    }

    pub fn cleanup(&self) {
        let now = Instant::now();
        let mut requests = self.requests.write();
        requests.retain(|_, (_, window_start)| {
            now.duration_since(*window_start) < self.window_duration * 2
        });
    }

    pub fn start_cleanup_task(self: Arc<Self>) {
        let limiter = self;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                limiter.cleanup();
            }
        });
    }
}

#[derive(Clone)]
pub struct EndpointRateLimiters {
    pub og_generation: Arc<IpRateLimiter>,
    pub csrf_token: Arc<IpRateLimiter>,
    pub image_optimization: Arc<IpRateLimiter>,
}

impl EndpointRateLimiters {
    pub fn new() -> Self {
        Self {
            og_generation: Arc::new(IpRateLimiter::new(10, 60)),
            csrf_token: Arc::new(IpRateLimiter::new(60, 60)),
            image_optimization: Arc::new(IpRateLimiter::new(30, 60)),
        }
    }

    pub fn start_cleanup_tasks(&self) {
        Arc::clone(&self.og_generation).start_cleanup_task();
        Arc::clone(&self.csrf_token).start_cleanup_task();
        Arc::clone(&self.image_optimization).start_cleanup_task();
    }
}

impl Default for EndpointRateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = IpRateLimiter::new(5, 60);

        for _ in 0..5 {
            assert!(limiter.check("192.168.1.1").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = IpRateLimiter::new(3, 60);

        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_err());
    }

    #[test]
    fn test_rate_limiter_separate_ips() {
        let limiter = IpRateLimiter::new(2, 60);

        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_err());

        assert!(limiter.check("192.168.1.2").is_ok());
        assert!(limiter.check("192.168.1.2").is_ok());
    }

    #[test]
    fn test_cleanup_removes_old_entries() {
        let limiter = IpRateLimiter::new(5, 1);

        limiter.check("192.168.1.1").unwrap();

        std::thread::sleep(Duration::from_secs(3));

        limiter.cleanup();

        let requests = limiter.requests.read();
        assert!(requests.is_empty());
    }

    #[test]
    fn test_endpoint_rate_limiters_creation() {
        let limiters = EndpointRateLimiters::new();

        assert!(limiters.og_generation.check("test").is_ok());
        assert!(limiters.csrf_token.check("test").is_ok());
    }
}
