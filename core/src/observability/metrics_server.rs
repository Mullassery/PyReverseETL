/// Metrics server for dashboard communication
/// Provides HTTP endpoint to stream pipeline metrics in real-time

use crate::pipeline::PipelineMetrics;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub events_processed: u64,
    pub events_failed: u64,
    pub average_latency_ms: f64,
    pub p99_latency_ms: u64,
    pub throughput_eps: f64,
    pub queue_depth: usize,
    pub quality_checks_passed: u64,
    pub quality_checks_failed: u64,
    pub schema_changes_detected: u64,
    pub compliance_rules_applied: u64,
    pub error_count: u64,
}

impl From<(&PipelineMetrics, u64)> for MetricsSnapshot {
    fn from((metrics, error_count): (&PipelineMetrics, u64)) -> Self {
        Self {
            timestamp: Utc::now(),
            events_processed: metrics.events_processed,
            events_failed: metrics.events_failed,
            average_latency_ms: metrics.average_latency_ms,
            p99_latency_ms: metrics.p99_latency_ms,
            throughput_eps: metrics.throughput_eps,
            queue_depth: metrics.queue_depth,
            quality_checks_passed: metrics.quality_checks_passed,
            quality_checks_failed: metrics.quality_checks_failed,
            schema_changes_detected: metrics.schema_changes_detected,
            compliance_rules_applied: metrics.compliance_rules_applied,
            error_count,
        }
    }
}

/// Metrics history for trend analysis
#[derive(Debug, Clone)]
pub struct MetricsHistory {
    pub snapshots: Vec<MetricsSnapshot>,
    pub max_size: usize,
}

impl MetricsHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            snapshots: Vec::new(),
            max_size,
        }
    }

    pub fn add(&mut self, snapshot: MetricsSnapshot) {
        self.snapshots.push(snapshot);
        if self.snapshots.len() > self.max_size {
            self.snapshots.remove(0);
        }
    }

    pub fn latest(&self) -> Option<&MetricsSnapshot> {
        self.snapshots.last()
    }

    pub fn trend_throughput(&self) -> Option<f64> {
        if self.snapshots.len() < 2 {
            return None;
        }
        let recent = &self.snapshots[self.snapshots.len() - 1];
        let older = &self.snapshots[self.snapshots.len() - 2];
        Some(recent.throughput_eps - older.throughput_eps)
    }
}

/// Metrics server that manages pipeline metrics
pub struct MetricsServer {
    history: Arc<RwLock<MetricsHistory>>,
    current: Arc<RwLock<Option<MetricsSnapshot>>>,
}

impl MetricsServer {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(MetricsHistory::new(300))), // 5 min history @ 1sec intervals
            current: Arc::new(RwLock::new(None)),
        }
    }

    /// Record new metrics snapshot
    pub async fn record_metrics(&self, metrics: &PipelineMetrics, error_count: u64) {
        let snapshot = MetricsSnapshot::from((metrics, error_count));
        let mut history = self.history.write().await;
        history.add(snapshot.clone());
        let mut current = self.current.write().await;
        *current = Some(snapshot);
    }

    /// Get current metrics snapshot
    pub async fn get_current(&self) -> Option<MetricsSnapshot> {
        self.current.read().await.clone()
    }

    /// Get metrics history
    pub async fn get_history(&self) -> Vec<MetricsSnapshot> {
        self.history.read().await.snapshots.clone()
    }

    /// Get latest N snapshots
    pub async fn get_last_n(&self, n: usize) -> Vec<MetricsSnapshot> {
        let history = self.history.read().await;
        let start = if history.snapshots.len() > n {
            history.snapshots.len() - n
        } else {
            0
        };
        history.snapshots[start..].to_vec()
    }

    /// Calculate average metrics over a time window
    pub async fn average_over_window(&self, window_secs: usize) -> Option<MetricsSnapshot> {
        let history = self.history.read().await;
        if history.snapshots.is_empty() {
            return None;
        }

        let now = Utc::now();
        let cutoff = now - chrono::Duration::seconds(window_secs as i64);

        let filtered: Vec<_> = history
            .snapshots
            .iter()
            .filter(|s| s.timestamp >= cutoff)
            .collect();

        if filtered.is_empty() {
            return None;
        }

        let count = filtered.len() as f64;
        let avg_events_processed = filtered.iter().map(|s| s.events_processed).sum::<u64>() as f64 / count;
        let avg_throughput = filtered.iter().map(|s| s.throughput_eps).sum::<f64>() / count;
        let avg_latency = filtered.iter().map(|s| s.average_latency_ms).sum::<f64>() / count;
        let avg_p99_latency = filtered.iter().map(|s| s.p99_latency_ms as f64).sum::<f64>() / count;

        Some(MetricsSnapshot {
            timestamp: now,
            events_processed: avg_events_processed as u64,
            throughput_eps: avg_throughput,
            average_latency_ms: avg_latency,
            p99_latency_ms: avg_p99_latency as u64,
            ..filtered[0].clone()
        })
    }
}

