# Task #1: Days 5-7 Planning - Full StatGuardian API Integration

**Date:** 2026-07-26  
**Phase:** 5-7 day continuation of StatGuardian integration  
**Status:** READY TO START  

---

## What Days 3-4 Accomplished

✅ **Pipeline Integration Complete**
- GovernanceEngine now integrated into ActivationPipeline
- Quality checks run before event processing (hard fail on validation failure)
- Schema changes detected but informational (don't block)
- Compliance rules applied to entity attributes
- Metrics tracked: quality_checks_passed/failed, schema_changes_detected, compliance_rules_applied
- Builder pattern allows optional governance: `pipeline.with_governance(engine)`

**Current State:**
- All governance components are in place
- StatGuardianGate is a stub implementation (returns hardcoded mocks)
- Quality gate, schema evolution, compliance rules are testable mocks
- Pipeline compiles successfully with governance integration

---

## Days 5-7: Real StatGuardian Integration

### Goal
Replace mock implementations with production-grade HTTP client to StatGuardian API

### Three Main Components to Implement

#### 1. HTTP Client for StatGuardian

**File**: `core/src/governance/statguardian_client.rs` (NEW)

```rust
pub struct StatGuardianClient {
    base_url: String,
    api_key: String,
    timeout: Duration,
    client: reqwest::Client,
}

impl StatGuardianClient {
    pub async fn validate(&self, entity: &Entity) -> Result<ValidationResult> {
        let request = ValidateRequest {
            entity: entity.clone(),
            contract_id: "default",
            strict: true,
        };

        let response = self.client
            .post(&format!("{}/validate", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(Error::ValidationGateFailed(
                format!("StatGuardian API error: {}", response.status())
            ))
        }
    }

    pub async fn detect_schema_changes(&self, entity: &Entity) -> Result<Vec<SchemaChange>> {
        let request = SchemaCheckRequest {
            entity: entity.clone(),
        };

        let response = self.client
            .post(&format!("{}/detect-changes", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        Ok(response.json().await?)
    }
}
```

**Key Features:**
- Bearer token authentication
- Configurable timeout per request
- Error handling for network failures
- Automatic retry logic (exponential backoff)
- Response caching (optional)

#### 2. Credential Management

**File**: `core/src/governance/credentials.rs` (NEW)

```rust
pub struct GovernanceCredentials {
    /// StatGuardian API key
    pub api_key: String,
    /// Optional authentication method
    pub auth_method: AuthMethod,
}

pub enum AuthMethod {
    /// Bearer token
    Bearer(String),
    /// API Key header
    ApiKey(String),
    /// Basic auth (username:password)
    BasicAuth(String, String),
}

impl GovernanceCredentials {
    /// Load from environment variables
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("STATGUARDIAN_API_KEY")
            .map_err(|_| Error::ConfigError(
                "STATGUARDIAN_API_KEY not set".to_string()
            ))?;
        
        Ok(Self {
            api_key,
            auth_method: AuthMethod::Bearer(
                std::env::var("STATGUARDIAN_TOKEN")?
            ),
        })
    }

    /// Load from config file
    pub fn from_config(config: &GovernanceConfig) -> Result<Self> {
        // Implementation
    }
}
```

**Security Considerations:**
- Never log credentials
- Store API keys in environment variables or secret manager
- Support multiple auth methods
- Validate credentials at startup

#### 3. Response Parsing & Caching

**File**: Update `core/src/governance/mod.rs`

```rust
pub struct CachedStatGuardianGate {
    client: Arc<StatGuardianClient>,
    cache: Arc<Mutex<HashMap<String, CachedValidation>>>,
    cache_ttl: Duration,
}

#[derive(Clone)]
struct CachedValidation {
    result: ValidationResult,
    cached_at: Instant,
}

impl CachedStatGuardianGate {
    pub async fn validate(&self, entity: &Entity) -> Result<ValidationResult> {
        // Check cache first
        let cache_key = format!("{:?}", entity.id);
        
        if let Some(cached) = self.cache.lock().await.get(&cache_key) {
            if cached.cached_at.elapsed() < self.cache_ttl {
                return Ok(cached.result.clone());
            }
        }

        // Call API if cache miss
        let result = self.client.validate(entity).await?;
        
        // Store in cache
        self.cache.lock().await.insert(
            cache_key,
            CachedValidation {
                result: result.clone(),
                cached_at: Instant::now(),
            }
        );

        Ok(result)
    }
}
```

**Caching Strategy:**
- Cache validation results for configurable TTL (default: 5 minutes)
- Cache key: entity ID + quality contract
- Manual cache invalidation on schema changes
- Per-entity cache size limits to prevent memory bloat

### Configuration Updates

**Update**: `core/src/governance/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub statguardian_url: String,
    pub api_key: String,
    pub timeout_ms: u64,
    pub quality_threshold: f64,
    
    // NEW: Retry policy
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    
    // NEW: Caching
    pub cache_enabled: bool,
    pub cache_ttl_secs: u64,
    pub cache_max_entries: usize,
    
    // Feature flags
    pub quality_gates_enabled: bool,
    pub schema_checks_enabled: bool,
    pub compliance_rules_enabled: bool,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            statguardian_url: "http://localhost:8080".to_string(),
            api_key: String::new(),
            timeout_ms: 5000,
            quality_threshold: 0.9,
            max_retries: 3,
            retry_backoff_ms: 100,
            cache_enabled: true,
            cache_ttl_secs: 300,
            cache_max_entries: 10000,
            quality_gates_enabled: true,
            schema_checks_enabled: true,
            compliance_rules_enabled: true,
        }
    }
}
```

### Error Handling Strategy

```rust
pub enum GovernanceError {
    /// StatGuardian API unreachable
    ApiUnavailable(String),
    /// Request timeout
    RequestTimeout,
    /// Invalid API response
    InvalidResponse(String),
    /// Authentication failed
    AuthenticationFailed,
    /// Rate limit exceeded
    RateLimited { retry_after_secs: u64 },
    /// Validation check failed
    ValidationFailed { quality_score: f64, reason: String },
}

impl RetryPolicy {
    pub fn should_retry(&self, error: &GovernanceError) -> bool {
        matches!(
            error,
            GovernanceError::ApiUnavailable(_)
            | GovernanceError::RequestTimeout
            | GovernanceError::RateLimited { .. }
        )
    }

    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        // Exponential backoff: 100ms * 2^attempt
        let base_ms = self.backoff_ms;
        let delay_ms = base_ms * (2u64.pow(attempt));
        Duration::from_millis(delay_ms)
    }
}
```

### Testing Strategy

**Integration Tests** (6-8 tests):

```rust
#[tokio::test]
async fn test_statguardian_valid_entity() {
    // Mock StatGuardian server
    let server = mock_server();
    server.expect_validate()
        .returning(|_| ValidationResult {
            passed: true,
            quality_score: 0.95,
            ...
        });

    let client = StatGuardianClient::new(server.url(), "test-key");
    let entity = Entity::new(...);
    let result = client.validate(&entity).await.unwrap();
    
    assert!(result.passed);
    assert_eq!(result.quality_score, 0.95);
}

#[tokio::test]
async fn test_statguardian_timeout() {
    // Test timeout handling
}

#[tokio::test]
async fn test_statguardian_retry_on_500() {
    // Test automatic retry on server error
}

#[tokio::test]
async fn test_response_caching() {
    // Test cache hit and miss
}

#[tokio::test]
async fn test_concurrent_validation() {
    // Test thread-safe validation
}

#[tokio::test]
async fn test_credential_loading() {
    // Test credential loading from env
}

#[tokio::test]
async fn test_auth_header_setting() {
    // Test bearer token in headers
}

#[tokio::test]
async fn test_rate_limit_handling() {
    // Test 429 response handling
}
```

### Acceptance Criteria

- ✅ HTTP client successfully calls StatGuardian API
- ✅ Bearer token authentication working
- ✅ Timeout respected (no hanging requests)
- ✅ Retries on transient failures (exponential backoff)
- ✅ Caching reduces API calls
- ✅ Credentials loaded from environment/config
- ✅ Error handling for all failure modes
- ✅ All 6-8 integration tests passing

### Dependencies to Add

**Cargo.toml**:
```toml
reqwest = { version = "0.11", features = ["json"] }
backoff = "0.4"  # For retry logic
lru = "0.12"     # For LRU cache (optional)
```

---

## Implementation Order

1. **Day 5**: Create HTTP client infrastructure
   - StatGuardianClient struct
   - Request/response types
   - Basic error handling

2. **Day 6**: Add auth & retry logic
   - Credential management
   - Bearer token auth
   - Exponential backoff retry

3. **Day 7**: Caching & integration tests
   - Response caching
   - Cache TTL management
   - 6-8 comprehensive tests

---

## Success Metrics

**Code Quality:**
- 0 compilation errors
- <30 warnings
- 100% test pass rate
- Type-safe HTTP client

**Functionality:**
- Successful calls to StatGuardian API
- Quality validation working end-to-end
- Schema changes detected from API
- Compliance rules applied from API

**Performance:**
- API call latency < 500ms (p99)
- Cache hit rate > 80% under typical load
- Retry logic doesn't exceed 10s total latency

**Security:**
- No credentials logged
- Secure auth header handling
- Timeout prevents hanging connections
- Proper error messages (no leaks)

---

## Files to Create/Modify

**New Files:**
- `core/src/governance/statguardian_client.rs` (250-300 LOC)
- `core/src/governance/credentials.rs` (100-150 LOC)

**Modified Files:**
- `core/src/governance/mod.rs` - update config, add client exports
- `core/src/governance/quality_gate.rs` - implement with real client
- `Cargo.toml` - add reqwest, backoff

**Total Lines:** ~600-700 LOC for complete implementation

---

## Risk Mitigation

**Risk**: Network failure breaks pipeline
**Mitigation**: Retry with exponential backoff, timeout limits, graceful degradation

**Risk**: Credential exposure
**Mitigation**: Environment variables only, never log credentials

**Risk**: API overload
**Mitigation**: Rate limit handling, cache responses, circuit breaker (optional)

**Risk**: Slow API responses
**Mitigation**: Configurable timeout, request cancellation

---

## Next Phase (Days 8-10)

Once Days 5-7 complete:
- E2E testing with real StatGuardian instance
- Documentation and runbook
- Performance benchmarking
- v2.1.0 release preparation
