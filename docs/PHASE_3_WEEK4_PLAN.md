# Phase 3 Week 4: Real-Time Activation Pipeline Implementation Plan

## Objectives
- End-to-end ActivationPipeline orchestration
- LatencyTracker for performance metrics
- Error recovery & backpressure handling
- 18+ new tests (165+ total)

## New Modules

### core/src/pipeline/mod.rs
Module exports for pipeline components

### core/src/pipeline/activation_pipeline.rs
```rust
pub struct ActivationPipeline {
    workflow: Arc<Workflow>,
    destination: Arc<Destination>,
    event_processor: Arc<EventProcessor>,
    latency_tracker: Arc<LatencyTracker>,
    retry_policy: Arc<RetryPolicy>,
    checkpoint_mgr: Arc<CheckpointManager>,
}

pub struct PipelineMetrics {
    pub events_processed: u64,
    pub events_failed: u64,
    pub average_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub throughput_eps: f64,
}

pub struct PipelineStatus {
    pub running: bool,
    pub last_event_at: Option<DateTime<Utc>>,
    pub metrics: PipelineMetrics,
    pub error_count: u64,
}

impl ActivationPipeline {
    pub async fn new(
        workflow: Arc<Workflow>,
        destination: Arc<Destination>,
    ) -> Result<Self>
    
    pub async fn start(&self) -> Result<()>
    pub async fn stop(&self) -> Result<()>
    pub async fn process_event(&self, event: Event) -> Result<()>
    pub async fn process_batch(&self, events: Vec<Event>) -> Result<BatchResult>
    pub async fn status(&self) -> PipelineStatus
    pub async fn metrics(&self) -> PipelineMetrics
}
```

### core/src/pipeline/latency_tracker.rs
```rust
pub struct LatencyTracker {
    latencies: Arc<Mutex<VecDeque<u64>>>,
    max_samples: usize,
}

pub struct LatencyStats {
    pub min_ms: u64,
    pub max_ms: u64,
    pub mean_ms: f64,
    pub p50_ms: u64,
    pub p99_ms: u64,
    pub p999_ms: u64,
}

impl LatencyTracker {
    pub fn new(max_samples: usize) -> Self
    pub async fn record(&self, latency_ms: u64)
    pub async fn stats(&self) -> LatencyStats
    pub async fn reset(&self)
}
```

### core/src/pipeline/backpressure.rs
```rust
pub struct BackpressureManager {
    queue_limit: usize,
    current_load: Arc<AtomicUsize>,
}

pub enum BackpressureSignal {
    /// Process normally
    Ok,
    /// Slow down processing
    Warn,
    /// Reject new events, apply backpressure
    Reject,
}

impl BackpressureManager {
    pub fn new(queue_limit: usize) -> Self
    pub fn check_load(&self) -> BackpressureSignal
    pub async fn acquire(&self) -> Result<()>
    pub async fn release(&self)
}
```

## Tests (18+ total)

### test_activation_pipeline.rs (6 tests)
- test_pipeline_creation: Pipeline instantiated with workflow/destination
- test_pipeline_process_event: Single event processed end-to-end
- test_pipeline_process_batch: Multiple events batched and processed
- test_pipeline_status: Pipeline status reflects current state
- test_pipeline_start_stop: Pipeline lifecycle management
- test_pipeline_error_handling: Errors recovered via retry policy

### test_latency_tracker.rs (5 tests)
- test_record_latency: Latency recorded
- test_latency_stats: Stats calculated correctly
- test_percentile_calculation: P50/P99/P999 accurate
- test_latency_rolling_window: Old samples removed (max_samples limit)
- test_latency_reset: Tracker reset clears history

### test_backpressure.rs (4 tests)
- test_backpressure_ok_state: Normal load returns Ok
- test_backpressure_warn_state: High load returns Warn
- test_backpressure_reject_state: Critical load returns Reject
- test_backpressure_acquire_release: Queue slots managed correctly

### test_pipeline_integration.rs (3+ tests)
- test_end_to_end_activation: Workflow → Event → Processor → Destination
- test_pipeline_with_checkpoint: Checkpoint recovery on restart
- test_pipeline_failure_recovery: Failed events retried and recovered

## Schema Extensions

No new tables - uses existing:
- sync_runs (for tracking pipeline runs)
- sync_records (for tracking per-entity results)
- changelogs (for CDC changes)
- checkpoints (for recovery)

## Implementation Order
1. core/src/pipeline/latency_tracker.rs (5 tests)
2. core/src/pipeline/backpressure.rs (4 tests)
3. core/src/pipeline/activation_pipeline.rs (6 tests)
4. core/src/pipeline/mod.rs
5. Update core/src/lib.rs to export pipeline module
6. test_pipeline_integration.rs (3 tests)
7. All tests passing, commit

## Success Criteria
- ✅ 18+ new tests passing
- ✅ 165+ total tests passing (142 + 9 CDC + 18 pipeline)
- ✅ End-to-end pipeline operational
- ✅ Latency tracking accurate (P99 < 1s for most operations)
- ✅ Backpressure prevents OOM
- ✅ Checkpoint recovery working
- ✅ Ready for v1.5.0 release

## Performance Targets
- Event latency: < 100ms median (< 1s P99)
- Throughput: 1,000+ events/sec
- Backpressure trigger: 80% queue utilization
- Rejection threshold: 95% queue utilization
