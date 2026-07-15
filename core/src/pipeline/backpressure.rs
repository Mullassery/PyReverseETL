use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Signal indicating system load state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureSignal {
    /// Normal load - process events at full speed
    Ok,
    /// Elevated load - consider slowing down
    Warn,
    /// High load - apply backpressure, reject new events
    Reject,
}

/// Manages backpressure to prevent system overload
pub struct BackpressureManager {
    queue_limit: usize,
    warn_threshold: usize,
    current_load: Arc<AtomicUsize>,
}

impl BackpressureManager {
    /// Create new backpressure manager
    /// warn_threshold typically 80% of queue_limit
    /// reject triggers at 95% of queue_limit
    pub fn new(queue_limit: usize) -> Self {
        let warn_threshold = (queue_limit * 80) / 100;

        Self {
            queue_limit,
            warn_threshold,
            current_load: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Check current load and return signal
    pub fn check_load(&self) -> BackpressureSignal {
        let load = self.current_load.load(Ordering::Acquire);
        let reject_threshold = (self.queue_limit * 95) / 100;

        if load >= reject_threshold {
            BackpressureSignal::Reject
        } else if load >= self.warn_threshold {
            BackpressureSignal::Warn
        } else {
            BackpressureSignal::Ok
        }
    }

    /// Acquire a queue slot (increment load)
    pub fn acquire(&self) -> crate::Result<()> {
        let signal = self.check_load();

        match signal {
            BackpressureSignal::Ok | BackpressureSignal::Warn => {
                self.current_load.fetch_add(1, Ordering::Release);
                Ok(())
            }
            BackpressureSignal::Reject => Err(crate::Error::ConfigError(
                "Backpressure: queue full, rejecting new events".to_string(),
            )),
        }
    }

    /// Release a queue slot (decrement load)
    pub fn release(&self) {
        let current = self.current_load.load(Ordering::Acquire);
        if current > 0 {
            self.current_load.fetch_sub(1, Ordering::Release);
        }
    }

    /// Get current load percentage (0-100)
    pub fn load_percent(&self) -> u32 {
        let load = self.current_load.load(Ordering::Acquire);
        ((load as u32 * 100) / self.queue_limit as u32).min(100)
    }

    /// Get current queue depth
    pub fn queue_depth(&self) -> usize {
        self.current_load.load(Ordering::Acquire)
    }

    /// Reset load to zero
    pub fn reset(&self) {
        self.current_load.store(0, Ordering::Release);
    }
}

impl Default for BackpressureManager {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure_ok_state() {
        let manager = BackpressureManager::new(100);

        manager.acquire().unwrap();
        manager.acquire().unwrap();

        assert_eq!(manager.check_load(), BackpressureSignal::Ok);
    }

    #[test]
    fn test_backpressure_warn_state() {
        let manager = BackpressureManager::new(100);

        for _ in 0..80 {
            manager.acquire().ok();
        }

        assert_eq!(manager.check_load(), BackpressureSignal::Warn);
    }

    #[test]
    fn test_backpressure_reject_state() {
        let manager = BackpressureManager::new(100);

        for _ in 0..96 {
            manager.acquire().ok();
        }

        assert_eq!(manager.check_load(), BackpressureSignal::Reject);
    }

    #[test]
    fn test_backpressure_acquire_release() {
        let manager = BackpressureManager::new(100);

        assert_eq!(manager.queue_depth(), 0);

        manager.acquire().unwrap();
        manager.acquire().unwrap();
        assert_eq!(manager.queue_depth(), 2);

        manager.release();
        assert_eq!(manager.queue_depth(), 1);

        manager.release();
        assert_eq!(manager.queue_depth(), 0);
    }

    #[test]
    fn test_load_percent() {
        let manager = BackpressureManager::new(100);

        for _ in 0..25 {
            manager.acquire().ok();
        }

        assert_eq!(manager.load_percent(), 25);
    }

    #[test]
    fn test_reject_on_acquire() {
        let manager = BackpressureManager::new(100);

        for _ in 0..96 {
            manager.acquire().ok();
        }

        let result = manager.acquire();
        assert!(result.is_err());
    }

    #[test]
    fn test_reset() {
        let manager = BackpressureManager::new(100);

        manager.acquire().ok();
        manager.acquire().ok();
        assert_eq!(manager.queue_depth(), 2);

        manager.reset();
        assert_eq!(manager.queue_depth(), 0);
        assert_eq!(manager.check_load(), BackpressureSignal::Ok);
    }
}
