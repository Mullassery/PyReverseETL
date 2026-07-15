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

    #[test]
    fn test_append_change() {
        let changelog = ChangeLog::new(":memory:").unwrap();
        let change = create_test_change("1");

        let entry_id = changelog.append(change).unwrap();

        assert!(!entry_id.is_empty());
        let entries = changelog.all_entries().unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_get_unprocessed() {
        let changelog = ChangeLog::new(":memory:").unwrap();

        let change1 = create_test_change("1");
        let entry_id1 = changelog.append(change1).unwrap();

        let change2 = create_test_change("2");
        changelog.append(change2).unwrap();

        changelog.mark_processed(entry_id1).unwrap();

        let unprocessed = changelog.get_unprocessed().unwrap();
        assert_eq!(unprocessed.len(), 1);
        assert_eq!(unprocessed[0].change.entity_id, "2");
    }

    #[test]
    fn test_mark_processed() {
        let changelog = ChangeLog::new(":memory:").unwrap();
        let change = create_test_change("1");

        let entry_id = changelog.append(change).unwrap();
        changelog.mark_processed(entry_id.clone()).unwrap();

        let entries = changelog.all_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].processed);
        assert!(entries[0].processed_at.is_some());
    }

    #[test]
    fn test_changelog_entries_limit() {
        let changelog = ChangeLog::new(":memory:").unwrap();

        for i in 1..=10 {
            let change = create_test_change(&i.to_string());
            changelog.append(change).unwrap();
        }

        let limited = changelog.entries(5).unwrap();
        assert_eq!(limited.len(), 5);
    }

    #[test]
    fn test_changelog_clear() {
        let changelog = ChangeLog::new(":memory:").unwrap();
        let change = create_test_change("1");
        changelog.append(change).unwrap();

        assert_eq!(changelog.all_entries().unwrap().len(), 1);

        changelog.clear().unwrap();
        assert_eq!(changelog.all_entries().unwrap().len(), 0);
    }
}
