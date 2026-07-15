use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Statistics for latency measurements
#[derive(Debug, Clone)]
pub struct LatencyStats {
    /// Minimum latency observed (milliseconds)
    pub min_ms: u64,
    /// Maximum latency observed (milliseconds)
    pub max_ms: u64,
    /// Mean latency (milliseconds)
    pub mean_ms: f64,
    /// 50th percentile latency (milliseconds)
    pub p50_ms: u64,
    /// 99th percentile latency (milliseconds)
    pub p99_ms: u64,
    /// 99.9th percentile latency (milliseconds)
    pub p999_ms: u64,
    /// Total samples recorded
    pub sample_count: usize,
}

/// Tracks latency metrics with percentile calculations
pub struct LatencyTracker {
    latencies: Arc<Mutex<VecDeque<u64>>>,
    max_samples: usize,
}

impl LatencyTracker {
    /// Create a new latency tracker with specified sample window
    pub fn new(max_samples: usize) -> Self {
        Self {
            latencies: Arc::new(Mutex::new(VecDeque::with_capacity(max_samples))),
            max_samples,
        }
    }

    /// Record a latency measurement in milliseconds
    pub async fn record(&self, latency_ms: u64) {
        let mut latencies = self.latencies.lock().await;

        latencies.push_back(latency_ms);

        // Maintain rolling window
        while latencies.len() > self.max_samples {
            latencies.pop_front();
        }
    }

    /// Get current latency statistics
    pub async fn stats(&self) -> LatencyStats {
        let latencies = self.latencies.lock().await;

        if latencies.is_empty() {
            return LatencyStats {
                min_ms: 0,
                max_ms: 0,
                mean_ms: 0.0,
                p50_ms: 0,
                p99_ms: 0,
                p999_ms: 0,
                sample_count: 0,
            };
        }

        let mut sorted: Vec<u64> = latencies.iter().copied().collect();
        sorted.sort_unstable();

        let min_ms = *sorted.first().unwrap_or(&0);
        let max_ms = *sorted.last().unwrap_or(&0);
        let mean_ms = sorted.iter().sum::<u64>() as f64 / sorted.len() as f64;

        let p50_idx = (sorted.len() * 50) / 100;
        let p99_idx = (sorted.len() * 99) / 100;
        let p999_idx = (sorted.len() * 999) / 1000;

        let p50_ms = sorted.get(p50_idx).copied().unwrap_or(0);
        let p99_ms = sorted.get(p99_idx).copied().unwrap_or(max_ms);
        let p999_ms = sorted.get(p999_idx).copied().unwrap_or(max_ms);

        LatencyStats {
            min_ms,
            max_ms,
            mean_ms,
            p50_ms,
            p99_ms,
            p999_ms,
            sample_count: latencies.len(),
        }
    }

    /// Reset all recorded latencies
    pub async fn reset(&self) {
        self.latencies.lock().await.clear();
    }

    /// Get number of recorded samples
    pub async fn sample_count(&self) -> usize {
        self.latencies.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_latency() {
        let tracker = LatencyTracker::new(100);

        tracker.record(10).await;
        tracker.record(20).await;
        tracker.record(30).await;

        assert_eq!(tracker.sample_count().await, 3);
    }

    #[tokio::test]
    async fn test_latency_stats() {
        let tracker = LatencyTracker::new(100);

        for i in 1..=10 {
            tracker.record(i * 10).await;
        }

        let stats = tracker.stats().await;

        assert_eq!(stats.min_ms, 10);
        assert_eq!(stats.max_ms, 100);
        assert_eq!(stats.sample_count, 10);
        assert!(stats.mean_ms > 0.0);
    }

    #[tokio::test]
    async fn test_percentile_calculation() {
        let tracker = LatencyTracker::new(1000);

        for i in 1..=100 {
            tracker.record(i).await;
        }

        let stats = tracker.stats().await;

        assert!(stats.p50_ms > 0);
        assert!(stats.p99_ms >= stats.p50_ms);
        assert!(stats.p999_ms >= stats.p99_ms);
    }

    #[tokio::test]
    async fn test_latency_rolling_window() {
        let tracker = LatencyTracker::new(5);

        for i in 1..=10 {
            tracker.record(i).await;
        }

        // Should only have last 5 samples
        assert_eq!(tracker.sample_count().await, 5);

        let stats = tracker.stats().await;
        assert_eq!(stats.min_ms, 6); // Oldest sample should be 6
        assert_eq!(stats.max_ms, 10); // Newest sample should be 10
    }

    #[tokio::test]
    async fn test_latency_reset() {
        let tracker = LatencyTracker::new(100);

        tracker.record(10).await;
        tracker.record(20).await;

        assert_eq!(tracker.sample_count().await, 2);

        tracker.reset().await;

        assert_eq!(tracker.sample_count().await, 0);
    }

    #[tokio::test]
    async fn test_empty_stats() {
        let tracker = LatencyTracker::new(100);
        let stats = tracker.stats().await;

        assert_eq!(stats.min_ms, 0);
        assert_eq!(stats.max_ms, 0);
        assert_eq!(stats.sample_count, 0);
    }
}
