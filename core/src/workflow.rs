use serde::{Deserialize, Serialize};

/// Workflow defines WHERE data comes from and HOW to extract it.
///
/// A workflow specifies a data source (table, model, query, audience, or event)
/// and extraction method (batch, incremental, CDC, streaming, or event-driven).
///
/// What Workflow DOES:
/// ✓ Specify data source (table, model, query, audience, event stream)
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
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Table { table_name: String },
    Model { model_name: String },
    Query { sql: String },
    Audience { audience_id: String },
    Event { event_type: String },
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
}
