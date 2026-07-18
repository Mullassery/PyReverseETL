/// Structured logging for sync operations
/// Integrates with tracing crate for OTel compatibility

use tracing::{info, warn, error, debug};

/// Logger for sync operations
pub struct SyncLogger;

impl SyncLogger {
    /// Log sync started
    pub fn sync_started(sync_name: &str, run_id: &str, source: &str, destination: &str) {
        info!(
            sync_name = sync_name,
            run_id = run_id,
            source = source,
            destination = destination,
            "Sync started"
        );
    }

    /// Log sync completed successfully
    pub fn sync_completed(
        sync_name: &str,
        run_id: &str,
        events_processed: u64,
        duration_secs: u64,
        throughput: f64,
    ) {
        info!(
            sync_name = sync_name,
            run_id = run_id,
            events_processed = events_processed,
            duration_secs = duration_secs,
            throughput_events_per_sec = throughput,
            "Sync completed successfully"
        );
    }

    /// Log sync failed
    pub fn sync_failed(sync_name: &str, run_id: &str, reason: &str) {
        error!(
            sync_name = sync_name,
            run_id = run_id,
            reason = reason,
            "Sync failed"
        );
    }

    /// Log partial sync (some events failed)
    pub fn sync_partial(
        sync_name: &str,
        run_id: &str,
        processed: u64,
        failed: u64,
        error_rate: f64,
    ) {
        warn!(
            sync_name = sync_name,
            run_id = run_id,
            events_processed = processed,
            events_failed = failed,
            error_rate_percent = error_rate,
            "Sync completed with errors"
        );
    }

    /// Log check source started
    pub fn source_check_started(sync_name: &str, source: &str) {
        debug!(
            sync_name = sync_name,
            source = source,
            "Checking source for changes"
        );
    }

    /// Log events found
    pub fn events_found(count: u64, bytes: u64) {
        debug!(
            event_count = count,
            bytes = bytes,
            "Found events to process"
        );
    }

    /// Log transformation started
    pub fn transformation_started(sync_name: &str, transform_type: &str) {
        debug!(
            sync_name = sync_name,
            transform_type = transform_type,
            "Transformation started"
        );
    }

    /// Log transformation completed
    pub fn transformation_completed(duration_ms: u64, events_transformed: u64) {
        debug!(
            duration_ms = duration_ms,
            events_transformed = events_transformed,
            "Transformation completed"
        );
    }

    /// Log transformation failed
    pub fn transformation_failed(reason: &str, event_count: u64) {
        error!(
            reason = reason,
            failed_count = event_count,
            "Transformation failed"
        );
    }

    /// Log write started
    pub fn write_started(destination: &str, event_count: u64) {
        debug!(
            destination = destination,
            event_count = event_count,
            "Writing to destination"
        );
    }

    /// Log write completed
    pub fn write_completed(duration_ms: u64, written_count: u64) {
        debug!(
            duration_ms = duration_ms,
            written_count = written_count,
            "Write completed"
        );
    }

    /// Log write failed
    pub fn write_failed(destination: &str, reason: &str, failed_count: u64) {
        error!(
            destination = destination,
            reason = reason,
            failed_count = failed_count,
            "Write to destination failed"
        );
    }

    /// Log rate limiting detected
    pub fn rate_limited(destination: &str, retry_after_secs: u64) {
        warn!(
            destination = destination,
            retry_after_secs = retry_after_secs,
            "Rate limited by destination - will retry"
        );
    }

    /// Log retry attempt
    pub fn retry_attempt(attempt: u32, max_retries: u32, reason: &str) {
        info!(
            attempt = attempt,
            max_retries = max_retries,
            reason = reason,
            "Retrying operation"
        );
    }

    /// Log retry exhausted
    pub fn retry_exhausted(operation: &str, max_attempts: u32) {
        error!(
            operation = operation,
            max_attempts = max_attempts,
            "Max retry attempts exhausted - sending to dead letter queue"
        );
    }

    /// Log error tracked
    pub fn error_tracked(sync_name: &str, run_id: &str, error_type: &str, count: u64) {
        info!(
            sync_name = sync_name,
            run_id = run_id,
            error_type = error_type,
            error_count = count,
            "Errors tracked in dead letter queue"
        );
    }

    /// Log recovery from cache
    pub fn recovered_from_cache(sync_name: &str, event_count: u64) {
        info!(
            sync_name = sync_name,
            event_count = event_count,
            "Recovered events from cache after failure"
        );
    }

    /// Log configuration loaded
    pub fn config_loaded(sync_name: &str, source: &str, destination: &str) {
        info!(
            sync_name = sync_name,
            source = source,
            destination = destination,
            "Configuration loaded and validated"
        );
    }

    /// Log configuration validation error
    pub fn config_invalid(sync_name: &str, reason: &str) {
        error!(
            sync_name = sync_name,
            reason = reason,
            "Configuration validation failed"
        );
    }

    /// Log scaling event
    pub fn scaling(component: &str, from_value: u32, to_value: u32, reason: &str) {
        info!(
            component = component,
            from_value = from_value,
            to_value = to_value,
            reason = reason,
            "Auto-scaling triggered"
        );
    }

    /// Log performance metrics
    pub fn performance_metrics(
        sync_name: &str,
        throughput: f64,
        latency_ms: f64,
        error_rate: f64,
    ) {
        info!(
            sync_name = sync_name,
            throughput_events_per_sec = throughput,
            avg_latency_ms = latency_ms,
            error_rate_percent = error_rate,
            "Performance metrics"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_can_be_instantiated() {
        let _logger = SyncLogger;
        // Logger methods should not panic
    }

    #[test]
    fn test_sync_completed_logging() {
        // Just verify these methods compile and don't panic
        SyncLogger::sync_completed("test_sync", "run-123", 1000, 60, 16.67);
    }

    #[test]
    fn test_transformation_logging() {
        SyncLogger::transformation_started("test_sync", "Python");
        SyncLogger::transformation_completed(5000, 1000);
    }

    #[test]
    fn test_error_tracking() {
        SyncLogger::error_tracked("test_sync", "run-123", "NetworkError", 50);
    }
}
