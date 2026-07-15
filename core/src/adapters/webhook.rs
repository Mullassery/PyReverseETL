use super::{AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping, OperationResult};
use crate::Entity;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Generic webhook adapter for custom HTTP endpoints
pub struct WebhookAdapter {
    url: String,
    method: String,
    auth: AuthMethod,
    headers: HashMap<String, String>,
    timeout_secs: u32,
}

impl WebhookAdapter {
    /// Create a new webhook adapter
    pub fn new(config: &HashMap<String, Value>, auth: AuthMethod) -> Result<Self, AdapterError> {
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::InvalidConfiguration("Missing 'url' in config".to_string()))?
            .to_string();

        let method = config
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("POST")
            .to_string();

        let timeout_secs = config
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30) as u32;

        let headers = Self::build_headers(&auth);

        Ok(WebhookAdapter {
            url,
            method: method.to_uppercase(),
            auth,
            headers,
            timeout_secs,
        })
    }

    /// Build HTTP headers from auth method
    fn build_headers(auth: &AuthMethod) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        match auth {
            AuthMethod::Bearer { token } => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", token));
            }
            AuthMethod::ApiKey { key } => {
                headers.insert("X-API-Key".to_string(), key.clone());
            }
            AuthMethod::Basic { username, password } => {
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::encode(credentials);
                headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
            }
            _ => {}
        }

        headers
    }

    /// Transform entity to webhook payload
    fn transform_entity(&self, entity: &Entity, mappings: &[FieldMapping]) -> Result<Value, AdapterError> {
        let mut payload = json!({});

        for mapping in mappings {
            if let Some(value) = entity.get_attribute(&mapping.source_field) {
                let transformed = match &mapping.transformation {
                    Some(trans) => self.apply_transformation(value, trans)?,
                    None => value.clone(),
                };
                payload[&mapping.destination_field] = transformed;
            } else if let Some(value) = entity.get_trait(&mapping.source_field) {
                let transformed = match &mapping.transformation {
                    Some(trans) => self.apply_transformation(value, trans)?,
                    None => value.clone(),
                };
                payload[&mapping.destination_field] = transformed;
            } else if mapping.required {
                return Err(AdapterError::FieldMappingError(format!(
                    "Required field '{}' not found in entity",
                    mapping.source_field
                )));
            }
        }

        Ok(payload)
    }

    /// Apply transformation to a value
    fn apply_transformation(&self, value: &Value, transformation: &super::Transformation) -> Result<Value, AdapterError> {
        use super::Transformation;

        match transformation {
            Transformation::Identity => Ok(value.clone()),
            Transformation::Uppercase => {
                if let Some(s) = value.as_str() {
                    Ok(Value::String(s.to_uppercase()))
                } else {
                    Ok(value.clone())
                }
            }
            Transformation::Lowercase => {
                if let Some(s) = value.as_str() {
                    Ok(Value::String(s.to_lowercase()))
                } else {
                    Ok(value.clone())
                }
            }
            Transformation::ToTimestamp => {
                if let Some(s) = value.as_str() {
                    Ok(Value::String(chrono::DateTime::parse_from_rfc3339(s)
                        .map(|dt| dt.with_timezone(&chrono::Utc).to_rfc3339())
                        .unwrap_or_else(|_| s.to_string())))
                } else {
                    Ok(value.clone())
                }
            }
            Transformation::RoundDecimals(decimals) => {
                if let Some(f) = value.as_f64() {
                    let multiplier = 10_f64.powi(*decimals as i32);
                    let rounded = (f * multiplier).round() / multiplier;
                    Ok(Value::Number(serde_json::Number::from_f64(rounded).unwrap_or(serde_json::Number::from(0))))
                } else {
                    Ok(value.clone())
                }
            }
            Transformation::Custom(_) => {
                // Custom transformations would require scripting engine
                Err(AdapterError::NotImplemented("Custom transformations not yet supported".to_string()))
            }
        }
    }
}

impl DestinationAdapter for WebhookAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.url.is_empty() {
            return Err(AdapterError::AuthenticationFailed("No webhook URL configured".to_string()));
        }
        Ok(())
    }

    fn upsert(&self, entity: &Entity, mappings: &[FieldMapping]) -> Result<OperationResult, AdapterError> {
        let _payload = self.transform_entity(entity, mappings)?;

        // Simulate HTTP request (in real implementation, would use reqwest or similar)
        let external_id = entity.id.clone();

        Ok(OperationResult {
            id: entity.id.clone(),
            success: true,
            external_id: Some(external_id),
            error_message: None,
        })
    }

    fn batch_upsert(&self, entities: Vec<Entity>, mappings: &[FieldMapping]) -> Result<BatchResult, AdapterError> {
        let total = entities.len() as u32;
        let mut successful = 0;
        let mut failed = 0;
        let mut results = Vec::new();

        let start = std::time::Instant::now();

        for entity in entities {
            match self.upsert(&entity, mappings) {
                Ok(result) => {
                    if result.success {
                        successful += 1;
                    } else {
                        failed += 1;
                    }
                    results.push(result);
                }
                Err(e) => {
                    failed += 1;
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
            failed,
            results,
            duration_ms,
        })
    }

    fn delete(&self, id: &str) -> Result<(), AdapterError> {
        if id.is_empty() {
            return Err(AdapterError::ValidationError("ID cannot be empty".to_string()));
        }
        Ok(())
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        Ok(DestinationSchema {
            fields: HashMap::new(),
            required_fields: Vec::new(),
            max_batch_size: 1000,
        })
    }

    fn name(&self) -> &str {
        "webhook"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_creation() {
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!("https://example.com/webhook"));

        let auth = AuthMethod::Bearer {
            token: "test_token".to_string(),
        };

        let adapter = WebhookAdapter::new(&config, auth).unwrap();
        assert_eq!(adapter.name(), "webhook");
    }

    #[test]
    fn test_webhook_missing_url() {
        let config = HashMap::new();
        let auth = AuthMethod::Bearer {
            token: "test".to_string(),
        };

        let result = WebhookAdapter::new(&config, auth);
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_authentication() {
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!("https://example.com/webhook"));

        let auth = AuthMethod::Bearer {
            token: "test_token".to_string(),
        };

        let adapter = WebhookAdapter::new(&config, auth).unwrap();
        assert!(adapter.authenticate().is_ok());
    }

    #[test]
    fn test_transformation_uppercase() {
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!("https://example.com/webhook"));

        let auth = AuthMethod::Bearer {
            token: "test".to_string(),
        };

        let adapter = WebhookAdapter::new(&config, auth).unwrap();
        let value = json!("hello");
        let result = adapter.apply_transformation(&value, &super::super::Transformation::Uppercase).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_batch_upsert() {
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!("https://example.com/webhook"));

        let auth = AuthMethod::Bearer {
            token: "test".to_string(),
        };

        let adapter = WebhookAdapter::new(&config, auth).unwrap();
        let entities = vec![
            Entity::new(crate::entity::EntityType::Customer, "id", "cust_1"),
            Entity::new(crate::entity::EntityType::Customer, "id", "cust_2"),
        ];

        let result = adapter.batch_upsert(entities, &[]).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.successful, 2);
    }
}
