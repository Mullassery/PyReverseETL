use pyo3::prelude::*;
use pyreverseetl_core::{Workflow, Destination, Activation, Entity, SyncRun};
use pyreverseetl_core::workflow::SourceType;
use pyreverseetl_core::destination::DestinationType;
use pyreverseetl_core::entity::EntityType;

#[pyclass]
pub struct PyWorkflow {
    inner: Workflow,
}

#[pymethods]
impl PyWorkflow {
    #[staticmethod]
    #[pyo3(signature = (name, owner, source_type, table_name=None))]
    pub fn new(name: &str, owner: &str, source_type: &str, table_name: Option<&str>) -> Self {
        let source = match source_type {
            "table" => SourceType::Table {
                table_name: table_name.unwrap_or("default").to_string(),
            },
            _ => SourceType::Table {
                table_name: table_name.unwrap_or("default").to_string(),
            },
        };
        PyWorkflow {
            inner: Workflow::new(name, owner, source),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn with_description(&mut self, desc: &str) -> PyResult<()> {
        self.inner = self.inner.clone().with_description(desc);
        Ok(())
    }

    pub fn add_mapping(&mut self, source: &str, dest: &str) -> PyResult<()> {
        self.inner = self.inner.clone().add_mapping(source, dest);
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) -> PyResult<()> {
        self.inner = self.inner.clone().set_enabled(enabled);
        Ok(())
    }
}

#[pyclass]
pub struct PyDestination {
    inner: Destination,
}

#[pymethods]
impl PyDestination {
    #[staticmethod]
    pub fn new(name: &str, dest_type: &str) -> Self {
        let dtype = match dest_type {
            "salesforce" => DestinationType::Salesforce,
            "hubspot" => DestinationType::HubSpot,
            "kafka" => DestinationType::Kafka,
            "webhook" => DestinationType::Webhook,
            _ => DestinationType::Webhook,
        };
        PyDestination {
            inner: Destination::new(name, dtype),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn set_enabled(&mut self, enabled: bool) -> PyResult<()> {
        self.inner = self.inner.clone().set_enabled(enabled);
        Ok(())
    }
}

#[pyclass]
pub struct PyActivation {
    inner: Activation,
}

#[pymethods]
impl PyActivation {
    #[staticmethod]
    pub fn new(name: &str, workflow_id: &str, owner: &str) -> Self {
        PyActivation {
            inner: Activation::new(name, workflow_id, owner),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn add_destination(&mut self, dest_id: &str) -> PyResult<()> {
        self.inner = self.inner.clone().add_destination(dest_id);
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) -> PyResult<()> {
        self.inner = self.inner.clone().set_enabled(enabled);
        Ok(())
    }
}

#[pyclass]
pub struct PyEntity {
    inner: Entity,
}

#[pymethods]
impl PyEntity {
    #[staticmethod]
    pub fn new(entity_type: &str, key_field: &str, id: &str) -> Self {
        let etype = match entity_type {
            "customer" => EntityType::Customer,
            "account" => EntityType::Account,
            "company" => EntityType::Company,
            _ => EntityType::Customer,
        };
        PyEntity {
            inner: Entity::new(etype, key_field, id),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    pub fn add_trait(&mut self, name: &str, value: &str) -> PyResult<()> {
        self.inner = self.inner.clone().add_trait(name, serde_json::json!(value));
        Ok(())
    }

    pub fn add_attribute(&mut self, key: &str, value: &str) -> PyResult<()> {
        self.inner = self.inner.clone().add_attribute(key, serde_json::json!(value));
        Ok(())
    }
}

#[pyclass]
pub struct PySyncRun {
    inner: SyncRun,
}

#[pymethods]
impl PySyncRun {
    #[staticmethod]
    pub fn new(workflow_id: &str, activation_id: &str) -> Self {
        PySyncRun {
            inner: SyncRun::new(workflow_id, activation_id),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    pub fn status(&self) -> String {
        self.inner.status.to_string()
    }

    pub fn mark_running(&mut self) -> PyResult<()> {
        self.inner.mark_running();
        Ok(())
    }

    pub fn mark_success(&mut self) -> PyResult<()> {
        self.inner.mark_success();
        Ok(())
    }

    pub fn record_processed(&mut self, count: u64) -> PyResult<()> {
        self.inner.record_processed(count);
        Ok(())
    }
}

#[pymodule]
fn pyreverseetl(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<PyWorkflow>()?;
    m.add_class::<PyDestination>()?;
    m.add_class::<PyActivation>()?;
    m.add_class::<PyEntity>()?;
    m.add_class::<PySyncRun>()?;
    Ok(())
}
