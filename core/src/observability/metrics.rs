/// OpenTelemetry metrics for sync operations
/// Track throughput, latency, errors, and other KPIs

use std::sync::Arc;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};

/// Metrics collector for a sync operation
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// Sync run ID
    pub run_id: String,
    /// Sync name
    pub sync_name: String,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,
    /// Status: Running, Success, Failed, Partial
    pub status: String,
    /// Number of events processed
    pub events_processed: u64,
    /// Number of events failed
    pub events_failed: u64,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Transformation duration (milliseconds)
    pub transform_duration_ms: Option<u64>,
    /// Write duration (milliseconds)
    pub write_duration_ms: Option<u64>,
    /// Peak throughput (events/sec)
    pub peak_throughput: Option<f64>,
    /// Average latency per event (milliseconds)
    pub avg_latency_ms: Option<f64>,
}

impl MetricsCollector {
    pub fn new(run_id: String, sync_name: String) -> Self {
        Self {
            run_id,
            sync_name,
            start_time: Utc::now(),
            end_time: None,
            status: "Running".to_string(),
            events_processed: 0,
            events_failed: 0,
            bytes_processed: 0,
            transform_duration_ms: None,
            write_duration_ms: None,
            peak_throughput: None,
            avg_latency_ms: None,
        }
    }

    /// Mark sync as completed successfully
    pub fn mark_success(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = "Success".to_string();
    }

    /// Mark sync as failed
    pub fn mark_failed(&mut self, reason: &str) {
        self.end_time = Some(Utc::now());
        self.status = format!("Failed: {}", reason);
    }

    /// Mark sync as partially completed
    pub fn mark_partial(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = "Partial".to_string();
    }

    /// Record events processed
    pub fn add_events_processed(&mut self, count: u64, bytes: u64) {
        self.events_processed += count;
        self.bytes_processed += bytes;
    }

    /// Record failed events
    pub fn add_events_failed(&mut self, count: u64) {
        self.events_failed += count;
    }

    /// Calculate throughput (events per second)
    pub fn throughput(&self) -> f64 {
        if let Some(end) = self.end_time {
            let duration = end - self.start_time;
            let secs = duration.num_seconds() as f64;
            if secs > 0.0 {
                self.events_processed as f64 / secs
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Calculate total duration
    pub fn total_duration(&self) -> Duration {
        let end = self.end_time.unwrap_or_else(Utc::now);
        Duration::from_secs_f64((end - self.start_time).num_seconds() as f64)
    }

    /// Error rate (percentage)
    pub fn error_rate(&self) -> f64 {
        let total = self.events_processed + self.events_failed;
        if total > 0 {
            (self.events_failed as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Success rate (percentage)
    pub fn success_rate(&self) -> f64 {
        100.0 - self.error_rate()
    }

    /// Format metrics for logging
    pub fn summary(&self) -> String {
        format!(
            "Sync Summary: {} [{}] | Events: {}/{} | Duration: {}s | Throughput: {:.1} evt/sec | Rate: {:.1}%",
            self.sync_name,
            self.status,
            self.events_processed,
            self.events_processed + self.events_failed,
            self.total_duration().as_secs(),
            self.throughput(),
            self.success_rate()
        )
    }
}

/// Sync metrics snapshot for reporting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncMetrics {
    pub run_id: String,
    pub sync_name: String,
    pub status: String,
    pub events_processed: u64,
    pub events_failed: u64,
    pub error_rate_percent: f64,
    pub success_rate_percent: f64,
    pub throughput_events_per_sec: f64,
    pub duration_seconds: u64,
    pub bytes_processed: u64,
    pub start_time: String,
    pub end_time: Option<String>,
}

impl From<&MetricsCollector> for SyncMetrics {
    fn from(collector: &MetricsCollector) -> Self {
        SyncMetrics {
            run_id: collector.run_id.clone(),
            sync_name: collector.sync_name.clone(),
            status: collector.status.clone(),
            events_processed: collector.events_processed,
            events_failed: collector.events_failed,
            error_rate_percent: collector.error_rate(),
            success_rate_percent: collector.success_rate(),
            throughput_events_per_sec: collector.throughput(),
            duration_seconds: collector.total_duration().as_secs(),
            bytes_processed: collector.bytes_processed,
            start_time: collector.start_time.to_rfc3339(),
            end_time: collector.end_time.map(|t| t.to_rfc3339()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new(
            "run-123".to_string(),
            "test_sync".to_string(),
        );
        assert_eq!(collector.status, "Running");
        assert_eq!(collector.events_processed, 0);
        assert_eq!(collector.events_failed, 0);
    }

    #[test]
    fn test_metrics_collector_processing() {
        let mut collector = MetricsCollector::new(
            "run-123".to_string(),
            "test_sync".to_string(),
        );

        collector.add_events_processed(1000, 50000);
        assert_eq!(collector.events_processed, 1000);
        assert_eq!(collector.bytes_processed, 50000);

        collector.add_events_failed(10);
        assert_eq!(collector.events_failed, 10);
    }

    #[test]
    fn test_error_rate_calculation() {
        let mut collector = MetricsCollector::new(
            "run-123".to_string(),
            "test_sync".to_string(),
        );

        collector.add_events_processed(900, 0);
        collector.add_events_failed(100);

        assert!((collector.error_rate() - 10.0).abs() < 0.1);
        assert!((collector.success_rate() - 90.0).abs() < 0.1);
    }

    #[test]
    fn test_metrics_summary() {
        let mut collector = MetricsCollector::new(
            "run-123".to_string(),
            "test_sync".to_string(),
        );

        collector.add_events_processed(1000, 50000);
        collector.mark_success();

        let summary = collector.summary();
        assert!(summary.contains("test_sync"));
        assert!(summary.contains("Success"));
        assert!(summary.contains("1000"));
    }

    #[test]
    fn test_sync_metrics_snapshot() {
        let mut collector = MetricsCollector::new(
            "run-456".to_string(),
            "orders_sync".to_string(),
        );

        collector.add_events_processed(5000, 250000);
        collector.add_events_failed(50);
        collector.mark_success();

        let metrics: SyncMetrics = (&collector).into();
        assert_eq!(metrics.run_id, "run-456");
        assert_eq!(metrics.sync_name, "orders_sync");
        assert_eq!(metrics.events_processed, 5000);
        assert_eq!(metrics.events_failed, 50);
    }
}
