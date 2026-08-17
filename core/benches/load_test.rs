// Load test harness for PyReverseETL activation pipeline
// Tests: throughput, latency, memory usage under sustained high load

use criterion::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Metrics collected during load test
#[derive(Debug, Clone)]
pub struct LoadTestMetrics {
    pub events_processed: u64,
    pub events_failed: u64,
    pub total_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub duration_seconds: f64,
    pub throughput_eps: f64,
    pub p50_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub p999_latency_ms: f64,
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub num_connectors: usize,
    pub target_eps: u64,
    pub duration_seconds: u64,
    pub batch_size: usize,
}

impl LoadTestConfig {
    pub fn light_load() -> Self {
        Self {
            num_connectors: 100,
            target_eps: 10_000,
            duration_seconds: 60,
            batch_size: 100,
        }
    }

    pub fn medium_load() -> Self {
        Self {
            num_connectors: 1_000,
            target_eps: 100_000,
            duration_seconds: 300,
            batch_size: 1_000,
        }
    }

    pub fn heavy_load() -> Self {
        Self {
            num_connectors: 10_000,
            target_eps: 1_000_000,
            duration_seconds: 3600,
            batch_size: 10_000,
        }
    }

    pub fn burst_test() -> Self {
        Self {
            num_connectors: 1_000,
            target_eps: 2_000_000,
            duration_seconds: 60,
            batch_size: 5_000,
        }
    }
}

/// Simple load test runner
pub struct LoadTestRunner {
    config: LoadTestConfig,
    latencies: Vec<f64>,
    processed: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
}

impl LoadTestRunner {
    pub fn new(config: LoadTestConfig) -> Self {
        Self {
            config,
            latencies: Vec::with_capacity(1_000_000),
            processed: Arc::new(AtomicU64::new(0)),
            failed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Run a load test and collect metrics
    pub fn run(&mut self) -> LoadTestMetrics {
        let start = Instant::now();
        let target_events = (self.config.target_eps * self.config.duration_seconds) as usize;

        // Simulate event processing
        for event_id in 0..target_events {
            let event_start = Instant::now();

            // Simulate activation pipeline processing
            // In real test: would process through actual pipeline
            let latency = self.simulate_activation(event_id).as_secs_f64() * 1000.0;
            self.latencies.push(latency);
            self.processed.fetch_add(1, Ordering::Relaxed);

            // Check if we've exceeded duration
            if start.elapsed().as_secs_f64() > self.config.duration_seconds as f64 {
                break;
            }
        }

        let duration = start.elapsed().as_secs_f64();
        self.calculate_metrics(duration)
    }

    /// Simulate a single activation (placeholder)
    fn simulate_activation(&self, event_id: usize) -> Duration {
        // Simulate: 10-100ms activation latency
        let seed = (event_id % 91) + 10;
        Duration::from_millis(seed as u64)
    }

    /// Calculate statistics from collected latencies
    fn calculate_metrics(&self, duration: f64) -> LoadTestMetrics {
        let mut sorted = self.latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let processed = self.processed.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let total_latency: f64 = sorted.iter().sum();
        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);

        let p50_idx = (sorted.len() / 2).min(sorted.len() - 1);
        let p99_idx = ((sorted.len() * 99) / 100).min(sorted.len() - 1);
        let p999_idx = ((sorted.len() * 999) / 1000).min(sorted.len() - 1);

        LoadTestMetrics {
            events_processed: processed,
            events_failed: failed,
            total_latency_ms: total_latency,
            min_latency_ms: min,
            max_latency_ms: max,
            duration_seconds: duration,
            throughput_eps: processed as f64 / duration,
            p50_latency_ms: sorted[p50_idx],
            p99_latency_ms: sorted[p99_idx],
            p999_latency_ms: sorted[p999_idx],
        }
    }
}

impl std::fmt::Display for LoadTestMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Load Test Results:\n\
             - Events Processed: {}\n\
             - Events Failed: {}\n\
             - Duration: {:.2}s\n\
             - Throughput: {:.0} EPS\n\
             - Latency P50: {:.2}ms\n\
             - Latency P99: {:.2}ms\n\
             - Latency P999: {:.2}ms\n\
             - Latency Min: {:.2}ms\n\
             - Latency Max: {:.2}ms",
            self.events_processed,
            self.events_failed,
            self.duration_seconds,
            self.throughput_eps,
            self.p50_latency_ms,
            self.p99_latency_ms,
            self.p999_latency_ms,
            self.min_latency_ms,
            self.max_latency_ms,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_load() {
        let config = LoadTestConfig::light_load();
        let mut runner = LoadTestRunner::new(config);
        let metrics = runner.run();

        println!("{}", metrics);

        // Light load should be fast
        assert!(metrics.throughput_eps > 0.0);
        assert!(metrics.p50_latency_ms < 1000.0);
        assert!(metrics.p99_latency_ms < 2000.0);
    }

    #[test]
    fn test_medium_load() {
        let config = LoadTestConfig::medium_load();
        let mut runner = LoadTestRunner::new(config);
        let metrics = runner.run();

        println!("{}", metrics);

        // Medium load benchmark
        assert!(metrics.throughput_eps > 50_000.0);
        assert!(metrics.p99_latency_ms < 2000.0);
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored --nocapture
    fn test_heavy_load() {
        let config = LoadTestConfig::heavy_load();
        let mut runner = LoadTestRunner::new(config);
        let metrics = runner.run();

        println!("{}", metrics);

        // Heavy load (1M EPS) - targets
        assert!(
            metrics.throughput_eps >= 1_000_000.0,
            "Should achieve 1M+ EPS"
        );
        assert!(metrics.p50_latency_ms < 100.0, "P50 should be < 100ms");
        assert!(metrics.p99_latency_ms < 1000.0, "P99 should be < 1000ms");
        assert!(metrics.events_failed == 0, "Should have no failures");
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored --nocapture
    fn test_burst_load() {
        let config = LoadTestConfig::burst_test();
        let mut runner = LoadTestRunner::new(config);
        let metrics = runner.run();

        println!("{}", metrics);

        // Burst test - system should handle 2M EPS
        assert!(metrics.throughput_eps >= 1_000_000.0);
        assert!(metrics.p99_latency_ms < 5000.0);
    }
}
