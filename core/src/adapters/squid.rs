//! Squid-CRM destination adapter.
//!
//! This is the concrete reverse-ETL -> Squid contract:
//!
//! - POST to `/rest/people` or `/rest/companies` with Squid's native create
//!   payload shape (the request body is the object input, not a `data.create*`
//!   envelope; that envelope is the response shape).
//! - Send a stable `Idempotency-Key` per source record, so a retry after a
//!   transport failure replays Squid's stored response instead of creating a
//!   duplicate.
//! - Send `X-Correlation-Id` using the same stable key for log correlation.
//!
//! Configuration:
//!
//! ```json
//! {
//!   "base_url": "http://localhost:3000",
//!   "object": "people",
//!   "idempotency_key_prefix": "pyreverseetl",
//!   "auth": {"type": "bearer", "token": "..."}
//! }
//! ```
//!
//! Supported objects are `people` (or `person`) and `companies` (or
//! `company`). Only an explicit field allowlist is forwarded, so warehouse
//! columns are not accidentally expanded into the CRM. Person records require
//! a first/last name and a primary email; the latter is required by this
//! adapter even though Squid itself allows an empty email string.
//!
//! The standard Squid object endpoints are create-only. This adapter is
//! therefore idempotent for retries, but it does not update an existing
//! Squid record. A later change to the same source record is a new activation
//! intent and should be routed through an explicit update contract rather
//! than silently creating a second person.

use super::{
    AdapterError, AuthMethod, BatchResult, DestinationAdapter, DestinationSchema, FieldMapping,
    FieldType, OperationResult, RetryPolicy,
};
use crate::Entity;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SquidObject {
    People,
    Companies,
}

impl SquidObject {
    fn parse(raw: &str) -> Result<Self, AdapterError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "people" | "person" | "persons" => Ok(Self::People),
            "companies" | "company" => Ok(Self::Companies),
            other => Err(AdapterError::InvalidConfiguration(format!(
                "unsupported Squid object '{other}' (supported: people, companies)"
            ))),
        }
    }

    fn path(self) -> &'static str {
        match self {
            Self::People => "people",
            Self::Companies => "companies",
        }
    }

    fn response_key(self) -> &'static str {
        match self {
            Self::People => "createPerson",
            Self::Companies => "createCompany",
        }
    }
}

pub struct SquidAdapter {
    base_url: String,
    object: SquidObject,
    headers: HashMap<String, String>,
    client: reqwest::blocking::Client,
    retry_policy: RetryPolicy,
    idempotency_key_prefix: String,
}

impl SquidAdapter {
    pub fn new(config: &HashMap<String, Value>, auth: AuthMethod) -> Result<Self, AdapterError> {
        let base_url = config
            .get("base_url")
            .and_then(Value::as_str)
            .or_else(|| config.get("url").and_then(Value::as_str))
            .map(|value| value.trim_end_matches('/'))
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AdapterError::InvalidConfiguration(
                    "Missing 'base_url' in Squid destination config".to_string(),
                )
            })?
            .to_string();

        let object = SquidObject::parse(
            config
                .get("object")
                .and_then(Value::as_str)
                .unwrap_or("people"),
        )?;

        let timeout_secs = config
            .get("timeout_secs")
            .and_then(Value::as_u64)
            .unwrap_or(30)
            .clamp(1, 300);
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|error| AdapterError::ConnectionError(error.to_string()))?;

        let headers = build_headers(&auth)?;
        let idempotency_key_prefix = config
            .get("idempotency_key_prefix")
            .and_then(Value::as_str)
            .map(sanitize_component)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "pyreverseetl".to_string());

        Ok(Self {
            base_url,
            object,
            headers,
            client,
            retry_policy: RetryPolicy::default(),
            idempotency_key_prefix,
        })
    }

    fn idempotency_key(&self, entity_id: &str) -> String {
        let id = sanitize_component(entity_id);
        let id = if id.is_empty() {
            format!(
                "record-{}",
                crate::idempotency::content_hash(&json!(entity_id))
            )
        } else {
            id
        };
        let key = format!(
            "{}:{}:{id}",
            self.idempotency_key_prefix,
            self.object.path()
        );
        key.chars().take(255).collect()
    }

    fn transform_entity(&self, entity: &Entity) -> Result<Value, AdapterError> {
        match self.object {
            SquidObject::People => person_payload(entity),
            SquidObject::Companies => company_payload(entity),
        }
    }

    fn send(&self, payload: &Value, idempotency_key: &str) -> Result<Value, AdapterError> {
        let url = format!("{}/rest/{}", self.base_url, self.object.path());

        self.retry_policy.execute_blocking(|| {
            let mut request = self
                .client
                .post(&url)
                .header("Accept", "application/json")
                .header("Idempotency-Key", idempotency_key)
                .header("X-Correlation-Id", idempotency_key)
                .json(payload);

            for (key, value) in &self.headers {
                request = request.header(key.as_str(), value.as_str());
            }

            let response = request
                .send()
                .map_err(|error| AdapterError::NetworkError(error.to_string()))?;
            let status = response.status();
            let retry_after_ms = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(|seconds| (seconds.saturating_mul(1_000)).min(30_000) as u32)
                .unwrap_or(5_000);

            match status.as_u16() {
                200..=299 => response
                    .json::<Value>()
                    .map_err(|error| AdapterError::OperationFailed(error.to_string())),
                401 | 403 => Err(AdapterError::AuthenticationFailed(format!(
                    "Squid rejected credentials: HTTP {status}"
                ))),
                409 => Err(AdapterError::OperationFailed(
                    "Squid idempotency conflict: the key was reused with a different body"
                        .to_string(),
                )),
                429 => Err(AdapterError::RateLimitExceeded { retry_after_ms }),
                500..=599 => Err(AdapterError::ConnectionError(format!(
                    "Squid server error: HTTP {status}"
                ))),
                _ => Err(AdapterError::OperationFailed(format!(
                    "Squid returned HTTP {status}"
                ))),
            }
        })
    }
}

