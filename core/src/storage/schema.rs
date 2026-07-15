use crate::Result;
use rusqlite::Connection;

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS workflows (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            version INTEGER NOT NULL,
            owner TEXT NOT NULL,
            source_type TEXT NOT NULL,
            sync_mode TEXT NOT NULL,
            mappings TEXT NOT NULL,
            schedule TEXT,
            enabled BOOLEAN NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS destinations (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            destination_type TEXT NOT NULL,
            config TEXT NOT NULL,
            version INTEGER NOT NULL,
            enabled BOOLEAN NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS activations (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            workflow_id TEXT NOT NULL,
            version INTEGER NOT NULL,
            owner TEXT NOT NULL,
            destinations TEXT NOT NULL,
            policies TEXT NOT NULL,
            enabled BOOLEAN NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (workflow_id) REFERENCES workflows(id)
        );

        CREATE TABLE IF NOT EXISTS entities (
            id TEXT PRIMARY KEY,
            entity_type TEXT NOT NULL,
            key_field TEXT NOT NULL,
            attributes TEXT NOT NULL,
            traits TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sync_runs (
            id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            activation_id TEXT NOT NULL,
            status TEXT NOT NULL,
            rows_processed INTEGER NOT NULL,
            rows_failed INTEGER NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            error_message TEXT,
            FOREIGN KEY (workflow_id) REFERENCES workflows(id),
            FOREIGN KEY (activation_id) REFERENCES activations(id)
        );

        CREATE TABLE IF NOT EXISTS sync_records (
            id TEXT PRIMARY KEY,
            sync_run_id TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            destination_id TEXT NOT NULL,
            action TEXT NOT NULL,
            payload TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (sync_run_id) REFERENCES sync_runs(id),
            FOREIGN KEY (entity_id) REFERENCES entities(id),
            FOREIGN KEY (destination_id) REFERENCES destinations(id)
        );

        CREATE INDEX IF NOT EXISTS idx_activations_workflow ON activations(workflow_id);
        CREATE INDEX IF NOT EXISTS idx_sync_runs_workflow ON sync_runs(workflow_id);
        CREATE INDEX IF NOT EXISTS idx_sync_runs_activation ON sync_runs(activation_id);
        CREATE INDEX IF NOT EXISTS idx_sync_records_sync_run ON sync_records(sync_run_id);
        CREATE INDEX IF NOT EXISTS idx_sync_records_entity ON sync_records(entity_id);
        CREATE INDEX IF NOT EXISTS idx_sync_records_destination ON sync_records(destination_id);
        "
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_initialization() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let conn = rusqlite::Connection::open_in_memory()?;
        init_schema(&conn)?;

        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        assert!(tables.contains(&"workflows".to_string()));
        assert!(tables.contains(&"destinations".to_string()));
        assert!(tables.contains(&"activations".to_string()));
        assert!(tables.contains(&"entities".to_string()));
        assert!(tables.contains(&"sync_runs".to_string()));
        assert!(tables.contains(&"sync_records".to_string()));

        Ok(())
    }
}
