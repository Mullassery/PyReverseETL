pub mod cached_gate;
pub mod compliance_rules;
pub mod credentials;
/// Governance Layer - Quality Gates & Compliance Rules
///
/// Integrates StatGuardian validation into activation pipeline:
/// - Quality gates block low-quality data
/// - Schema evolution detection and migration
/// - Compliance rules enforcement (PII masking, retention, etc.)
/// - Audit trail for all governance decisions
pub mod quality_gate;
pub mod retry_policy;
pub mod schema_evolution;
pub mod statguardian_client;

pub use cached_gate::{CacheStats, CachedQualityGate};
pub use compliance_rules::{
    ComplianceCheckResult, ComplianceEngine, ComplianceRule, DefaultComplianceEngine, RuleAction,
    RuleType,
};
pub use credentials::{AuthMethod, GovernanceCredentials};
pub use quality_gate::{QualityGate, StatGuardianGate, ValidationResult};
pub use retry_policy::{RateLimiter, RetryPolicy};
pub use schema_evolution::{SchemaChange, SchemaChangeType, SchemaEvolution};
pub use statguardian_client::{
    SchemaCheckRequest, SchemaCheckResponse, StatGuardianClient, ValidateRequest, ValidateResponse,
};

#[cfg(test)]
pub use compliance_rules::MockComplianceEngine;
#[cfg(test)]
pub use quality_gate::MockQualityGate;
#[cfg(test)]
pub use schema_evolution::MockSchemaEvolution;

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
                    return Err(Error::ValidationGateFailed(format!(
                        "Quality score {} below threshold {}",
                        quality_score, self.config.quality_threshold
                    )));
                }
            }
        }

        // Check 2: Schema Evolution
        if self.config.schema_checks_enabled {
            let schema_changes = self.schema_evolution.detect_changes(entity).await?;
            if !schema_changes.is_empty() {
                issues.push(format!(
                    "Schema changes detected: {} modifications",
                    schema_changes.len()
                ));
            }
        }

        // Check 3: Compliance Rules
        if self.config.compliance_rules_enabled {
            let compliance_result = self.compliance_engine.check_compliance(entity).await?;
            if !compliance_result.compliant {
                issues.extend(compliance_result.violations);
            }
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

    // -- check_entity()'s compliance-rules check used to be a comment-only no-op
    // (`compliance_rules_enabled` was checked but nothing was actually done), so
    // real ComplianceEngine violations never affected `passed`. These use the real
    // DefaultComplianceEngine (not MockComplianceEngine, which always says "compliant")
    // to prove the wiring actually runs and actually affects the result.

    fn entity_with_attributes(attrs: serde_json::Value) -> Entity {
        use crate::entity::EntityType;
        let mut entity = Entity::new(EntityType::Custom("test".to_string()), "id", "test");
        entity.attributes = attrs;
        entity
    }

    #[tokio::test]
    async fn check_entity_fails_when_real_compliance_engine_finds_a_violation() {
        let config = GovernanceConfig::default();
        let quality_gate = Arc::new(MockQualityGate::new(true, 0.95));
        let schema_evolution = Arc::new(MockSchemaEvolution::no_changes());
        let compliance_engine = Arc::new(DefaultComplianceEngine::with_default_rules());

        let engine =
            GovernanceEngine::new(config, quality_gate, schema_evolution, compliance_engine);
        let entity = entity_with_attributes(serde_json::json!({"email": "unmasked@example.com"}));

        let result = engine.check_entity(&entity).await.unwrap();

        assert!(!result.passed);
        assert!(result.issues.iter().any(|i| i.contains("email")));
    }

    #[tokio::test]
    async fn check_entity_passes_when_real_compliance_engine_finds_no_violation() {
        let config = GovernanceConfig::default();
        let quality_gate = Arc::new(MockQualityGate::new(true, 0.95));
        let schema_evolution = Arc::new(MockSchemaEvolution::no_changes());
        let compliance_engine = Arc::new(DefaultComplianceEngine::with_default_rules());

        let engine =
            GovernanceEngine::new(config, quality_gate, schema_evolution, compliance_engine);
        let entity = entity_with_attributes(serde_json::json!({"email": "****"}));

        let result = engine.check_entity(&entity).await.unwrap();

        assert!(result.passed);
        assert!(result.issues.is_empty());
    }

    #[tokio::test]
    async fn check_entity_skips_compliance_check_when_disabled() {
        let mut config = GovernanceConfig::default();
        config.compliance_rules_enabled = false;
        let quality_gate = Arc::new(MockQualityGate::new(true, 0.95));
        let schema_evolution = Arc::new(MockSchemaEvolution::no_changes());
        let compliance_engine = Arc::new(DefaultComplianceEngine::with_default_rules());

        let engine =
            GovernanceEngine::new(config, quality_gate, schema_evolution, compliance_engine);
        // Would be a real violation if compliance checking ran.
        let entity = entity_with_attributes(serde_json::json!({"email": "unmasked@example.com"}));

        let result = engine.check_entity(&entity).await.unwrap();

        assert!(result.passed);
    }
}
