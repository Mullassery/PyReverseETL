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

    pub fn get_config(&self, key: &str) -> Option<&serde_json::Value> {
        self.config.get(key)
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = chrono::Utc::now();
    }

    pub fn increment_version(&mut self) {
        self.version += 1;
        self.update_timestamp();
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

    #[test]
    fn test_destination_disabled() {
        let dest = Destination::new("Test", DestinationType::Webhook)
            .set_enabled(false);
        assert!(!dest.enabled);
    }

    #[test]
    fn test_destination_get_config() {
        let dest = Destination::new("Test", DestinationType::Kafka)
            .set_config("broker", serde_json::json!("localhost:9092"));

        let broker = dest.get_config("broker");
        assert!(broker.is_some());
        assert_eq!(broker.unwrap().as_str().unwrap(), "localhost:9092");
    }

    #[test]
    fn test_destination_version_increment() {
        let mut dest = Destination::new("Test", DestinationType::Salesforce);
        assert_eq!(dest.version, 1);

        let original_updated = dest.updated_at;
        dest.increment_version();
        assert_eq!(dest.version, 2);
        assert!(dest.updated_at > original_updated);
    }

    #[test]
    fn test_destination_all_types() {
        let types = vec![
            DestinationType::Salesforce,
            DestinationType::HubSpot,
            DestinationType::Braze,
            DestinationType::Kafka,
            DestinationType::Webhook,
        ];

        for dest_type in types {
            let dest = Destination::new("Test", dest_type);
            assert_eq!(dest.version, 1);
            assert!(dest.enabled);
        }
    }

    #[test]
    fn test_destination_complex_config() {
        let dest = Destination::new("HubSpot API", DestinationType::HubSpot)
            .set_config("api_key", serde_json::json!("key123"))
            .set_config("base_url", serde_json::json!("https://api.hubapi.com"))
            .set_config("rate_limit", serde_json::json!(1000));

        assert_eq!(dest.config.len(), 3);
        assert_eq!(
            dest.get_config("api_key").unwrap().as_str().unwrap(),
            "key123"
        );
    }
}
