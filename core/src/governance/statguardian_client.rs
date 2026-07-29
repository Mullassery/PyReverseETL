/// HTTP Client for StatGuardian API
///
/// Handles communication with StatGuardian service for quality validation,
/// schema change detection, and compliance rule retrieval.

use crate::{Entity, Result, Error};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::sync::Arc;
use reqwest::Client;

/// Request to validate entity against quality contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRequest {
    pub entity: Entity,
    #[serde(default)]
    pub contract_id: String,
    #[serde(default)]
    pub strict: bool,
}

/// Response from quality validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    pub passed: bool,
    pub quality_score: f64,
    pub issues: Vec<String>,
    pub schema_version: String,
}

/// Request to detect schema changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaCheckRequest {
    pub entity: Entity,
    #[serde(default)]
    pub current_version: String,
}

/// Response from schema change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaCheckResponse {
    pub changes: Vec<SchemaChangeDetail>,
    pub migration_required: bool,
}

/// Individual schema change detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChangeDetail {
    pub field: String,
    pub change_type: String,
    pub old_type: Option<String>,
    pub new_type: Option<String>,
}

/// HTTP client for StatGuardian API
pub struct StatGuardianClient {
    base_url: String,
    api_key: String,
    timeout: Duration,
    client: Client,
}

impl StatGuardianClient {
    /// Create a new StatGuardian client
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_timeout(base_url, api_key, Duration::from_secs(5))
    }

    /// Create a client with custom timeout
    pub fn with_timeout(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout,
            client,
        }
    }

    /// Validate entity against quality contract
    pub async fn validate(&self, entity: &Entity) -> Result<crate::governance::ValidationResult> {
        let request = ValidateRequest {
            entity: entity.clone(),
            contract_id: "default".to_string(),
            strict: true,
        };

        let url = format!("{}/validate", self.base_url);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::ConfigError(
                format!("StatGuardian API request failed: {}", e)
            ))?;

        if response.status().is_success() {
            let validate_response: ValidateResponse = response
                .json()
                .await
                .map_err(|e| Error::ConfigError(
                    format!("Failed to parse StatGuardian response: {}", e)
                ))?;

            Ok(crate::governance::ValidationResult {
                passed: validate_response.passed,
                quality_score: validate_response.quality_score,
                issues: validate_response.issues,
                schema_version: validate_response.schema_version,
            })
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(Error::ConfigError(
                format!("StatGuardian API error {}: {}", status, error_text)
            ))
        }
    }

    /// Detect schema changes in entity
    pub async fn detect_schema_changes(&self, entity: &Entity) -> Result<Vec<crate::governance::SchemaChange>> {
        let request = SchemaCheckRequest {
            entity: entity.clone(),
            current_version: "v1.0.0".to_string(),
        };

        let url = format!("{}/detect-changes", self.base_url);

        let response = self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::ConfigError(
                format!("Schema check request failed: {}", e)
            ))?;

        if response.status().is_success() {
            let schema_response: SchemaCheckResponse = response
                .json()
                .await
                .map_err(|e| Error::ConfigError(
                    format!("Failed to parse schema response: {}", e)
                ))?;

            let changes = schema_response.changes
                .into_iter()
                .map(|detail| {
                    let change_type = match detail.change_type.as_str() {
                        "Added" => crate::governance::SchemaChangeType::Added,
                        "Removed" => crate::governance::SchemaChangeType::Removed,
                        "TypeChanged" => crate::governance::SchemaChangeType::TypeChanged,
                        "Renamed" => crate::governance::SchemaChangeType::Renamed,
                        _ => crate::governance::SchemaChangeType::Added,
                    };

                    crate::governance::SchemaChange {
                        field_name: detail.field,
                        change_type,
                        old_type: detail.old_type,
                        new_type: detail.new_type,
                        details: None,
                    }
                })
                .collect();

            Ok(changes)
        } else {
            let status = response.status();
            Err(Error::ConfigError(
                format!("Schema check failed with status {}", status)
            ))
        }
    }

    /// Check connectivity to StatGuardian API
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);

        match self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = StatGuardianClient::new("http://localhost:8080", "test-key");
        assert_eq!(client.base_url, "http://localhost:8080");
        assert_eq!(client.timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_client_with_custom_timeout() {
        let timeout = Duration::from_secs(10);
        let client = StatGuardianClient::with_timeout(
            "http://localhost:8080",
            "test-key",
            timeout,
        );
        assert_eq!(client.timeout, timeout);
    }

    #[test]
    fn test_validate_request_serialization() {
        use crate::entity::EntityType;
        let entity = Entity::new(EntityType::Customer, "email", "test@example.com");
        let request = ValidateRequest {
            entity,
            contract_id: "customers".to_string(),
            strict: true,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("contract_id"));
        assert!(json.contains("customers"));
    }

    #[test]
    fn test_validate_response_deserialization() {
        let json = r#"{
            "passed": true,
            "quality_score": 0.95,
            "issues": [],
            "schema_version": "v1.0.0"
        }"#;

        let response: ValidateResponse = serde_json::from_str(json).unwrap();
        assert!(response.passed);
        assert_eq!(response.quality_score, 0.95);
    }

    #[test]
    fn test_schema_change_detail() {
        let json = r#"{
            "field": "email",
            "change_type": "TypeChanged",
            "old_type": "string",
            "new_type": "text"
        }"#;

        let detail: SchemaChangeDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.field, "email");
        assert_eq!(detail.change_type, "TypeChanged");
    }
}
