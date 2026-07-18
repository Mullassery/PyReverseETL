/// Rate Limiting for Destinations
///
/// Prevent overwhelming external systems with configurable rate limits.
/// Supports token bucket, leaky bucket, and quota-based strategies.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;

/// Rate limiting strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RateLimitStrategy {
    /// Token bucket: fixed tokens per interval
    TokenBucket,
    /// Leaky bucket: smooth out bursts
    LeakyBucket,
    /// Fixed quota per time window
    Quota,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Strategy to use
    pub strategy: RateLimitStrategy,

    /// Requests per interval (e.g., 100 per second)
    pub requests_per_interval: u64,

    /// Time interval (e.g., 1 second)
    pub interval: Duration,

    /// Max burst size (for token bucket)
    pub max_burst: Option<u64>,

    /// Enable adaptive rate limiting based on errors
    pub adaptive: bool,

    /// Cooldown after rate limit hit
    pub cooldown: Duration,

    /// Tags for rate limit group (e.g., "salesforce", "api_v2")
    pub tag: Option<String>,
}

impl RateLimitConfig {
    /// Create token bucket (burst allowed)
    pub fn token_bucket(requests_per_sec: u64) -> Self {
        Self {
            strategy: RateLimitStrategy::TokenBucket,
            requests_per_interval: requests_per_sec,
            interval: Duration::from_secs(1),
            max_burst: Some(requests_per_sec * 2),
            adaptive: false,
            cooldown: Duration::from_secs(1),
            tag: None,
        }
    }

    /// Create leaky bucket (smooth rate)
    pub fn leaky_bucket(requests_per_sec: u64) -> Self {
        Self {
            strategy: RateLimitStrategy::LeakyBucket,
            requests_per_interval: requests_per_sec,
            interval: Duration::from_secs(1),
            max_burst: None,
            adaptive: false,
            cooldown: Duration::from_millis(100),
            tag: None,
        }
    }

    /// Create quota-based limit (strict per window)
    pub fn quota(requests_per_hour: u64) -> Self {
        Self {
            strategy: RateLimitStrategy::Quota,
            requests_per_interval: requests_per_hour,
            interval: Duration::from_secs(3600),
            max_burst: None,
            adaptive: false,
            cooldown: Duration::from_secs(60),
            tag: None,
        }
    }

    /// Enable adaptive rate limiting
    pub fn with_adaptive(mut self, enabled: bool) -> Self {
        self.adaptive = enabled;
        self
    }

    /// Set tag for rate limit group
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }
}

/// Rate limiter instance
pub struct RateLimiter {
    config: RateLimitConfig,
    state: Arc<Mutex<RateLimiterState>>,
}

struct RateLimiterState {
    tokens: f64,
    last_refill: Instant,
    last_limit_time: Option<Instant>,
    error_count: u64,
    success_count: u64,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let state = RateLimiterState {
            tokens: config.max_burst.unwrap_or(config.requests_per_interval) as f64,
            last_refill: Instant::now(),
            last_limit_time: None,
            error_count: 0,
            success_count: 0,
        };

        Self {
            config,
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Check if request is allowed (non-blocking)
    pub fn is_allowed(&self) -> bool {
        let mut state = self.state.lock();

        // Check cooldown period
        if let Some(last_limit) = state.last_limit_time {
            if Instant::now().duration_since(last_limit) < self.config.cooldown {
                return false;
            }
        }

        match self.config.strategy {
            RateLimitStrategy::TokenBucket => self.check_token_bucket(&mut state),
            RateLimitStrategy::LeakyBucket => self.check_leaky_bucket(&mut state),
            RateLimitStrategy::Quota => self.check_quota(&mut state),
        }
    }

    /// Wait until request is allowed (blocking)
    pub async fn acquire_permit(&self) {
        loop {
            if self.is_allowed() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Record successful request
    pub fn record_success(&self) {
        let mut state = self.state.lock();
        state.success_count += 1;
    }

    /// Record failed request
    pub fn record_error(&self) {
        let mut state = self.state.lock();
        state.error_count += 1;

        // Adaptive: reduce rate on errors
        if self.config.adaptive && state.error_count > 3 {
            state.last_limit_time = Some(Instant::now());
        }
    }

    /// Get current rate limit stats
    pub fn stats(&self) -> RateLimitStats {
        let state = self.state.lock();
        RateLimitStats {
            tokens_available: state.tokens.max(0.0),
            total_requests: state.success_count + state.error_count,
            successful_requests: state.success_count,
            failed_requests: state.error_count,
            success_rate: if (state.success_count + state.error_count) > 0 {
                (state.success_count as f64 / (state.success_count + state.error_count) as f64) * 100.0
            } else {
                100.0
            },
        }
    }

    /// Reset rate limiter
    pub fn reset(&self) {
        let mut state = self.state.lock();
        state.tokens = self.config.max_burst.unwrap_or(self.config.requests_per_interval) as f64;
        state.last_refill = Instant::now();
        state.error_count = 0;
        state.success_count = 0;
    }

    fn check_token_bucket(&self, state: &mut RateLimiterState) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill);

        // Refill tokens
        let refill_rate = self.config.requests_per_interval as f64 / self.config.interval.as_secs_f64();
        let tokens_to_add = elapsed.as_secs_f64() * refill_rate;
        state.tokens += tokens_to_add;

        let max_tokens = self.config.max_burst.unwrap_or(self.config.requests_per_interval) as f64;
        state.tokens = state.tokens.min(max_tokens);
        state.last_refill = now;

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn check_leaky_bucket(&self, state: &mut RateLimiterState) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill);

        // Leak: smooth out requests over time
        let interval_secs = self.config.interval.as_secs_f64();
        let min_interval = interval_secs / self.config.requests_per_interval as f64;

        state.tokens += elapsed.as_secs_f64();
        state.last_refill = now;

        if state.tokens >= min_interval {
            state.tokens -= min_interval;
            true
        } else {
            false
        }
    }

