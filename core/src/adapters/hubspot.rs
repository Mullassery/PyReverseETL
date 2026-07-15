use super::{AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping, FieldType, OperationResult};
use crate::Entity;
use serde_json::{json, Value};
use std::collections::HashMap;

/// HubSpot CRM API adapter (public API)
pub struct HubSpotAdapter {
    api_key: String,
    object_type: String,
    dedup_email: bool,
}

impl HubSpotAdapter {
    /// Create a new HubSpot adapter
    pub fn new(config: &HashMap<String, Value>, auth: AuthMethod) -> Result<Self, AdapterError> {
        let api_key = match auth {
            AuthMethod::ApiKey { key } => key,
            _ => {
                return Err(AdapterError::AuthenticationFailed(
                    "HubSpot requires API key authentication".to_string(),
                ))
            }
        };

        let object_type = config
            .get("object")
            .and_then(|v| v.as_str())
            .unwrap_or("contacts")
            .to_string();

        let dedup_email = config
            .get("dedup_email")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Ok(HubSpotAdapter {
            api_key,
            object_type,
            dedup_email,
        })
    }
}

impl DestinationAdapter for HubSpotAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.api_key.is_empty() {
            return Err(AdapterError::AuthenticationFailed("No API key configured".to_string()));
        }
        // In production: validate API key with HubSpot
        Ok(())
    }

    fn upsert(&self, entity: &Entity, mappings: &[FieldMapping]) -> Result<OperationResult, AdapterError> {
        // Transform entity to HubSpot format
        let mut properties = json!({});

        for mapping in mappings {
            if let Some(value) = entity.get_attribute(&mapping.source_field) {
                properties[&mapping.destination_field] = value.clone();
            } else if let Some(value) = entity.get_trait(&mapping.source_field) {
                properties[&mapping.destination_field] = value.clone();
            }
        }

        // In production: make HTTP POST/PATCH request to HubSpot CRM API
        let external_id = if self.dedup_email {
            properties.get("email").and_then(|v| v.as_str()).map(|s| s.to_string())
        } else {
            Some(entity.id.clone())
        };

        Ok(OperationResult {
            id: entity.id.clone(),
            success: true,
            external_id,
            error_message: None,
        })
    }

    fn batch_upsert(&self, entities: Vec<Entity>, mappings: &[FieldMapping]) -> Result<BatchResult, AdapterError> {
        // HubSpot batch API limits: 100 contacts per request, 10 requests/second
        let max_batch_size = 100;
        if entities.len() > max_batch_size {
            return Err(AdapterError::BatchSizeExceeded {
                max_size: max_batch_size as u32,
                requested: entities.len() as u32,
            });
        }

        let total = entities.len() as u32;
        let mut successful = 0;
        let mut results = Vec::new();

        let start = std::time::Instant::now();

        for entity in entities {
            match self.upsert(&entity, mappings) {
                Ok(result) => {
                    if result.success {
                        successful += 1;
                    }
                    results.push(result);
                }
                Err(e) => {
                    results.push(OperationResult {
                        id: entity.id.clone(),
                        success: false,
                        external_id: None,
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(BatchResult {
            total,
            successful,
            failed: total - successful,
            results,
            duration_ms,
        })
    }

    fn delete(&self, id: &str) -> Result<(), AdapterError> {
        if id.is_empty() {
            return Err(AdapterError::ValidationError("ID cannot be empty".to_string()));
        }
        // In production: make HTTP DELETE request to HubSpot
        Ok(())
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        // In production: fetch schema from HubSpot CRM properties endpoint
        let mut fields = HashMap::new();
        fields.insert("email".to_string(), FieldType::Email);
        fields.insert("firstname".to_string(), FieldType::String { max_length: Some(50) });
        fields.insert("lastname".to_string(), FieldType::String { max_length: Some(50) });
        fields.insert("phone".to_string(), FieldType::String { max_length: Some(20) });
        fields.insert("lifecyclestage".to_string(), FieldType::String { max_length: None });

        Ok(DestinationSchema {
            fields,
            required_fields: vec!["email".to_string()],
            max_batch_size: 100,
        })
    }

    fn name(&self) -> &str {
        "hubspot"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hubspot_creation() {
        let config = HashMap::new();
        let auth = AuthMethod::ApiKey {
            key: "test_key".to_string(),
        };

        let adapter = HubSpotAdapter::new(&config, auth).unwrap();
        assert_eq!(adapter.name(), "hubspot");
    }

    #[test]
    fn test_hubspot_missing_api_key() {
        let config = HashMap::new();
        let auth = AuthMethod::Bearer {
            token: "token".to_string(),
        };

        let result = HubSpotAdapter::new(&config, auth);
        assert!(result.is_err());
    }

    #[test]
    fn test_hubspot_batch_limit() {
        let config = HashMap::new();
        let auth = AuthMethod::ApiKey {
            key: "test_key".to_string(),
        };

        let adapter = HubSpotAdapter::new(&config, auth).unwrap();
        let large_batch: Vec<Entity> = (0..150)
            .map(|i| Entity::new(crate::entity::EntityType::Customer, "id", &format!("cust_{}", i)))
            .collect();

        let result = adapter.batch_upsert(large_batch, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_hubspot_schema() {
        let config = HashMap::new();
        let auth = AuthMethod::ApiKey {
            key: "test_key".to_string(),
        };

        let adapter = HubSpotAdapter::new(&config, auth).unwrap();
        let schema = adapter.get_schema().unwrap();
        assert_eq!(schema.max_batch_size, 100);
        assert!(schema.fields.contains_key("email"));
    }

    #[test]
    fn test_hubspot_dedup_email() {
        let mut config = HashMap::new();
        config.insert("dedup_email".to_string(), json!(true));

        let auth = AuthMethod::ApiKey {
            key: "test_key".to_string(),
        };

        let adapter = HubSpotAdapter::new(&config, auth).unwrap();
        assert!(adapter.dedup_email);
    }
}
