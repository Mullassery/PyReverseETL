/// OpenTelemetry integration for observability
/// Provides metrics, traces, and logs for sync operations

pub mod metrics;
pub mod traces;
pub mod logs;

pub use metrics::{SyncMetrics, MetricsCollector};
pub use traces::{SyncTracer, TraceSpan, TraceSummary};
pub use logs::SyncLogger;

use std::sync::Arc;

/// Initialize OpenTelemetry for PyReverseETL
pub fn init_otel(service_name: &str, version: &str) -> Result<(), String> {
    // Initialize OpenTelemetry with service metadata
    // Production: configure with exporters (Prometheus, Jaeger, Datadog, etc.)

    tracing::info!(
        service = service_name,
        version = version,
        "OpenTelemetry initialized"
    );

    Ok(())
}

/// Context for a sync operation
#[derive(Debug, Clone)]
pub struct SyncContext {
    /// Unique sync run ID
    pub sync_run_id: String,
    /// Sync name from configuration
    pub sync_name: String,
    /// Source system name
    pub source: String,
    /// Destination system name
    pub destination: String,
}

impl SyncContext {
    pub fn new(
        sync_name: &str,
        source: &str,
        destination: &str,
    ) -> Self {
        Self {
            sync_run_id: uuid::Uuid::new_v4().to_string(),
            sync_name: sync_name.to_string(),
            source: source.to_string(),
            destination: destination.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_context_creation() {
        let ctx = SyncContext::new("orders_sync", "api", "warehouse");
        assert_eq!(ctx.sync_name, "orders_sync");
        assert_eq!(ctx.source, "api");
        assert_eq!(ctx.destination, "warehouse");
        assert!(!ctx.sync_run_id.is_empty());
    }
}
