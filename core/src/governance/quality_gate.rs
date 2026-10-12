/// Quality Gate - Integration with StatGuardian
///
/// Validates data quality before activation:
/// - Quality score checking
/// - Drift detection
/// - Threshold enforcement
use super::statguardian_client::StatGuardianClient;
use crate::{Entity, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Result of quality validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// Quality score (0.0-1.0)
    pub quality_score: f64,
    /// Any issues found
    pub issues: Vec<String>,
    /// Schema version of validated entity
    pub schema_version: String,
}

/// Drift report from quality checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub detected: bool,
    pub drift_percentage: f64,
    pub affected_fields: Vec<String>,
}

/// Quality gate trait - validates data quality
#[async_trait]
pub trait QualityGate: Send + Sync {
    /// Validate entity against quality contract
    async fn validate(&self, entity: &Entity) -> Result<ValidationResult>;

    /// Detect data drift
    async fn check_drift(&self, entity: &Entity) -> Result<DriftReport>;

    /// Get quality threshold
    async fn get_quality_threshold(&self) -> Result<f64>;
}

/// StatGuardian-backed quality gate.
///
/// Delegates to a real [`StatGuardianClient`] over HTTP: `validate` calls the
/// StatGuardian `/validate` endpoint and `check_drift` calls `/detect-changes`
/// and maps the returned schema changes onto a [`DriftReport`]. If the
/// StatGuardian service is unreachable or returns an error, the calls return an
/// error rather than silently passing.
pub struct StatGuardianGate {
    client: Arc<StatGuardianClient>,
}

impl StatGuardianGate {
    /// Create a gate for the given base URL and timeout.
    ///
    /// No bearer token is sent; use [`Self::with_api_key`] when the
    /// StatGuardian deployment requires authentication.
    pub fn new(endpoint: String, timeout_ms: u64) -> Self {
        Self::with_api_key(endpoint, String::new(), timeout_ms)
    }

    /// Create a gate that authenticates with the given API key.
    pub fn with_api_key(endpoint: String, api_key: String, timeout_ms: u64) -> Self {
        let client =
            StatGuardianClient::with_timeout(endpoint, api_key, Duration::from_millis(timeout_ms));
        Self {
            client: Arc::new(client),
        }
    }
}

#[async_trait]
impl QualityGate for StatGuardianGate {
    async fn validate(&self, entity: &Entity) -> Result<ValidationResult> {
        self.client.validate(entity).await
    }

    async fn check_drift(&self, entity: &Entity) -> Result<DriftReport> {
        let changes = self.client.detect_schema_changes(entity).await?;
        let total_fields = entity
            .attributes
            .as_object()
            .map(|obj| obj.len())
            .unwrap_or(0)
            .max(changes.len());
        let drift_percentage = if total_fields == 0 {
            0.0
        } else {
            (changes.len() as f64 / total_fields as f64) * 100.0
        };

        Ok(DriftReport {
            detected: !changes.is_empty(),
            drift_percentage,
            affected_fields: changes.into_iter().map(|c| c.field_name).collect(),
        })
    }

    async fn get_quality_threshold(&self) -> Result<f64> {
        Ok(0.9)
    }
}

/// Mock quality gate for testing
#[cfg(test)]
pub struct MockQualityGate {
    should_pass: bool,
    quality_score: f64,
}

#[cfg(test)]
impl MockQualityGate {
    pub fn new(should_pass: bool, quality_score: f64) -> Self {
        Self {
            should_pass,
            quality_score,
        }
    }
}

#[cfg(test)]
#[async_trait]
impl QualityGate for MockQualityGate {
    async fn validate(&self, _entity: &Entity) -> Result<ValidationResult> {
        Ok(ValidationResult {
            passed: self.should_pass,
            quality_score: self.quality_score,
            issues: if self.should_pass {
                vec![]
            } else {
                vec!["Mock validation failed".to_string()]
            },
            schema_version: "v1.0.0".to_string(),
        })
    }

    async fn check_drift(&self, _entity: &Entity) -> Result<DriftReport> {
        Ok(DriftReport {
            detected: false,
            drift_percentage: 0.0,
            affected_fields: vec![],
        })
    }

    async fn get_quality_threshold(&self) -> Result<f64> {
        Ok(0.9)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_quality_gate_passes() {
        use crate::entity::EntityType;
        let gate = MockQualityGate::new(true, 0.95);
        let entity = Entity::new(EntityType::Custom("test".to_string()), "id", "test");

        let result = gate.validate(&entity).await.unwrap();
        assert!(result.passed);
        assert_eq!(result.quality_score, 0.95);
    }

    #[tokio::test]
    async fn test_mock_quality_gate_fails() {
        use crate::entity::EntityType;
        let gate = MockQualityGate::new(false, 0.5);
        let entity = Entity::new(EntityType::Custom("test".to_string()), "id", "test");

        let result = gate.validate(&entity).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.quality_score, 0.5);
        assert!(!result.issues.is_empty());
    }

    #[tokio::test]
    async fn test_drift_detection() {
        use crate::entity::EntityType;
        let gate = MockQualityGate::new(true, 0.95);
        let entity = Entity::new(EntityType::Custom("test".to_string()), "id", "test");

        let drift = gate.check_drift(&entity).await.unwrap();
        assert!(!drift.detected);
        assert_eq!(drift.drift_percentage, 0.0);
    }

    #[tokio::test]
    async fn test_quality_threshold() {
        let gate = MockQualityGate::new(true, 0.95);
        let threshold = gate.get_quality_threshold().await.unwrap();
        assert_eq!(threshold, 0.9);
    }
}
