use super::{
    AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping,
    OperationResult,
};
use crate::Entity;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

/// Generic webhook adapter for custom HTTP endpoints.
///
/// Makes real HTTP requests via `reqwest::blocking` (the `DestinationAdapter`
/// trait is synchronous, so a blocking client is the correct tool here rather
/// than threading a Tokio runtime through every call site).
pub struct WebhookAdapter {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    client: reqwest::blocking::Client,
}

impl WebhookAdapter {
    /// Create a new webhook adapter
    pub fn new(config: &HashMap<String, Value>, auth: AuthMethod) -> Result<Self, AdapterError> {
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdapterError::InvalidConfiguration("Missing 'url' in config".to_string())
            })?
            .to_string();

        let method = config
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("POST")
            .to_string();

        let timeout_secs = config
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let headers = Self::build_headers(&auth);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| AdapterError::ConnectionError(e.to_string()))?;

        Ok(WebhookAdapter {
            url,
            method: method.to_uppercase(),
            headers,
            client,
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
                use base64::Engine;
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
                headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
            }
            _ => {}
        }

        headers
    }

    /// Transform entity to webhook payload
    fn transform_entity(
        &self,
        entity: &Entity,
        mappings: &[FieldMapping],
    ) -> Result<Value, AdapterError> {
        let mut payload = json!({});

        if mappings.is_empty() {
            // No explicit mapping configured: pass the entity's attributes through
            // as-is, augmented with its id, so a caller can activate raw records
            // (e.g. from the sync executor) without hand-writing a 1:1 mapping.
            payload = entity.attributes.clone();
            if let Some(obj) = payload.as_object_mut() {
                obj.entry("id".to_string())
                    .or_insert_with(|| json!(entity.id));
            }
            return Ok(payload);
        }

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
    fn apply_transformation(
        &self,
        value: &Value,
        transformation: &super::Transformation,
    ) -> Result<Value, AdapterError> {
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
                    Ok(Value::String(
                        chrono::DateTime::parse_from_rfc3339(s)
                            .map(|dt| dt.with_timezone(&chrono::Utc).to_rfc3339())
                            .unwrap_or_else(|_| s.to_string()),
                    ))
                } else {
                    Ok(value.clone())
                }
            }
            Transformation::RoundDecimals(decimals) => {
                if let Some(f) = value.as_f64() {
                    let multiplier = 10_f64.powi(*decimals as i32);
                    let rounded = (f * multiplier).round() / multiplier;
                    Ok(Value::Number(
                        serde_json::Number::from_f64(rounded)
                            .unwrap_or(serde_json::Number::from(0)),
                    ))
                } else {
                    Ok(value.clone())
                }
            }
            Transformation::Custom(_) => {
                // Custom transformations would require scripting engine
                Err(AdapterError::NotImplemented(
                    "Custom transformations not yet supported".to_string(),
                ))
            }
        }
    }

    fn send(&self, payload: &Value) -> Result<(), AdapterError> {
        let mut req = match self.method.as_str() {
            "PATCH" => self.client.patch(&self.url),
            "PUT" => self.client.put(&self.url),
            _ => self.client.post(&self.url),
        }
        .json(payload);

        for (key, value) in &self.headers {
            req = req.header(key.as_str(), value.as_str());
        }

        let response = req
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        match response.status().as_u16() {
            200..=299 => Ok(()),
            401 | 403 => Err(AdapterError::AuthenticationFailed(format!(
                "webhook rejected credentials: HTTP {}",
                response.status()
            ))),
            429 => Err(AdapterError::RateLimitExceeded {
                retry_after_ms: 5000,
            }),
            status => Err(AdapterError::OperationFailed(format!(
                "webhook returned HTTP {status}"
            ))),
        }
    }
}

