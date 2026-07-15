use super::{AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping, FieldType, OperationResult};
use crate::Entity;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Marketo Lead Management API adapter (public API)
pub struct MarketoAdapter {
    client_id: String,
    client_secret: String,
    api_host: String,
    dedup_field: String,
}

impl MarketoAdapter {
    /// Create a new Marketo adapter
    pub fn new(config: &HashMap<String, Value>, auth: AuthMethod) -> Result<Self, AdapterError> {
        let (client_id, client_secret) = match auth {
            AuthMethod::OAuth {
                client_id,
                client_secret,
                ..
            } => (client_id, client_secret),
            _ => {
                return Err(AdapterError::AuthenticationFailed(
                    "Marketo requires OAuth authentication".to_string(),
                ))
            }
        };

        let api_host = config
            .get("api_host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::InvalidConfiguration("Missing 'api_host'".to_string()))?
            .to_string();

        let dedup_field = config
            .get("dedup_field")
            .and_then(|v| v.as_str())
            .unwrap_or("email")
            .to_string();

        Ok(MarketoAdapter {
            client_id,
            client_secret,
            api_host,
            dedup_field,
        })
    }
}

impl DestinationAdapter for MarketoAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.api_host.is_empty() || self.client_id.is_empty() {
            return Err(AdapterError::AuthenticationFailed("Missing Marketo credentials".to_string()));
        }
        // In production: implement OAuth token exchange
        Ok(())
    }

    fn upsert(&self, entity: &Entity, mappings: &[FieldMapping]) -> Result<OperationResult, AdapterError> {
        // Transform entity to Marketo lead format
        let mut lead = json!({});

        for mapping in mappings {
            if let Some(value) = entity.get_attribute(&mapping.source_field) {
                lead[&mapping.destination_field] = value.clone();
            } else if let Some(value) = entity.get_trait(&mapping.source_field) {
                lead[&mapping.destination_field] = value.clone();
            }
        }

        // In production: make HTTP POST request to Marketo bulk leads endpoint
        let external_id = lead
            .get(&self.dedup_field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(OperationResult {
            id: entity.id.clone(),
            success: true,
            external_id,
            error_message: None,
        })
    }

    fn batch_upsert(&self, entities: Vec<Entity>, mappings: &[FieldMapping]) -> Result<BatchResult, AdapterError> {
        // Marketo batch leads limits: 300 leads per batch, 10 calls/second
        let max_batch_size = 300;
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
        // In production: make HTTP request to delete lead
        Ok(())
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        // In production: fetch schema from Marketo describe endpoint
        let mut fields = HashMap::new();
        fields.insert("email".to_string(), FieldType::Email);
        fields.insert("firstName".to_string(), FieldType::String { max_length: Some(255) });
        fields.insert("lastName".to_string(), FieldType::String { max_length: Some(255) });
        fields.insert("phone".to_string(), FieldType::String { max_length: Some(20) });
        fields.insert("company".to_string(), FieldType::String { max_length: Some(255) });

        Ok(DestinationSchema {
            fields,
            required_fields: vec!["email".to_string()],
            max_batch_size: 300,
        })
    }

    fn name(&self) -> &str {
        "marketo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketo_creation() {
        let mut config = HashMap::new();
        config.insert("api_host".to_string(), json!("https://123-ABC-456.mktorest.com"));

        let auth = AuthMethod::OAuth {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: None,
        };

        let adapter = MarketoAdapter::new(&config, auth).unwrap();
        assert_eq!(adapter.name(), "marketo");
    }

    #[test]
    fn test_marketo_batch_limit() {
        let mut config = HashMap::new();
        config.insert("api_host".to_string(), json!("https://123-ABC-456.mktorest.com"));

        let auth = AuthMethod::OAuth {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: None,
        };

        let adapter = MarketoAdapter::new(&config, auth).unwrap();
        let large_batch: Vec<Entity> = (0..500)
            .map(|i| Entity::new(crate::entity::EntityType::Customer, "id", &format!("lead_{}", i)))
            .collect();

        let result = adapter.batch_upsert(large_batch, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_marketo_schema() {
        let mut config = HashMap::new();
        config.insert("api_host".to_string(), json!("https://123-ABC-456.mktorest.com"));

        let auth = AuthMethod::OAuth {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: None,
        };

        let adapter = MarketoAdapter::new(&config, auth).unwrap();
        let schema = adapter.get_schema().unwrap();
        assert_eq!(schema.max_batch_size, 300);
        assert!(schema.fields.contains_key("email"));
    }

    #[test]
    fn test_marketo_dedup_field() {
        let mut config = HashMap::new();
        config.insert("api_host".to_string(), json!("https://123-ABC-456.mktorest.com"));
        config.insert("dedup_field".to_string(), json!("email"));

        let auth = AuthMethod::OAuth {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: None,
        };

        let adapter = MarketoAdapter::new(&config, auth).unwrap();
        assert_eq!(adapter.dedup_field, "email");
    }
}
