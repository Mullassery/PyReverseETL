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

    pub fn get_trait(&self, name: &str) -> Option<&serde_json::Value> {
        self.traits.get(name)
    }

    pub fn get_attribute(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.get(key)
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = chrono::Utc::now();
    }

    pub fn trait_count(&self) -> usize {
        if let serde_json::Value::Object(obj) = &self.traits {
            obj.len()
        } else {
            0
        }
    }

    pub fn attribute_count(&self) -> usize {
        if let serde_json::Value::Object(obj) = &self.attributes {
            obj.len()
        } else {
            0
        }
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

    #[test]
    fn test_entity_get_trait() {
        let entity = Entity::new(EntityType::Customer, "customer_id", "cust_1")
            .add_trait("lifetime_value", json!(5000.0));

        let ltv = entity.get_trait("lifetime_value");
        assert!(ltv.is_some());
        assert_eq!(ltv.unwrap().as_f64().unwrap(), 5000.0);
    }

    #[test]
    fn test_entity_get_attribute() {
        let entity = Entity::new(EntityType::Account, "account_id", "acc_1")
            .add_attribute("tier", json!("enterprise"));

        let tier = entity.get_attribute("tier");
        assert!(tier.is_some());
        assert_eq!(tier.unwrap().as_str().unwrap(), "enterprise");
    }

    #[test]
    fn test_entity_count_traits_and_attributes() {
        let entity = Entity::new(EntityType::Customer, "customer_id", "cust_1")
            .add_trait("ltv", json!(1000.0))
            .add_trait("churn_risk", json!(0.05))
            .add_attribute("segment", json!("high_value"))
            .add_attribute("region", json!("north"))
            .add_attribute("status", json!("active"));

        assert_eq!(entity.trait_count(), 2);
        assert_eq!(entity.attribute_count(), 3);
    }

    #[test]
    fn test_entity_all_types() {
        let types = vec![
            EntityType::Customer,
            EntityType::Account,
            EntityType::Company,
            EntityType::Lead,
            EntityType::Subscription,
            EntityType::Order,
            EntityType::Product,
        ];

        for entity_type in types {
            let entity = Entity::new(entity_type, "id", "test");
            assert_eq!(entity.trait_count(), 0);
            assert_eq!(entity.attribute_count(), 0);
        }
    }

    #[test]
    fn test_entity_complex_structure() {
        let entity = Entity::new(EntityType::Customer, "customer_id", "premium_customer_123")
            .add_trait("lifetime_value", json!(50000))
            .add_trait("purchase_frequency", json!(12))
            .add_trait("avg_order_value", json!(4166.67))
            .add_attribute("segment", json!("vip"))
            .add_attribute("region", json!("us-west-2"))
            .add_attribute("subscription_tier", json!("enterprise"))
            .add_attribute("churn_risk_score", json!(0.02));

        assert_eq!(entity.id, "premium_customer_123");
        assert_eq!(entity.trait_count(), 3);
        assert_eq!(entity.attribute_count(), 4);
        assert!(entity.get_trait("lifetime_value").is_some());
        assert!(entity.get_attribute("subscription_tier").is_some());
    }

    #[test]
    fn test_entity_timestamp_update() {
        let mut entity = Entity::new(EntityType::Customer, "customer_id", "cust_1");
        let original_updated = entity.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        entity.update_timestamp();
        assert!(entity.updated_at >= original_updated);
    }
}
