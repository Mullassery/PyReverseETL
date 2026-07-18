pub mod error;
pub mod activation;
pub mod workflow;
pub mod destination;
pub mod entity;
pub mod sync;
pub mod storage;
pub mod statguardian;
pub mod streamxl;
pub mod streampdf;
pub mod adapters;
pub mod streaming;
pub mod cdc;
pub mod pipeline;
pub mod sources;

pub use error::{Error, Result};
pub use activation::Activation;
pub use workflow::Workflow;
pub use destination::Destination;
pub use entity::Entity;
pub use sync::{SyncEngine, SyncRun, SyncRecord, SyncStatus};
pub use storage::Repository;
pub use statguardian::{ValidationGate, ValidationResult, ValidationStatus, StatGuardianConfig};
pub use streamxl::{StreamXLSource, StreamXLConfig, StreamXLAccessMethod};
pub use streampdf::{StreamPDFSource, StreamPDFConfig, ExtractionMode, ExtractedData};
pub use adapters::{DestinationAdapter, FieldMapping, AuthMethod, OperationResult, BatchResult};
pub use cdc::{Change, ChangeDetector, ChangeType, ChangeLog, ChangeLogEntry, Checkpoint, CheckpointManager};
pub use pipeline::{ActivationPipeline, PipelineMetrics, PipelineStatus, LatencyTracker, BackpressureManager};
pub use streaming::{Event, EventType, EventSource, EventProcessor, EventHandler};
pub use sources::{
    KafkaSource, KafkaConfig, KafkaMessage, EventSourceConnector,
    SyncFrequency, PollingConfig, ChangePoller, PollingMetrics, PollResult, SharedPollingState,
};
