//! Local idempotency ledger.
//!
//! Per docs/ROADMAP.md's verified gap: duplicate protection relied entirely
//! on destination-side upsert keys (real, e.g. HubSpot `idProperty=email`,
//! Salesforce external-ID upsert) -- which don't cover crash-mid-batch
//! recovery, no-op-change detection, or adapters with no upsert semantics at
//! all (the webhook adapter just POSTs, so re-running a sync after a crash
//! re-POSTs every record, duplicates and all).
//!
//! `IdempotencyLedger` is a real, persisted (SQLite) record of "this exact
//! record content was already successfully sent to this destination".
//! Content-hash-based (not just record-id-based) so a genuinely *changed*
//! record is never skipped, even if its id was seen before -- this is what
//! gives real no-op-change detection, not just duplicate-send prevention.

use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

pub struct IdempotencyLedger {
    conn: Mutex<Connection>,
}

impl IdempotencyLedger {
    /// Open (creating if absent) a real SQLite file as the ledger.
    pub fn open(path: &Path) -> crate::Result<Self> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// SQLite-backed but in-memory -- for tests that want to exercise the
    /// real SQL code path without touching disk.
    #[cfg(test)]
    pub fn in_memory() -> crate::Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> crate::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS synced_records (
                destination_key TEXT NOT NULL,
                record_id TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                synced_at TEXT NOT NULL,
                PRIMARY KEY (destination_key, record_id)
            )",
            [],
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// True only if this exact record content (`content_hash`) was already
    /// successfully synced to this destination. A record whose id was seen
    /// before but whose content actually changed (different hash) is never
    /// considered already-synced -- it's a real change, so it's sent again.
    pub fn already_synced(
        &self,
        destination_key: &str,
        record_id: &str,
        content_hash: &str,
    ) -> crate::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<String> = conn
            .query_row(
                "SELECT content_hash FROM synced_records WHERE destination_key = ?1 AND record_id = ?2",
                params![destination_key, record_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(existing.as_deref() == Some(content_hash))
    }

    /// Record that `record_id` with this exact `content_hash` was
    /// successfully synced to `destination_key`. Idempotent to call
    /// multiple times, including after a crash mid-batch -- the next run
    /// re-reads this same real state rather than trusting in-memory
    /// progress that didn't survive the crash.
    pub fn mark_synced(
        &self,
        destination_key: &str,
        record_id: &str,
        content_hash: &str,
    ) -> crate::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO synced_records (destination_key, record_id, content_hash, synced_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(destination_key, record_id) DO UPDATE SET
                content_hash = excluded.content_hash,
                synced_at = excluded.synced_at",
            params![
                destination_key,
                record_id,
                content_hash,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }
}

/// A deterministic content hash for a record payload, independent of key
/// insertion order (so hashing the same logical content twice always
/// produces the same hash, regardless of which `serde_json::Map`
/// implementation/ordering produced the `Value`).
pub fn content_hash(value: &serde_json::Value) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let canonical = canonicalize(value).to_string();
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn canonicalize(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), canonicalize(v)))
                .collect();
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(canonicalize).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn content_hash_is_stable_regardless_of_key_order() {
        let a = json!({"email": "x@y.com", "name": "X"});
        let b = json!({"name": "X", "email": "x@y.com"});
        assert_eq!(content_hash(&a), content_hash(&b));
    }

    #[test]
    fn content_hash_changes_when_content_changes() {
        let a = json!({"email": "x@y.com"});
        let b = json!({"email": "y@z.com"});
        assert_ne!(content_hash(&a), content_hash(&b));
    }

    #[test]
    fn unseen_record_is_not_already_synced() {
        let ledger = IdempotencyLedger::in_memory().unwrap();
        assert!(!ledger.already_synced("webhook:x", "rec1", "hash1").unwrap());
    }

    #[test]
    fn marked_record_is_already_synced_with_same_hash() {
        let ledger = IdempotencyLedger::in_memory().unwrap();
        ledger.mark_synced("webhook:x", "rec1", "hash1").unwrap();
        assert!(ledger.already_synced("webhook:x", "rec1", "hash1").unwrap());
    }

    #[test]
    fn changed_content_is_not_already_synced_even_with_same_id() {
        let ledger = IdempotencyLedger::in_memory().unwrap();
        ledger.mark_synced("webhook:x", "rec1", "hash1").unwrap();
        assert!(
            !ledger.already_synced("webhook:x", "rec1", "hash2").unwrap(),
            "a genuinely changed record must never be treated as already-synced"
        );
    }

    #[test]
    fn same_record_id_in_different_destinations_does_not_collide() {
        let ledger = IdempotencyLedger::in_memory().unwrap();
        ledger.mark_synced("webhook:a", "rec1", "hash1").unwrap();
        assert!(!ledger.already_synced("webhook:b", "rec1", "hash1").unwrap());
    }

    #[test]
    fn mark_synced_twice_updates_the_hash() {
        let ledger = IdempotencyLedger::in_memory().unwrap();
        ledger.mark_synced("webhook:x", "rec1", "hash1").unwrap();
        ledger.mark_synced("webhook:x", "rec1", "hash2").unwrap();
        assert!(ledger.already_synced("webhook:x", "rec1", "hash2").unwrap());
        assert!(!ledger.already_synced("webhook:x", "rec1", "hash1").unwrap());
    }

    #[test]
    fn persists_across_separate_connections_to_the_same_file() {
        let dir = std::env::temp_dir().join(format!("pyreverseetl_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("idempotency.db");

        {
            let ledger = IdempotencyLedger::open(&path).unwrap();
            ledger.mark_synced("webhook:x", "rec1", "hash1").unwrap();
        }
        {
            let ledger = IdempotencyLedger::open(&path).unwrap();
            assert!(ledger.already_synced("webhook:x", "rec1", "hash1").unwrap());
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
