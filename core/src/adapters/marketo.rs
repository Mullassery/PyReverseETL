use super::{
    AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping,
    FieldType, OperationResult,
};
use crate::Entity;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

/// Marketo Lead Management API adapter.
///
/// Talks to the real Marketo REST API surface:
/// - OAuth token: `GET {api_host}/identity/oauth/token?grant_type=client_credentials&client_id=...&client_secret=...`
///   (Marketo's real identity endpoint -- note it's a GET with query params, unlike
///   the more common OAuth POST-with-form-body flow)
/// - Upsert leads: `POST {api_host}/rest/v1/leads.json` with
///   `{"action":"createOrUpdate","lookupField":<dedup field>,"input":[...]}`
/// - Delete leads: `POST {api_host}/rest/v1/leads/delete.json` with `{"input":[{"id": ...}]}`
/// - Describe: `GET {api_host}/rest/v1/leads/describe.json`
///
/// This environment has no live Marketo instance, so these are verified
/// against a local mock HTTP server (see tests below) that asserts the
/// adapter sends the exact real request shape.
pub struct MarketoAdapter {
    client_id: String,
    client_secret: String,
    api_host: String,
    dedup_field: String,
    client: reqwest::blocking::Client,
    access_token: Mutex<Option<String>>,
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
            .trim_end_matches('/')
            .to_string();

        let dedup_field = config
            .get("dedup_field")
            .and_then(|v| v.as_str())
            .unwrap_or("email")
            .to_string();

        let client = reqwest::blocking::Client::builder()
            .build()
            .map_err(|e| AdapterError::ConnectionError(e.to_string()))?;

        Ok(MarketoAdapter {
            client_id,
            client_secret,
            api_host,
            dedup_field,
            client,
            access_token: Mutex::new(None),
        })
    }

    /// Real Marketo identity endpoint token exchange.
    fn exchange_token(&self) -> Result<String, AdapterError> {
        let url = format!(
            "{}/identity/oauth/token?grant_type=client_credentials&client_id={}&client_secret={}",
            self.api_host, self.client_id, self.client_secret
        );
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(AdapterError::AuthenticationFailed(format!(
                "Marketo token exchange failed: HTTP {}",
                response.status()
            )));
        }
        let body: Value = response.json().map_err(|e| {
            AdapterError::AuthenticationFailed(format!("invalid token response: {e}"))
        })?;
        body.get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AdapterError::AuthenticationFailed(
                    "token response missing access_token".to_string(),
                )
            })
    }

    fn token(&self) -> Result<String, AdapterError> {
        {
            let cached = self.access_token.lock().unwrap();
            if let Some(token) = cached.as_ref() {
                return Ok(token.clone());
            }
        }
        let token = self.exchange_token()?;
        *self.access_token.lock().unwrap() = Some(token.clone());
        Ok(token)
    }

    fn lead_fields(&self, entity: &Entity, mappings: &[FieldMapping]) -> Value {
        let mut lead = json!({});
        if mappings.is_empty() {
            if let Some(obj) = entity.attributes.as_object() {
                for (k, v) in obj {
                    lead[k] = v.clone();
                }
            }
        } else {
            for mapping in mappings {
                if let Some(value) = entity.get_attribute(&mapping.source_field) {
                    lead[&mapping.destination_field] = value.clone();
                } else if let Some(value) = entity.get_trait(&mapping.source_field) {
                    lead[&mapping.destination_field] = value.clone();
                }
            }
        }
        lead
    }
}