impl Default for MetricsServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_history() {
        let mut history = MetricsHistory::new(3);

        let snap1 = MetricsSnapshot {
            timestamp: Utc::now(),
            events_processed: 100,
            events_failed: 0,
            average_latency_ms: 50.0,
            p99_latency_ms: 100,
            throughput_eps: 100.0,
            queue_depth: 5,
            quality_checks_passed: 100,
            quality_checks_failed: 0,
            schema_changes_detected: 0,
            compliance_rules_applied: 100,
            error_count: 0,
        };

        history.add(snap1.clone());
        assert_eq!(history.latest().unwrap().events_processed, 100);
        assert_eq!(history.snapshots.len(), 1);
    }

    #[test]
    fn test_metrics_history_max_size() {
        let mut history = MetricsHistory::new(2);

        for i in 1..=5 {
            let snap = MetricsSnapshot {
                timestamp: Utc::now(),
                events_processed: i as u64 * 100,
                events_failed: 0,
                average_latency_ms: 50.0,
                p99_latency_ms: 100,
                throughput_eps: i as f64 * 100.0,
                queue_depth: 5,
                quality_checks_passed: i as u64 * 100,
                quality_checks_failed: 0,
                schema_changes_detected: 0,
                compliance_rules_applied: i as u64 * 100,
                error_count: 0,
            };
            history.add(snap);
        }

        assert_eq!(history.snapshots.len(), 2);
        assert_eq!(history.latest().unwrap().events_processed, 500);
    }

    #[tokio::test]
    async fn test_metrics_server() {
        let server = MetricsServer::new();

        let metrics = PipelineMetrics {
            events_processed: 1000,
            events_failed: 10,
            average_latency_ms: 50.0,
            p99_latency_ms: 100,
            throughput_eps: 1000.0,
            queue_depth: 10,
            quality_checks_passed: 990,
            quality_checks_failed: 10,
            schema_changes_detected: 5,
            compliance_rules_applied: 1000,
        };

        server.record_metrics(&metrics, 5).await;

        let current = server.get_current().await;
        assert!(current.is_some());
        assert_eq!(current.unwrap().events_processed, 1000);
    }

    #[tokio::test]
    async fn test_metrics_server_history() {
        let server = MetricsServer::new();

        for i in 1..=3 {
            let metrics = PipelineMetrics {
                events_processed: i * 100,
                events_failed: i,
                average_latency_ms: 50.0,
                p99_latency_ms: 100,
                throughput_eps: (i * 100) as f64,
                queue_depth: 10,
                quality_checks_passed: (i * 100) - i,
                quality_checks_failed: i,
                schema_changes_detected: 0,
                compliance_rules_applied: i * 100,
            };
            server.record_metrics(&metrics, i as u64).await;
        }

        let history = server.get_history().await;
        assert_eq!(history.len(), 3);
    }
}
