use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::statguardian::ValidationGate;

/// Activation defines WHERE and HOW to sync data to operational systems.
///
/// Activation maps a Workflow (data source) to one or more Destinations (where to send it).
/// By default, every Activation requires validation from StatGuardian before syncing data.
///
/// What Activation DOES:
/// ✓ Specify sync source, destinations, and schedules
/// ✓ Define field mappings and transformations
/// ✓ Apply validation gates from StatGuardian (by default)
/// ✓ Track sync status and outcomes
/// ✓ Block bad data from reaching operational systems
///
/// What Activation does NOT do:
/// ✗ Create audiences (that's ClusterAudienceKit)
/// ✗ Validate data (that's StatGuardian)
/// ✗ Define customer journeys (that's PyCustomerJourney)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activation {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workflow_id: String,
    pub version: u32,
    pub owner: String,
    pub destinations: Vec<String>,
    pub policies: HashMap<String, serde_json::Value>,
    /// StatGuardian validation gates (required by default)
    #[serde(default)]
    pub validation_gates: Vec<ValidationGate>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Activation {
    pub fn new(name: impl Into<String>, workflow_id: impl Into<String>, owner: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Activation {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            workflow_id: workflow_id.into(),
            version: 1,
            owner: owner.into(),
            destinations: Vec::new(),
            policies: HashMap::new(),
            validation_gates: Vec::new(),  // Start with no gates, can be added
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn add_destination(mut self, dest_id: impl Into<String>) -> Self {
        self.destinations.push(dest_id.into());
        self
    }

    pub fn set_policy(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.policies.insert(key.into(), value);
        self
    }

    /// Add a StatGuardian validation gate
    /// Before syncing, validation results for this dataset will be checked
    pub fn add_validation_gate(mut self, gate: ValidationGate) -> Self {
        self.validation_gates.push(gate);
        self
    }

    /// Add multiple validation gates at once
    pub fn with_validation_gates(mut self, gates: Vec<ValidationGate>) -> Self {
        self.validation_gates = gates;
        self
    }

    /// Check if this activation requires validation before sync
    pub fn requires_validation(&self) -> bool {
        !self.validation_gates.is_empty()
    }

    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn get_policy(&self, key: &str) -> Option<&serde_json::Value> {
        self.policies.get(key)
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
    use crate::statguardian::ValidationGate;

    #[test]
    fn test_activation_creation() {
        let activation = Activation::new("Send LTV to Salesforce", "wf_123", "data_team");
        assert_eq!(activation.name, "Send LTV to Salesforce");
        assert_eq!(activation.workflow_id, "wf_123");
        assert!(activation.enabled);
        assert!(!activation.requires_validation());
    }

    #[test]
    fn test_activation_builder() {
        let activation = Activation::new("Test", "wf_1", "owner")
            .with_description("Test activation")
            .add_destination("salesforce")
            .add_destination("hubspot");
        assert_eq!(activation.destinations.len(), 2);
    }

    #[test]
    fn test_activation_with_validation_gates() {
        let gate = ValidationGate::new("customers_dataset");
        let activation = Activation::new("LTV Sync", "wf_1", "owner")
            .add_validation_gate(gate);

        assert!(activation.requires_validation());
        assert_eq!(activation.validation_gates.len(), 1);
    }

    #[test]
    fn test_activation_statguardian_integration() {
        let gate1 = ValidationGate::new("customers").block_on_failure(true);
        let gate2 = ValidationGate::new("transactions").block_on_failure(false);

        let activation = Activation::new("Test", "wf_1", "owner")
            .with_validation_gates(vec![gate1, gate2]);

        assert_eq!(activation.validation_gates.len(), 2);
        assert!(activation.requires_validation());
    }

    #[test]
    fn test_activation_policies() {
        let activation = Activation::new("Test", "wf_1", "owner")
            .set_policy("batch_size", serde_json::json!(1000))
            .set_policy("timeout_seconds", serde_json::json!(300))
            .set_policy("retry_count", serde_json::json!(3));

        assert_eq!(activation.policies.len(), 3);
        assert_eq!(
            activation.get_policy("batch_size").unwrap().as_i64().unwrap(),
            1000
        );
    }

    #[test]
    fn test_activation_disabled() {
        let activation = Activation::new("Test", "wf_1", "owner")
            .set_enabled(false);

        assert!(!activation.enabled);
    }

    #[test]
    fn test_activation_version_increment() {
        let mut activation = Activation::new("Test", "wf_1", "owner");
        assert_eq!(activation.version, 1);

        let original_updated = activation.updated_at;
        activation.increment_version();
        assert_eq!(activation.version, 2);
        assert!(activation.updated_at > original_updated);
    }

    #[test]
    fn test_activation_complete_workflow() {
        let gate = ValidationGate::new("customer_data").block_on_failure(true);

        let activation = Activation::new("Revenue Sync", "wf_revenue", "analytics_team")
            .with_description("Sync revenue data to CRM daily")
            .add_destination("salesforce_prod")
            .add_destination("hubspot_prod")
            .set_policy("sync_frequency", serde_json::json!("daily"))
            .set_policy("batch_size", serde_json::json!(5000))
            .add_validation_gate(gate);

        assert_eq!(activation.name, "Revenue Sync");
        assert_eq!(activation.destinations.len(), 2);
        assert_eq!(activation.policies.len(), 2);
        assert!(activation.requires_validation());
    }
}
