use super::Change;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use uuid::Uuid;

/// Entry in the changelog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeLogEntry {
    /// Unique entry ID
    pub id: String,
    /// The change recorded
    pub change: Change,
    /// Whether this change has been processed
    pub processed: bool,
    /// When the entry was created
    pub created_at: DateTime<Utc>,
    /// When the entry was processed (if applicable)
    pub processed_at: Option<DateTime<Utc>>,
}

/// Persistent changelog using JSON lines format
pub struct ChangeLog {
    path: String,
}

impl ChangeLog {
    /// Create or open a changelog file
    pub fn new(path: &str) -> crate::Result<Self> {
        let path_obj = Path::new(path);
        if let Some(parent) = path_obj.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| crate::Error::from(e))?;

        Ok(Self {
            path: path.to_string(),
        })
    }

    /// Append a new change to the changelog
    pub fn append(&self, change: Change) -> crate::Result<String> {
        let entry = ChangeLogEntry {
            id: Uuid::new_v4().to_string(),
            change,
            processed: false,
            created_at: Utc::now(),
            processed_at: None,
        };

        let json_line = serde_json::to_string(&entry)?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|e| crate::Error::from(e))?;

        writeln!(file, "{}", json_line).map_err(|e| crate::Error::from(e))?;

        Ok(entry.id)
    }

    /// Get all unprocessed entries
    pub fn get_unprocessed(&self) -> crate::Result<Vec<ChangeLogEntry>> {
        Ok(self
            .read_entries()?
            .into_iter()
            .filter(|e| !e.processed)
            .collect::<Vec<_>>())
    }

    /// Mark an entry as processed
    pub fn mark_processed(&self, entry_id: String) -> crate::Result<()> {
        let mut entries = self.read_entries()?;

        for entry in &mut entries {
            if entry.id == entry_id {
                entry.processed = true;
                entry.processed_at = Some(Utc::now());
                break;
            }
        }

        self.write_entries(&entries)?;
        Ok(())
    }

    /// Get entries with limit
    pub fn entries(&self, limit: usize) -> crate::Result<Vec<ChangeLogEntry>> {
        let mut all = self.read_entries()?;
        all.truncate(limit);
        Ok(all)
    }

    /// Get all entries
    pub fn all_entries(&self) -> crate::Result<Vec<ChangeLogEntry>> {
        self.read_entries()
    }

    /// Get entries for a specific sync run
    pub fn entries_by_sync_run(&self, sync_run_id: String) -> crate::Result<Vec<ChangeLogEntry>> {
        Ok(self
            .read_entries()?
            .into_iter()
            .filter(|e| e.change.entity_id.starts_with(&format!("{}:", sync_run_id)))
            .collect())
    }

    // Helper: Read all entries from file
    fn read_entries(&self) -> crate::Result<Vec<ChangeLogEntry>> {
        let file = match OpenOptions::new().read(true).open(&self.path) {
            Ok(f) => f,
            Err(_) => return Ok(Vec::new()),
        };

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            if let Ok(line) = line {
                if let Ok(entry) = serde_json::from_str::<ChangeLogEntry>(&line) {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    // Helper: Write all entries to file (overwrites)
    fn write_entries(&self, entries: &[ChangeLogEntry]) -> crate::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| crate::Error::from(e))?;

        for entry in entries {
            let json_line = serde_json::to_string(entry)?;
            writeln!(file, "{}", json_line).map_err(|e| crate::Error::from(e))?;
        }

        Ok(())
    }

    /// Clear all entries
    pub fn clear(&self) -> crate::Result<()> {
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| crate::Error::from(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_change(entity_id: &str) -> Change {
        Change {
            entity_id: entity_id.to_string(),
            change_type: super::super::ChangeType::Created,
            before: None,
            after: json!({"id": entity_id}),
            timestamp: Utc::now(),
            changed_fields: vec![],
        }
    }

    /// A real ChangeLog is a real file on disk (see the struct docs - "Persistent
    /// changelog using JSON lines format"); there is no actual in-memory mode
    /// despite the ":memory:" literal these tests used to pass as the path. Since
    /// cargo test runs tests in parallel by default, every test opening the same
    /// literal ":memory:" file in append mode was writing into (and reading back)
    /// each other's entries, causing the counts asserted below to depend on
    /// scheduling. Each test now gets its own real, isolated temp file.
    fn temp_changelog_path(test_name: &str) -> String {
        std::env::temp_dir()
            .join(format!("pyreverseetl-changelog-test-{test_name}-{}.jsonl", Uuid::new_v4()))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn test_append_change() {
        let path = temp_changelog_path("append_change");
        let changelog = ChangeLog::new(&path).unwrap();
        let change = create_test_change("1");

        let entry_id = changelog.append(change).unwrap();

        assert!(!entry_id.is_empty());
        let entries = changelog.all_entries().unwrap();
        assert_eq!(entries.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_get_unprocessed() {
        let path = temp_changelog_path("get_unprocessed");
        let changelog = ChangeLog::new(&path).unwrap();

        let change1 = create_test_change("1");
        let entry_id1 = changelog.append(change1).unwrap();

        let change2 = create_test_change("2");
        changelog.append(change2).unwrap();

        changelog.mark_processed(entry_id1).unwrap();

        let unprocessed = changelog.get_unprocessed().unwrap();
        assert_eq!(unprocessed.len(), 1);
        assert_eq!(unprocessed[0].change.entity_id, "2");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_mark_processed() {
        let path = temp_changelog_path("mark_processed");
        let changelog = ChangeLog::new(&path).unwrap();
        let change = create_test_change("1");

        let entry_id = changelog.append(change).unwrap();
        changelog.mark_processed(entry_id.clone()).unwrap();

        let entries = changelog.all_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].processed);
        assert!(entries[0].processed_at.is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_changelog_entries_limit() {
        let path = temp_changelog_path("entries_limit");
        let changelog = ChangeLog::new(&path).unwrap();

        for i in 1..=10 {
            let change = create_test_change(&i.to_string());
            changelog.append(change).unwrap();
        }

        let limited = changelog.entries(5).unwrap();
        assert_eq!(limited.len(), 5);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_changelog_clear() {
        let path = temp_changelog_path("clear");
        let changelog = ChangeLog::new(&path).unwrap();
        let change = create_test_change("1");
        changelog.append(change).unwrap();

        assert_eq!(changelog.all_entries().unwrap().len(), 1);

        changelog.clear().unwrap();
        assert_eq!(changelog.all_entries().unwrap().len(), 0);
        let _ = std::fs::remove_file(&path);
    }
}
