/// Quality Gate - Integration with StatGuardian
///
/// Validates data quality before activation:
/// - Quality score checking
/// - Drift detection
/// - Threshold enforcement

use crate::{Entity, Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

/// StatGuardian-backed quality gate
pub struct StatGuardianGate {
    endpoint: String,
    timeout_ms: u64,
}

impl StatGuardianGate {
    pub fn new(endpoint: String, timeout_ms: u64) -> Self {
        Self {
            endpoint,
            timeout_ms,
        }
    }
}

#[async_trait]
impl QualityGate for StatGuardianGate {
    async fn validate(&self, entity: &Entity) -> Result<ValidationResult> {
        // In production, this would call StatGuardian API
        // For now, return mock validation result
        Ok(ValidationResult {
            passed: true,
            quality_score: 0.95,
            issues: vec![],
            schema_version: "v1.0.0".to_string(),
        })
    }

    async fn check_drift(&self, entity: &Entity) -> Result<DriftReport> {
        // In production, this would call StatGuardian drift detection
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
        let gate = MockQualityGate::new(true, 0.95);
        let entity = Entity {
            id: "test".to_string(),
            data: serde_json::json!({}),
            metadata: Default::default(),
        };

        let result = gate.validate(&entity).await.unwrap();
        assert!(result.passed);
        assert_eq!(result.quality_score, 0.95);
    }

    #[tokio::test]
    async fn test_mock_quality_gate_fails() {
        let gate = MockQualityGate::new(false, 0.5);
        let entity = Entity {
            id: "test".to_string(),
            data: serde_json::json!({}),
            metadata: Default::default(),
        };

        let result = gate.validate(&entity).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.quality_score, 0.5);
        assert!(!result.issues.is_empty());
    }

    #[tokio::test]
    async fn test_drift_detection() {
        let gate = MockQualityGate::new(true, 0.95);
        let entity = Entity {
            id: "test".to_string(),
            data: serde_json::json!({}),
            metadata: Default::default(),
        };

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
