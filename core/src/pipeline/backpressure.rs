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

    /// The load at/above which new work is rejected (95% of `queue_limit`).
    fn reject_threshold(&self) -> usize {
        (self.queue_limit * 95) / 100
    }

    /// Check current load and return signal
    pub fn check_load(&self) -> BackpressureSignal {
        let load = self.current_load.load(Ordering::Acquire);

        if load >= self.reject_threshold() {
            BackpressureSignal::Reject
        } else if load >= self.warn_threshold {
            BackpressureSignal::Warn
        } else {
            BackpressureSignal::Ok
        }
    }

    /// Acquire a queue slot (increment load).
    ///
    /// Implemented as a single atomic `fetch_update` (compare-and-swap loop)
    /// rather than a separate load-then-branch-then-`fetch_add`: the latter
    /// is a classic TOCTOU race under real concurrent callers (which is
    /// exactly how `ActivationPipeline` uses this -- one shared
    /// `Arc<BackpressureManager>` across concurrently processed events) --
    /// N callers can all read a load just under `reject_threshold`, all see
    /// `Ok`/`Warn`, and all then increment, overshooting the "hard" reject
    /// cap by up to N-1. `fetch_update` re-reads and re-checks the threshold
    /// atomically on every retry, so only as many callers as there are
    /// actually-available slots can ever succeed.
    pub fn acquire(&self) -> crate::Result<()> {
        let reject_threshold = self.reject_threshold();
        self.current_load
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                if current >= reject_threshold {
                    None
                } else {
                    Some(current + 1)
                }
            })
            .map(|_| ())
            .map_err(|_| {
                crate::Error::ConfigError(
                    "Backpressure: queue full, rejecting new events".to_string(),
                )
            })
    }

    /// Release a queue slot (decrement load).
    ///
    /// Also a single atomic `fetch_update` rather than a separate
    /// load-then-branch-then-`fetch_sub`: the latter is racy the same way
    /// `acquire()` was -- two concurrent `release()` calls can both read
    /// `current_load == 1`, both see `current > 0`, and both unconditionally
    /// `fetch_sub(1)`, taking the *unsigned* counter from 1 to 0 to a wrapped
    /// `usize::MAX`. Once that happens `check_load()` sees an enormous load
    /// forever, permanently returning `Reject` -- a total, unrecoverable
    /// lockout of the backpressure manager, not just a transient overshoot.
    /// `checked_sub` inside `fetch_update` makes "decrement, but never below
    /// zero" atomic, so this can no longer happen.
    pub fn release(&self) {
        let _ = self
            .current_load
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_sub(1)
            });
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
    fn concurrent_acquire_never_overshoots_the_reject_threshold() {
        // Regression test: acquire() used to be check_load() (one atomic
        // read) followed by a *separate* fetch_add -- classic TOCTOU. Driven
        // with a barrier to force maximum contention at the boundary (many
        // real OS threads all calling the real, public `acquire()`
        // concurrently from load = reject_threshold - 1), this reliably
        // reproduced more than one success where sequential semantics only
        // allow exactly one, overshooting the "hard" cap by up to N-1. Now
        // that `acquire()` is a single atomic `fetch_update`, at most one of
        // these concurrent callers can win the last slot.
        use std::sync::{Arc, Barrier};
        use std::thread;

        let manager = Arc::new(BackpressureManager::new(100));
        for _ in 0..94 {
            manager.acquire().unwrap();
        }
        assert_eq!(manager.queue_depth(), 94);

        let n_threads = 20;
        let barrier = Arc::new(Barrier::new(n_threads));
        let mut handles = Vec::new();
        for _ in 0..n_threads {
            let m = manager.clone();
            let b = barrier.clone();
            handles.push(thread::spawn(move || {
                b.wait();
                m.acquire().is_ok()
            }));
        }
        let successes: usize = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|ok| *ok)
            .count();

        assert_eq!(
            successes,
            1,
            "exactly 1 of {n_threads} concurrent acquire() calls should win the last slot from \
             load=94 (reject_threshold=95); final queue_depth={}",
            manager.queue_depth()
        );
        assert_eq!(manager.queue_depth(), 95);
        assert_eq!(manager.check_load(), BackpressureSignal::Reject);
    }

    #[test]
    fn concurrent_release_never_underflows_below_zero() {
        // Regression test: release() used to be check-then-act too --
        // `if current > 0 { fetch_sub(1) }` -- the read and the unconditional
        // fetch_sub are separate atomic ops. Many concurrent real `release()`
        // calls racing down from a small load used to be able to take the
        // *unsigned* counter below zero, wrapping to usize::MAX and making
        // check_load() return Reject forever (a permanent lockout, not a
        // transient blip). `release()` is now a single atomic `fetch_update`
        // using `checked_sub`, so it can never go below zero regardless of
        // how many more `release()` calls than acquired slots race in.
        use std::sync::{Arc, Barrier};
        use std::thread;

        let manager = Arc::new(BackpressureManager::new(100));
        manager.acquire().unwrap();
        assert_eq!(manager.queue_depth(), 1);

        // More concurrent release() calls than there are slots to release.
        let n_threads = 10;
        let barrier = Arc::new(Barrier::new(n_threads));
        let mut handles = Vec::new();
        for _ in 0..n_threads {
            let m = manager.clone();
            let b = barrier.clone();
            handles.push(thread::spawn(move || {
                b.wait();
                m.release();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(
            manager.queue_depth(),
            0,
            "current_load must saturate at 0, never wrap around to usize::MAX"
        );
        assert_eq!(manager.check_load(), BackpressureSignal::Ok);
    }

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
