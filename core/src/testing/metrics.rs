// Metrics collection for connector testing
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorMetrics {
    pub connector_id: String,
    pub records_processed: u64,
    pub bytes_transferred: u64,
    pub latency_ms: f64,
    pub throughput_rps: f64,
    pub error_rate: f64,
    pub memory_used_mb: f64,
    pub errors: u64,
    pub retries: u64,
    pub circuit_breaker_trips: u64,
}

/// Thread-safe metrics collector for connectors
pub struct MetricsCollector {
    connector_id: String,
    records: Arc<AtomicU64>,
    bytes: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    retries: Arc<AtomicU64>,
    cb_trips: Arc<AtomicU64>,
}

impl MetricsCollector {
    pub fn new(connector_id: impl Into<String>) -> Self {
        Self {
            connector_id: connector_id.into(),
            records: Arc::new(AtomicU64::new(0)),
            bytes: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            retries: Arc::new(AtomicU64::new(0)),
            cb_trips: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_processed(&self, count: u64) {
        self.records.fetch_add(count, Ordering::Relaxed);
    }

    pub fn record_bytes(&self, bytes: u64) {
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_retry(&self) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_circuit_breaker_trip(&self) {
        self.cb_trips.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_metrics(&self) -> ConnectorMetrics {
        let records = self.records.load(Ordering::Relaxed);
        let bytes = self.bytes.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);

        let error_rate = if records > 0 {
            errors as f64 / records as f64
        } else {
            0.0
        };

        ConnectorMetrics {
            connector_id: self.connector_id.clone(),
            records_processed: records,
            bytes_transferred: bytes,
            latency_ms: 0.0, // Would be calculated from timing
            throughput_rps: 0.0, // Would be calculated
            error_rate,
            memory_used_mb: 0.0, // Would be measured
            errors,
            retries: self.retries.load(Ordering::Relaxed),
            circuit_breaker_trips: self.cb_trips.load(Ordering::Relaxed),
        }
    }

    pub fn reset(&self) {
        self.records.store(0, Ordering::Relaxed);
        self.bytes.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.retries.store(0, Ordering::Relaxed);
        self.cb_trips.store(0, Ordering::Relaxed);
    }
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            connector_id: self.connector_id.clone(),
            records: Arc::clone(&self.records),
            bytes: Arc::clone(&self.bytes),
            errors: Arc::clone(&self.errors),
            retries: Arc::clone(&self.retries),
            cb_trips: Arc::clone(&self.cb_trips),
        }
    }
}

impl ConnectorMetrics {
    pub fn new(connector_id: impl Into<String>) -> Self {
        Self {
            connector_id: connector_id.into(),
            records_processed: 0,
            bytes_transferred: 0,
            latency_ms: 0.0,
            throughput_rps: 0.0,
            error_rate: 0.0,
            memory_used_mb: 0.0,
            errors: 0,
            retries: 0,
            circuit_breaker_trips: 0,
        }
    }

    pub fn with_records(mut self, count: u64) -> Self {
        self.records_processed = count;
        self
    }

    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes_transferred = bytes;
        self
    }

    pub fn with_latency(mut self, ms: f64) -> Self {
        self.latency_ms = ms;
        self
    }

    pub fn with_throughput(mut self, rps: f64) -> Self {
        self.throughput_rps = rps;
        self
    }

    pub fn with_error_rate(mut self, rate: f64) -> Self {
        self.error_rate = rate;
        self
    }

    pub fn with_errors(mut self, count: u64) -> Self {
        self.errors = count;
        self
    }

    /// Calculate effective throughput (records/second)
    pub fn calc_throughput(&self, duration_seconds: f64) -> f64 {
        if duration_seconds > 0.0 {
            self.records_processed as f64 / duration_seconds
        } else {
            0.0
        }
    }

    /// Calculate effective latency per record (ms)
    pub fn calc_avg_latency(&self, total_latency_ms: f64) -> f64 {
        if self.records_processed > 0 {
            total_latency_ms / self.records_processed as f64
        } else {
            0.0
        }
    }

    /// Calculate error rate percentage
    pub fn calc_error_rate_percent(&self) -> f64 {
        if self.records_processed > 0 {
            (self.errors as f64 / self.records_processed as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Generate summary string
    pub fn summary(&self) -> String {
        format!(
            "{}: {} records, {} MB, {:.1}% error rate, {:.1} records/sec",
            self.connector_id,
            self.records_processed,
            self.bytes_transferred / 1_000_000,
            self.calc_error_rate_percent(),
            self.throughput_rps
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_records() {
        let collector = MetricsCollector::new("test");
        collector.record_processed(100);
        collector.record_processed(50);

        let metrics = collector.get_metrics();
        assert_eq!(metrics.records_processed, 150);
    }

    #[test]
    fn test_metrics_collector_errors() {
        let collector = MetricsCollector::new("test");
        collector.record_processed(1000);
        collector.record_error();
        collector.record_error();
        collector.record_error();

        let metrics = collector.get_metrics();
        assert_eq!(metrics.errors, 3);
        assert!((metrics.error_rate - 0.003).abs() < 0.0001);
    }

    #[test]
    fn test_metrics_collector_clone() {
        let collector = MetricsCollector::new("test");
        collector.record_processed(100);

        let cloned = collector.clone();
        cloned.record_processed(50);

        let metrics = collector.get_metrics();
        assert_eq!(metrics.records_processed, 150);
    }

    #[test]
    fn test_metrics_reset() {
        let collector = MetricsCollector::new("test");
        collector.record_processed(100);
        collector.record_error();

        collector.reset();

        let metrics = collector.get_metrics();
        assert_eq!(metrics.records_processed, 0);
        assert_eq!(metrics.errors, 0);
    }

    #[test]
    fn test_calc_throughput() {
        let metrics = ConnectorMetrics::new("test")
            .with_records(1000);

        let throughput = metrics.calc_throughput(10.0);
        assert_eq!(throughput, 100.0); // 1000 records / 10 seconds
    }

    #[test]
    fn test_calc_avg_latency() {
        let metrics = ConnectorMetrics::new("test")
            .with_records(100);

        let avg_latency = metrics.calc_avg_latency(10000.0);
        assert_eq!(avg_latency, 100.0); // 10000 ms / 100 records
    }

    #[test]
    fn test_calc_error_rate_percent() {
        let metrics = ConnectorMetrics::new("test")
            .with_records(1000)
            .with_errors(10);

        let error_rate = metrics.calc_error_rate_percent();
        assert_eq!(error_rate, 1.0); // 10 / 1000 * 100
    }
}