    fn check_quota(&self, state: &mut RateLimiterState) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill);

        if elapsed >= self.config.interval {
            // Reset quota window
            state.tokens = self.config.requests_per_interval as f64;
            state.last_refill = now;
        }

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            state.last_limit_time = Some(now);
            false
        }
    }
}

/// Rate limit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStats {
    pub tokens_available: f64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
}

/// Global rate limiter registry
pub struct RateLimiterRegistry {
    limiters: Arc<Mutex<std::collections::HashMap<String, Arc<RateLimiter>>>>,
}

impl RateLimiterRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            limiters: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Register rate limiter for a destination
    pub fn register(&self, id: &str, limiter: Arc<RateLimiter>) {
        let mut limiters = self.limiters.lock();
        limiters.insert(id.to_string(), limiter);
    }

    /// Get rate limiter
    pub fn get(&self, id: &str) -> Option<Arc<RateLimiter>> {
        let limiters = self.limiters.lock();
        limiters.get(id).cloned()
    }

    /// Get or create rate limiter
    pub fn get_or_create(&self, id: &str, config: RateLimitConfig) -> Arc<RateLimiter> {
        let limiters = self.limiters.lock();
        if let Some(limiter) = limiters.get(id) {
            return limiter.clone();
        }
        drop(limiters);

        let limiter = Arc::new(RateLimiter::new(config));
        self.register(id, limiter.clone());
        limiter
    }

    /// Remove rate limiter
    pub fn unregister(&self, id: &str) {
        let mut limiters = self.limiters.lock();
        limiters.remove(id);
    }

    /// Get all registered limiters
    pub fn list_all(&self) -> Vec<(String, Arc<RateLimiter>)> {
        let limiters = self.limiters.lock();
        limiters
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl Default for RateLimiterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let config = RateLimitConfig::token_bucket(100);
        let limiter = RateLimiter::new(config);

        // Should allow first 100 requests
        for _ in 0..100 {
            assert!(limiter.is_allowed());
        }

        // 101st should be denied
        assert!(!limiter.is_allowed());
    }

    #[test]
    fn test_quota_based() {
        let config = RateLimitConfig::quota(1000);
        let limiter = RateLimiter::new(config);

        // Allow 1000 requests per hour
        assert!(limiter.is_allowed());

        let stats = limiter.stats();
        assert_eq!(stats.total_requests, 1);
    }

    #[test]
    fn test_rate_limiting_stats() {
        let config = RateLimitConfig::token_bucket(10);
        let limiter = RateLimiter::new(config);

        for _ in 0..5 {
            limiter.is_allowed();
            limiter.record_success();
        }

        let stats = limiter.stats();
        assert_eq!(stats.successful_requests, 5);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.success_rate, 100.0);
    }

    #[test]
    fn test_registry() {
        let registry = RateLimiterRegistry::new();
        let config = RateLimitConfig::token_bucket(50);
        let limiter = Arc::new(RateLimiter::new(config));

        registry.register("salesforce", limiter.clone());
        let retrieved = registry.get("salesforce");
        assert!(retrieved.is_some());
    }
}
