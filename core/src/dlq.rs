//! Local dead-letter queue (quarantine) for records that failed to write.
//!
//! Per the verified audit gap (RECHECK-018): a sync that partially fails left
//! the operator with only an aggregate `rows_failed` count -- there was no
//! record of *which* records failed, why, or their payload, so re-driving a
//! failed batch after a transient destination error (a rate limit, a 5xx, a
//! single malformed record) meant re-reading the whole source and guessing.
//!
//! `DeadLetterQueue` is a real, persisted (SQLite) quarantine: every record
//! that a destination rejected is stored with its run id, destination key,
//! the reason for rejection, and its exact payload, so a failed record can be
//! inspected and replayed deliberately instead of re-running the entire sync.

use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

/// One quarantined record: what failed, where it was headed, and why.
#[derive(Debug, Clone, Serialize)]
pub struct DeadLetter {
    pub run_id: String,
    pub destination_key: String,
    pub record_id: String,
    pub reason: String,
    /// The exact record payload (compact JSON) that was rejected.
    pub payload: String,
    pub failed_at: String,
}

pub struct DeadLetterQueue {
    conn: Mutex<Connection>,
}

impl DeadLetterQueue {
    /// Open (creating if absent) a real SQLite file as the quarantine store.
    pub fn open(path: &Path) -> crate::Result<Self> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// SQLite-backed but in-memory -- exercises the real SQL code path without
    /// touching disk (tests).
    #[cfg(test)]
    pub fn in_memory() -> crate::Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> crate::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS dead_letters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                destination_key TEXT NOT NULL,
                record_id TEXT NOT NULL,
                reason TEXT NOT NULL,
                payload TEXT NOT NULL,
                failed_at TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_dead_letters_run ON dead_letters(run_id)",
            [],
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Quarantine one failed record. Appends a new row every time (a record
    /// that fails across several runs leaves a row per run), so the history of
    /// repeated failures is preserved rather than overwritten.
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &self,
        run_id: &str,
        destination_key: &str,
        record_id: &str,
        reason: &str,
        payload: &str,
    ) -> crate::Result<()> {
        let failed_at = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO dead_letters
                (run_id, destination_key, record_id, reason, payload, failed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![run_id, destination_key, record_id, reason, payload, failed_at],
        )?;
        Ok(())
    }

    /// Every record quarantined by a single run, in insertion order.
    pub fn list_for_run(&self, run_id: &str) -> crate::Result<Vec<DeadLetter>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT run_id, destination_key, record_id, reason, payload, failed_at
             FROM dead_letters WHERE run_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![run_id], |row| {
            Ok(DeadLetter {
                run_id: row.get(0)?,
                destination_key: row.get(1)?,
                record_id: row.get(2)?,
                reason: row.get(3)?,
                payload: row.get(4)?,
                failed_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Total number of quarantined records across every run.
    pub fn count(&self) -> crate::Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM dead_letters", [], |row| row.get(0))?;
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_lists_failures_per_run() {
        let dlq = DeadLetterQueue::in_memory().unwrap();
        dlq.record("run-1", "webhook:x", "rec-1", "HTTP 429", "{\"a\":1}")
            .unwrap();
        dlq.record("run-1", "webhook:x", "rec-2", "HTTP 500", "{\"b\":2}")
            .unwrap();
        dlq.record("run-2", "webhook:x", "rec-3", "boom", "{\"c\":3}")
            .unwrap();

        let run1 = dlq.list_for_run("run-1").unwrap();
        assert_eq!(run1.len(), 2);
        assert_eq!(run1[0].record_id, "rec-1");
        assert_eq!(run1[0].reason, "HTTP 429");
        assert_eq!(run1[0].payload, "{\"a\":1}");
        assert_eq!(run1[1].record_id, "rec-2");

        assert_eq!(dlq.list_for_run("run-2").unwrap().len(), 1);
        assert_eq!(dlq.count().unwrap(), 3);
    }

    #[test]
    fn repeated_failures_are_appended_not_overwritten() {
        let dlq = DeadLetterQueue::in_memory().unwrap();
        dlq.record("run-1", "webhook:x", "rec-1", "HTTP 429", "{}")
            .unwrap();
        dlq.record("run-2", "webhook:x", "rec-1", "HTTP 429", "{}")
            .unwrap();
        assert_eq!(dlq.count().unwrap(), 2);
    }
}
