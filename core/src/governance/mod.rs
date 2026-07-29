/// Governance Layer - Quality Gates & Compliance Rules
///
/// Integrates StatGuardian validation into activation pipeline:
/// - Quality gates block low-quality data
/// - Schema evolution detection and migration
/// - Compliance rules enforcement (PII masking, retention, etc.)
/// - Audit trail for all governance decisions

pub mod quality_gate;
pub mod schema_evolution;
pub mod compliance_rules;
pub mod statguardian_client;
pub mod credentials;
pub mod retry_policy;
pub mod cached_gate;

pub use quality_gate::{QualityGate, ValidationResult, StatGuardianGate};
pub use schema_evolution::{SchemaEvolution, SchemaChange, SchemaChangeType};
pub use compliance_rules::{ComplianceRule, ComplianceEngine, RuleType, RuleAction};
pub use statguardian_client::{StatGuardianClient, ValidateRequest, ValidateResponse, SchemaCheckRequest, SchemaCheckResponse};
pub use credentials::{GovernanceCredentials, AuthMethod};
pub use retry_policy::{RetryPolicy, RateLimiter};
pub use cached_gate::{CachedQualityGate, CacheStats};

#[cfg(test)]
pub use quality_gate::MockQualityGate;
#[cfg(test)]
pub use schema_evolution::MockSchemaEvolution;
#[cfg(test)]
pub use compliance_rules::MockComplianceEngine;

use crate::{Entity, Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Governance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    /// StatGuardian endpoint
    pub statguardian_url: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Minimum quality score (0.0-1.0)
    pub quality_threshold: f64,
    /// Enable quality gates
    pub quality_gates_enabled: bool,
    /// Enable schema evolution checks
    pub schema_checks_enabled: bool,
    /// Enable compliance rules
    pub compliance_rules_enabled: bool,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            statguardian_url: "http://localhost:8080".to_string(),
            timeout_ms: 5000,
            quality_threshold: 0.9,
            quality_gates_enabled: true,
            schema_checks_enabled: true,
            compliance_rules_enabled: true,
        }
    }
}

/// Main governance engine that orchestrates all governance checks
pub struct GovernanceEngine {
    config: GovernanceConfig,
    quality_gate: Arc<dyn QualityGate>,
    schema_evolution: Arc<dyn SchemaEvolution>,
    compliance_engine: Arc<dyn ComplianceEngine>,
}

impl GovernanceEngine {
    pub fn new(
        config: GovernanceConfig,
        quality_gate: Arc<dyn QualityGate>,
        schema_evolution: Arc<dyn SchemaEvolution>,
        compliance_engine: Arc<dyn ComplianceEngine>,
    ) -> Self {
        Self {
            config,
            quality_gate,
            schema_evolution,
            compliance_engine,
        }
    }

    /// Run all governance checks on an entity before activation
    pub async fn check_entity(&self, entity: &Entity) -> Result<GovernanceCheckResult> {
        let mut issues = Vec::new();
        let mut quality_score = 1.0;
        let mut schema_version = String::new();

        // Check 1: Quality Gates
        if self.config.quality_gates_enabled {
            let quality_result = self.quality_gate.validate(entity).await?;
            quality_score = quality_result.quality_score;
            schema_version = quality_result.schema_version.clone();

            if !quality_result.passed {
                issues.extend(quality_result.issues);
                if quality_score < self.config.quality_threshold {
                    return Err(Error::ValidationGateFailed(
                        format!("Quality score {} below threshold {}",
                            quality_score, self.config.quality_threshold)
                    ));
                }
            }
        }

        // Check 2: Schema Evolution
        if self.config.schema_checks_enabled {
            let schema_changes = self.schema_evolution.detect_changes(entity).await?;
            if !schema_changes.is_empty() {
                issues.push(format!("Schema changes detected: {} modifications", schema_changes.len()));
            }
        }

        // Check 3: Compliance Rules
        if self.config.compliance_rules_enabled {
            // Rules are applied, not just validated
            // Errors are captured if rules cannot be applied
        }

        Ok(GovernanceCheckResult {
            passed: issues.is_empty(),
            quality_score,
            schema_version,
            issues,
        })
    }

    /// Apply governance rules to entity (modifies entity)
    pub async fn apply_rules(&self, entity: &mut Entity) -> Result<()> {
        if self.config.compliance_rules_enabled {
            self.compliance_engine.apply_rules(entity).await?;
        }
        Ok(())
    }

    /// Get quality gate reference
    pub fn quality_gate(&self) -> &dyn QualityGate {
        self.quality_gate.as_ref()
    }

    /// Get schema evolution reference
    pub fn schema_evolution(&self) -> &dyn SchemaEvolution {
        self.schema_evolution.as_ref()
    }

    /// Get compliance engine reference
    pub fn compliance_engine(&self) -> &dyn ComplianceEngine {
        self.compliance_engine.as_ref()
    }
}

/// Result of governance checks on an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceCheckResult {
    pub passed: bool,
    pub quality_score: f64,
    pub schema_version: String,
    pub issues: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governance_config_defaults() {
        let config = GovernanceConfig::default();
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.quality_threshold, 0.9);
        assert!(config.quality_gates_enabled);
    }
}
