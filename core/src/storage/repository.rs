use crate::{Activation, Destination, Entity, Result, Workflow, SyncRun, SyncRecord};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct Repository {
    conn: Arc<Mutex<Connection>>,
}

impl Repository {
    pub fn new(conn: Connection) -> Result<Self> {
        super::init_schema(&conn)?;
        Ok(Repository {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn save_workflow(&self, workflow: &Workflow) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let mappings_json = serde_json::to_string(&workflow.mappings)?;
        let sync_mode_json = serde_json::to_string(&workflow.sync_mode)?;
        let source_type_json = serde_json::to_string(&workflow.source_type)?;
        let schedule_json = serde_json::to_string(&workflow.schedule)?;
        let rate_limit_json = serde_json::to_string(&workflow.rate_limit)?;
        let event_stream_config_json = serde_json::to_string(&workflow.event_stream_config)?;

        conn.execute(
            "INSERT OR REPLACE INTO workflows
             (id, name, description, version, owner, source_type, sync_mode, mappings, schedule, rate_limit, event_stream_config, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                &workflow.id,
                &workflow.name,
                &workflow.description,
                workflow.version,
                &workflow.owner,
                source_type_json,
                sync_mode_json,
                mappings_json,
                schedule_json,
                rate_limit_json,
                event_stream_config_json,
                workflow.enabled,
                workflow.created_at.to_rfc3339(),
                workflow.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn get_workflow(&self, workflow_id: &str) -> Result<Option<Workflow>> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, name, description, version, owner, source_type, sync_mode, mappings, schedule, rate_limit, event_stream_config, enabled, created_at, updated_at
             FROM workflows WHERE id = ?1",
        )?;

        let workflow = stmt.query_row(rusqlite::params![workflow_id], |row| {
            let source_type_json: String = row.get(5)?;
            let sync_mode_json: String = row.get(6)?;
            let mappings_json: String = row.get(7)?;
            let schedule_json: String = row.get(8)?;
            let rate_limit_json: String = row.get(9)?;
            let event_stream_config_json: String = row.get(10)?;

            Ok(Workflow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                version: row.get(3)?,
                owner: row.get(4)?,
                source_type: serde_json::from_str(&source_type_json).unwrap_or_default(),
                sync_mode: serde_json::from_str(&sync_mode_json).unwrap_or_default(),
                mappings: serde_json::from_str(&mappings_json).unwrap_or_default(),
                schedule: serde_json::from_str(&schedule_json).ok(),
                rate_limit: serde_json::from_str(&rate_limit_json).ok(),
                event_stream_config: serde_json::from_str(&event_stream_config_json).ok(),
                enabled: row.get(11)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(13)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match workflow {
            Ok(w) => Ok(Some(w)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_destination(&self, destination: &Destination) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let config_json = serde_json::to_string(&destination.config)?;
        let dest_type = serde_json::to_string(&destination.destination_type)?;

        conn.execute(
            "INSERT OR REPLACE INTO destinations
             (id, name, destination_type, config, version, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &destination.id,
                &destination.name,
                dest_type,
                config_json,
                destination.version,
                destination.enabled,
                destination.created_at.to_rfc3339(),
                destination.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn save_activation(&self, activation: &Activation) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let destinations_json = serde_json::to_string(&activation.destinations)?;
        let policies_json = serde_json::to_string(&activation.policies)?;

        conn.execute(
            "INSERT OR REPLACE INTO activations
             (id, name, description, workflow_id, version, owner, destinations, policies, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                &activation.id,
                &activation.name,
                &activation.description,
                &activation.workflow_id,
                activation.version,
                &activation.owner,
                destinations_json,
                policies_json,
                activation.enabled,
                activation.created_at.to_rfc3339(),
                activation.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn save_entity(&self, entity: &Entity) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let entity_type_json = serde_json::to_string(&entity.entity_type)?;
        let attributes_json = serde_json::to_string(&entity.attributes)?;
        let traits_json = serde_json::to_string(&entity.traits)?;

        conn.execute(
            "INSERT OR REPLACE INTO entities
             (id, entity_type, key_field, attributes, traits, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                &entity.id,
                entity_type_json,
                &entity.key_field,
                attributes_json,
                traits_json,
                entity.created_at.to_rfc3339(),
                entity.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn save_sync_run(&self, sync_run: &SyncRun) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let status_json = serde_json::to_string(&sync_run.status)?;

        conn.execute(
            "INSERT OR REPLACE INTO sync_runs
             (id, workflow_id, activation_id, status, rows_processed, rows_failed, started_at, completed_at, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &sync_run.id,
                &sync_run.workflow_id,
                &sync_run.activation_id,
                status_json,
                sync_run.rows_processed,
                sync_run.rows_failed,
                sync_run.started_at.to_rfc3339(),
                sync_run.completed_at.map(|t| t.to_rfc3339()),
                &sync_run.error_message,
            ],
        )?;

        Ok(())
    }

    pub fn get_destination(&self, dest_id: &str) -> Result<Option<Destination>> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, name, destination_type, config, version, enabled, created_at, updated_at
             FROM destinations WHERE id = ?1",
        )?;

        let destination = stmt.query_row(rusqlite::params![dest_id], |row| {
            let dest_type_json: String = row.get(2)?;
            let config_json: String = row.get(3)?;

            Ok(Destination {
                id: row.get(0)?,
                name: row.get(1)?,
                destination_type: serde_json::from_str(&dest_type_json).unwrap_or_default(),
                config: serde_json::from_str(&config_json).unwrap_or_default(),
                version: row.get(4)?,
                enabled: row.get(5)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        });

        match destination {
            Ok(d) => Ok(Some(d)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, name, description, version, owner, source_type, sync_mode, mappings, schedule, rate_limit, event_stream_config, enabled, created_at, updated_at
             FROM workflows ORDER BY created_at DESC",
        )?;

        let workflows = stmt.query_map([], |row| {
            let source_type_json: String = row.get(5)?;
            let sync_mode_json: String = row.get(6)?;
            let mappings_json: String = row.get(7)?;
            let schedule_json: String = row.get(8)?;
            let rate_limit_json: String = row.get(9)?;
            let event_stream_config_json: String = row.get(10)?;

            Ok(Workflow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                version: row.get(3)?,
                owner: row.get(4)?,
                source_type: serde_json::from_str(&source_type_json).unwrap_or_default(),
                sync_mode: serde_json::from_str(&sync_mode_json).unwrap_or_default(),
                mappings: serde_json::from_str(&mappings_json).unwrap_or_default(),
                schedule: serde_json::from_str(&schedule_json).ok(),
                rate_limit: serde_json::from_str(&rate_limit_json).ok(),
                event_stream_config: serde_json::from_str(&event_stream_config_json).ok(),
                enabled: row.get(11)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(13)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            })
        })?;

        let mut result = Vec::new();
        for workflow in workflows {
            result.push(workflow?);
        }
        Ok(result)
    }

    pub fn delete_workflow(&self, workflow_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        conn.execute(
            "DELETE FROM workflows WHERE id = ?1",
            rusqlite::params![workflow_id],
        )?;

        Ok(())
    }

    pub fn workflow_count(&self) -> Result<u64> {
        let conn = self.conn.lock().map_err(|_| {
            crate::Error::StorageError("Failed to acquire lock".to_string())
        })?;

        let mut stmt = conn.prepare("SELECT COUNT(*) FROM workflows")?;
        let count: u64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::SourceType;

    #[test]
    fn test_workflow_storage() -> Result<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let repo = Repository::new(conn)?;

        let workflow = Workflow::new("Test Workflow", "owner", SourceType::Table {
            table_name: "customers".to_string(),
        });
        repo.save_workflow(&workflow)?;

        let retrieved = repo.get_workflow(&workflow.id)?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Workflow");

        Ok(())
    }

    #[test]
    fn test_destination_storage() -> Result<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let repo = Repository::new(conn)?;

        let destination = Destination::new("Test Salesforce", crate::destination::DestinationType::Salesforce);
        repo.save_destination(&destination)?;

        let conn = repo.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name FROM destinations WHERE id = ?1")?;
        let name: String = stmt.query_row(rusqlite::params![&destination.id], |row| row.get(0))?;
        assert_eq!(name, "Test Salesforce");

        Ok(())
    }

    #[test]
    fn test_activation_storage() -> Result<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let repo = Repository::new(conn)?;

        let workflow = Workflow::new("WF", "owner", SourceType::Table {
            table_name: "t".to_string(),
        });
        repo.save_workflow(&workflow)?;

        let activation = Activation::new("Test Activation", &workflow.id, "owner");
        repo.save_activation(&activation)?;

        let conn = repo.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name FROM activations WHERE id = ?1")?;
        let name: String = stmt.query_row(rusqlite::params![&activation.id], |row| row.get(0))?;
        assert_eq!(name, "Test Activation");

        Ok(())
    }

    #[test]
    fn test_get_destination() -> Result<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let repo = Repository::new(conn)?;

        let destination = Destination::new("Test Dest", crate::destination::DestinationType::HubSpot);
        repo.save_destination(&destination)?;

        let retrieved = repo.get_destination(&destination.id)?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Dest");

        Ok(())
    }

    #[test]
    fn test_list_workflows() -> Result<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let repo = Repository::new(conn)?;

        let wf1 = Workflow::new("WF1", "owner", SourceType::Table {
            table_name: "t1".to_string(),
        });
        let wf2 = Workflow::new("WF2", "owner", SourceType::Table {
            table_name: "t2".to_string(),
        });

        repo.save_workflow(&wf1)?;
        repo.save_workflow(&wf2)?;

        let workflows = repo.list_workflows()?;
        assert_eq!(workflows.len(), 2);

        Ok(())
    }

    #[test]
    fn test_workflow_count() -> Result<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let repo = Repository::new(conn)?;

        assert_eq!(repo.workflow_count()?, 0);

        let wf = Workflow::new("WF", "owner", SourceType::Table {
            table_name: "t".to_string(),
        });
        repo.save_workflow(&wf)?;

        assert_eq!(repo.workflow_count()?, 1);

        Ok(())
    }

    #[test]
    fn test_delete_workflow() -> Result<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let repo = Repository::new(conn)?;

        let wf = Workflow::new("WF", "owner", SourceType::Table {
            table_name: "t".to_string(),
        });
        repo.save_workflow(&wf)?;
        assert_eq!(repo.workflow_count()?, 1);

        repo.delete_workflow(&wf.id)?;
        assert_eq!(repo.workflow_count()?, 0);

        let retrieved = repo.get_workflow(&wf.id)?;
        assert!(retrieved.is_none());

        Ok(())
    }

    #[test]
    fn test_workflow_with_rate_limit_persistence() -> Result<()> {
        use crate::workflow::RateLimit;

        let conn = rusqlite::Connection::open_in_memory()?;
        let repo = Repository::new(conn)?;

        let workflow = Workflow::new("WF with RateLimit", "owner", SourceType::Table {
            table_name: "t".to_string(),
        })
        .set_rate_limit(RateLimit {
            records_per_second: 100,
            burst_size: Some(500),
        });

        repo.save_workflow(&workflow)?;
        let retrieved = repo.get_workflow(&workflow.id)?;

        assert!(retrieved.is_some());
        let w = retrieved.unwrap();
        assert!(w.rate_limit.is_some());
        assert_eq!(w.rate_limit.unwrap().records_per_second, 100);

        Ok(())
    }
}
