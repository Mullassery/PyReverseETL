pub mod activation;
pub mod adapters;
pub mod cdc;
pub mod connectors;
pub mod destination;
pub mod entity;
pub mod error;
pub mod executor;
pub mod governance;
pub mod lineage;
pub mod observability;
pub mod pipeline;
pub mod sources;
pub mod statguardian;
pub mod storage;
pub mod streaming;
pub mod streampdf;
pub mod streamxl;
pub mod sync;
pub mod testing;
pub mod transformers;
pub mod workflow;

pub use activation::Activation;
pub use adapters::{AuthMethod, BatchResult, DestinationAdapter, FieldMapping, OperationResult};
pub use cdc::{
    Change, ChangeDetector, ChangeLog, ChangeLogEntry, ChangeType, Checkpoint, CheckpointManager,
};
pub use destination::Destination;
pub use entity::Entity;
pub use error::{Error, Result};
pub use executor::{execute_sync, DestinationSpec, ExecuteOptions, ExecutionResult, SourceSpec};
pub use lineage::{LineageEdge, LineageGraph, LineageNode, LineageNodeKind, LineageStore};
pub use observability::{
    init_otel, launch_dashboard, DashboardConfig, MetricsCollector, MetricsHistory, MetricsServer,
    MetricsSnapshot, Platform, SyncContext, SyncLogger, SyncMetrics, SyncTracer, TraceSpan,
    TraceSummary,
};
pub use pipeline::{
    ActivationPipeline, BackpressureManager, LatencyTracker, PipelineMetrics, PipelineStatus,
};
pub use sources::{
    ChangePoller, EventSourceConnector, KafkaConfig, KafkaMessage, KafkaSource, PollResult,
    PollingConfig, PollingMetrics, SharedPollingState, SyncFrequency,
};
pub use statguardian::{StatGuardianConfig, ValidationGate, ValidationResult, ValidationStatus};
pub use storage::Repository;
pub use streaming::{Event, EventHandler, EventProcessor, EventSource, EventType};
pub use streampdf::{ExtractedData, ExtractionMode, StreamPDFConfig, StreamPDFSource};
pub use streamxl::{StreamXLAccessMethod, StreamXLConfig, StreamXLSource};
pub use sync::{SyncEngine, SyncRecord, SyncRun, SyncStatus};
pub use transformers::{
    SparkConfig, SparkJobResult, SparkTransformer, TransformationConfig, TransformationPipeline,
    TransformationResult, TransformationStage, TransformationStatus, Transformer,
};
pub use workflow::Workflow;
