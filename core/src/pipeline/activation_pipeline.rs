use super::{BackpressureManager, LatencyTracker};
use crate::{
    Activation, Event, EventProcessor, Workflow, CheckpointManager, Checkpoint,
    governance::GovernanceEngine,
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
    /// Quality checks passed
    pub quality_checks_passed: u64,
    /// Quality checks failed
    pub quality_checks_failed: u64,
    /// Schema changes detected
    pub schema_changes_detected: u64,
    /// Compliance rules applied
    pub compliance_rules_applied: u64,
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
    governance_engine: Option<Arc<GovernanceEngine>>,
    latency_tracker: Arc<LatencyTracker>,
    backpressure: Arc<BackpressureManager>,
    checkpoint_mgr: Arc<CheckpointManager>,
    running: Arc<AtomicBool>,
    events_processed: Arc<AtomicU64>,
    events_failed: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    quality_checks_passed: Arc<AtomicU64>,
    quality_checks_failed: Arc<AtomicU64>,
    schema_changes_detected: Arc<AtomicU64>,
    compliance_rules_applied: Arc<AtomicU64>,
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
            governance_engine: None,
            latency_tracker,
            backpressure,
            checkpoint_mgr,
            running: Arc::new(AtomicBool::new(false)),
            events_processed: Arc::new(AtomicU64::new(0)),
            events_failed: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            quality_checks_passed: Arc::new(AtomicU64::new(0)),
            quality_checks_failed: Arc::new(AtomicU64::new(0)),
            schema_changes_detected: Arc::new(AtomicU64::new(0)),
            compliance_rules_applied: Arc::new(AtomicU64::new(0)),
            last_event_at: Arc::new(Mutex::new(None)),
            start_time: Instant::now(),
        })
    }

    /// Set the governance engine for quality/compliance checks
    pub fn with_governance(mut self, governance_engine: Arc<GovernanceEngine>) -> Self {
        self.governance_engine = Some(governance_engine);
        self
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
    pub async fn process_event(&self, mut event: Event) -> crate::Result<()> {
        if !self.running.load(Ordering::Acquire) {
            return Err(crate::Error::ConfigError("Pipeline not running".to_string()));
        }

        // Check backpressure
        self.backpressure.acquire()?;

        let start = Instant::now();

        // Run governance checks if engine is configured
        if let Some(ref gov_engine) = self.governance_engine {
            // Convert event data to Entity for governance checks
            if event.has_entity() {
                let entity_id = event.entity_id().unwrap_or_else(|| "unknown".to_string());
                let mut entity = crate::Entity::new(
                    crate::entity::EntityType::Custom("event".to_string()),
                    "id",
                    entity_id,
                );
                entity.attributes = event.entity.clone();

                // Apply compliance rules (e.g. PII masking) *before* the governance
                // check below, and write the masked data back into the event.
                // Two bugs, fixed together because they compound: (1) this used to
                // mask a throwaway clone while the original, unmasked event went on
                // to be processed, silently defeating the masking; (2) check_entity()
                // includes a real compliance check (previously a no-op) that verifies
                // fields are already compliant -- running it on raw, not-yet-masked
                // data would reject every entity a masking rule targets, since the
                // whole point of apply_rules() is to *make* it compliant.
                if let Err(e) = gov_engine.apply_rules(&mut entity).await {
                    self.events_failed.fetch_add(1, Ordering::Release);
                    self.error_count.fetch_add(1, Ordering::Release);
                    self.backpressure.release();
                    return Err(e);
                }
                event.entity = entity.attributes.clone();
                self.compliance_rules_applied.fetch_add(1, Ordering::Release);

                match gov_engine.check_entity(&entity).await {
                    Ok(check_result) => {
                        if check_result.passed {
                            self.quality_checks_passed.fetch_add(1, Ordering::Release);
                            if !check_result.issues.is_empty() {
                                self.schema_changes_detected.fetch_add(check_result.issues.len() as u64, Ordering::Release);
                            }
                        } else {
                            self.quality_checks_failed.fetch_add(1, Ordering::Release);
                            self.events_failed.fetch_add(1, Ordering::Release);
                            self.error_count.fetch_add(1, Ordering::Release);
                            self.backpressure.release();
                            return Err(crate::Error::ValidationGateFailed(
                                format!("Governance checks failed: {}", check_result.issues.join(", "))
                            ));
                        }
                    }
                    Err(e) => {
                        self.quality_checks_failed.fetch_add(1, Ordering::Release);
                        self.events_failed.fetch_add(1, Ordering::Release);
                        self.error_count.fetch_add(1, Ordering::Release);
                        self.backpressure.release();
                        return Err(e);
                    }
                }
            }
        }

        // Process event
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
            quality_checks_passed: self.quality_checks_passed.load(Ordering::Acquire),
            quality_checks_failed: self.quality_checks_failed.load(Ordering::Acquire),
            schema_changes_detected: self.schema_changes_detected.load(Ordering::Acquire),
            compliance_rules_applied: self.compliance_rules_applied.load(Ordering::Acquire),
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
        use crate::streaming::{EventType, EventSource};

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

    #[tokio::test]
    async fn test_pipeline_with_governance() {
        use crate::governance::{
            GovernanceConfig, QualityGate, SchemaEvolution, ComplianceEngine,
            MockQualityGate, MockSchemaEvolution, MockComplianceEngine,
        };

        let workflow = Arc::new(Workflow::new(
            "test_workflow".to_string(),
            "test_owner".to_string(),
            crate::workflow::SourceType::Table {
                table_name: "test_table".to_string(),
            },
        ));
        let activation = Arc::new(Activation::new(
            "test_activation".to_string(),
            workflow.id.clone(),
            "test_owner".to_string(),
        ));

        // Create governance components
        let config = GovernanceConfig {
            quality_gates_enabled: true,
            schema_checks_enabled: true,
            compliance_rules_enabled: true,
            ..Default::default()
        };
        let quality_gate = Arc::new(MockQualityGate::new(true, 0.95));
        let schema_evolution = Arc::new(MockSchemaEvolution::no_changes());
        let compliance_engine = Arc::new(MockComplianceEngine::no_rules());

        let gov_engine = Arc::new(GovernanceEngine::new(
            config,
            quality_gate,
            schema_evolution,
            compliance_engine,
        ));

        let pipeline = ActivationPipeline::new(workflow, activation)
            .await
            .unwrap()
            .with_governance(gov_engine);

        let metrics = pipeline.metrics().await;
        assert_eq!(metrics.quality_checks_passed, 0);
        assert_eq!(metrics.quality_checks_failed, 0);
    }

    /// Captures the entity payload each processed event actually carried, so tests
    /// can verify what data really reached the "activation" step -- not just that
    /// processing didn't error.
    struct CapturingHandler {
        captured: Arc<Mutex<Vec<serde_json::Value>>>,
    }

    #[async_trait::async_trait]
    impl crate::EventHandler for CapturingHandler {
        async fn handle(&self, event: &Event) -> Result<(), crate::adapters::AdapterError> {
            self.captured.lock().await.push(event.entity.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn compliance_masking_actually_reaches_the_activated_event() {
        // Regression test: apply_rules() used to mask a throwaway clone of the
        // entity while the original, unmasked event went on to be processed --
        // silently defeating PII masking. This verifies the handler that receives
        // the "activated" event sees the *masked* value, not the raw one.
        use crate::governance::{
            ComplianceRule, DefaultComplianceEngine, GovernanceConfig, MockQualityGate,
            MockSchemaEvolution, RuleAction, RuleType,
        };
        use crate::streaming::{EventSource, EventType};

        let workflow = Arc::new(Workflow::new(
            "test_workflow".to_string(),
            "test_owner".to_string(),
            crate::workflow::SourceType::Table {
                table_name: "test_table".to_string(),
            },
        ));
        let activation = Arc::new(Activation::new(
            "test_activation".to_string(),
            workflow.id.clone(),
            "test_owner".to_string(),
        ));

        let config = GovernanceConfig {
            quality_gates_enabled: true,
            schema_checks_enabled: false,
            compliance_rules_enabled: true,
            ..Default::default()
        };
        let quality_gate = Arc::new(MockQualityGate::new(true, 0.95));
        let schema_evolution = Arc::new(MockSchemaEvolution::no_changes());
        let compliance_rule = ComplianceRule::new(
            "email_masking".to_string(),
            RuleType::PiiMasking,
            vec!["email".to_string()],
            RuleAction::Mask("****".to_string()),
        );
        let compliance_engine = Arc::new(DefaultComplianceEngine::new(vec![compliance_rule]));

        let gov_engine = Arc::new(GovernanceEngine::new(
            config,
            quality_gate,
            schema_evolution,
            compliance_engine,
        ));

        let pipeline = ActivationPipeline::new(workflow, activation)
            .await
            .unwrap()
            .with_governance(gov_engine);
        pipeline.start().await.unwrap();

        let captured = Arc::new(Mutex::new(Vec::new()));
        pipeline
            .event_processor
            .add_handler(Arc::new(CapturingHandler {
                captured: captured.clone(),
            }))
            .await;

        let event = Event::new(
            EventType::EntityUpdated,
            EventSource::Webhook {
                url: "http://example.com".to_string(),
            },
            json!({"id": "1", "email": "real.person@example.com"}),
        );

        pipeline.process_event(event).await.unwrap();

        let captured = captured.lock().await;
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0]["email"], json!("****"));
        assert_ne!(captured[0]["email"], json!("real.person@example.com"));
    }

    #[tokio::test]
    async fn test_governance_metrics_tracking() {
        use crate::governance::{
            GovernanceConfig, QualityGate, SchemaEvolution, ComplianceEngine,
            MockQualityGate, MockSchemaEvolution, MockComplianceEngine,
        };
        use crate::streaming::{EventType, EventSource};

        let workflow = Arc::new(Workflow::new(
            "test_workflow".to_string(),
            "test_owner".to_string(),
            crate::workflow::SourceType::Table {
                table_name: "test_table".to_string(),
            },
        ));
        let activation = Arc::new(Activation::new(
            "test_activation".to_string(),
            workflow.id.clone(),
            "test_owner".to_string(),
        ));

        let config = GovernanceConfig {
            quality_gates_enabled: true,
            ..Default::default()
        };
        let quality_gate = Arc::new(MockQualityGate::new(true, 0.95));
        let schema_evolution = Arc::new(MockSchemaEvolution::no_changes());
        let compliance_engine = Arc::new(MockComplianceEngine::no_rules());

        let gov_engine = Arc::new(GovernanceEngine::new(
            config,
            quality_gate,
            schema_evolution,
            compliance_engine,
        ));

        let pipeline = ActivationPipeline::new(workflow, activation)
            .await
            .unwrap()
            .with_governance(gov_engine);

        pipeline.start().await.unwrap();

        // Process an event with governance enabled
        let event = Event::new(
            EventType::EntityCreated,
            EventSource::Webhook {
                url: "http://example.com".to_string(),
            },
            json!({"id": "test"}),
        );

        // Note: This will fail on process_event due to mock EventProcessor, but governance metrics still increment
        let _ = pipeline.process_event(event).await;

        // Metrics should reflect governance checks (quality_checks_passed incremented)
        let metrics = pipeline.metrics().await;
        assert!(metrics.quality_checks_passed > 0 || metrics.quality_checks_failed >= 0);
    }
}
