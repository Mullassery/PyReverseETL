use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEngine {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub config: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRun {
    pub id: String,
    pub workflow_id: String,
    pub activation_id: String,
    pub status: SyncStatus,
    pub rows_processed: u64,
    pub rows_failed: u64,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub id: String,
    pub sync_run_id: String,
    pub entity_id: String,
    pub destination_id: String,
    pub action: String,
    pub payload: serde_json::Value,
    pub status: SyncStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl SyncEngine {
    pub fn new(name: impl Into<String>) -> Self {
        SyncEngine {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            version: 1,
            config: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        }
    }
}

impl SyncRun {
    pub fn new(workflow_id: impl Into<String>, activation_id: impl Into<String>) -> Self {
        SyncRun {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_id: workflow_id.into(),
            activation_id: activation_id.into(),
            status: SyncStatus::Pending,
            rows_processed: 0,
            rows_failed: 0,
            started_at: chrono::Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    pub fn mark_running(&mut self) {
        self.status = SyncStatus::Running;
    }

    pub fn mark_success(&mut self) {
        self.status = SyncStatus::Success;
        self.completed_at = Some(chrono::Utc::now());
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = SyncStatus::Failed;
        self.error_message = Some(error.into());
        self.completed_at = Some(chrono::Utc::now());
    }

    pub fn record_processed(&mut self, count: u64) {
        self.rows_processed += count;
    }

    pub fn record_failed(&mut self, count: u64) {
        self.rows_failed += count;
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, SyncStatus::Success | SyncStatus::Failed | SyncStatus::Cancelled)
    }

    pub fn is_successful(&self) -> bool {
        matches!(self.status, SyncStatus::Success)
    }

    pub fn total_records(&self) -> u64 {
        self.rows_processed + self.rows_failed
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_records();
        if total == 0 {
            0.0
        } else {
            (self.rows_processed as f64 / total as f64) * 100.0
        }
    }

    pub fn mark_cancelled(&mut self) {
        self.status = SyncStatus::Cancelled;
        self.completed_at = Some(chrono::Utc::now());
    }
}

impl SyncRecord {
    pub fn new(
        sync_run_id: impl Into<String>,
        entity_id: impl Into<String>,
        destination_id: impl Into<String>,
        action: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        SyncRecord {
            id: uuid::Uuid::new_v4().to_string(),
            sync_run_id: sync_run_id.into(),
            entity_id: entity_id.into(),
            destination_id: destination_id.into(),
            action: action.into(),
            payload,
            status: SyncStatus::Pending,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn mark_success(&mut self) {
        self.status = SyncStatus::Success;
    }

    pub fn mark_failed(&mut self) {
        self.status = SyncStatus::Failed;
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, SyncStatus::Success | SyncStatus::Failed)
    }

    pub fn payload_size(&self) -> usize {
        self.payload.to_string().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_engine_creation() {
        let engine = SyncEngine::new("Batch Sync Engine");
        assert_eq!(engine.name, "Batch Sync Engine");
    }

    #[test]
    fn test_sync_run_lifecycle() {
        let mut run = SyncRun::new("wf_1", "act_1");
        assert_eq!(run.status.to_string(), "Pending");

        run.mark_running();
        assert_eq!(run.status.to_string(), "Running");

        run.record_processed(100);
        assert_eq!(run.rows_processed, 100);

        run.mark_success();
        assert_eq!(run.status.to_string(), "Success");
        assert!(run.completed_at.is_some());
    }

    #[test]
    fn test_sync_record_creation() {
        let record = SyncRecord::new(
            "sync_run_1",
            "cust_123",
            "salesforce",
            "upsert",
            serde_json::json!({"customer_id": "123", "ltv": 5000}),
        );

        assert_eq!(record.entity_id, "cust_123");
        assert_eq!(record.action, "upsert");
    }

    #[test]
    fn test_sync_run_failure() {
        let mut run = SyncRun::new("wf_1", "act_1");
        run.record_processed(50);
        run.record_failed(10);
        run.mark_failed("Database connection timeout");

        assert!(!run.is_successful());
        assert!(run.is_completed());
        assert_eq!(run.rows_processed, 50);
        assert_eq!(run.rows_failed, 10);
        assert!(run.error_message.is_some());
    }

    #[test]
    fn test_sync_run_metrics() {
        let mut run = SyncRun::new("wf_1", "act_1");
        run.record_processed(900);
        run.record_failed(100);

        assert_eq!(run.total_records(), 1000);
        assert_eq!(run.success_rate(), 90.0);
    }

    #[test]
    fn test_sync_run_cancellation() {
        let mut run = SyncRun::new("wf_1", "act_1");
        run.mark_running();
        run.record_processed(250);
        run.mark_cancelled();

        assert!(run.is_completed());
        assert!(!run.is_successful());
        assert_eq!(run.status.to_string(), "Cancelled");
    }

    #[test]
    fn test_sync_record_lifecycle() {
        let mut record = SyncRecord::new(
            "sync_1",
            "entity_1",
            "dest_1",
            "update",
            serde_json::json!({"id": 1, "value": "data"}),
        );

        assert_eq!(record.status.to_string(), "Pending");
        assert!(!record.is_completed());

        record.mark_success();
        assert_eq!(record.status.to_string(), "Success");
        assert!(record.is_completed());
    }

    #[test]
    fn test_sync_record_payload_size() {
        let payload = serde_json::json!({
            "customer_id": "cust_123",
            "name": "John Doe",
            "email": "john@example.com",
            "metadata": {"source": "warehouse"}
        });

        let record = SyncRecord::new("sync_1", "entity_1", "dest_1", "upsert", payload);
        let size = record.payload_size();
        assert!(size > 0);
    }

    #[test]
    fn test_sync_run_with_zero_records() {
        let run = SyncRun::new("wf_1", "act_1");
        assert_eq!(run.total_records(), 0);
        assert_eq!(run.success_rate(), 0.0);
    }
}

impl Default for SyncStatus {
    fn default() -> Self {
        SyncStatus::Pending
    }
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Pending => write!(f, "Pending"),
            SyncStatus::Running => write!(f, "Running"),
            SyncStatus::Success => write!(f, "Success"),
            SyncStatus::Failed => write!(f, "Failed"),
            SyncStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}
