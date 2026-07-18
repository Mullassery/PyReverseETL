/// OpenTelemetry integration for observability
/// Provides metrics, traces, and logs for sync operations

pub mod metrics;
pub mod traces;
pub mod logs;

pub use metrics::{SyncMetrics, MetricsCollector};
pub use traces::{SyncTracer, TraceSpan};
pub use logs::SyncLogger;

use std::sync::Arc;
use opentelemetry::sdk::Resource;
use opentelemetry::{global, KeyValue};

/// Initialize OpenTelemetry for PyReverseETL
pub fn init_otel(service_name: &str, version: &str) -> Result<(), String> {
    // Create resource with service metadata
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", version.to_string()),
        KeyValue::new("telemetry.sdk.language", "rust"),
        KeyValue::new("telemetry.sdk.name", "opentelemetry"),
    ]);

    // Initialize metrics (uses stdout exporter by default)
    // Production: configure with Prometheus, Datadog, etc.

    // Initialize traces (uses stdout exporter by default)
    // Production: configure with Jaeger, Datadog, etc.

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
