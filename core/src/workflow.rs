use serde::{Deserialize, Serialize};

/// Workflow defines WHERE data comes from and HOW to extract it.
///
/// A workflow specifies a data source (warehouse table, model, query, audience, event,
/// spreadsheet via StreamXL, or PDF via PyStreamPDF) and extraction method
/// (batch, incremental, CDC, streaming, or event-driven).
///
/// Integrated data sources:
/// - Warehouse: Table, Model, Query, Audience, Event
/// - Spreadsheets: StreamXL sheets with column mapping
/// - PDFs: PyStreamPDF with intelligent extraction and token efficiency
///
/// What Workflow DOES:
/// ✓ Specify data source (warehouse, spreadsheet, PDF)
/// ✓ Define sync mode (Batch, Incremental, CDC, Streaming, EventDriven)
/// ✓ Map source fields to destination fields
/// ✓ Schedule execution (optional)
///
/// What Workflow does NOT do:
/// ✗ Validate source data (that's StatGuardian)
/// ✗ Create audiences (that's ClusterAudienceKit)
/// ✗ Define journeys (that's PyCustomerJourney)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: u32,
    pub owner: String,
    pub source_type: SourceType,
    pub sync_mode: SyncMode,
    pub mappings: Vec<FieldMapping>,
    pub schedule: Option<Schedule>,
    pub rate_limit: Option<RateLimit>,
    pub event_stream_config: Option<EventStreamConfig>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    // Warehouse sources
    Table { table_name: String },
    Model { model_name: String },
    Query { sql: String },
    Audience { audience_id: String },
    Event { event_type: String },
    // StreamXL integration - spreadsheet data sources
    StreamXL { sheet_name: String, api_url: String },
    // PyStreamPDF integration - PDF data sources with intelligent extraction
    StreamPDF { pdf_path: String, extraction_query: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMode {
    Batch,
    Incremental { key_column: String },
    CDC { stream_name: String },
    Streaming { topic: String },
    EventDriven,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub source_field: String,
    pub destination_field: String,
    pub transformation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub cron: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub records_per_second: u32,
    pub burst_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStreamConfig {
    pub broker_urls: Vec<String>,
    pub topic: String,
    pub consumer_group: Option<String>,
    pub rate_limit_per_sec: u32,
    pub batch_size: u32,
    pub flush_interval_ms: u32,
}

impl Default for SourceType {
    fn default() -> Self {
        SourceType::Table { table_name: "default".to_string() }
    }
}

impl Default for SyncMode {
    fn default() -> Self {
        SyncMode::Batch
    }
}

impl Workflow {
    pub fn new(name: impl Into<String>, owner: impl Into<String>, source_type: SourceType) -> Self {
        let now = chrono::Utc::now();
        Workflow {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            version: 1,
            owner: owner.into(),
            source_type,
            sync_mode: SyncMode::Batch,
            mappings: Vec::new(),
            schedule: None,
            rate_limit: None,
            event_stream_config: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn set_sync_mode(mut self, mode: SyncMode) -> Self {
        self.sync_mode = mode;
        self
    }

    pub fn add_mapping(mut self, source: impl Into<String>, dest: impl Into<String>) -> Self {
        self.mappings.push(FieldMapping {
            source_field: source.into(),
            destination_field: dest.into(),
            transformation: None,
        });
        self
    }

    pub fn set_schedule(mut self, cron: impl Into<String>, tz: impl Into<String>) -> Self {
        self.schedule = Some(Schedule {
            cron: cron.into(),
            timezone: tz.into(),
        });
        self
    }

    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn set_rate_limit(mut self, limit: RateLimit) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    pub fn set_event_stream_config(mut self, config: EventStreamConfig) -> Self {
        self.event_stream_config = Some(config);
        self
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = chrono::Utc::now();
    }

    pub fn increment_version(&mut self) {
        self.version += 1;
        self.update_timestamp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let wf = Workflow::new("LTV Sync", "data_team", SourceType::Table {
            table_name: "customers".to_string(),
        });
        assert_eq!(wf.name, "LTV Sync");
        assert!(wf.enabled);
    }

    #[test]
    fn test_workflow_builder() {
        let wf = Workflow::new("Test", "owner", SourceType::Table {
            table_name: "t".to_string(),
        })
        .with_description("Test workflow")
        .add_mapping("customer_id", "customerId")
        .add_mapping("ltv", "customerLTV");

        assert_eq!(wf.mappings.len(), 2);
    }

    #[test]
    fn test_workflow_sync_modes() {
        let wf = Workflow::new("Test", "owner", SourceType::Table {
            table_name: "t".to_string(),
        })
        .set_sync_mode(SyncMode::Incremental {
            key_column: "updated_at".to_string(),
        });

        match wf.sync_mode {
            SyncMode::Incremental { key_column } => {
                assert_eq!(key_column, "updated_at");
            }
            _ => panic!("Wrong sync mode"),
        }
    }

    #[test]
    fn test_workflow_rate_limit() {
        let wf = Workflow::new("Test", "owner", SourceType::Table {
            table_name: "t".to_string(),
        })
        .set_rate_limit(RateLimit {
            records_per_second: 100,
            burst_size: Some(500),
        });

        assert!(wf.rate_limit.is_some());
        assert_eq!(wf.rate_limit.unwrap().records_per_second, 100);
    }

    #[test]
    fn test_workflow_event_stream_config() {
        let config = EventStreamConfig {
            broker_urls: vec!["kafka://localhost:9092".to_string()],
            topic: "activations".to_string(),
            consumer_group: Some("group1".to_string()),
            rate_limit_per_sec: 1000,
            batch_size: 100,
            flush_interval_ms: 5000,
        };
        let wf = Workflow::new("Test", "owner", SourceType::Event {
            event_type: "customer_update".to_string(),
        })
        .set_event_stream_config(config);

        assert!(wf.event_stream_config.is_some());
        let cfg = wf.event_stream_config.unwrap();
        assert_eq!(cfg.topic, "activations");
        assert_eq!(cfg.batch_size, 100);
    }

    #[test]
    fn test_workflow_version_increment() {
        let mut wf = Workflow::new("Test", "owner", SourceType::Table {
            table_name: "t".to_string(),
        });
        assert_eq!(wf.version, 1);

        let original_updated = wf.updated_at;
        wf.increment_version();
        assert_eq!(wf.version, 2);
        assert!(wf.updated_at > original_updated);
    }

    #[test]
    fn test_workflow_with_all_features() {
        let wf = Workflow::new("Full Test", "data_team", SourceType::Table {
            table_name: "customers".to_string(),
        })
        .with_description("Complete workflow test")
        .set_sync_mode(SyncMode::Streaming { topic: "events".to_string() })
        .add_mapping("id", "customerId")
        .add_mapping("email", "customerEmail")
        .set_schedule("0 9 * * *", "America/New_York")
        .set_rate_limit(RateLimit {
            records_per_second: 50,
            burst_size: None,
        })
        .set_enabled(true);

        assert_eq!(wf.name, "Full Test");
        assert_eq!(wf.owner, "data_team");
        assert_eq!(wf.mappings.len(), 2);
        assert!(wf.schedule.is_some());
        assert!(wf.rate_limit.is_some());
    }

    #[test]
    fn test_workflow_disabled() {
        let wf = Workflow::new("Test", "owner", SourceType::Table {
            table_name: "t".to_string(),
        })
        .set_enabled(false);

        assert!(!wf.enabled);
    }
}
