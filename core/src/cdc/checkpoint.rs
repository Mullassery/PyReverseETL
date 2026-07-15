use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Checkpoint for tracking CDC progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique checkpoint ID
    pub id: String,
    /// Associated sync run ID
    pub sync_run_id: String,
    /// Last processed change entry ID
    pub last_processed_id: Option<String>,
    /// Timestamp of last processed change
    pub last_processed_at: DateTime<Utc>,
    /// Total changes processed
    pub total_processed: u64,
    /// When checkpoint was created
    pub created_at: DateTime<Utc>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(sync_run_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sync_run_id,
            last_processed_id: None,
            last_processed_at: Utc::now(),
            total_processed: 0,
            created_at: Utc::now(),
        }
    }

    /// Update checkpoint with newly processed entry
    pub fn mark_entry_processed(&mut self, entry_id: String, total: u64) {
        self.last_processed_id = Some(entry_id);
        self.last_processed_at = Utc::now();
        self.total_processed = total;
    }
}

/// In-memory checkpoint manager
pub struct CheckpointManager {
    checkpoints: std::sync::Arc<tokio::sync::Mutex<HashMap<String, Checkpoint>>>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new() -> Self {
        Self {
            checkpoints: std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Save or update a checkpoint
    pub async fn save(&self, checkpoint: Checkpoint) -> crate::Result<()> {
        let mut checkpoints = self.checkpoints.lock().await;
        checkpoints.insert(checkpoint.id.clone(), checkpoint);
        Ok(())
    }

    /// Get checkpoint by ID
    pub async fn get(&self, id: String) -> crate::Result<Option<Checkpoint>> {
        let checkpoints = self.checkpoints.lock().await;
        Ok(checkpoints.get(&id).cloned())
    }

    /// Get latest checkpoint for a sync run
    pub async fn get_latest(&self, sync_run_id: String) -> crate::Result<Option<Checkpoint>> {
        let checkpoints = self.checkpoints.lock().await;
        let mut latest: Option<Checkpoint> = None;

        for cp in checkpoints.values() {
            if cp.sync_run_id == sync_run_id {
                if let Some(ref mut l) = latest {
                    if cp.created_at > l.created_at {
                        latest = Some(cp.clone());
                    }
                } else {
                    latest = Some(cp.clone());
                }
            }
        }

        Ok(latest)
    }

    /// List checkpoints for a sync run
    pub async fn list_by_sync_run(
        &self,
        sync_run_id: String,
        limit: usize,
    ) -> crate::Result<Vec<Checkpoint>> {
        let checkpoints = self.checkpoints.lock().await;
        let mut result: Vec<Checkpoint> = checkpoints
            .values()
            .filter(|cp| cp.sync_run_id == sync_run_id)
            .cloned()
            .collect();

        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        result.truncate(limit);

        Ok(result)
    }

    /// List all checkpoints
    pub async fn list_all(&self, limit: usize) -> crate::Result<Vec<Checkpoint>> {
        let checkpoints = self.checkpoints.lock().await;
        let mut result: Vec<Checkpoint> = checkpoints.values().cloned().collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        result.truncate(limit);

        Ok(result)
    }

    /// Delete a checkpoint
    pub async fn delete(&self, id: String) -> crate::Result<()> {
        let mut checkpoints = self.checkpoints.lock().await;
        checkpoints.remove(&id);
        Ok(())
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_checkpoint() {
        let manager = CheckpointManager::new();
        let checkpoint = Checkpoint::new("sync_1".to_string());

        manager.save(checkpoint.clone()).await.unwrap();

        let retrieved = manager.get(checkpoint.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().sync_run_id, "sync_1");
    }

    #[tokio::test]
    async fn test_get_latest() {
        let manager = CheckpointManager::new();

        let cp1 = Checkpoint::new("sync_1".to_string());
        manager.save(cp1).await.unwrap();

        let mut cp2 = Checkpoint::new("sync_1".to_string());
        cp2.total_processed = 10;
        manager.save(cp2).await.unwrap();

        let latest = manager.get_latest("sync_1".to_string()).await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().total_processed, 10);
    }

    #[tokio::test]
    async fn test_list_by_sync_run() {
        let manager = CheckpointManager::new();

        for _ in 1..=5 {
            let checkpoint = Checkpoint::new("sync_1".to_string());
            manager.save(checkpoint).await.unwrap();
        }

        for _ in 1..=3 {
            let checkpoint = Checkpoint::new("sync_2".to_string());
            manager.save(checkpoint).await.unwrap();
        }

        let sync_1_checkpoints = manager.list_by_sync_run("sync_1".to_string(), 10).await.unwrap();
        let sync_2_checkpoints = manager.list_by_sync_run("sync_2".to_string(), 10).await.unwrap();

        assert_eq!(sync_1_checkpoints.len(), 5);
        assert_eq!(sync_2_checkpoints.len(), 3);
    }

    #[tokio::test]
    async fn test_checkpoint_update() {
        let manager = CheckpointManager::new();
        let mut checkpoint = Checkpoint::new("sync_1".to_string());

        manager.save(checkpoint.clone()).await.unwrap();

        checkpoint.mark_entry_processed("entry_123".to_string(), 42);
        manager.save(checkpoint.clone()).await.unwrap();

        let retrieved = manager.get(checkpoint.id).await.unwrap().unwrap();
        assert_eq!(retrieved.total_processed, 42);
        assert_eq!(retrieved.last_processed_id, Some("entry_123".to_string()));
    }

    #[tokio::test]
    async fn test_delete_checkpoint() {
        let manager = CheckpointManager::new();
        let checkpoint = Checkpoint::new("sync_1".to_string());
        let cp_id = checkpoint.id.clone();

        manager.save(checkpoint).await.unwrap();
        manager.delete(cp_id.clone()).await.unwrap();

        let retrieved = manager.get(cp_id).await.unwrap();
        assert!(retrieved.is_none());
    }
}
