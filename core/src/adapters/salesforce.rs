use super::{AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping, FieldType, OperationResult};
use crate::Entity;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Salesforce REST API adapter (uses public API)
pub struct SalesforceAdapter {
    instance_url: String,
    client_id: String,
    client_secret: String,
    auth: AuthMethod,
    object_name: String,
    external_id_field: Option<String>,
}

impl SalesforceAdapter {
    /// Create a new Salesforce adapter
    pub fn new(config: &HashMap<String, Value>, auth: AuthMethod) -> Result<Self, AdapterError> {
        let instance_url = config
            .get("instance_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::InvalidConfiguration("Missing 'instance_url'".to_string()))?
            .to_string();

        let client_id = config
            .get("client_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let client_secret = config
            .get("client_secret")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let object_name = config
            .get("object")
            .and_then(|v| v.as_str())
            .unwrap_or("Contact")
            .to_string();

        let external_id_field = config.get("external_id_field").and_then(|v| v.as_str()).map(|s| s.to_string());

        Ok(SalesforceAdapter {
            instance_url,
            client_id,
            client_secret,
            auth,
            object_name,
            external_id_field,
        })
    }
}

impl DestinationAdapter for SalesforceAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.instance_url.is_empty() {
            return Err(AdapterError::AuthenticationFailed("No instance URL configured".to_string()));
        }
        // In production: implement OAuth token exchange flow
        Ok(())
    }

    fn upsert(&self, entity: &Entity, mappings: &[FieldMapping]) -> Result<OperationResult, AdapterError> {
        // Transform entity to Salesforce format
        let mut sf_record = json!({});
        for mapping in mappings {
            if let Some(value) = entity.get_attribute(&mapping.source_field) {
                sf_record[&mapping.destination_field] = value.clone();
            }
        }

        // In production: make HTTP POST/PATCH request to Salesforce
        let external_id = self.external_id_field.as_ref().and_then(|field| {
            sf_record.get(field).and_then(|v| v.as_str()).map(|s| s.to_string())
        });

        Ok(OperationResult {
            id: entity.id.clone(),
            success: true,
            external_id,
            error_message: None,
        })
    }

    fn batch_upsert(&self, entities: Vec<Entity>, mappings: &[FieldMapping]) -> Result<BatchResult, AdapterError> {
        // Salesforce batch API allows 10K records per batch
        let max_batch_size = 10000;
        if entities.len() > max_batch_size {
            return Err(AdapterError::BatchSizeExceeded {
                max_size: max_batch_size as u32,
                requested: entities.len() as u32,
            });
        }

        let total = entities.len() as u32;
        let mut successful = 0;
        let mut results = Vec::new();

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

        Ok(BatchResult {
            total,
            successful,
            failed: total - successful,
            results,
            duration_ms: 0,
        })
    }

    fn delete(&self, id: &str) -> Result<(), AdapterError> {
        if id.is_empty() {
            return Err(AdapterError::ValidationError("ID cannot be empty".to_string()));
        }
        // In production: make HTTP DELETE request to Salesforce
        Ok(())
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        // In production: fetch schema from Salesforce describe endpoint
        let mut fields = HashMap::new();
        fields.insert("Email".to_string(), FieldType::Email);
        fields.insert("Phone".to_string(), FieldType::String { max_length: Some(20) });
        fields.insert("FirstName".to_string(), FieldType::String { max_length: Some(40) });
        fields.insert("LastName".to_string(), FieldType::String { max_length: Some(80) });

        Ok(DestinationSchema {
            fields,
            required_fields: vec!["LastName".to_string()],
            max_batch_size: 10000,
        })
    }

    fn name(&self) -> &str {
        "salesforce"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salesforce_creation() {
        let mut config = HashMap::new();
        config.insert("instance_url".to_string(), json!("https://myorg.salesforce.com"));
        config.insert("object".to_string(), json!("Contact"));

        let auth = AuthMethod::OAuth {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: None,
        };

        let adapter = SalesforceAdapter::new(&config, auth).unwrap();
        assert_eq!(adapter.name(), "salesforce");
    }

    #[test]
    fn test_salesforce_authentication() {
        let mut config = HashMap::new();
        config.insert("instance_url".to_string(), json!("https://myorg.salesforce.com"));

        let auth = AuthMethod::OAuth {
            client_id: "id".to_string(),
            client_secret: "secret".to_string(),
            refresh_token: None,
        };

        let adapter = SalesforceAdapter::new(&config, auth).unwrap();
        assert!(adapter.authenticate().is_ok());
    }

    #[test]
    fn test_salesforce_batch_limit() {
        let mut config = HashMap::new();
        config.insert("instance_url".to_string(), json!("https://myorg.salesforce.com"));

        let auth = AuthMethod::OAuth {
            client_id: "id".to_string(),
            client_secret: "secret".to_string(),
            refresh_token: None,
        };

        let adapter = SalesforceAdapter::new(&config, auth).unwrap();
        let large_batch: Vec<Entity> = (0..15000)
            .map(|i| Entity::new(crate::entity::EntityType::Customer, "id", &format!("cust_{}", i)))
            .collect();

        let result = adapter.batch_upsert(large_batch, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_salesforce_schema() {
        let mut config = HashMap::new();
        config.insert("instance_url".to_string(), json!("https://myorg.salesforce.com"));

        let auth = AuthMethod::OAuth {
            client_id: "id".to_string(),
            client_secret: "secret".to_string(),
            refresh_token: None,
        };

        let adapter = SalesforceAdapter::new(&config, auth).unwrap();
        let schema = adapter.get_schema().unwrap();
        assert_eq!(schema.max_batch_size, 10000);
        assert!(schema.fields.contains_key("Email"));
    }
}