fn build_headers(auth: &AuthMethod) -> Result<HashMap<String, String>, AdapterError> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    match auth {
        // Squid API keys are bearer tokens. Accept the generic ApiKey variant
        // as a bearer credential rather than sending a header Squid ignores.
        AuthMethod::Bearer { token } | AuthMethod::ApiKey { key: token } => {
            headers.insert("Authorization".to_string(), format!("Bearer {token}"));
        }
        AuthMethod::Basic { username, password } => {
            use base64::Engine;
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            headers.insert("Authorization".to_string(), format!("Basic {encoded}"));
        }
        AuthMethod::OAuth { .. } => {
            return Err(AdapterError::InvalidConfiguration(
                "Squid destinations use a bearer API key, not OAuth".to_string(),
            ));
        }
    }

    Ok(headers)
}

fn value_for<'a>(entity: &'a Entity, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| {
        entity
            .get_attribute(key)
            .or_else(|| entity.get_trait(key))
            .filter(|value| !value.is_null())
    })
}

fn string_for(entity: &Entity, keys: &[&str]) -> Option<String> {
    value_for(entity, keys)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn nested_string_for(entity: &Entity, object_key: &str, nested_key: &str) -> Option<String> {
    value_for(entity, &[object_key])
        .and_then(Value::as_object)
        .and_then(|object| object.get(nested_key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn person_payload(entity: &Entity) -> Result<Value, AdapterError> {
    let first_name = string_for(entity, &["first_name", "firstName", "given_name"])
        .or_else(|| nested_string_for(entity, "name", "firstName"))
        .unwrap_or_default();
    let last_name = string_for(entity, &["last_name", "lastName", "family_name"])
        .or_else(|| nested_string_for(entity, "name", "lastName"))
        .unwrap_or_default();
    if first_name.is_empty() && last_name.is_empty() {
        return Err(AdapterError::ValidationError(
            "Squid person activation requires first_name or last_name".to_string(),
        ));
    }

    let email = string_for(
        entity,
        &[
            "email",
            "primary_email",
            "primaryEmail",
            "emails.primaryEmail",
        ],
    )
    .or_else(|| nested_string_for(entity, "emails", "primaryEmail"))
    .ok_or_else(|| {
        AdapterError::ValidationError("Squid person activation requires email".to_string())
    })?;

    let mut payload = Map::new();
    payload.insert(
        "name".into(),
        json!({"firstName": first_name, "lastName": last_name}),
    );
    payload.insert("emails".into(), json!({"primaryEmail": email}));

    if let Some(job_title) = string_for(entity, &["job_title", "jobTitle"]) {
        payload.insert("jobTitle".into(), json!(job_title));
    }
    if let Some(phone) = string_for(
        entity,
        &[
            "phone",
            "primary_phone",
            "primaryPhone",
            "phones.primaryPhoneNumber",
        ],
    )
    .or_else(|| nested_string_for(entity, "phones", "primaryPhoneNumber"))
    {
        payload.insert("phones".into(), json!({"primaryPhoneNumber": phone}));
    }
    if let Some(company_id) = string_for(entity, &["company_id", "companyId"]) {
        payload.insert("companyId".into(), json!(company_id));
    }

    Ok(Value::Object(payload))
}

fn company_payload(entity: &Entity) -> Result<Value, AdapterError> {
    let name = string_for(entity, &["name", "company_name", "companyName"]).ok_or_else(|| {
        AdapterError::ValidationError("Squid company activation requires name".to_string())
    })?;
    let mut payload = Map::new();
    payload.insert("name".into(), json!(name));
    if let Some(domain) = string_for(
        entity,
        &["domain", "domain_name", "domainName", "website", "url"],
    ) {
        payload.insert(
            "domainName".into(),
            json!({"primaryLinkLabel": "", "primaryLinkUrl": domain}),
        );
    }
    Ok(Value::Object(payload))
}

fn sanitize_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':' | '@')
            {
                character
            } else {
                '-'
            }
        })
        .take(120)
        .collect()
}

