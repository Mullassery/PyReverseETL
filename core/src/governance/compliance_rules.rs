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
                        // Mask the field in entity.attributes
                        if let Some(obj) = entity.attributes.as_object_mut() {
                            if obj.contains_key(field) {
                                obj[field] = serde_json::Value::String(pattern.clone());
                            }
                        }
                    }
                    RuleAction::Remove => {
                        if let Some(obj) = entity.attributes.as_object_mut() {
                            obj.remove(field);
                        }
                    }
                    RuleAction::Truncate(len) => {
                        if let Some(obj) = entity.attributes.as_object_mut() {
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

    async fn check_compliance(&self, entity: &Entity) -> Result<ComplianceCheckResult> {
        let mut violations = Vec::new();

        for rule in &self.rules {
            for field in &rule.target_fields {
                let current_value = entity
                    .attributes
                    .as_object()
                    .and_then(|obj| obj.get(field));

                match &rule.action {
                    RuleAction::Mask(pattern) => {
                        // A field that's absent has nothing to mask, so it's compliant.
                        // A field still holding its raw (non-masked) value is not.
                        if let Some(value) = current_value {
                            if value.as_str() != Some(pattern.as_str()) {
                                violations.push(format!(
                                    "Field '{field}' is not masked (rule '{}')",
                                    rule.id
                                ));
                            }
                        }
                    }
                    RuleAction::Remove => {
                        if current_value.is_some() {
                            violations.push(format!(
                                "Field '{field}' should have been removed (rule '{}')",
                                rule.id
                            ));
                        }
                    }
                    RuleAction::Truncate(max_len) => {
                        if let Some(value) = current_value.and_then(|v| v.as_str()) {
                            if value.chars().count() > *max_len {
                                violations.push(format!(
                                    "Field '{field}' exceeds max length {max_len} (rule '{}')",
                                    rule.id
                                ));
                            }
                        }
                    }
                    RuleAction::Encrypt => {
                        // apply_rules() has no real implementation for Encrypt (see
                        // above) -- report that honestly as a violation rather than
                        // claiming compliance for a field that was never encrypted.
                        if current_value.is_some() {
                            violations.push(format!(
                                "Field '{field}' requires encryption, which is not yet implemented (rule '{}')",
                                rule.id
                            ));
                        }
                    }
                }
            }
        }

        Ok(ComplianceCheckResult {
            compliant: violations.is_empty(),
            violations,
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
        use crate::entity::EntityType;
        let engine = MockComplianceEngine::no_rules();
        let entity = Entity::new(EntityType::Custom("test".to_string()), "id", "test");

        let result = engine.check_compliance(&entity).await.unwrap();
        assert!(result.compliant);
    }

    // -- DefaultComplianceEngine: the real, production implementation (as opposed
    // to MockComplianceEngine above, which is test-only and never runs real logic).
    // check_compliance() used to unconditionally return `compliant: true` regardless
    // of the entity's actual data; these tests exercise the real per-rule checks.

    fn entity_with_attributes(attrs: serde_json::Value) -> Entity {
        use crate::entity::EntityType;
        let mut entity = Entity::new(EntityType::Custom("test".to_string()), "id", "test");
        entity.attributes = attrs;
        entity
    }

    #[tokio::test]
    async fn default_engine_apply_rules_actually_masks_the_field() {
        let engine = DefaultComplianceEngine::with_default_rules();
        let mut entity = entity_with_attributes(serde_json::json!({
            "email": "real.person@example.com",
            "phone": "555-0100",
        }));

        engine.apply_rules(&mut entity).await.unwrap();

        assert_eq!(entity.attributes["email"], serde_json::json!("****"));
        assert_eq!(entity.attributes["phone"], serde_json::json!("****"));
    }

    #[tokio::test]
    async fn default_engine_apply_rules_leaves_untargeted_fields_alone() {
        let engine = DefaultComplianceEngine::with_default_rules();
        let mut entity = entity_with_attributes(serde_json::json!({
            "email": "real.person@example.com",
            "name": "Real Person",
        }));

        engine.apply_rules(&mut entity).await.unwrap();

        assert_eq!(entity.attributes["name"], serde_json::json!("Real Person"));
    }

    #[tokio::test]
    async fn default_engine_apply_rules_removes_field_for_remove_action() {
        let rule = ComplianceRule::new(
            "ssn_removal".to_string(),
            RuleType::PiiMasking,
            vec!["ssn".to_string()],
            RuleAction::Remove,
        );
        let engine = DefaultComplianceEngine::new(vec![rule]);
        let mut entity = entity_with_attributes(serde_json::json!({"ssn": "123-45-6789"}));

        engine.apply_rules(&mut entity).await.unwrap();

        assert!(entity.attributes.get("ssn").is_none());
    }

    #[tokio::test]
    async fn default_engine_apply_rules_truncates_field_for_truncate_action() {
        let rule = ComplianceRule::new(
            "bio_truncate".to_string(),
            RuleType::Retention,
            vec!["bio".to_string()],
            RuleAction::Truncate(5),
        );
        let engine = DefaultComplianceEngine::new(vec![rule]);
        let mut entity = entity_with_attributes(serde_json::json!({"bio": "a very long biography"}));

        engine.apply_rules(&mut entity).await.unwrap();

        assert_eq!(entity.attributes["bio"], serde_json::json!("a ver"));
    }

    #[tokio::test]
    async fn default_engine_check_compliance_flags_unmasked_pii_as_a_real_violation() {
        // Before the fix, this always returned compliant=true regardless of input.
        let engine = DefaultComplianceEngine::with_default_rules();
        let entity = entity_with_attributes(serde_json::json!({
            "email": "still.raw@example.com",
        }));

        let result = engine.check_compliance(&entity).await.unwrap();

        assert!(!result.compliant);
        assert!(result.violations.iter().any(|v| v.contains("email")));
    }

    #[tokio::test]
    async fn default_engine_check_compliance_passes_once_field_is_masked() {
        let engine = DefaultComplianceEngine::with_default_rules();
        let entity = entity_with_attributes(serde_json::json!({
            "email": "****",
            "phone": "****",
        }));

        let result = engine.check_compliance(&entity).await.unwrap();

        assert!(result.compliant);
        assert!(result.violations.is_empty());
    }

    #[tokio::test]
    async fn default_engine_check_compliance_passes_when_targeted_field_is_absent() {
        // Nothing to mask if the field was never present in the first place.
        let engine = DefaultComplianceEngine::with_default_rules();
        let entity = entity_with_attributes(serde_json::json!({"name": "Real Person"}));

        let result = engine.check_compliance(&entity).await.unwrap();

        assert!(result.compliant);
    }

    #[tokio::test]
    async fn default_engine_check_compliance_flags_field_that_should_have_been_removed() {
        let rule = ComplianceRule::new(
            "ssn_removal".to_string(),
            RuleType::PiiMasking,
            vec!["ssn".to_string()],
            RuleAction::Remove,
        );
        let engine = DefaultComplianceEngine::new(vec![rule]);
        let entity = entity_with_attributes(serde_json::json!({"ssn": "123-45-6789"}));

        let result = engine.check_compliance(&entity).await.unwrap();

        assert!(!result.compliant);
        assert!(result.violations.iter().any(|v| v.contains("ssn")));
    }

    #[tokio::test]
    async fn default_engine_check_compliance_flags_field_exceeding_truncate_limit() {
        let rule = ComplianceRule::new(
            "bio_truncate".to_string(),
            RuleType::Retention,
            vec!["bio".to_string()],
            RuleAction::Truncate(5),
        );
        let engine = DefaultComplianceEngine::new(vec![rule.clone()]);

        let too_long = entity_with_attributes(serde_json::json!({"bio": "way too long"}));
        let result = engine.check_compliance(&too_long).await.unwrap();
        assert!(!result.compliant);

        let within_limit = entity_with_attributes(serde_json::json!({"bio": "short"}));
        let result = engine.check_compliance(&within_limit).await.unwrap();
        assert!(result.compliant);
    }

    #[tokio::test]
    async fn default_engine_check_compliance_honestly_flags_unimplemented_encryption() {
        // apply_rules() has no real encryption implementation, so a field targeted
        // by an Encrypt rule can never actually be made compliant -- check_compliance()
        // must say so rather than silently claiming success for a no-op.
        let rule = ComplianceRule::new(
            "card_encryption".to_string(),
            RuleType::Compliance,
            vec!["card_number".to_string()],
            RuleAction::Encrypt,
        );
        let engine = DefaultComplianceEngine::new(vec![rule]);
        let entity = entity_with_attributes(serde_json::json!({"card_number": "4111111111111111"}));

        let result = engine.check_compliance(&entity).await.unwrap();

        assert!(!result.compliant);
        assert!(result.violations.iter().any(|v| v.contains("card_number")));
    }

    #[tokio::test]
    async fn default_engine_apply_then_check_round_trip_is_compliant() {
        // The real, end-to-end guarantee this module exists for: apply real rules,
        // then a real compliance check on the result must actually pass.
        let engine = DefaultComplianceEngine::with_default_rules();
        let mut entity = entity_with_attributes(serde_json::json!({
            "email": "real.person@example.com",
            "phone": "555-0100",
            "name": "Real Person",
        }));

        engine.apply_rules(&mut entity).await.unwrap();
        let result = engine.check_compliance(&entity).await.unwrap();

        assert!(result.compliant, "violations: {:?}", result.violations);
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
