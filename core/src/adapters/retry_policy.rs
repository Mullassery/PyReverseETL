use crate::adapters::AdapterError;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::sleep;

/// Retry policy with exponential backoff
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30_000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with custom settings
    pub fn new(max_retries: u32, initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            initial_delay_ms,
            max_delay_ms,
            backoff_multiplier: 2.0,
        }
    }

    /// Execute an async function with retry logic
    pub async fn execute<F, T, Fut>(&self, mut f: F) -> Result<T, AdapterError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, AdapterError>>,
    {
        let mut attempt = 0;

        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !Self::is_retryable(&e) {
                        return Err(e);
                    }

                    if attempt >= self.max_retries {
                        return Err(e);
                    }

                    let delay = self.calculate_backoff(attempt);
                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Calculate backoff duration for the given attempt number
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        let delay_ms = (self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32))
            .min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(delay_ms)
    }

    /// Check if an error is retryable
    fn is_retryable(error: &AdapterError) -> bool {
        matches!(
            error,
            AdapterError::ConnectionError(_)
                | AdapterError::NetworkError(_)
                | AdapterError::Timeout
                | AdapterError::RateLimitExceeded { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_delay_ms, 100);
        assert_eq!(policy.max_delay_ms, 30_000);
    }

    #[test]
    fn test_calculate_backoff() {
        let policy = RetryPolicy::default();

        let delay_0 = policy.calculate_backoff(0);
        let delay_1 = policy.calculate_backoff(1);
        let delay_2 = policy.calculate_backoff(2);
        let delay_3 = policy.calculate_backoff(3);

        assert_eq!(delay_0.as_millis(), 100);
        assert_eq!(delay_1.as_millis(), 200);
        assert_eq!(delay_2.as_millis(), 400);
        assert_eq!(delay_3.as_millis(), 800);
    }

    #[test]
    fn test_backoff_capped_at_max() {
        let policy = RetryPolicy::new(10, 100, 5000);

        // Exponential would be 100 * 2^8 = 25600, but capped at 5000
        let delay = policy.calculate_backoff(8);
        assert!(delay.as_millis() <= 5000);
    }

    #[tokio::test]
    async fn test_execute_success_first_attempt() {
        use std::sync::{Arc, Mutex};

        let policy = RetryPolicy::default();
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let cc = call_count_clone.clone();
                async move {
                    *cc.lock().unwrap() += 1;
                    Ok::<i32, AdapterError>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_execute_retry_then_success() {
        use std::sync::{Arc, Mutex};

        let policy = RetryPolicy::default();
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let cc = call_count_clone.clone();
                async move {
                    let mut count = cc.lock().unwrap();
                    *count += 1;
                    let current = *count;
                    drop(count);

                    if current < 3 {
                        Err(AdapterError::NetworkError("timeout".to_string()))
                    } else {
                        Ok::<i32, AdapterError>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*call_count.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_execute_non_retryable_error() {
        use std::sync::{Arc, Mutex};

        let policy = RetryPolicy::default();
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let cc = call_count_clone.clone();
                async move {
                    *cc.lock().unwrap() += 1;
                    Err::<i32, AdapterError>(AdapterError::AuthenticationFailed("invalid key".to_string()))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_execute_max_retries() {
        use std::sync::{Arc, Mutex};

        let policy = RetryPolicy::new(2, 10, 100);
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let result = policy
            .execute(|| {
                let cc = call_count_clone.clone();
                async move {
                    *cc.lock().unwrap() += 1;
                    Err::<i32, AdapterError>(AdapterError::NetworkError("always fails".to_string()))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(*call_count.lock().unwrap(), 3); // Initial + 2 retries
    }

    #[test]
    fn test_is_retryable_connection_error() {
        let err = AdapterError::ConnectionError("reset".to_string());
        assert!(RetryPolicy::is_retryable(&err));
    }

    #[test]
    fn test_is_retryable_network_error() {
        let err = AdapterError::NetworkError("timeout".to_string());
        assert!(RetryPolicy::is_retryable(&err));
    }

    #[test]
    fn test_is_retryable_timeout() {
        let err = AdapterError::Timeout;
        assert!(RetryPolicy::is_retryable(&err));
    }

    #[test]
    fn test_is_not_retryable_auth_error() {
        let err = AdapterError::AuthenticationFailed("invalid".to_string());
        assert!(!RetryPolicy::is_retryable(&err));
    }
}
