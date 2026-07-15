use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: EntityType,
    pub key_field: String,
    pub attributes: serde_json::Value,
    pub traits: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Customer,
    Account,
    Company,
    Lead,
    Subscription,
    Order,
    Product,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trait {
    pub name: String,
    pub value: serde_json::Value,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audience {
    pub id: String,
    pub name: String,
    pub definition: String,
    pub size: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Entity {
    pub fn new(entity_type: EntityType, key_field: impl Into<String>, id: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Entity {
            id: id.into(),
            entity_type,
            key_field: key_field.into(),
            attributes: json!({}),
            traits: json!({}),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_trait(mut self, name: impl Into<String>, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut obj) = self.traits {
            obj.insert(name.into(), value);
        }
        self
    }

    pub fn add_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut obj) = self.attributes {
            obj.insert(key.into(), value);
        }
        self
    }
}

impl Default for EntityType {
    fn default() -> Self {
        EntityType::Customer
    }
}

impl EntityType {
    pub fn as_str(&self) -> &str {
        match self {
            EntityType::Customer => "customer",
            EntityType::Account => "account",
            EntityType::Company => "company",
            EntityType::Lead => "lead",
            EntityType::Subscription => "subscription",
            EntityType::Order => "order",
            EntityType::Product => "product",
            EntityType::Custom(name) => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(EntityType::Customer, "customer_id", "cust_123");
        assert_eq!(entity.entity_type.as_str(), "customer");
        assert_eq!(entity.id, "cust_123");
    }

    #[test]
    fn test_entity_traits() {
        let entity = Entity::new(EntityType::Customer, "customer_id", "cust_1")
            .add_trait("lifetime_value", json!(5000.0))
            .add_trait("churn_risk", json!(0.15));

        assert!(entity.traits.get("lifetime_value").is_some());
    }

    #[test]
    fn test_entity_attributes() {
        let entity = Entity::new(EntityType::Customer, "customer_id", "cust_1")
            .add_attribute("segment", json!("premium"))
            .add_attribute("region", json!("us-west"));

        assert_eq!(entity.attributes.get("segment").and_then(|v| v.as_str()), Some("premium"));
    }
}
