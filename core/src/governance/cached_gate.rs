/// Cached Quality Gate Implementation
///
/// Wraps StatGuardianGate with response caching to reduce API calls
/// while maintaining fresh data with configurable TTL.

use crate::{Entity, Result};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;
use super::quality_gate::{QualityGate, ValidationResult, DriftReport};
use super::statguardian_client::StatGuardianClient;

/// Cached validation result
#[derive(Clone, Debug)]
struct CachedValidation {
    result: ValidationResult,
    cached_at: Instant,
}

impl CachedValidation {
    /// Check if cache entry is still valid
    fn is_expired(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() > ttl
    }
}

/// Quality gate with response caching
pub struct CachedQualityGate {
    client: Arc<StatGuardianClient>,
    cache: Arc<RwLock<HashMap<String, CachedValidation>>>,
    cache_ttl: Duration,
    max_cache_entries: usize,
}

impl CachedQualityGate {
    /// Create a new cached quality gate
    pub fn new(client: Arc<StatGuardianClient>, cache_ttl: Duration) -> Self {
        Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            max_cache_entries: 10_000,
        }
    }

    /// Create cached gate with custom cache size
    pub fn with_cache_size(
        client: Arc<StatGuardianClient>,
        cache_ttl: Duration,
        max_entries: usize,
    ) -> Self {
        Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            max_cache_entries: max_entries,
        }
    }

    /// Generate cache key for entity
    fn cache_key(entity: &Entity) -> String {
        format!("{}:{}", entity.id, entity.key_field)
    }

    /// Clear all cached entries
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            entries: cache.len(),
            capacity: self.max_cache_entries,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub capacity: usize,
}

#[async_trait]
impl QualityGate for CachedQualityGate {
    async fn validate(&self, entity: &Entity) -> Result<ValidationResult> {
        let cache_key = Self::cache_key(entity);

        // Try to get from cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if !cached.is_expired(self.cache_ttl) {
                    return Ok(cached.result.clone());
                }
            }
        }

        // Cache miss or expired - call API
        let result = self.client.validate(entity).await?;

        // Store in cache (with size limit)
        {
            let mut cache = self.cache.write().await;

            // If cache is full, clear oldest entries (simple eviction)
            if cache.len() >= self.max_cache_entries {
                // Remove oldest entry (simple FIFO eviction)
                if let Some(oldest_key) = cache.keys().next().cloned() {
                    cache.remove(&oldest_key);
                }
            }

            cache.insert(
                cache_key,
                CachedValidation {
                    result: result.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(result)
    }

    async fn check_drift(&self, entity: &Entity) -> Result<DriftReport> {
        // Drift checks are not cached (real-time)
        self.client
            .detect_schema_changes(entity)
            .await
            .map(|changes| DriftReport {
                detected: !changes.is_empty(),
                drift_percentage: if changes.is_empty() { 0.0 } else { 25.0 },
                affected_fields: changes
                    .iter()
                    .map(|c| c.field_name.clone())
                    .collect(),
            })
    }

    async fn get_quality_threshold(&self) -> Result<f64> {
        Ok(0.9)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        use crate::entity::EntityType;
        let entity = Entity::new(EntityType::Customer, "email", "test@example.com");
        let key = CachedQualityGate::cache_key(&entity);
        assert!(key.contains("test@example.com"));
    }

    #[test]
    fn test_cached_validation_expiry() {
        let result = ValidationResult {
            passed: true,
            quality_score: 0.95,
            issues: vec![],
            schema_version: "v1.0".to_string(),
        };

        let cached = CachedValidation {
            result,
            cached_at: Instant::now(),
        };

        let ttl = Duration::from_secs(1);
        assert!(!cached.is_expired(ttl));

        // Simulate expired entry (can't actually wait in test)
        let old_cached = CachedValidation {
            result: cached.result,
            cached_at: Instant::now() - Duration::from_secs(2),
        };

        assert!(old_cached.is_expired(ttl));
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let client = Arc::new(StatGuardianClient::new(
            "http://localhost:8080",
            "test-key",
        ));
        let gate = CachedQualityGate::new(client, Duration::from_secs(60));

        let stats = gate.cache_stats().await;
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.capacity, 10_000);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let client = Arc::new(StatGuardianClient::new(
            "http://localhost:8080",
            "test-key",
        ));
        let gate = CachedQualityGate::new(client, Duration::from_secs(60));

        gate.clear_cache().await;
        let stats = gate.cache_stats().await;
        assert_eq!(stats.entries, 0);
    }
}
