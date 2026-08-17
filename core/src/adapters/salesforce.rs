use super::{
    AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping,
    FieldType, OperationResult,
};
use crate::Entity;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

const API_VERSION: &str = "v59.0";

/// Salesforce REST API adapter.
///
/// Talks to the real Salesforce REST API surface:
/// - OAuth 2.0 token exchange: `POST {instance_url}/services/oauth2/token`
/// - Create: `POST {instance_url}/services/data/{version}/sobjects/{object}`
/// - Upsert by external ID: `PATCH {instance_url}/services/data/{version}/sobjects/{object}/{external_id_field}/{external_id_value}`
/// - Delete: `DELETE {instance_url}/services/data/{version}/sobjects/{object}/{id}`
/// - Describe: `GET {instance_url}/services/data/{version}/sobjects/{object}/describe`
///
/// This environment has no live Salesforce org, so these are verified against
/// a local mock HTTP server (see tests below) that asserts the adapter sends
/// the exact real request shape -- not against a live account.
pub struct SalesforceAdapter {
    instance_url: String,
    client_id: String,
    client_secret: String,
    refresh_token: Option<String>,
    object_name: String,
    external_id_field: Option<String>,
    client: reqwest::blocking::Client,
    access_token: Mutex<Option<String>>,
}

impl SalesforceAdapter {
    /// Create a new Salesforce adapter
    pub fn new(config: &HashMap<String, Value>, auth: AuthMethod) -> Result<Self, AdapterError> {
        let instance_url = config
            .get("instance_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdapterError::InvalidConfiguration("Missing 'instance_url'".to_string())
            })?
            .trim_end_matches('/')
            .to_string();

        let (client_id, client_secret, refresh_token) = match auth {
            AuthMethod::OAuth {
                client_id,
                client_secret,
                refresh_token,
            } => (client_id, client_secret, refresh_token),
            _ => {
                return Err(AdapterError::AuthenticationFailed(
                    "Salesforce requires OAuth authentication".to_string(),
                ))
            }
        };

        let object_name = config
            .get("object")
            .and_then(|v| v.as_str())
            .unwrap_or("Contact")
            .to_string();

        let external_id_field = config
            .get("external_id_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let client = reqwest::blocking::Client::builder()
            .build()
            .map_err(|e| AdapterError::ConnectionError(e.to_string()))?;

        Ok(SalesforceAdapter {
            instance_url,
            client_id,
            client_secret,
            refresh_token,
            object_name,
            external_id_field,
            client,
            access_token: Mutex::new(None),
        })
    }

