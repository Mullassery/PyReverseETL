use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_creation() {
        let activation = Activation::new("Send LTV to Salesforce", "wf_123", "data_team");
        assert_eq!(activation.name, "Send LTV to Salesforce");
        assert_eq!(activation.workflow_id, "wf_123");
        assert!(activation.enabled);
    }

    #[test]
    fn test_activation_builder() {
        let activation = Activation::new("Test", "wf_1", "owner")
            .with_description("Test activation")
            .add_destination("salesforce")
            .add_destination("hubspot");
        assert_eq!(activation.destinations.len(), 2);
    }
}
