use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct IpRateLimiter {
    requests: Arc<Mutex<LruCache<String, (u32, Instant)>>>,
    max_requests: u32,
    window_duration: Duration,
}

impl IpRateLimiter {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self::with_capacity(max_requests, window_seconds, 10_000)
    }

    pub fn with_capacity(max_requests: u32, window_seconds: u64, max_tracked_ips: usize) -> Self {
        let capacity =
            NonZeroUsize::new(max_tracked_ips).expect("max_tracked_ips must be greater than zero");
        Self {
            requests: Arc::new(Mutex::new(LruCache::new(capacity))),
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }

    pub fn capacity(&self) -> usize {
        self.requests.lock().cap().get()
    }

    pub fn check(&self, ip: &str) -> Result<(), u64> {
        let now = Instant::now();
        let mut requests = self.requests.lock();

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
            requests.put(ip.to_string(), (1, now));
            Ok(())
        }
    }

    pub fn cleanup(&self) {
        let now = Instant::now();
        let mut requests = self.requests.lock();

        let keys_to_remove: Vec<String> = requests
            .iter()
            .filter_map(|(key, (_, window_start))| {
                if now.duration_since(*window_start) >= self.window_duration * 2 {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_remove {
            requests.pop(&key);
        }
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
        Self::for_environment(false)
    }

    pub fn for_environment(is_production: bool) -> Self {
        if is_production {
            Self {
                og_generation: Arc::new(IpRateLimiter::with_capacity(100, 60, 10_000)),
                csrf_token: Arc::new(IpRateLimiter::with_capacity(300, 60, 10_000)),
                image_optimization: Arc::new(IpRateLimiter::with_capacity(200, 60, 10_000)),
            }
        } else {
            Self {
                og_generation: Arc::new(IpRateLimiter::with_capacity(1000, 60, 100_000)),
                csrf_token: Arc::new(IpRateLimiter::with_capacity(1000, 60, 100_000)),
                image_optimization: Arc::new(IpRateLimiter::with_capacity(1000, 60, 100_000)),
            }
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
#[allow(clippy::disallowed_methods)]
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

        let requests = limiter.requests.lock();
        assert!(requests.is_empty());
    }

    #[test]
    fn test_endpoint_rate_limiters_creation() {
        let limiters = EndpointRateLimiters::new();

        assert!(limiters.og_generation.check("test").is_ok());
        assert!(limiters.csrf_token.check("test").is_ok());
    }

    #[test]
    fn test_memory_cap_enforced() {
        let limiter = IpRateLimiter::with_capacity(10, 60, 3);

        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.2").is_ok());
        assert!(limiter.check("192.168.1.3").is_ok());

        assert!(limiter.check("192.168.1.4").is_ok());

        let requests = limiter.requests.lock();
        assert_eq!(requests.len(), 3);
        assert!(requests.contains(&"192.168.1.4".to_string()));
    }

    #[test]
    fn test_memory_cap_evicts_oldest() {
        let limiter = IpRateLimiter::with_capacity(10, 60, 2);

        assert!(limiter.check("192.168.1.1").is_ok());
        std::thread::sleep(Duration::from_millis(10));

        assert!(limiter.check("192.168.1.2").is_ok());
        std::thread::sleep(Duration::from_millis(10));

        assert!(limiter.check("192.168.1.3").is_ok());

        let requests = limiter.requests.lock();
        assert_eq!(requests.len(), 2);
        assert!(!requests.contains(&"192.168.1.1".to_string()));
        assert!(requests.contains(&"192.168.1.2".to_string()));
        assert!(requests.contains(&"192.168.1.3".to_string()));
    }

    #[test]
    fn test_memory_cap_existing_ip_not_evicted() {
        let limiter = IpRateLimiter::with_capacity(5, 60, 2);

        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.2").is_ok());

        assert!(limiter.check("192.168.1.1").is_ok());

        let requests = limiter.requests.lock();
        assert_eq!(requests.len(), 2);
        assert!(requests.contains(&"192.168.1.1".to_string()));
        assert!(requests.contains(&"192.168.1.2".to_string()));
    }

    #[test]
    fn test_memory_cap_under_attack() {
        let limiter = IpRateLimiter::with_capacity(10, 60, 100);

        for i in 0..1000 {
            let ip = format!("192.168.{}.{}", i / 256, i % 256);
            assert!(limiter.check(&ip).is_ok());
        }

        let requests = limiter.requests.lock();
        assert_eq!(requests.len(), 100);
    }

    #[test]
    fn test_production_limits_have_reasonable_capacity() {
        let limiters = EndpointRateLimiters::for_environment(true);

        assert_eq!(limiters.og_generation.capacity(), 10_000);
        assert_eq!(limiters.csrf_token.capacity(), 10_000);
        assert_eq!(limiters.image_optimization.capacity(), 10_000);
    }

    #[test]
    fn test_development_limits_have_higher_capacity() {
        let limiters = EndpointRateLimiters::for_environment(false);

        assert_eq!(limiters.og_generation.capacity(), 100_000);
        assert_eq!(limiters.csrf_token.capacity(), 100_000);
        assert_eq!(limiters.image_optimization.capacity(), 100_000);
    }
}
