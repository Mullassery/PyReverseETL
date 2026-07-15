use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alert message compatible with OpenTelemetry and monitoring systems
/// Can be exported to monitoring backends without building alerting logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMessage {
    /// Unique alert identifier
    pub alert_id: String,
    /// Alert severity level
    pub severity: AlertSeverity,
    /// Alert category (drift, error, performance, etc.)
    pub category: AlertCategory,
    /// Human-readable alert message
    pub message: String,
    /// Additional context data for monitoring systems
    pub context: HashMap<String, String>,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Resource/entity affected
    pub resource: String,
    /// Tags for filtering/routing in monitoring systems
    pub tags: Vec<String>,
}

/// Alert severity levels (compatible with common monitoring systems)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum AlertSeverity {
    /// Information only
    Info,
    /// Warning - something unusual but not critical
    Warning,
    /// Error - operation failed
    Error,
    /// Critical - system impaired
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }
}

/// Alert categories for classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertCategory {
    /// Schema drift detected
    SchemaDrift,
    /// Type mismatch between expected and actual
    TypeMismatch,
    /// Field missing from entity
    MissingField,
    /// Unexpected null/empty values
    NullValueDetected,
    /// Adapter connection error
    ConnectionError,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Sync operation failed
    SyncFailure,
    /// Performance degradation
    Performance,
    /// Authentication/authorization issue
    AuthError,
    /// Validation failed
    ValidationError,
    /// Custom alert
    Custom(String),
}

impl AlertCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::SchemaDrift => "schema_drift",
            Self::TypeMismatch => "type_mismatch",
            Self::MissingField => "missing_field",
            Self::NullValueDetected => "null_value_detected",
            Self::ConnectionError => "connection_error",
            Self::RateLimitExceeded => "rate_limit_exceeded",
            Self::SyncFailure => "sync_failure",
            Self::Performance => "performance",
            Self::AuthError => "auth_error",
            Self::ValidationError => "validation_error",
            Self::Custom(s) => s,
        }
    }
}

/// Schema drift alert (when detected schema differs from expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDriftAlert {
    pub adapter: String,
    pub expected_fields: Vec<String>,
    pub actual_fields: Vec<String>,
    pub missing_fields: Vec<String>,
    pub unexpected_fields: Vec<String>,
    pub type_mismatches: Vec<(String, String, String)>, // (field, expected, actual)
}

impl SchemaDriftAlert {
    pub fn to_alert_message(&self) -> AlertMessage {
        let mut context = HashMap::new();
        context.insert("adapter".to_string(), self.adapter.clone());
        context.insert("missing_fields".to_string(), self.missing_fields.join(", "));
        context.insert("unexpected_fields".to_string(), self.unexpected_fields.join(", "));
        context.insert(
            "type_mismatches".to_string(),
            self.type_mismatches
                .iter()
                .map(|(f, e, a)| format!("{}({} → {})", f, e, a))
                .collect::<Vec<_>>()
                .join("; "),
        );

        AlertMessage {
            alert_id: format!("drift-{}-{}", self.adapter, chrono::Utc::now().timestamp()),
            severity: AlertSeverity::Warning,
            category: AlertCategory::SchemaDrift,
            message: format!(
                "Schema drift detected in {}: {} missing, {} unexpected, {} type mismatches",
                self.adapter,
                self.missing_fields.len(),
                self.unexpected_fields.len(),
                self.type_mismatches.len()
            ),
            context,
            timestamp: chrono::Utc::now().to_rfc3339(),
            resource: format!("adapter/{}", self.adapter),
            tags: vec!["schema".to_string(), "drift".to_string(), "adapter".to_string()],
        }
    }
}

/// Type mismatch alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMismatchAlert {
    pub field: String,
    pub expected_type: String,
    pub actual_type: String,
    pub entity_id: String,
}

impl TypeMismatchAlert {
    pub fn to_alert_message(&self) -> AlertMessage {
        let mut context = HashMap::new();
        context.insert("field".to_string(), self.field.clone());
        context.insert("expected_type".to_string(), self.expected_type.clone());
        context.insert("actual_type".to_string(), self.actual_type.clone());
        context.insert("entity_id".to_string(), self.entity_id.clone());

        AlertMessage {
            alert_id: format!("mismatch-{}-{}", self.entity_id, chrono::Utc::now().timestamp()),
            severity: AlertSeverity::Error,
            category: AlertCategory::TypeMismatch,
            message: format!(
                "Type mismatch in field '{}': expected {}, got {} (entity: {})",
                self.field, self.expected_type, self.actual_type, self.entity_id
            ),
            context,
            timestamp: chrono::Utc::now().to_rfc3339(),
            resource: format!("entity/{}", self.entity_id),
            tags: vec!["validation".to_string(), "type".to_string()],
        }
    }
}

