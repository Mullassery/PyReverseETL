use crate::Entity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod error;
pub mod mapping;
pub mod schema_detect;
pub mod alert_compat;
pub mod retry_policy;
pub mod webhook;
pub mod salesforce;
pub mod hubspot;
pub mod marketo;

pub use error::AdapterError;

/// Field transformation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transformation {
    /// Pass through as-is
    Identity,
    /// Convert to uppercase
    Uppercase,
    /// Convert to lowercase
    Lowercase,
    /// Format as ISO 8601 timestamp
    ToTimestamp,
    /// Round to N decimal places
    RoundDecimals(u32),
    /// Custom transformation expression
    Custom(String),
}

/// Field mapping between source and destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub source_field: String,
    pub destination_field: String,
    pub transformation: Option<Transformation>,
    pub required: bool,
}

/// Destination schema describing available fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationSchema {
    pub fields: HashMap<String, FieldType>,
    pub required_fields: Vec<String>,
    pub max_batch_size: u32,
}

/// Field type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    String { max_length: Option<u32> },
    Integer,
    Float,
    Boolean,
    DateTime,
    Email,
    Url,
    Custom(String),
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// API key (HubSpot, Marketo)
    ApiKey { key: String },
    /// OAuth 2.0 (Salesforce)
    OAuth {
        client_id: String,
        client_secret: String,
        refresh_token: Option<String>,
    },
    /// Basic HTTP authentication
    Basic {
        username: String,
        password: String,
    },
    /// Bearer token
    Bearer { token: String },
}

/// Upsert mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpsertMode {
    /// Only create new records
    Create,
    /// Only update existing records
    Update,
    /// Create or update (default)
    CreateOrUpdate,
}

/// Result of a single record operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub id: String,
    pub success: bool,
    pub external_id: Option<String>,
    pub error_message: Option<String>,
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub total: u32,
    pub successful: u32,
    pub failed: u32,
    pub results: Vec<OperationResult>,
    pub duration_ms: u64,
}

/// Core adapter trait - implemented by all destination adapters
pub trait DestinationAdapter: Send + Sync {
    /// Authenticate and validate connection to destination
    fn authenticate(&self) -> Result<(), AdapterError>;

    /// Upsert a single entity to destination
    fn upsert(&self, entity: &Entity, mappings: &[FieldMapping]) -> Result<OperationResult, AdapterError>;

    /// Batch upsert multiple entities
    fn batch_upsert(&self, entities: Vec<Entity>, mappings: &[FieldMapping]) -> Result<BatchResult, AdapterError>;

    /// Delete entity by ID
    fn delete(&self, id: &str) -> Result<(), AdapterError>;

    /// Get destination-specific field schema
    fn get_schema(&self) -> Result<DestinationSchema, AdapterError>;

    /// Test the connection
    fn test_connection(&self) -> Result<(), AdapterError> {
        self.authenticate()
    }

    /// Get adapter name
    fn name(&self) -> &str;

    /// Get adapter version
    fn version(&self) -> &str {
        "0.1.0"
    }
}

/// Adapter factory for creating destination-specific adapters
pub struct AdapterFactory;

impl AdapterFactory {
    /// Create an adapter for the given destination type
    pub fn create_adapter(
        destination_type: &str,
        config: &HashMap<String, serde_json::Value>,
        auth: &AuthMethod,
    ) -> Result<Box<dyn DestinationAdapter>, AdapterError> {
        match destination_type {
            "webhook" => {
                let adapter = webhook::WebhookAdapter::new(config, auth.clone())?;
                Ok(Box::new(adapter))
            }
            "salesforce" => {
                let adapter = salesforce::SalesforceAdapter::new(config, auth.clone())?;
                Ok(Box::new(adapter))
            }
            "hubspot" => {
                let adapter = hubspot::HubSpotAdapter::new(config, auth.clone())?;
                Ok(Box::new(adapter))
            }
            "marketo" => {
                let adapter = marketo::MarketoAdapter::new(config, auth.clone())?;
                Ok(Box::new(adapter))
            }
            _ => Err(AdapterError::UnsupportedDestination(destination_type.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformation_identity() {
        let t = Transformation::Identity;
        assert_eq!(format!("{:?}", t), "Identity");
    }

    #[test]
    fn test_field_mapping_required() {
        let mapping = FieldMapping {
            source_field: "email".to_string(),
            destination_field: "Email".to_string(),
            transformation: None,
            required: true,
        };
        assert!(mapping.required);
    }

    #[test]
    fn test_upsert_mode_default() {
        let mode = UpsertMode::CreateOrUpdate;
        assert_eq!(mode, UpsertMode::CreateOrUpdate);
    }

    #[test]
    fn test_auth_method_api_key() {
        let auth = AuthMethod::ApiKey {
            key: "test_key".to_string(),
        };
        match auth {
            AuthMethod::ApiKey { key } => assert_eq!(key, "test_key"),
            _ => panic!("Wrong auth type"),
        }
    }

    #[test]
    fn test_operation_result_success() {
        let result = OperationResult {
            id: "123".to_string(),
            success: true,
            external_id: Some("ext_123".to_string()),
            error_message: None,
        };
        assert!(result.success);
        assert!(result.error_message.is_none());
    }
}
