use crate::Error;
use backoff::future::retry;
use backoff::ExponentialBackoff;
/// Retry Policy for StatGuardian API Requests
///
/// Implements exponential backoff retry logic for transient failures.
/// Handles network errors, timeouts, and rate limiting.
use std::time::Duration;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(max_retries: u32, initial_backoff_ms: u64) -> Self {
        Self {
            max_retries,
            initial_backoff_ms,
            max_backoff_ms: 30_000, // 30 seconds max
            multiplier: 2.0,
        }
    }

    /// Default retry policy (3 retries, 100ms initial backoff)
    pub fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 30_000,
            multiplier: 2.0,
        }
    }

    /// Determine if error is retryable
    pub fn should_retry(&self, error: &Error) -> bool {
        match error {
            Error::ConfigError(msg) => {
                // Retry on network/timeout errors
                msg.contains("timeout")
                    || msg.contains("request failed")
                    || msg.contains("connection")
                    || msg.contains("temporary")
                    || msg.contains("429") // Rate limit
            }
            _ => false,
        }
    }

    /// Calculate backoff delay for attempt
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let base_ms = self.initial_backoff_ms as f64;
        let delay_ms =
            (base_ms * self.multiplier.powi(attempt as i32)).min(self.max_backoff_ms as f64) as u64;
        Duration::from_millis(delay_ms)
    }

    /// Create backoff config for use with backoff crate
    pub fn backoff_config(&self) -> ExponentialBackoff {
        ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_millis(
                self.initial_backoff_ms + (self.max_backoff_ms * self.max_retries as u64),
            )),
            initial_interval: Duration::from_millis(self.initial_backoff_ms),
            max_interval: Duration::from_millis(self.max_backoff_ms),
            multiplier: self.multiplier,
            ..Default::default()
        }
    }
}

/// Rate limiter for API calls
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Requests per second limit
    pub requests_per_second: u32,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            requests_per_second,
        }
    }

    /// Get delay before next request
    pub fn delay_between_requests(&self) -> Duration {
        if self.requests_per_second == 0 {
            Duration::from_millis(0)
        } else {
            Duration::from_millis(1000 / self.requests_per_second as u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_backoff_ms, 100);
    }

    #[test]
    fn test_retry_policy_creation() {
        let policy = RetryPolicy::new(5, 50);
        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.initial_backoff_ms, 50);
    }

    #[test]
    fn test_should_retry_timeout() {
        let policy = RetryPolicy::default();
        let error = Error::ConfigError("request timeout".to_string());
        assert!(policy.should_retry(&error));
    }

    #[test]
    fn test_should_retry_connection() {
        let policy = RetryPolicy::default();
        let error = Error::ConfigError("connection failed".to_string());
        assert!(policy.should_retry(&error));
    }

    #[test]
    fn test_should_retry_rate_limit() {
        let policy = RetryPolicy::default();
        let error = Error::ConfigError("429 Too Many Requests".to_string());
        assert!(policy.should_retry(&error));
    }

    #[test]
    fn test_should_not_retry_validation_error() {
        let policy = RetryPolicy::default();
        let error = Error::ConfigError("validation failed".to_string());
        assert!(!policy.should_retry(&error));
    }

    #[test]
    fn test_backoff_delay_exponential() {
        let policy = RetryPolicy::new(3, 100);

        // Check exponential growth
        let delay0 = policy.backoff_delay(0);
        let delay1 = policy.backoff_delay(1);
        let delay2 = policy.backoff_delay(2);

        assert_eq!(delay0, Duration::from_millis(100));
        assert_eq!(delay1, Duration::from_millis(200));
        assert_eq!(delay2, Duration::from_millis(400));
    }

    #[test]
    fn test_backoff_delay_respects_max() {
        let policy = RetryPolicy {
            max_retries: 10,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            multiplier: 2.0,
        };

        // After several attempts, should cap at max_backoff
        let delay10 = policy.backoff_delay(10);
        assert!(delay10.as_millis() <= policy.max_backoff_ms as u128);
    }

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(100);
        assert_eq!(limiter.requests_per_second, 100);
    }

    #[test]
    fn test_rate_limiter_delay() {
        let limiter = RateLimiter::new(10); // 10 req/sec = 100ms between
        let delay = limiter.delay_between_requests();
        assert_eq!(delay, Duration::from_millis(100));
    }

    #[test]
    fn test_rate_limiter_zero_limit() {
        let limiter = RateLimiter::new(0);
        let delay = limiter.delay_between_requests();
        assert_eq!(delay, Duration::from_millis(0));
    }
}
