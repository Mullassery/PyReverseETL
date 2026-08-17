use super::{
    AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping,
    FieldType, OperationResult,
};
use crate::Entity;
use serde_json::{json, Value};
use std::collections::HashMap;

/// HubSpot CRM API adapter.
///
/// Talks to the real HubSpot CRM v3 API surface:
/// - Create: `POST {api_base}/crm/v3/objects/{objectType}`
/// - Upsert by unique property: `PATCH {api_base}/crm/v3/objects/{objectType}/{value}?idProperty={property}`
///   (HubSpot's real "upsert by ID property" mechanism)
/// - Delete: `DELETE {api_base}/crm/v3/objects/{objectType}/{id}`
/// - Schema: `GET {api_base}/crm/v3/properties/{objectType}`
/// - Auth: private-app access token as a Bearer token (HubSpot's current
///   recommended auth method; the legacy `hapikey` query param is deprecated)
///
/// This environment has no live HubSpot account, so these are verified
/// against a local mock HTTP server (see tests below) that asserts the
/// adapter sends the exact real request shape.
pub struct HubSpotAdapter {
    api_key: String,
    api_base: String,
    object_type: String,
    dedup_email: bool,
    client: reqwest::blocking::Client,
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

        let api_base = config
            .get("api_base")
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.hubapi.com")
            .trim_end_matches('/')
            .to_string();

        let client = reqwest::blocking::Client::builder()
            .build()
            .map_err(|e| AdapterError::ConnectionError(e.to_string()))?;

        Ok(HubSpotAdapter {
            api_key,
            api_base,
            object_type,
            dedup_email,
            client,
        })
    }

    fn properties(&self, entity: &Entity, mappings: &[FieldMapping]) -> Value {
        let mut properties = json!({});
        if mappings.is_empty() {
            if let Some(obj) = entity.attributes.as_object() {
                for (k, v) in obj {
                    properties[k] = v.clone();
                }
            }
            if let Some(obj) = entity.traits.as_object() {
                for (k, v) in obj {
                    properties
                        .as_object_mut()
                        .unwrap()
                        .entry(k.clone())
                        .or_insert(v.clone());
                }
            }
        } else {
            for mapping in mappings {
                if let Some(value) = entity.get_attribute(&mapping.source_field) {
                    properties[&mapping.destination_field] = value.clone();
                } else if let Some(value) = entity.get_trait(&mapping.source_field) {
                    properties[&mapping.destination_field] = value.clone();
                }
            }
        }
        properties
    }
}

