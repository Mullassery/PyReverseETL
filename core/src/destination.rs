use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub id: String,
    pub name: String,
    pub destination_type: DestinationType,
    pub config: HashMap<String, serde_json::Value>,
    pub version: u32,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DestinationType {
    Salesforce,
    HubSpot,
    Braze,
    CustomerIO,
    MetaAds,
    GoogleAds,
    Zendesk,
    Mixpanel,
    Amplitude,
    PostHog,
    Kafka,
    Webhook,
    Custom(String),
}

impl Destination {
    pub fn new(name: impl Into<String>, dest_type: DestinationType) -> Self {
        let now = chrono::Utc::now();
        Destination {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            destination_type: dest_type,
            config: HashMap::new(),
            version: 1,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn set_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Default for DestinationType {
    fn default() -> Self {
        DestinationType::Webhook
    }
}

impl DestinationType {
    pub fn as_str(&self) -> &str {
        match self {
            DestinationType::Salesforce => "salesforce",
            DestinationType::HubSpot => "hubspot",
            DestinationType::Braze => "braze",
            DestinationType::CustomerIO => "customerio",
            DestinationType::MetaAds => "meta_ads",
            DestinationType::GoogleAds => "google_ads",
            DestinationType::Zendesk => "zendesk",
            DestinationType::Mixpanel => "mixpanel",
            DestinationType::Amplitude => "amplitude",
            DestinationType::PostHog => "posthog",
            DestinationType::Kafka => "kafka",
            DestinationType::Webhook => "webhook",
            DestinationType::Custom(name) => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destination_creation() {
        let dest = Destination::new("Production Salesforce", DestinationType::Salesforce);
        assert_eq!(dest.name, "Production Salesforce");
        assert!(dest.enabled);
    }

    #[test]
    fn test_destination_config() {
        let dest = Destination::new("Test", DestinationType::Kafka)
            .set_config("broker", serde_json::json!("localhost:9092"))
            .set_config("topic", serde_json::json!("activations"));

        assert_eq!(dest.config.len(), 2);
    }

    #[test]
    fn test_destination_type_str() {
        assert_eq!(DestinationType::Salesforce.as_str(), "salesforce");
        assert_eq!(DestinationType::HubSpot.as_str(), "hubspot");
    }
}