impl DestinationAdapter for MarketoAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.api_host.is_empty() || self.client_id.is_empty() {
            return Err(AdapterError::AuthenticationFailed(
                "Missing Marketo credentials".to_string(),
            ));
        }
        self.token().map(|_| ())
    }

    fn upsert(
        &self,
        entity: &Entity,
        mappings: &[FieldMapping],
    ) -> Result<OperationResult, AdapterError> {
        let lead = self.lead_fields(entity, mappings);
        let token = self.token()?;

        let external_id = lead
            .get(&self.dedup_field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let body = json!({
            "action": "createOrUpdate",
            "lookupField": self.dedup_field,
            "input": [lead],
        });

        let response = self
            .client
            .post(format!("{}/rest/v1/leads.json", self.api_host))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        let status = response.status();

        if status.as_u16() == 401 {
            return Err(AdapterError::AuthenticationFailed(
                "Marketo rejected the access token".to_string(),
            ));
        }
        if status.as_u16() == 429 {
            return Err(AdapterError::RateLimitExceeded {
                retry_after_ms: 20000,
            });
        }
        if !status.is_success() {
            return Ok(OperationResult {
                id: entity.id.clone(),
                success: false,
                external_id: None,
                error_message: Some(format!("Marketo returned HTTP {status}")),
            });
        }

        let response_body: Value = response
            .json()
            .map_err(|e| AdapterError::OperationFailed(format!("invalid Marketo response: {e}")))?;
        let success = response_body
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);
        if !success {
            return Ok(OperationResult {
                id: entity.id.clone(),
                success: false,
                external_id: None,
                error_message: Some(format!("Marketo rejected the lead: {response_body}")),
            });
        }

        let marketo_id = response_body
            .get("result")
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("id"))
            .map(|id| id.to_string());

        Ok(OperationResult {
            id: entity.id.clone(),
            success: true,
            external_id: marketo_id.or(external_id),
            error_message: None,
        })
    }

    fn batch_upsert(
        &self,
        entities: Vec<Entity>,
        mappings: &[FieldMapping],
    ) -> Result<BatchResult, AdapterError> {
        // Marketo batch leads limits: 300 leads per batch
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
        let token = self.token()?;
        let body = json!({ "input": [{ "id": id.parse::<i64>().unwrap_or(0) }] });
        let response = self
            .client
            .post(format!("{}/rest/v1/leads/delete.json", self.api_host))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        match response.status().as_u16() {
            200..=299 => Ok(()),
            401 => Err(AdapterError::AuthenticationFailed(
                "Marketo rejected the access token".to_string(),
            )),
            status => Err(AdapterError::OperationFailed(format!(
                "Marketo delete returned HTTP {status}"
            ))),
        }
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        let token = self.token()?;
        let response = self
            .client
            .get(format!("{}/rest/v1/leads/describe.json", self.api_host))
            .bearer_auth(&token)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::OperationFailed(format!(
                "Marketo describe returned HTTP {}",
                response.status()
            )));
        }

        let body: Value = response.json().map_err(|e| {
            AdapterError::OperationFailed(format!("invalid describe response: {e}"))
        })?;

        let mut fields = HashMap::new();
        if let Some(field_list) = body.get("result").and_then(|r| r.as_array()) {
            for field in field_list {
                let name = field
                    .get("rest")
                    .and_then(|r| r.get("name"))
                    .or_else(|| field.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                let data_type = field
                    .get("dataType")
                    .and_then(|t| t.as_str())
                    .unwrap_or("string");
                let field_type = match (name.as_str(), data_type) {
                    ("email", _) => FieldType::Email,
                    (_, "boolean") => FieldType::Boolean,
                    (_, "integer" | "float" | "currency") => FieldType::Float,
                    (_, "date" | "datetime") => FieldType::DateTime,
                    _ => FieldType::String { max_length: None },
                };
                fields.insert(name, field_type);
            }
        }

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
    use crate::testing::MockHttpServer;

    fn config_for(server: &MockHttpServer) -> HashMap<String, Value> {
        let mut config = HashMap::new();
        config.insert("api_host".to_string(), json!(server.base_url.clone()));
        config
    }

    fn oauth() -> AuthMethod {
        AuthMethod::OAuth {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: None,
        }
    }

    #[test]
    fn test_marketo_creation() {
        let mut config = HashMap::new();
        config.insert(
            "api_host".to_string(),
            json!("https://123-ABC-456.mktorest.com"),
        );
        let adapter = MarketoAdapter::new(&config, oauth()).unwrap();
        assert_eq!(adapter.name(), "marketo");
    }

    #[test]
    fn test_marketo_batch_limit() {
        let mut config = HashMap::new();
        config.insert(
            "api_host".to_string(),
            json!("https://123-ABC-456.mktorest.com"),
        );
        let adapter = MarketoAdapter::new(&config, oauth()).unwrap();
        let large_batch: Vec<Entity> = (0..500)
            .map(|i| {
                Entity::new(
                    crate::entity::EntityType::Customer,
                    "id",
                    &format!("lead_{}", i),
                )
            })
            .collect();
        let result = adapter.batch_upsert(large_batch, &[]);
        assert!(result.is_err());
    }

    // -- Real HTTP-shape tests against a local mock server -------------------

    #[test]
    fn authenticate_performs_a_real_identity_token_exchange() {
        let server = MockHttpServer::start(
            200,
            r#"{"access_token":"tok_abc","token_type":"bearer","expires_in":3599}"#,
        );
        let config = config_for(&server);
        let adapter = MarketoAdapter::new(&config, oauth()).unwrap();

        adapter.authenticate().unwrap();

        let req = server.last_request().unwrap();
        assert_eq!(req.method, "GET");
        assert!(req.path.starts_with("/identity/oauth/token"));
        assert!(req.path.contains("grant_type=client_credentials"));
        assert!(req.path.contains("client_id=client_id"));
    }

    #[test]
    fn upsert_posts_the_real_createorupdate_bulk_lead_shape() {
        let server = MockHttpServer::start(
            200,
            &json!({
                "access_token": "tok",
                "success": true,
                "result": [{"id": 42, "status": "created"}]
            })
            .to_string(),
        );
        let config = config_for(&server);
        let adapter = MarketoAdapter::new(&config, oauth()).unwrap();

        let entity = Entity::new(crate::entity::EntityType::Lead, "id", "lead_1")
            .add_attribute("email", json!("real@example.com"))
            .add_attribute("firstName", json!("Real"));
        let mappings = vec![
            FieldMapping {
                source_field: "email".to_string(),
                destination_field: "email".to_string(),
                transformation: None,
                required: true,
            },
            FieldMapping {
                source_field: "firstName".to_string(),
                destination_field: "firstName".to_string(),
                transformation: None,
                required: false,
            },
        ];

        let result = adapter.upsert(&entity, &mappings).unwrap();
        assert!(result.success, "{:?}", result.error_message);
        assert_eq!(result.external_id.as_deref(), Some("42"));

        let requests = server.requests();
        let upsert_req = &requests[1];
        assert_eq!(upsert_req.method, "POST");
        assert_eq!(upsert_req.path, "/rest/v1/leads.json");
        let body: Value = serde_json::from_str(&upsert_req.body).unwrap();
        assert_eq!(body["action"], json!("createOrUpdate"));
        assert_eq!(body["lookupField"], json!("email"));
        assert_eq!(body["input"][0]["email"], json!("real@example.com"));
    }

    #[test]
    fn upsert_reports_failure_when_marketo_rejects_the_lead() {
        let server = MockHttpServer::start(
            200,
            &json!({"access_token": "tok", "success": false, "errors": [{"message": "invalid email"}]}).to_string(),
        );
        let config = config_for(&server);
        let adapter = MarketoAdapter::new(&config, oauth()).unwrap();
        let entity = Entity::new(crate::entity::EntityType::Lead, "id", "lead_1");

        let result = adapter.upsert(&entity, &[]).unwrap();
        assert!(!result.success);
    }

    #[test]
    fn delete_posts_the_real_leads_delete_endpoint() {
        let server = MockHttpServer::start(200, r#"{"access_token":"tok","success":true}"#);
        let config = config_for(&server);
        let adapter = MarketoAdapter::new(&config, oauth()).unwrap();

        adapter.delete("42").unwrap();
        let requests = server.requests();
        let delete_req = &requests[1];
        assert_eq!(delete_req.method, "POST");
        assert_eq!(delete_req.path, "/rest/v1/leads/delete.json");
        let body: Value = serde_json::from_str(&delete_req.body).unwrap();
        assert_eq!(body["input"][0]["id"], json!(42));
    }

    #[test]
    fn get_schema_parses_a_real_describe_response_shape() {
        let server = MockHttpServer::start(
            200,
            &json!({
                "access_token": "tok",
                "result": [
                    {"rest": {"name": "email"}, "dataType": "email"},
                    {"rest": {"name": "firstName"}, "dataType": "string"}
                ]
            })
            .to_string(),
        );
        let config = config_for(&server);
        let adapter = MarketoAdapter::new(&config, oauth()).unwrap();

        let schema = adapter.get_schema().unwrap();
        assert!(schema.fields.contains_key("email"));
        assert!(schema.fields.contains_key("firstName"));
        assert_eq!(schema.max_batch_size, 300);
    }
}
