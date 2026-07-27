# Phase 3 Week 3: CDC Engine Implementation Plan

## Objectives
- Change detection with before/after comparison
- Changelog with transaction log storage
- Checkpoint management for tracking processed changes
- 9 new tests (151 total)

## New Modules

### core/src/cdc/mod.rs
Module exports and organization

### core/src/cdc/change_detector.rs
```rust
pub struct ChangeDetector {
    previous_state: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

pub struct Change {
    pub entity_id: String,
    pub change_type: ChangeType,
    pub before: Option<serde_json::Value>,
    pub after: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub changed_fields: Vec<String>,
}

pub enum ChangeType {
    Created,
    Updated,
    Deleted,
}

impl ChangeDetector {
    pub fn new() -> Self
    pub async fn detect(&self, current: serde_json::Value) -> Vec<Change>
    pub async fn compare_values(before: &Value, after: &Value) -> Vec<String>
}
```

### core/src/cdc/changelog.rs
```rust
pub struct ChangeLog {
    path: String,
}

pub struct ChangeLogEntry {
    pub id: String,
    pub change: Change,
    pub processed: bool,
    pub created_at: DateTime<Utc>,
}

impl ChangeLog {
    pub fn new(path: &str) -> Result<Self>
    pub async fn append(&self, change: Change) -> Result<()>
    pub async fn get_unprocessed(&self) -> Result<Vec<ChangeLogEntry>>
    pub async fn mark_processed(&self, id: String) -> Result<()>
    pub async fn entries(&self, limit: usize) -> Result<Vec<ChangeLogEntry>>
}
```

### core/src/cdc/checkpoint.rs
```rust
pub struct Checkpoint {
    pub sync_run_id: String,
    pub last_processed_id: String,
    pub last_processed_at: DateTime<Utc>,
    pub total_processed: u64,
}

pub struct CheckpointManager {
    repo: Arc<Repository>,
}

impl CheckpointManager {
    pub fn new(repo: Arc<Repository>) -> Self
    pub async fn save(&self, checkpoint: Checkpoint) -> Result<()>
    pub async fn get_latest(&self) -> Result<Option<Checkpoint>>
    pub async fn list(&self, limit: usize) -> Result<Vec<Checkpoint>>
}
```

## Schema Changes

Add to sync_runs table:
```sql
-- Already has: id, workflow_id, status, started_at, completed_at, etc.
ALTER TABLE sync_runs ADD COLUMN checkpoint_id TEXT;
```

New table - changelogs:
```sql
CREATE TABLE changelogs (
    id TEXT PRIMARY KEY,
    sync_run_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    change_type TEXT NOT NULL (created, updated, deleted),
    before_value JSON,
    after_value JSON,
    changed_fields JSON,
    processed BOOLEAN DEFAULT 0,
    created_at TEXT NOT NULL,
    processed_at TEXT,
    FOREIGN KEY(sync_run_id) REFERENCES sync_runs(id)
);

CREATE INDEX idx_changelogs_sync_run ON changelogs(sync_run_id);
CREATE INDEX idx_changelogs_processed ON changelogs(processed);
```

New table - checkpoints:
```sql
CREATE TABLE checkpoints (
    id TEXT PRIMARY KEY,
    sync_run_id TEXT NOT NULL,
    last_processed_id TEXT,
    last_processed_at TEXT NOT NULL,
    total_processed INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY(sync_run_id) REFERENCES sync_runs(id)
);

CREATE INDEX idx_checkpoints_sync_run ON checkpoints(sync_run_id);
```

## Tests (9 total)

### test_change_detector.rs (3 tests)
- test_detect_created_entity: New entity detected as Created change
- test_detect_updated_entity: Changed fields tracked
- test_detect_deleted_entity: Entity removal detected

### test_changelog.rs (3 tests)
- test_append_change: Change entry saved to changelog
- test_get_unprocessed: Only unprocessed entries returned
- test_mark_processed: Entry marked as processed

### test_checkpoint.rs (3 tests)
- test_save_checkpoint: Checkpoint persisted
- test_get_latest: Latest checkpoint retrieved
- test_checkpoint_idempotent: Multiple saves with same ID update

## Implementation Order
1. core/src/cdc/change_detector.rs (3 tests)
2. core/src/cdc/changelog.rs (3 tests)
3. core/src/cdc/checkpoint.rs (3 tests)
4. Update schema.rs with new tables
5. Update repository.rs with changelog/checkpoint methods
6. core/src/cdc/mod.rs
7. Update core/src/lib.rs to export cdc module
8. Update Cargo.toml if needed
9. All tests passing, commit

## Success Criteria
- ✅ 9 new tests passing
- ✅ 151 total tests passing
- ✅ Change detection working for create/update/delete
- ✅ Changelog persisting changes
- ✅ Checkpoint tracking progress
- ✅ Integration with SyncRun workflow
