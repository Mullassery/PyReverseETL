# Task #1: StatGuardian Integration with PyReverseETL

**Duration**: 8-12 days (Phase 4 Week 4 + continuation)  
**Objective**: Add quality gates and governance to activation pipeline  
**Deliverable**: v2.1.0 with StatGuardian integration  
**Status**: 🚧 STARTING NOW

---

## Strategic Context

### Why StatGuardian?

From ARCHITECTURE.md: **Separation of Concerns**

```
Warehouse Intelligence
    ↓
StatGuardian (Trust It) ← Validates quality
    ↓
Trusted + Valid Data
    ↓
PyReverseETL (Activate It) ← Gates on validation
    ↓
Operational Systems
```

**Mission**: PyReverseETL activates only trusted, validated data.

---

## Implementation Strategy

### Three Integration Points

#### 1. **Pre-Activation Quality Gates** (Days 1-4)
- Check: Data quality contract validation
- Check: Drift detection results
- Check: Schema evolution compatibility
- **Result**: Block activations that fail quality checks

#### 2. **Schema Evolution Detection** (Days 5-7)
- Detect: Upstream schema changes
- Version: Activation templates
- Migrate: Field mappings
- **Result**: Handle schema changes gracefully

#### 3. **Governance Rules Enforcement** (Days 8-10)
- Apply: PII masking rules
- Enforce: Compliance policies (GDPR, CCPA)
- Audit: All rule applications
- **Result**: Transparent governance layer

---

## Architecture Design

### New Module Structure

```rust
// core/src/governance/mod.rs
pub mod quality_gate;        // Quality checks
pub mod schema_evolution;    // Schema handling
pub mod compliance_rules;    // Governance rules

pub use quality_gate::QualityGate;
pub use schema_evolution::SchemaEvolution;
pub use compliance_rules::ComplianceRule;
```

### Integration Points

```rust
// core/src/pipeline/activation_pipeline.rs
impl ActivationPipeline {
    pub async fn process_event(&self, event: &Event) -> Result<()> {
        // 1. Check quality gates from StatGuardian
        let quality_check = self.governance
            .check_quality(&event.entity)?;
        
        if !quality_check.passed {
            return Err(Error::ValidationGateFailed(
                quality_check.reason
            ));
        }

        // 2. Detect schema changes
        let schema_check = self.governance
            .detect_schema_changes(&event)?;
        
        // 3. Apply compliance rules
        let entity = self.governance
            .apply_compliance_rules(&event.entity)?;

        // 4. Proceed with activation
        self.activate(entity).await
    }
}
```

---

## Week-by-Week Breakdown

### Days 1-2: QualityGate Trait & Integration

**Objective**: Define quality gate interface

**Deliverables**:
1. Create `QualityGate` trait
2. Define `ValidationResult` struct
3. Hook into `ActivationPipeline`
4. Write validation logic

**Code Structure**:
```rust
// core/src/governance/quality_gate.rs

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub quality_score: f64,
    pub issues: Vec<String>,
    pub schema_version: String,
}

pub trait QualityGate: Send + Sync {
    async fn validate(&self, entity: &Entity) -> Result<ValidationResult>;
    async fn check_drift(&self, entity: &Entity) -> Result<DriftReport>;
    async fn get_quality_threshold(&self) -> f64;
}

pub struct StatGuardianGate {
    client: HttpClient,
    config: GovernanceConfig,
}

impl QualityGate for StatGuardianGate {
    async fn validate(&self, entity: &Entity) -> Result<ValidationResult> {
        // Call StatGuardian API for validation
        let response = self.client
            .post("/validate")
            .json(&entity)
            .send()
            .await?;
        
        Ok(response.json()?)
    }
}
```

**Tests**: 5-7 tests
- Quality check pass/fail
- Threshold enforcement
- Drift detection
- Integration with pipeline

---

### Days 3-4: Schema Evolution Detection

**Objective**: Handle upstream schema changes

**Deliverables**:
1. Create `SchemaEvolution` trait
2. Implement schema versioning
3. Add migration logic
4. Update field mappings

**Code Structure**:
```rust
// core/src/governance/schema_evolution.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChange {
    pub field_name: String,
    pub change_type: ChangeType,
    pub old_type: Option<String>,
    pub new_type: Option<String>,
}

pub enum ChangeType {
    Added,
    Removed,
    TypeChanged,
    RenameTo(String),
}

pub trait SchemaEvolution: Send + Sync {
    async fn detect_changes(&self, entity: &Entity) -> Result<Vec<SchemaChange>>;
    async fn migrate_mapping(&self, mapping: &FieldMapping, changes: &[SchemaChange]) -> Result<FieldMapping>;
    async fn get_schema_version(&self) -> Result<String>;
}
```

**Tests**: 5-7 tests
- Schema change detection
- Field mapping migration
- Version compatibility
- Backward compatibility

---

### Days 5-7: Compliance Rules Enforcement

**Objective**: Apply governance policies

**Deliverables**:
1. Create `ComplianceRule` trait
2. Implement rule engine
3. Add PII masking
4. Add audit logging

**Code Structure**:
```rust
// core/src/governance/compliance_rules.rs

#[derive(Debug, Clone)]
pub struct ComplianceRule {
    pub id: String,
    pub rule_type: RuleType,
    pub target_fields: Vec<String>,
    pub action: RuleAction,
}

pub enum RuleType {
    PiiMasking,
    Retention,
    Compliance,
    Custom(String),
}

pub enum RuleAction {
    Mask(String),        // Mask with pattern
    Remove,              // Delete field
    Truncate(usize),     // Keep first N chars
    Encrypt,             // Encrypt field
}

pub trait ComplianceEngine: Send + Sync {
    async fn apply_rules(&self, entity: &Entity) -> Result<Entity>;
    async fn audit_application(&self, entity_id: &str, rules: &[String]);
}
```