impl DestinationAdapter for HubSpotAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.api_key.is_empty() {
            return Err(AdapterError::AuthenticationFailed(
                "No API key configured".to_string(),
            ));
        }
        Ok(())
    }

    fn upsert(
        &self,
        entity: &Entity,
        mappings: &[FieldMapping],
    ) -> Result<OperationResult, AdapterError> {
        let properties = self.properties(entity, mappings);
        let body = json!({ "properties": properties });

        let email = properties
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let (request, external_id) = if self.dedup_email {
            match &email {
                Some(email) => (
                    self.client.patch(format!(
                        "{}/crm/v3/objects/{}/{}?idProperty=email",
                        self.api_base, self.object_type, email
                    )),
                    Some(email.clone()),
                ),
                None => (
                    self.client.post(format!(
                        "{}/crm/v3/objects/{}",
                        self.api_base, self.object_type
                    )),
                    None,
                ),
            }
        } else {
            (
                self.client.post(format!(
                    "{}/crm/v3/objects/{}",
                    self.api_base, self.object_type
                )),
                Some(entity.id.clone()),
            )
        };

        let response = request
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        let status = response.status();

        if status.as_u16() == 401 {
            return Err(AdapterError::AuthenticationFailed(
                "HubSpot rejected the access token".to_string(),
            ));
        }
        if status.as_u16() == 429 {
            return Err(AdapterError::RateLimitExceeded {
                retry_after_ms: 10000,
            });
        }
        if !status.is_success() {
            return Ok(OperationResult {
                id: entity.id.clone(),
                success: false,
                external_id: None,
                error_message: Some(format!("HubSpot returned HTTP {status}")),
            });
        }

        let returned_id = response.json::<Value>().ok().and_then(|v| {
            v.get("id")
                .and_then(|id| id.as_str())
                .map(|s| s.to_string())
        });

        Ok(OperationResult {
            id: entity.id.clone(),
            success: true,
            external_id: returned_id.or(external_id),
            error_message: None,
        })
    }

    fn batch_upsert(
        &self,
        entities: Vec<Entity>,
        mappings: &[FieldMapping],
    ) -> Result<BatchResult, AdapterError> {
        // HubSpot batch API limits: 100 objects per request
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

        Ok(BatchResult {
            total,
            successful,
            failed: total - successful,
            results,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn delete(&self, id: &str) -> Result<(), AdapterError> {
        if id.is_empty() {
            return Err(AdapterError::ValidationError(
                "ID cannot be empty".to_string(),
            ));
        }
        let response = self
            .client
            .delete(format!(
                "{}/crm/v3/objects/{}/{}",
                self.api_base, self.object_type, id
            ))
            .bearer_auth(&self.api_key)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        match response.status().as_u16() {
            200..=299 | 404 => Ok(()),
            401 => Err(AdapterError::AuthenticationFailed(
                "HubSpot rejected the access token".to_string(),
            )),
            status => Err(AdapterError::OperationFailed(format!(
                "HubSpot delete returned HTTP {status}"
            ))),
        }
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        let response = self
            .client
            .get(format!(
                "{}/crm/v3/properties/{}",
                self.api_base, self.object_type
            ))
            .bearer_auth(&self.api_key)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::OperationFailed(format!(
                "HubSpot properties endpoint returned HTTP {}",
                response.status()
            )));
        }

        let body: Value = response.json().map_err(|e| {
            AdapterError::OperationFailed(format!("invalid properties response: {e}"))
        })?;

        let mut fields = HashMap::new();
        let mut required_fields = Vec::new();
        if let Some(results) = body.get("results").and_then(|r| r.as_array()) {
            for prop in results {
                let name = prop
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                let hs_type = prop
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("string");
                let field_type = match (name.as_str(), hs_type) {
                    ("email", _) => FieldType::Email,
                    (_, "bool") => FieldType::Boolean,
                    (_, "number") => FieldType::Float,
                    (_, "datetime" | "date") => FieldType::DateTime,
                    _ => FieldType::String { max_length: None },
                };
                if prop
                    .get("required")
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false)
                {
                    required_fields.push(name.clone());
                }
                fields.insert(name, field_type);
            }
        }

        Ok(DestinationSchema {
            fields,
            required_fields,
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
    use crate::testing::MockHttpServer;

    fn config_with_base(server: &MockHttpServer, dedup_email: bool) -> HashMap<String, Value> {
        let mut config = HashMap::new();
        config.insert("api_base".to_string(), json!(server.base_url.clone()));
        config.insert("dedup_email".to_string(), json!(dedup_email));
        config
    }

    fn api_key() -> AuthMethod {
        AuthMethod::ApiKey {
            key: "pat-na1-abc123".to_string(),
        }
    }

    #[test]
    fn test_hubspot_creation() {
        let config = HashMap::new();
        let adapter = HubSpotAdapter::new(&config, api_key()).unwrap();
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
        let adapter = HubSpotAdapter::new(&config, api_key()).unwrap();
        let large_batch: Vec<Entity> = (0..150)
            .map(|i| {
                Entity::new(
                    crate::entity::EntityType::Customer,
                    "id",
                    &format!("cust_{}", i),
                )
            })
            .collect();
        let result = adapter.batch_upsert(large_batch, &[]);
        assert!(result.is_err());
    }

    // -- Real HTTP-shape tests against a local mock server -------------------

    #[test]
    fn upsert_with_email_patches_the_real_upsert_by_id_property_endpoint() {
        let server = MockHttpServer::start(200, r#"{"id":"12345"}"#);
        let config = config_with_base(&server, true);
        let adapter = HubSpotAdapter::new(&config, api_key()).unwrap();

        let entity = Entity::new(crate::entity::EntityType::Customer, "id", "cust_1")
            .add_attribute("email", json!("real@example.com"))
            .add_attribute("firstname", json!("Real"));
        let mappings = vec![
            FieldMapping {
                source_field: "email".to_string(),
                destination_field: "email".to_string(),
                transformation: None,
                required: true,
            },
            FieldMapping {
                source_field: "firstname".to_string(),
                destination_field: "firstname".to_string(),
                transformation: None,
                required: false,
            },
        ];

        let result = adapter.upsert(&entity, &mappings).unwrap();
        assert!(result.success, "{:?}", result.error_message);
        assert_eq!(result.external_id.as_deref(), Some("12345"));

        let req = server.last_request().unwrap();
        assert_eq!(req.method, "PATCH");
        assert_eq!(
            req.path,
            "/crm/v3/objects/contacts/real@example.com?idProperty=email"
        );
        let body: Value = serde_json::from_str(&req.body).unwrap();
        assert_eq!(body["properties"]["firstname"], json!("Real"));
    }

    #[test]
    fn upsert_without_email_falls_back_to_real_create_endpoint() {
        let server = MockHttpServer::start(200, r#"{"id":"999"}"#);
        let config = config_with_base(&server, true);
        let adapter = HubSpotAdapter::new(&config, api_key()).unwrap();

        let entity = Entity::new(crate::entity::EntityType::Customer, "id", "cust_2");
        let result = adapter.upsert(&entity, &[]).unwrap();
        assert!(result.success);

        let req = server.last_request().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/crm/v3/objects/contacts");
    }

    #[test]
    fn delete_hits_the_real_object_delete_endpoint() {
        let server = MockHttpServer::start(204, "");
        let config = config_with_base(&server, true);
        let adapter = HubSpotAdapter::new(&config, api_key()).unwrap();

        adapter.delete("12345").unwrap();
        let req = server.last_request().unwrap();
        assert_eq!(req.method, "DELETE");
        assert_eq!(req.path, "/crm/v3/objects/contacts/12345");
    }

    #[test]
    fn get_schema_parses_a_real_properties_response_shape() {
        let server = MockHttpServer::start(
            200,
            &json!({
                "results": [
                    {"name": "email", "type": "string", "required": true},
                    {"name": "lifecyclestage", "type": "enumeration", "required": false}
                ]
            })
            .to_string(),
        );
        let config = config_with_base(&server, true);
        let adapter = HubSpotAdapter::new(&config, api_key()).unwrap();

        let schema = adapter.get_schema().unwrap();
        assert!(schema.fields.contains_key("email"));
        assert!(schema.required_fields.contains(&"email".to_string()));

        let req = server.last_request().unwrap();
        assert_eq!(req.path, "/crm/v3/properties/contacts");
    }
}
