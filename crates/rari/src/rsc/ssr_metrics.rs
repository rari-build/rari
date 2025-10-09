use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct SsrTiming {
    pub parse_rsc_ms: f64,
    pub serialize_to_v8_ms: f64,
    pub v8_execution_ms: f64,
    pub load_template_ms: f64,
    pub inject_template_ms: f64,
    pub total_ms: f64,
}

impl SsrTiming {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn log_if_slow(&self, threshold_ms: f64) {
        if self.total_ms > threshold_ms {
            tracing::warn!(
                "Slow SSR render: {:.2}ms total (parse: {:.2}ms, serialize: {:.2}ms, v8: {:.2}ms, template: {:.2}ms, inject: {:.2}ms)",
                self.total_ms,
                self.parse_rsc_ms,
                self.serialize_to_v8_ms,
                self.v8_execution_ms,
                self.load_template_ms,
                self.inject_template_ms
            );
        }
    }

    pub fn log_breakdown(&self) {
        if self.total_ms > 0.0 {
            tracing::debug!(
                "SSR timing breakdown: parse={:.2}ms ({:.1}%), serialize={:.2}ms ({:.1}%), v8={:.2}ms ({:.1}%), template={:.2}ms ({:.1}%), inject={:.2}ms ({:.1}%)",
                self.parse_rsc_ms,
                (self.parse_rsc_ms / self.total_ms) * 100.0,
                self.serialize_to_v8_ms,
                (self.serialize_to_v8_ms / self.total_ms) * 100.0,
                self.v8_execution_ms,
                (self.v8_execution_ms / self.total_ms) * 100.0,
                self.load_template_ms,
                (self.load_template_ms / self.total_ms) * 100.0,
                self.inject_template_ms,
                (self.inject_template_ms / self.total_ms) * 100.0,
            );
        }
    }
}

pub struct TimingScope {
    start: Instant,
}

impl TimingScope {
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

impl Default for TimingScope {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_ssr_timing_new() {
        let timing = SsrTiming::new();
        assert_eq!(timing.parse_rsc_ms, 0.0);
        assert_eq!(timing.serialize_to_v8_ms, 0.0);
        assert_eq!(timing.v8_execution_ms, 0.0);
        assert_eq!(timing.load_template_ms, 0.0);
        assert_eq!(timing.inject_template_ms, 0.0);
        assert_eq!(timing.total_ms, 0.0);
    }

    #[test]
    fn test_ssr_timing_default() {
        let timing = SsrTiming::default();
        assert_eq!(timing.parse_rsc_ms, 0.0);
        assert_eq!(timing.total_ms, 0.0);
    }

    #[test]
    fn test_timing_scope_elapsed() {
        let scope = TimingScope::new();
        thread::sleep(Duration::from_millis(10));
        let elapsed = scope.elapsed_ms();

        assert!(elapsed >= 9.0, "Expected at least 9ms, got {:.2}ms", elapsed);
        assert!(elapsed < 50.0, "Expected less than 50ms, got {:.2}ms", elapsed);
    }

    #[test]
    fn test_timing_scope_default() {
        let scope = TimingScope::default();
        let elapsed = scope.elapsed_ms();
        assert!(elapsed >= 0.0);
    }

    #[test]
    fn test_log_breakdown_with_zero_total() {
        let timing = SsrTiming::new();
        timing.log_breakdown();
    }

    #[test]
    fn test_log_breakdown_with_values() {
        let timing = SsrTiming {
            parse_rsc_ms: 5.0,
            serialize_to_v8_ms: 3.0,
            v8_execution_ms: 10.0,
            load_template_ms: 1.0,
            inject_template_ms: 1.0,
            total_ms: 20.0,
        };
        timing.log_breakdown();
    }

    #[test]
    fn test_log_if_slow_below_threshold() {
        let timing = SsrTiming {
            parse_rsc_ms: 5.0,
            serialize_to_v8_ms: 3.0,
            v8_execution_ms: 10.0,
            load_template_ms: 1.0,
            inject_template_ms: 1.0,
            total_ms: 20.0,
        };
        timing.log_if_slow(50.0);
    }

    #[test]
    fn test_log_if_slow_above_threshold() {
        let timing = SsrTiming {
            parse_rsc_ms: 20.0,
            serialize_to_v8_ms: 15.0,
            v8_execution_ms: 30.0,
            load_template_ms: 5.0,
            inject_template_ms: 5.0,
            total_ms: 75.0,
        };
        timing.log_if_slow(50.0);
    }
}