/// Rate limit alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitAlert {
    pub adapter: String,
    pub limit_type: String, // "requests_per_second", "batch_size", etc.
    pub limit_value: u32,
    pub current_value: u32,
    pub retry_after_ms: Option<u32>,
}

impl RateLimitAlert {
    pub fn to_alert_message(&self) -> AlertMessage {
        let mut context = HashMap::new();
        context.insert("adapter".to_string(), self.adapter.clone());
        context.insert("limit_type".to_string(), self.limit_type.clone());
        context.insert("limit".to_string(), self.limit_value.to_string());
        context.insert("current".to_string(), self.current_value.to_string());
        if let Some(retry) = self.retry_after_ms {
            context.insert("retry_after_ms".to_string(), retry.to_string());
        }

        AlertMessage {
            alert_id: format!("rate-limit-{}-{}", self.adapter, chrono::Utc::now().timestamp()),
            severity: AlertSeverity::Warning,
            category: AlertCategory::RateLimitExceeded,
            message: format!(
                "Rate limit exceeded for {}: {} {} (limit: {})",
                self.adapter, self.current_value, self.limit_type, self.limit_value
            ),
            context,
            timestamp: chrono::Utc::now().to_rfc3339(),
            resource: format!("adapter/{}", self.adapter),
            tags: vec!["rate_limit".to_string(), "adapter".to_string()],
        }
    }
}

/// Alert builder for programmatic creation
pub struct AlertBuilder {
    alert_id: String,
    severity: AlertSeverity,
    category: AlertCategory,
    message: String,
    context: HashMap<String, String>,
    resource: String,
    tags: Vec<String>,
}

impl AlertBuilder {
    pub fn new(alert_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            alert_id: alert_id.into(),
            severity: AlertSeverity::Info,
            category: AlertCategory::Custom("generic".to_string()),
            message: message.into(),
            context: HashMap::new(),
            resource: "system".to_string(),
            tags: Vec::new(),
        }
    }

    pub fn severity(mut self, severity: AlertSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn category(mut self, category: AlertCategory) -> Self {
        self.category = category;
        self
    }

    pub fn context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = resource.into();
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn build(self) -> AlertMessage {
        AlertMessage {
            alert_id: self.alert_id,
            severity: self.severity,
            category: self.category,
            message: self.message,
            context: self.context,
            timestamp: chrono::Utc::now().to_rfc3339(),
            resource: self.resource,
            tags: self.tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Info < AlertSeverity::Warning);
        assert!(AlertSeverity::Warning < AlertSeverity::Error);
        assert!(AlertSeverity::Error < AlertSeverity::Critical);
    }

    #[test]
    fn test_schema_drift_alert_to_message() {
        let alert = SchemaDriftAlert {
            adapter: "salesforce".to_string(),
            expected_fields: vec!["email".to_string(), "name".to_string()],
            actual_fields: vec!["email".to_string()],
            missing_fields: vec!["name".to_string()],
            unexpected_fields: vec![],
            type_mismatches: vec![],
        };

        let msg = alert.to_alert_message();
        assert_eq!(msg.severity, AlertSeverity::Warning);
        assert_eq!(msg.category, AlertCategory::SchemaDrift);
        assert!(msg.message.contains("1 missing"));
    }

    #[test]
    fn test_type_mismatch_alert_to_message() {
        let alert = TypeMismatchAlert {
            field: "revenue".to_string(),
            expected_type: "Float".to_string(),
            actual_type: "String".to_string(),
            entity_id: "cust_123".to_string(),
        };

        let msg = alert.to_alert_message();
        assert_eq!(msg.severity, AlertSeverity::Error);
        assert_eq!(msg.category, AlertCategory::TypeMismatch);
        assert!(msg.message.contains("revenue"));
    }

    #[test]
    fn test_rate_limit_alert_to_message() {
        let alert = RateLimitAlert {
            adapter: "hubspot".to_string(),
            limit_type: "requests_per_second".to_string(),
            limit_value: 10,
            current_value: 15,
            retry_after_ms: Some(5000),
        };

        let msg = alert.to_alert_message();
        assert_eq!(msg.severity, AlertSeverity::Warning);
        assert_eq!(msg.category, AlertCategory::RateLimitExceeded);
    }

    #[test]
    fn test_alert_builder() {
        let alert = AlertBuilder::new("test-alert", "Test message")
            .severity(AlertSeverity::Critical)
            .category(AlertCategory::SyncFailure)
            .context("adapter", "marketo")
            .resource("workflow/wf_123")
            .tag("sync")
            .tag("error")
            .build();

        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert_eq!(alert.category, AlertCategory::SyncFailure);
        assert_eq!(alert.tags.len(), 2);
    }
}
