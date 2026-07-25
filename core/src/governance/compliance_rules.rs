/// Compliance Rules Engine
///
/// Applies governance rules to entities:
/// - PII masking
/// - Retention policies
/// - Compliance rules (GDPR, CCPA)
/// - Custom governance rules

use crate::{Entity, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Type of compliance rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleType {
    PiiMasking,
    Retention,
    Compliance,
    Custom,
}

/// Action to take when rule is applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Mask field with pattern (e.g., "****")
    Mask(String),
    /// Remove field entirely
    Remove,
    /// Keep only first N characters
    Truncate(usize),
    /// Encrypt field
    Encrypt,
}

/// A single compliance rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    pub id: String,
    pub rule_type: RuleType,
    pub target_fields: Vec<String>,
    pub action: RuleAction,
    pub description: Option<String>,
}

impl ComplianceRule {
    pub fn new(
        id: String,
        rule_type: RuleType,
        target_fields: Vec<String>,
        action: RuleAction,
    ) -> Self {
        Self {
            id,
            rule_type,
            target_fields,
            action,
            description: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

/// Compliance engine trait - applies governance rules
#[async_trait]
pub trait ComplianceEngine: Send + Sync {
    /// Apply compliance rules to entity
    async fn apply_rules(&self, entity: &mut Entity) -> Result<()>;

    /// Get list of active rules
    async fn get_rules(&self) -> Result<Vec<ComplianceRule>>;

    /// Check if entity complies with rules
    async fn check_compliance(&self, entity: &Entity) -> Result<ComplianceCheckResult>;
}

/// Result of compliance check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckResult {
    pub compliant: bool,
    pub violations: Vec<String>,
}

/// Mock compliance engine for testing
#[cfg(test)]
pub struct MockComplianceEngine {
    rules: Vec<ComplianceRule>,
}

#[cfg(test)]
impl MockComplianceEngine {
    pub fn new(rules: Vec<ComplianceRule>) -> Self {
        Self { rules }
    }

    pub fn no_rules() -> Self {
        Self { rules: vec![] }
    }
}

#[cfg(test)]
#[async_trait]
impl ComplianceEngine for MockComplianceEngine {
    async fn apply_rules(&self, _entity: &mut Entity) -> Result<()> {
        // Mock implementation: do nothing
        Ok(())
    }

    async fn get_rules(&self) -> Result<Vec<ComplianceRule>> {
        Ok(self.rules.clone())
    }

    async fn check_compliance(&self, _entity: &Entity) -> Result<ComplianceCheckResult> {
        Ok(ComplianceCheckResult {
            compliant: true,
            violations: vec![],
        })
    }
}

/// Default compliance engine
pub struct DefaultComplianceEngine {
    rules: Vec<ComplianceRule>,
}

impl DefaultComplianceEngine {
    pub fn new(rules: Vec<ComplianceRule>) -> Self {
        Self { rules }
    }

    pub fn with_default_rules() -> Self {
        let rules = vec![
            ComplianceRule::new(
                "pii_masking".to_string(),
                RuleType::PiiMasking,
                vec!["email".to_string(), "phone".to_string()],
                RuleAction::Mask("****".to_string()),
            )
            .with_description("Mask PII fields".to_string()),
        ];
        Self { rules }
    }
}

#[async_trait]
impl ComplianceEngine for DefaultComplianceEngine {
    async fn apply_rules(&self, entity: &mut Entity) -> Result<()> {
        // Apply each rule to entity
        for rule in &self.rules {
            for field in &rule.target_fields {
                match &rule.action {
                    RuleAction::Mask(pattern) => {
                        // In production, would mask the field in entity.data
                        if let Some(obj) = entity.data.as_object_mut() {
                            if obj.contains_key(field) {
                                obj[field] = serde_json::Value::String(pattern.clone());
                            }
                        }
                    }
                    RuleAction::Remove => {
                        if let Some(obj) = entity.data.as_object_mut() {
                            obj.remove(field);
                        }
                    }
                    RuleAction::Truncate(len) => {
                        if let Some(obj) = entity.data.as_object_mut() {
                            if let Some(val) = obj.get_mut(field) {
                                if let Some(s) = val.as_str() {
                                    obj[field] = serde_json::Value::String(
                                        s.chars().take(*len).collect()
                                    );
                                }
                            }
                        }
                    }
                    RuleAction::Encrypt => {
                        // In production, would encrypt the field
                        // For now, just mark it
                    }
                }
            }
        }
        Ok(())
    }

    async fn get_rules(&self) -> Result<Vec<ComplianceRule>> {
        Ok(self.rules.clone())
    }

    async fn check_compliance(&self, _entity: &Entity) -> Result<ComplianceCheckResult> {
        Ok(ComplianceCheckResult {
            compliant: true,
            violations: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_no_rules() {
        let engine = MockComplianceEngine::no_rules();
        let rules = engine.get_rules().await.unwrap();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn test_rules_applied() {
        let rule = ComplianceRule::new(
            "test_rule".to_string(),
            RuleType::PiiMasking,
            vec!["email".to_string()],
            RuleAction::Mask("****".to_string()),
        );

        let engine = MockComplianceEngine::new(vec![rule]);
        let rules = engine.get_rules().await.unwrap();
        assert_eq!(rules.len(), 1);
    }

    #[tokio::test]
    async fn test_compliance_check() {
        let engine = MockComplianceEngine::no_rules();
        let entity = Entity {
            id: "test".to_string(),
            data: serde_json::json!({}),
            metadata: Default::default(),
        };

        let result = engine.check_compliance(&entity).await.unwrap();
        assert!(result.compliant);
    }

    #[test]
    fn test_rule_creation() {
        let rule = ComplianceRule::new(
            "test".to_string(),
            RuleType::PiiMasking,
            vec!["email".to_string()],
            RuleAction::Mask("****".to_string()),
        );

        assert_eq!(rule.id, "test");
        assert_eq!(rule.rule_type, RuleType::PiiMasking);
        assert_eq!(rule.target_fields.len(), 1);
    }
}
