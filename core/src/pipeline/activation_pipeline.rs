use super::{BackpressureManager, LatencyTracker};
use crate::{
    Activation, Event, EventProcessor, Workflow, CheckpointManager, Checkpoint, EventType, EventSource,
};
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Metrics for pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineMetrics {
    /// Total events processed
    pub events_processed: u64,
    /// Total events failed
    pub events_failed: u64,
    /// Average latency in milliseconds
    pub average_latency_ms: f64,
    /// 99th percentile latency
    pub p99_latency_ms: u64,
    /// Events per second throughput
    pub throughput_eps: f64,
    /// Current queue depth
    pub queue_depth: usize,
}

/// Current status of pipeline
#[derive(Debug, Clone)]
pub struct PipelineStatus {
    /// Whether pipeline is actively running
    pub running: bool,
    /// Timestamp of last processed event
    pub last_event_at: Option<DateTime<Utc>>,
    /// Current metrics
    pub metrics: PipelineMetrics,
    /// Total error count
    pub error_count: u64,
}

/// End-to-end real-time activation pipeline
pub struct ActivationPipeline {
    workflow: Arc<Workflow>,
    activation: Arc<Activation>,
    event_processor: Arc<EventProcessor>,
    latency_tracker: Arc<LatencyTracker>,
    backpressure: Arc<BackpressureManager>,
    checkpoint_mgr: Arc<CheckpointManager>,
    running: Arc<AtomicBool>,
    events_processed: Arc<AtomicU64>,
    events_failed: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    last_event_at: Arc<Mutex<Option<DateTime<Utc>>>>,
    start_time: Instant,
}

impl ActivationPipeline {
    /// Create a new activation pipeline
    pub async fn new(
        workflow: Arc<Workflow>,
        activation: Arc<Activation>,
    ) -> crate::Result<Self> {
        let event_processor = Arc::new(EventProcessor::new(1000));
        let latency_tracker = Arc::new(LatencyTracker::new(10000));
        let backpressure = Arc::new(BackpressureManager::new(10000));
        let checkpoint_mgr = Arc::new(CheckpointManager::new());

        Ok(Self {
            workflow,
            activation,
            event_processor,
            latency_tracker,
            backpressure,
            checkpoint_mgr,
            running: Arc::new(AtomicBool::new(false)),
            events_processed: Arc::new(AtomicU64::new(0)),
            events_failed: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            last_event_at: Arc::new(Mutex::new(None)),
            start_time: Instant::now(),
        })
    }

    /// Start the pipeline
    pub async fn start(&self) -> crate::Result<()> {
        self.running.store(true, Ordering::Release);
        Ok(())
    }

    /// Stop the pipeline
    pub async fn stop(&self) -> crate::Result<()> {
        self.running.store(false, Ordering::Release);
        self.event_processor.flush().await?;
        Ok(())
    }

    /// Process a single event
    pub async fn process_event(&self, event: Event) -> crate::Result<()> {
        if !self.running.load(Ordering::Acquire) {
            return Err(crate::Error::ConfigError("Pipeline not running".to_string()));
        }

        // Check backpressure
        self.backpressure.acquire()?;

        let start = Instant::now();

        match self.event_processor.process_event(event).await {
            Ok(_) => {
                let latency = start.elapsed().as_millis() as u64;
                self.latency_tracker.record(latency).await;
                self.events_processed.fetch_add(1, Ordering::Release);
            }
            Err(e) => {
                self.events_failed.fetch_add(1, Ordering::Release);
                self.error_count.fetch_add(1, Ordering::Release);
                self.backpressure.release();
                return Err(crate::Error::ConfigError(format!("Event processing failed: {}", e)));
            }
        }

        self.backpressure.release();
        *self.last_event_at.lock().await = Some(Utc::now());

        Ok(())
    }

