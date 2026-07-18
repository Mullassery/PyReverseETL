/// OpenTelemetry traces for distributed tracing
/// Track execution flow from source through transformation to destination

use std::time::Instant;
use chrono::{DateTime, Utc};

/// Span for tracing a specific operation
#[derive(Debug, Clone)]
pub struct TraceSpan {
    /// Span ID
    pub span_id: String,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Span name (e.g., "read_source", "transform", "write_destination")
    pub operation: String,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Status: OK, FAILED
    pub status: String,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Attributes (key-value pairs)
    pub attributes: std::collections::HashMap<String, String>,
}

impl TraceSpan {
    pub fn new(parent_span_id: Option<String>, operation: &str) -> Self {
        Self {
            span_id: uuid::Uuid::new_v4().to_string(),
            parent_span_id,
            operation: operation.to_string(),
            start_time: Utc::now(),
            end_time: None,
            status: "Running".to_string(),
            error_message: None,
            attributes: std::collections::HashMap::new(),
        }
    }

    /// Mark span as successfully completed
    pub fn mark_ok(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = "OK".to_string();
    }

    /// Mark span as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.end_time = Some(Utc::now());
        self.status = "FAILED".to_string();
        self.error_message = Some(error.to_string());
    }

    /// Add attribute to span
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> u64 {
        let end = self.end_time.unwrap_or_else(Utc::now);
        ((end - self.start_time).num_milliseconds()) as u64
    }
}

/// Sync tracer for distributed tracing
pub struct SyncTracer {
    /// Root span
    root_span: TraceSpan,
    /// Child spans
    child_spans: Vec<TraceSpan>,
}

impl SyncTracer {
    pub fn new(sync_run_id: &str) -> Self {
        let mut root = TraceSpan::new(None, "sync_run");
        root.add_attribute("sync_run_id", sync_run_id);

        Self {
            root_span: root,
            child_spans: Vec::new(),
        }
    }

    /// Create a child span for an operation
    pub fn create_span(&mut self, operation: &str) -> TraceSpan {
        TraceSpan::new(Some(self.root_span.span_id.clone()), operation)
    }

    /// Record a span
    pub fn record_span(&mut self, span: TraceSpan) {
        self.child_spans.push(span);
    }

    /// Mark root span as completed
    pub fn complete(&mut self, status: &str) {
        self.root_span.status = status.to_string();
        self.root_span.end_time = Some(Utc::now());
    }

    /// Get trace summary
    pub fn summary(&self) -> TraceSummary {
        let mut total_duration_ms = 0u64;
        let mut error_count = 0u32;
        let mut operation_durations = std::collections::HashMap::new();

        for span in &self.child_spans {
            let duration = span.duration_ms();
            total_duration_ms += duration;

            if span.status == "FAILED" {
                error_count += 1;
            }

            operation_durations
                .entry(span.operation.clone())
                .and_modify(|d: &mut u64| *d += duration)
                .or_insert(duration);
        }

        TraceSummary {
            root_span_id: self.root_span.span_id.clone(),
            total_spans: self.child_spans.len() as u32,
            failed_spans: error_count,
            total_duration_ms,
            operation_durations,
        }
    }
}

/// Summary of a trace
#[derive(Debug, Clone)]
pub struct TraceSummary {
    pub root_span_id: String,
    pub total_spans: u32,
    pub failed_spans: u32,
    pub total_duration_ms: u64,
    pub operation_durations: std::collections::HashMap<String, u64>,
}

impl TraceSummary {
    pub fn success_rate(&self) -> f64 {
        if self.total_spans > 0 {
            ((self.total_spans - self.failed_spans) as f64 / self.total_spans as f64) * 100.0
        } else {
            100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_span_creation() {
        let span = TraceSpan::new(None, "read_source");
        assert_eq!(span.operation, "read_source");
        assert_eq!(span.status, "Running");
        assert!(span.parent_span_id.is_none());
    }

    #[test]
    fn test_trace_span_completion() {
        let mut span = TraceSpan::new(None, "transform");
        span.mark_ok();

        assert_eq!(span.status, "OK");
        assert!(span.end_time.is_some());
    }

    #[test]
    fn test_trace_span_failure() {
        let mut span = TraceSpan::new(None, "write");
        span.mark_failed("Connection timeout");

        assert_eq!(span.status, "FAILED");
        assert_eq!(span.error_message, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_sync_tracer() {
        let mut tracer = SyncTracer::new("run-789");

        let mut span1 = tracer.create_span("read_source");
        span1.mark_ok();
        tracer.record_span(span1);

        let mut span2 = tracer.create_span("transform");
        span2.mark_ok();
        tracer.record_span(span2);

        let mut span3 = tracer.create_span("write");
        span3.mark_ok();
        tracer.record_span(span3);

        tracer.complete("Success");

        let summary = tracer.summary();
        assert_eq!(summary.total_spans, 3);
        assert_eq!(summary.failed_spans, 0);
        assert!(summary.success_rate() > 99.9);
    }

    #[test]
    fn test_span_attributes() {
        let mut span = TraceSpan::new(None, "read_source");
        span.add_attribute("source_type", "kafka");
        span.add_attribute("topic", "orders");

        assert_eq!(span.attributes.get("source_type"), Some(&"kafka".to_string()));
        assert_eq!(span.attributes.get("topic"), Some(&"orders".to_string()));
    }
}