impl DestinationAdapter for SquidAdapter {
    fn authenticate(&self) -> Result<(), AdapterError> {
        if self.base_url.is_empty() {
            return Err(AdapterError::AuthenticationFailed(
                "No Squid base_url configured".to_string(),
            ));
        }
        if !self.headers.contains_key("Authorization") {
            return Err(AdapterError::AuthenticationFailed(
                "No Squid API key configured".to_string(),
            ));
        }
        Ok(())
    }

    fn upsert(
        &self,
        entity: &Entity,
        _mappings: &[FieldMapping],
    ) -> Result<OperationResult, AdapterError> {
        let payload = match self.transform_entity(entity) {
            Ok(payload) => payload,
            Err(error) => {
                return Ok(OperationResult {
                    id: entity.id.clone(),
                    success: false,
                    external_id: None,
                    error_message: Some(error.to_string()),
                });
            }
        };
        let key = self.idempotency_key(&entity.id);
        match self.send(&payload, &key) {
            Ok(response) => {
                let external_id = response
                    .get("data")
                    .and_then(|data| data.get(self.object.response_key()))
                    .and_then(|record| record.get("id"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                Ok(OperationResult {
                    id: entity.id.clone(),
                    success: true,
                    external_id,
                    error_message: None,
                })
            }
            Err(error) => Ok(OperationResult {
                id: entity.id.clone(),
                success: false,
                external_id: None,
                error_message: Some(error.to_string()),
            }),
        }
    }

    fn batch_upsert(
        &self,
        entities: Vec<Entity>,
        mappings: &[FieldMapping],
    ) -> Result<BatchResult, AdapterError> {
        let start = std::time::Instant::now();
        let total = entities.len() as u32;
        let mut successful = 0;
        let mut failed = 0;
        let mut results = Vec::with_capacity(entities.len());

        for entity in entities {
            let result = self.upsert(&entity, mappings)?;
            if result.success {
                successful += 1;
            } else {
                failed += 1;
            }
            results.push(result);
        }

        Ok(BatchResult {
            total,
            successful,
            failed,
            results,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn delete(&self, _id: &str) -> Result<(), AdapterError> {
        Err(AdapterError::NotImplemented(
            "Squid activation is create-only; deleting by a warehouse id is unsafe".to_string(),
        ))
    }

    fn get_schema(&self) -> Result<DestinationSchema, AdapterError> {
        let mut fields = HashMap::new();
        match self.object {
            SquidObject::People => {
                fields.insert(
                    "first_name".to_string(),
                    FieldType::String { max_length: None },
                );
                fields.insert(
                    "last_name".to_string(),
                    FieldType::String { max_length: None },
                );
                fields.insert("email".to_string(), FieldType::Email);
                fields.insert("phone".to_string(), FieldType::String { max_length: None });
                fields.insert(
                    "job_title".to_string(),
                    FieldType::String { max_length: None },
                );
                fields.insert(
                    "company_id".to_string(),
                    FieldType::String { max_length: None },
                );
            }
            SquidObject::Companies => {
                fields.insert("name".to_string(), FieldType::String { max_length: None });
                fields.insert("domain".to_string(), FieldType::Url);
            }
        }
        Ok(DestinationSchema {
            fields,
            required_fields: match self.object {
                SquidObject::People => {
                    vec!["first_name".into(), "last_name".into(), "email".into()]
                }
                SquidObject::Companies => vec!["name".into()],
            },
            max_batch_size: 1_000,
        })
    }

    fn name(&self) -> &str {
        "squid"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;
    use crate::testing::MockHttpServer;
    use serde_json::json;

    fn config(server: &MockHttpServer) -> HashMap<String, Value> {
        let mut config = HashMap::new();
        config.insert("base_url".to_string(), json!(server.base_url));
        config.insert("object".to_string(), json!("people"));
        config
    }

    fn person() -> Entity {
        Entity::new(EntityType::Customer, "email", "row-1")
            .add_attribute("first_name", json!("Ada"))
            .add_attribute("last_name", json!("Lovelace"))
            .add_attribute("email", json!("ada@example.com"))
            .add_attribute("job_title", json!("Engineer"))
            .add_attribute("company_id", json!("00000000-0000-4000-8000-000000000001"))
    }

    #[test]
    fn person_payload_uses_native_squid_shape_and_allowlist() {
        let mut entity = person();
        entity.attributes["secret_unrelated"] = json!("must not be forwarded");
        let server = MockHttpServer::start(201, r#"{"data":{"createPerson":{"id":"squid-1"}}}"#);
        let adapter = SquidAdapter::new(
            &config(&server),
            AuthMethod::Bearer {
                token: "test-token".to_string(),
            },
        )
        .unwrap();
        let result = adapter.upsert(&entity, &[]).unwrap();
        assert!(result.success);
        assert_eq!(result.external_id.as_deref(), Some("squid-1"));

        let request = server.last_request().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/rest/people");
        assert_eq!(request.headers["authorization"], "Bearer test-token");
        assert_eq!(
            request.headers["idempotency-key"],
            "pyreverseetl:people:row-1"
        );
        assert_eq!(
            request.headers["x-correlation-id"],
            "pyreverseetl:people:row-1"
        );
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["name"]["firstName"], "Ada");
        assert_eq!(body["name"]["lastName"], "Lovelace");
        assert_eq!(body["emails"]["primaryEmail"], "ada@example.com");
        assert_eq!(body["jobTitle"], "Engineer");
        assert_eq!(body["companyId"], "00000000-0000-4000-8000-000000000001");
        assert!(body.get("secret_unrelated").is_none());
    }

    #[test]
    fn idempotency_key_is_stable_for_retries() {
        let server = MockHttpServer::start(201, r#"{"data":{"createPerson":{"id":"squid-1"}}}"#);
        let adapter = SquidAdapter::new(
            &config(&server),
            AuthMethod::Bearer {
                token: "test-token".to_string(),
            },
        )
        .unwrap();
        let entity = person();
        adapter.upsert(&entity, &[]).unwrap();
        adapter.upsert(&entity, &[]).unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0].headers["idempotency-key"],
            requests[1].headers["idempotency-key"]
        );
    }

    #[test]
    fn missing_email_is_a_validation_error() {
        let server = MockHttpServer::start(201, "{}");
        let adapter = SquidAdapter::new(
            &config(&server),
            AuthMethod::Bearer {
                token: "test-token".to_string(),
            },
        )
        .unwrap();
        let entity = Entity::new(EntityType::Customer, "id", "row-1")
            .add_attribute("first_name", json!("Ada"))
            .add_attribute("last_name", json!("Lovelace"));
        let result = adapter.upsert(&entity, &[]).unwrap();
        assert!(!result.success);
        assert!(result.error_message.unwrap().contains("requires email"));
    }

    #[test]
    fn conflict_is_not_retried() {
        let server = MockHttpServer::start(409, r#"{"messages":["conflict"]}"#);
        let adapter = SquidAdapter::new(
            &config(&server),
            AuthMethod::Bearer {
                token: "test-token".to_string(),
            },
        )
        .unwrap();
        let result = adapter.upsert(&person(), &[]).unwrap();
        assert!(!result.success);
        assert!(result
            .error_message
            .unwrap()
            .contains("idempotency conflict"));
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn companies_use_the_company_endpoint_and_payload() {
        let server = MockHttpServer::start(201, r#"{"data":{"createCompany":{"id":"company-1"}}}"#);
        let mut cfg = config(&server);
        cfg.insert("object".to_string(), json!("company"));
        let adapter = SquidAdapter::new(
            &cfg,
            AuthMethod::ApiKey {
                key: "api-key".to_string(),
            },
        )
        .unwrap();
        let entity = Entity::new(EntityType::Company, "id", "company-row")
            .add_attribute("name", json!("Acme"))
            .add_attribute("domain", json!("https://acme.example"));
        let result = adapter.upsert(&entity, &[]).unwrap();
        assert!(result.success);
        assert_eq!(result.external_id.as_deref(), Some("company-1"));
        let request = server.last_request().unwrap();
        assert_eq!(request.path, "/rest/companies");
        assert_eq!(request.headers["authorization"], "Bearer api-key");
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(body["name"], "Acme");
        assert_eq!(body["domainName"]["primaryLinkUrl"], "https://acme.example");
    }
}