    /// Process a batch of events
    pub async fn process_batch(&self, events: Vec<Event>) -> crate::Result<usize> {
        if !self.running.load(Ordering::Acquire) {
            return Err(crate::Error::ConfigError("Pipeline not running".to_string()));
        }

        let mut successful = 0;

        for event in events {
            match self.process_event(event).await {
                Ok(_) => successful += 1,
                Err(_) => {
                    // Continue processing remaining events
                    continue;
                }
            }
        }

        Ok(successful)
    }

    /// Get current pipeline status
    pub async fn status(&self) -> PipelineStatus {
        PipelineStatus {
            running: self.running.load(Ordering::Acquire),
            last_event_at: *self.last_event_at.lock().await,
            metrics: self.metrics().await,
            error_count: self.error_count.load(Ordering::Acquire),
        }
    }

    /// Get current pipeline metrics
    pub async fn metrics(&self) -> PipelineMetrics {
        let stats = self.latency_tracker.stats().await;
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let events_processed = self.events_processed.load(Ordering::Acquire);

        let throughput_eps = if elapsed > 0.0 {
            events_processed as f64 / elapsed
        } else {
            0.0
        };

        PipelineMetrics {
            events_processed,
            events_failed: self.events_failed.load(Ordering::Acquire),
            average_latency_ms: stats.mean_ms,
            p99_latency_ms: stats.p99_ms,
            throughput_eps,
            queue_depth: self.backpressure.queue_depth(),
        }
    }

    /// Create and save a checkpoint
    pub async fn checkpoint(&self) -> crate::Result<()> {
        let checkpoint = Checkpoint::new(self.workflow.id.clone());
        self.checkpoint_mgr.save(checkpoint).await?;
        Ok(())
    }

    /// Get latest checkpoint for this workflow
    pub async fn get_checkpoint(&self) -> crate::Result<Option<Checkpoint>> {
        self.checkpoint_mgr
            .get_latest(self.workflow.id.clone())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn create_test_pipeline() -> ActivationPipeline {
        use crate::workflow::SourceType;

        let workflow = Arc::new(Workflow::new(
            "test_workflow".to_string(),
            "test_owner".to_string(),
            SourceType::Table {
                table_name: "test_table".to_string(),
            },
        ));
        let activation = Arc::new(Activation::new(
            "test_activation".to_string(),
            workflow.id.clone(),
            "test_owner".to_string(),
        ));

        ActivationPipeline::new(workflow, activation).await.unwrap()
    }

    #[tokio::test]
    async fn test_pipeline_creation() {
        let pipeline = create_test_pipeline().await;
        assert!(!pipeline.running.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn test_pipeline_start_stop() {
        let pipeline = create_test_pipeline().await;

        pipeline.start().await.unwrap();
        assert!(pipeline.running.load(Ordering::Acquire));

        pipeline.stop().await.unwrap();
        assert!(!pipeline.running.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn test_pipeline_status() {
        let pipeline = create_test_pipeline().await;
        pipeline.start().await.unwrap();

        let status = pipeline.status().await;
        assert!(status.running);
        assert_eq!(status.error_count, 0);
    }

    #[tokio::test]
    async fn test_pipeline_metrics() {
        let pipeline = create_test_pipeline().await;
        pipeline.start().await.unwrap();

        let metrics = pipeline.metrics().await;
        assert_eq!(metrics.events_processed, 0);
        assert_eq!(metrics.events_failed, 0);
    }

    #[tokio::test]
    async fn test_pipeline_checkpoint() {
        let pipeline = create_test_pipeline().await;

        pipeline.checkpoint().await.unwrap();
        let cp = pipeline.get_checkpoint().await.unwrap();

        assert!(cp.is_some());
    }

    #[tokio::test]
    async fn test_pipeline_not_running_error() {
        let pipeline = create_test_pipeline().await;

        let event = Event::new(
            EventType::EntityCreated,
            EventSource::Webhook {
                url: "http://example.com".to_string(),
            },
            json!({"id": "1"}),
        );

        let result = pipeline.process_event(event).await;
        assert!(result.is_err());
    }
}