**Tests**: 5-7 tests
- Rule application
- PII masking
- Compliance enforcement
- Audit trail

---

### Days 8-10: Integration & Testing

**Objective**: Full end-to-end integration

**Deliverables**:
1. Wire all components into pipeline
2. Create end-to-end tests
3. Documentation
4. Release notes

**Integration Tests** (5-7 tests):
- Quality gate blocks bad data
- Schema evolution handled
- Compliance rules applied
- Audit trail complete
- Error cases handled

---

## API Design

### StatGuardian API Expectations

```rust
// Expected StatGuardian endpoints

POST /validate
{
    "entity": { ... },
    "contract_id": "customers",
    "strict": true
}

Response:
{
    "passed": true,
    "quality_score": 0.95,
    "issues": [],
    "schema_version": "v3.2.1"
}

POST /detect-changes
{
    "entity": { ... },
    "schema_version": "v3.1.0"
}

Response:
{
    "changes": [
        { "field": "created_at", "type": "Added" }
    ],
    "migration_required": true
}
```

### Configuration

```yaml
# config.yaml
governance:
  statguardian:
    endpoint: "http://statguardian:8080"
    timeout_ms: 5000
    retry_policy: "exponential"
    quality_threshold: 0.9
    
  rules:
    - id: "pii_masking"
      type: "pii_masking"
      fields: ["email", "phone", "ssn"]
      action: "mask:****"
    
    - id: "gdpr_retention"
      type: "retention"
      fields: ["*"]
      action: "truncate:90d"
```

---

## Success Criteria

### Code Quality
- ✅ All tests passing (15+ new tests)
- ✅ No compiler warnings
- ✅ Type-safe implementation
- ✅ Proper error handling

### Functionality
- ✅ Quality gates block bad data
- ✅ Schema changes handled
- ✅ Compliance rules applied
- ✅ Audit trail complete

### Performance
- ✅ <50ms overhead per validation
- ✅ No deadlocks or race conditions
- ✅ Graceful degradation if StatGuardian unavailable
- ✅ Proper connection pooling

### Documentation
- ✅ API documentation
- ✅ Configuration guide
- ✅ Integration examples
- ✅ Troubleshooting guide

---

## Testing Strategy

### Unit Tests (5-7)
```rust
#[tokio::test]
async fn test_quality_gate_passes() {
    let gate = create_mock_gate();
    let entity = create_valid_entity();
    
    let result = gate.validate(&entity).await;
    assert!(result.is_ok());
    assert!(result.unwrap().passed);
}

#[tokio::test]
async fn test_quality_gate_fails() {
    let gate = create_mock_gate();
    let entity = create_invalid_entity();
    
    let result = gate.validate(&entity).await;
    assert!(result.is_ok());
    assert!(!result.unwrap().passed);
}
```

### Integration Tests (5-7)
```rust
#[tokio::test]
async fn test_activation_blocked_by_quality_gate() {
    let pipeline = setup_pipeline_with_governance();
    let bad_event = create_low_quality_event();
    
    let result = pipeline.process_event(&bad_event).await;
    assert!(result.is_err());
    assert_matches!(result, Err(Error::ValidationGateFailed(_)));
}
```

### End-to-End Tests (3-5)
```rust
#[tokio::test]
async fn test_complete_governance_flow() {
    // 1. Low quality data arrives
    // 2. Quality gate catches it
    // 3. Schema evolution detected
    // 4. Mapping updated
    // 5. Compliance rules applied
    // 6. Audit logged
    // 7. Activation successful
}
```

---

## Deliverables

### Code
- [ ] `core/src/governance/mod.rs`
- [ ] `core/src/governance/quality_gate.rs`
- [ ] `core/src/governance/schema_evolution.rs`
- [ ] `core/src/governance/compliance_rules.rs`
- [ ] Integration into `ActivationPipeline`
- [ ] 15+ tests

### Documentation
- [ ] `GOVERNANCE_INTEGRATION.md` — Integration guide
- [ ] `QUALITY_GATES_GUIDE.md` — Configuration guide
- [ ] `COMPLIANCE_RULES.md` — Rule reference
- [ ] `STATGUARDIAN_API.md` — API expectations
- [ ] v2.1.0 release notes

### Configuration
- [ ] Example `governance.yaml` config
- [ ] Environment variable reference
- [ ] Deployment guide

---

## Timeline

```
Days 1-2:   QualityGate foundation ▓▓░░░░░░░░
Days 3-4:   Schema Evolution      ▓▓▓▓░░░░░░
Days 5-7:   Compliance Rules      ▓▓▓▓▓▓▓░░░
Days 8-10:  Integration & Tests   ▓▓▓▓▓▓▓▓▓▓

Total:      10 days (8-12 range)
```

---

## Rollback Strategy

If integration fails:
1. ✅ Quality gates are optional (controlled by flag)
2. ✅ Can run without StatGuardian (local defaults)
3. ✅ Graceful degradation if service unavailable
4. ✅ Easy to disable in config

---

## Success Looks Like

```
Pre-Integration:
  Activation Pipeline
    ↓
  Destination
  
Post-Integration:
  Activation Pipeline
    ↓
  Quality Gate (from StatGuardian)
    ↓ [passed]
  Schema Evolution Check
    ↓ [compatible]
  Compliance Rules Engine
    ↓ [rules applied]
  Audit Logger
    ↓
  Destination
```

---

**Status**: 🚧 READY TO START  
**Next**: Begin Days 1-2 (QualityGate foundation)  
**Expected Completion**: 8-12 days  
**Target**: v2.1.0 release