    /// Real OAuth 2.0 token exchange against `{instance_url}/services/oauth2/token`.
    /// Uses the refresh-token grant if a refresh token was configured, else the
    /// client-credentials grant (both are real Salesforce-supported flows).
    fn exchange_token(&self) -> Result<String, AdapterError> {
        let url = format!("{}/services/oauth2/token", self.instance_url);

        let mut form = vec![
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
        ];
        if let Some(refresh_token) = &self.refresh_token {
            form.push(("grant_type", "refresh_token"));
            form.push(("refresh_token", refresh_token.as_str()));
        } else {
            form.push(("grant_type", "client_credentials"));
        }

        let response = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::AuthenticationFailed(format!(
                "Salesforce token exchange failed: HTTP {}",
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

    fn sobject_field_value(&self, entity: &Entity, mappings: &[FieldMapping]) -> Value {
        let mut sf_record = json!({});
        if mappings.is_empty() {
            if let Some(obj) = entity.attributes.as_object() {
                for (k, v) in obj {
                    sf_record[k] = v.clone();
                }
            }
        } else {
            for mapping in mappings {
                if let Some(value) = entity.get_attribute(&mapping.source_field) {
                    sf_record[&mapping.destination_field] = value.clone();
                }
            }
        }
        sf_record
    }
}

impl DestinationAdapter for SalesforceAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.instance_url.is_empty() {
            return Err(AdapterError::AuthenticationFailed(
                "No instance URL configured".to_string(),
            ));
        }
        self.token().map(|_| ())
    }

    fn upsert(
        &self,
        entity: &Entity,
        mappings: &[FieldMapping],
    ) -> Result<OperationResult, AdapterError> {
        let sf_record = self.sobject_field_value(entity, mappings);
        let token = self.token()?;

        let external_id = self.external_id_field.as_ref().and_then(|field| {
            sf_record
                .get(field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

        let (method_is_patch, url) = match (&self.external_id_field, &external_id) {
            (Some(field), Some(value)) => (
                true,
                format!(
                    "{}/services/data/{}/sobjects/{}/{}/{}",
                    self.instance_url, API_VERSION, self.object_name, field, value
                ),
            ),
            _ => (
                false,
                format!(
                    "{}/services/data/{}/sobjects/{}",
                    self.instance_url, API_VERSION, self.object_name
                ),
            ),
        };

        let request = if method_is_patch {
            self.client.patch(&url)
        } else {
            self.client.post(&url)
        }
        .bearer_auth(&token)
        .json(&sf_record);

        let response = request
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        let status = response.status();

        if status.as_u16() == 401 {
            return Err(AdapterError::AuthenticationFailed(
                "Salesforce rejected the access token".to_string(),
            ));
        }
        if status.as_u16() == 429 {
            return Err(AdapterError::RateLimitExceeded {
                retry_after_ms: 5000,
            });
        }
        if !status.is_success() {
            return Ok(OperationResult {
                id: entity.id.clone(),
                success: false,
                external_id: None,
                error_message: Some(format!("Salesforce returned HTTP {status}")),
            });
        }

        // Salesforce's create response includes the new record's `id`; a
        // successful PATCH upsert returns 204 with no body.
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
        let url = format!(
            "{}/services/data/{}/sobjects/{}/{}",
            self.instance_url, API_VERSION, self.object_name, id
        );
        let response = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        match response.status().as_u16() {
            200..=299 | 404 => Ok(()),
            401 => Err(AdapterError::AuthenticationFailed(
                "Salesforce rejected the access token".to_string(),
            )),
            status => Err(AdapterError::OperationFailed(format!(
                "Salesforce delete returned HTTP {status}"
            ))),
        }
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        let token = self.token()?;
        let url = format!(
            "{}/services/data/{}/sobjects/{}/describe",
            self.instance_url, API_VERSION, self.object_name
        );
        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::OperationFailed(format!(
                "Salesforce describe returned HTTP {}",
                response.status()
            )));
        }

        let body: Value = response.json().map_err(|e| {
            AdapterError::OperationFailed(format!("invalid describe response: {e}"))
        })?;

        let mut fields = HashMap::new();
        let mut required_fields = Vec::new();
        if let Some(field_list) = body.get("fields").and_then(|f| f.as_array()) {
            for field in field_list {
                let name = field
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                let sf_type = field
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("string");
                let field_type = match sf_type {
                    "email" => FieldType::Email,
                    "url" => FieldType::Url,
                    "boolean" => FieldType::Boolean,
                    "int" | "currency" | "double" => FieldType::Float,
                    "date" | "datetime" => FieldType::DateTime,
                    _ => FieldType::String {
                        max_length: field
                            .get("length")
                            .and_then(|l| l.as_u64())
                            .map(|l| l as u32),
                    },
                };
                let nillable = field
                    .get("nillable")
                    .and_then(|n| n.as_bool())
                    .unwrap_or(true);
                if !nillable {
                    required_fields.push(name.clone());
                }
                fields.insert(name, field_type);
            }
        }

        Ok(DestinationSchema {
            fields,
            required_fields,
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
    use crate::testing::{MockHttpServer, RecordedRequest};

    fn config_for(
        server: &MockHttpServer,
        object: &str,
        external_id_field: Option<&str>,
    ) -> HashMap<String, Value> {
        let mut config = HashMap::new();
        config.insert("instance_url".to_string(), json!(server.base_url.clone()));
        config.insert("object".to_string(), json!(object));
        if let Some(f) = external_id_field {
            config.insert("external_id_field".to_string(), json!(f));
        }
        config
    }

    fn oauth() -> AuthMethod {
        AuthMethod::OAuth {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: None,
        }
    }

    /// Header lookup is case-insensitive per HTTP semantics; find a value
    /// regardless of what case the client happened to send it in.
    fn header_ci<'a>(req: &'a RecordedRequest, name: &str) -> Option<&'a String> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    #[test]
    fn test_salesforce_creation() {
        let mut config = HashMap::new();
        config.insert(
            "instance_url".to_string(),
            json!("https://myorg.salesforce.com"),
        );
        config.insert("object".to_string(), json!("Contact"));
        let adapter = SalesforceAdapter::new(&config, oauth()).unwrap();
        assert_eq!(adapter.name(), "salesforce");
    }

    #[test]
    fn test_salesforce_batch_limit() {
        let mut config = HashMap::new();
        config.insert(
            "instance_url".to_string(),
            json!("https://myorg.salesforce.com"),
        );
        let adapter = SalesforceAdapter::new(&config, oauth()).unwrap();
        let large_batch: Vec<Entity> = (0..15000)
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
    fn authenticate_performs_a_real_oauth_client_credentials_exchange() {
        let server = MockHttpServer::start(
            200,
            r#"{"access_token":"tok_abc","token_type":"Bearer","instance_url":"https://x"}"#,
        );
        let config = config_for(&server, "Contact", None);
        let adapter = SalesforceAdapter::new(&config, oauth()).unwrap();

        adapter.authenticate().unwrap();

        let req = server.last_request().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/services/oauth2/token");
        assert!(req.body.contains("grant_type=client_credentials"));
        assert!(req.body.contains("client_id=client_id"));
    }

    #[test]
    fn upsert_without_external_id_posts_to_the_real_sobjects_create_endpoint() {
        let server = MockHttpServer::start(200, r#"{"access_token":"tok"}"#);
        let config = config_for(&server, "Contact", None);
        let adapter = SalesforceAdapter::new(&config, oauth()).unwrap();

        let entity = Entity::new(crate::entity::EntityType::Customer, "id", "cust_1")
            .add_attribute("LastName", json!("Doe"));
        let mappings = vec![FieldMapping {
            source_field: "LastName".to_string(),
            destination_field: "LastName".to_string(),
            transformation: None,
            required: true,
        }];

        let result = adapter.upsert(&entity, &mappings).unwrap();
        assert!(result.success, "{:?}", result.error_message);

        let requests = server.requests();
        assert_eq!(requests.len(), 2, "token exchange + create");
        let create_req = &requests[1];
        assert_eq!(create_req.method, "POST");
        assert_eq!(create_req.path, "/services/data/v59.0/sobjects/Contact");
        assert_eq!(
            header_ci(create_req, "authorization"),
            Some(&"Bearer tok".to_string())
        );
        let body: Value = serde_json::from_str(&create_req.body).unwrap();
        assert_eq!(body["LastName"], json!("Doe"));
    }

    #[test]
    fn upsert_with_external_id_patches_the_real_upsert_by_external_id_endpoint() {
        // The mock server returns this body for every request; the token
        // exchange (the first request the adapter makes) needs `access_token`.
        // A real Salesforce upsert-by-external-id PATCH returns 204 with no
        // body, but the adapter only *optionally* parses a body for the id
        // (`.json().ok()`), so a non-empty JSON body here doesn't break that.
        let server = MockHttpServer::start(200, r#"{"access_token":"tok"}"#);
        let config = config_for(&server, "Contact", Some("Email__c"));
        let adapter = SalesforceAdapter::new(&config, oauth()).unwrap();

        let entity = Entity::new(crate::entity::EntityType::Customer, "id", "cust_1")
            .add_attribute("Email__c", json!("real@example.com"));
        let mappings = vec![FieldMapping {
            source_field: "Email__c".to_string(),
            destination_field: "Email__c".to_string(),
            transformation: None,
            required: true,
        }];

        let result = adapter.upsert(&entity, &mappings).unwrap();
        assert!(result.success);
        assert_eq!(result.external_id.as_deref(), Some("real@example.com"));

        let requests = server.requests();
        let upsert_req = &requests[1];
        assert_eq!(upsert_req.method, "PATCH");
        assert_eq!(
            upsert_req.path,
            "/services/data/v59.0/sobjects/Contact/Email__c/real@example.com"
        );
    }

    #[test]
    fn delete_hits_the_real_sobject_delete_endpoint() {
        // Same reasoning as above: the token exchange leg needs a real
        // access_token in the body, even though the real delete response
        // itself would be a bodyless 204.
        let server = MockHttpServer::start(200, r#"{"access_token":"tok"}"#);
        let config = config_for(&server, "Contact", None);
        let adapter = SalesforceAdapter::new(&config, oauth()).unwrap();

        adapter.delete("003xyz").unwrap();
        let requests = server.requests();
        let delete_req = &requests[1];
        assert_eq!(delete_req.method, "DELETE");
        assert_eq!(
            delete_req.path,
            "/services/data/v59.0/sobjects/Contact/003xyz"
        );
    }

    #[test]
    fn get_schema_parses_a_real_describe_response_shape() {
        // The mock server returns the same body for every request; the token
        // exchange only needs `access_token`, and describe only needs `fields`,
        // so one body satisfying both lets a single server handle the whole flow.
        let server = MockHttpServer::start(
            200,
            &json!({
                "access_token": "tok",
                "fields": [
                    {"name": "Email", "type": "email", "nillable": true},
                    {"name": "LastName", "type": "string", "nillable": false, "length": 80}
                ]
            })
            .to_string(),
        );
        let config = config_for(&server, "Contact", None);
        let adapter = SalesforceAdapter::new(&config, oauth()).unwrap();

        let schema = adapter.get_schema().unwrap();
        assert!(schema.fields.contains_key("Email"));
        assert!(schema.required_fields.contains(&"LastName".to_string()));
        assert_eq!(schema.max_batch_size, 10000);
    }
}