impl DestinationAdapter for WebhookAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.url.is_empty() {
            return Err(AdapterError::AuthenticationFailed(
                "No webhook URL configured".to_string(),
            ));
        }
        Ok(())
    }

    fn upsert(
        &self,
        entity: &Entity,
        mappings: &[FieldMapping],
    ) -> Result<OperationResult, AdapterError> {
        let payload = self.transform_entity(entity, mappings)?;

        match self.send(&payload) {
            Ok(()) => Ok(OperationResult {
                id: entity.id.clone(),
                success: true,
                external_id: Some(entity.id.clone()),
                error_message: None,
            }),
            Err(e) => Ok(OperationResult {
                id: entity.id.clone(),
                success: false,
                external_id: None,
                error_message: Some(e.to_string()),
            }),
        }
    }

    fn batch_upsert(
        &self,
        entities: Vec<Entity>,
        mappings: &[FieldMapping],
    ) -> Result<BatchResult, AdapterError> {
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
            return Err(AdapterError::ValidationError(
                "ID cannot be empty".to_string(),
            ));
        }
        let mut req = self.client.delete(&self.url).json(&json!({"id": id}));
        for (key, value) in &self.headers {
            req = req.header(key.as_str(), value.as_str());
        }
        let response = req
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        match response.status().as_u16() {
            200..=299 | 404 => Ok(()),
            status => Err(AdapterError::OperationFailed(format!(
                "webhook delete returned HTTP {status}"
            ))),
        }
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        // Webhooks have no discoverable schema (unlike Salesforce/HubSpot describe
        // endpoints) -- there is no real endpoint to call here, so this stays a
        // permissive default rather than fabricating fields that don't exist.
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
    use crate::testing::MockHttpServer;

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
        let result = adapter
            .apply_transformation(&value, &super::super::Transformation::Uppercase)
            .unwrap();
        assert_eq!(result, "HELLO");
    }

    // -- Real HTTP tests against a local mock server ------------------------

    #[test]
    fn upsert_makes_a_real_post_request_with_auth_header_and_json_body() {
        let server = MockHttpServer::start(200, r#"{"ok":true}"#);
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let auth = AuthMethod::Bearer {
            token: "secret-token".to_string(),
        };
        let adapter = WebhookAdapter::new(&config, auth).unwrap();

        let entity = Entity::new(crate::entity::EntityType::Customer, "id", "cust_1")
            .add_attribute("email", json!("real@example.com"));
        let mappings = vec![FieldMapping {
            source_field: "email".to_string(),
            destination_field: "email".to_string(),
            transformation: None,
            required: true,
        }];

        let result = adapter.upsert(&entity, &mappings).unwrap();
        assert!(result.success, "{:?}", result.error_message);

        let req = server.last_request().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(
            req.headers.get("authorization"),
            Some(&"Bearer secret-token".to_string())
        );
        let body: Value = serde_json::from_str(&req.body).unwrap();
        assert_eq!(body["email"], json!("real@example.com"));
    }

    #[test]
    fn upsert_without_mappings_passes_entity_attributes_through() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let adapter = WebhookAdapter::new(
            &config,
            AuthMethod::Bearer {
                token: "t".to_string(),
            },
        )
        .unwrap();

        let entity = Entity::new(crate::entity::EntityType::Customer, "id", "cust_9")
            .add_attribute("ltv", json!(5000));
        let result = adapter.upsert(&entity, &[]).unwrap();
        assert!(result.success);

        let req = server.last_request().unwrap();
        let body: Value = serde_json::from_str(&req.body).unwrap();
        assert_eq!(body["ltv"], json!(5000));
        assert_eq!(body["id"], json!("cust_9"));
    }

    #[test]
    fn upsert_surfaces_real_http_error_status_as_a_failed_result() {
        let server = MockHttpServer::start(401, r#"{"error":"unauthorized"}"#);
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let adapter = WebhookAdapter::new(
            &config,
            AuthMethod::Bearer {
                token: "bad".to_string(),
            },
        )
        .unwrap();

        let entity = Entity::new(crate::entity::EntityType::Customer, "id", "cust_1");
        let result = adapter.upsert(&entity, &[]).unwrap();
        assert!(!result.success);
        assert!(result.error_message.unwrap().contains("Authentication"));
    }

    #[test]
    fn batch_upsert_sends_one_real_request_per_entity() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let adapter = WebhookAdapter::new(
            &config,
            AuthMethod::Bearer {
                token: "t".to_string(),
            },
        )
        .unwrap();

        let entities = vec![
            Entity::new(crate::entity::EntityType::Customer, "id", "cust_1"),
            Entity::new(crate::entity::EntityType::Customer, "id", "cust_2"),
        ];

        let result = adapter.batch_upsert(entities, &[]).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.successful, 2);
        assert_eq!(server.requests().len(), 2, "one real HTTP call per entity");
    }

    #[test]
    fn delete_makes_a_real_delete_request() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let adapter = WebhookAdapter::new(
            &config,
            AuthMethod::Bearer {
                token: "t".to_string(),
            },
        )
        .unwrap();

        adapter.delete("cust_1").unwrap();
        let req = server.last_request().unwrap();
        assert_eq!(req.method, "DELETE");
        assert!(req.body.contains("cust_1"));
    }
}
