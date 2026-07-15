# PyReverseETL Workflow Integration Guide

PyReverseETL integrates seamlessly with workflow orchestration tools through CLI commands and REST APIs for data activation, synchronization, and destination management.

## Quick Start

### Option 1: CLI (Bash/Shell)

```bash
# Create workflow
pyreverseetl create-workflow ltv_sync "LTV to CRM" snowflake customers

# Create activation (workflow → destination)
pyreverseetl create-activation ltv_to_sf ltv_sync salesforce incremental

# Execute sync
pyreverseetl execute ltv_to_sf 5000

# Check status
pyreverseetl status run_ltv_to_sf_12345
```

### Option 2: REST API

```bash
# Start server
python -m pyreverseetl.server

# Create workflow (HTTP POST)
curl -X POST http://localhost:8000/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "workflow_id": "ltv_sync",
    "config": {
      "name": "LTV to CRM",
      "source": "snowflake",
      "table": "customers"
    }
  }'

# Execute activation
curl -X POST http://localhost:8000/activations/ltv_to_sf/execute \
  -H "Content-Type: application/json" \
  -d '{"limit": 5000}'
```

---

## Workflow Tool Integration

### n8n (No-Code Workflow)

**Create Workflow Node:**
```
Method: POST
URL: http://localhost:8000/workflows
Body:
{
  "workflow_id": "{{ $node.trigger.json.workflow_id }}",
  "config": {
    "name": "{{ $node.trigger.json.name }}",
    "source": "{{ $node.trigger.json.source }}",
    "table": "{{ $node.trigger.json.table }}"
  }
}
```

**Execute Activation Node:**
```
Method: POST
URL: http://localhost:8000/activations/{{ $node.trigger.json.activation_id }}/execute
Body:
{
  "limit": {{ $node.trigger.json.row_limit }}
}
```

---

### Power Automate (Microsoft Cloud)

```
Method: POST
URI: https://your-server/api/activations/{{ triggerBody()?['activation_id'] }}/execute
Headers:
  Content-Type: application/json
Body:
{
  "limit": @{triggerBody()?['row_limit']}
}
```

---

### Temporal (Durable Workflows)

```typescript
import * as wf from "@temporalio/workflow";
import axios from "axios";

export async function dataActivationWorkflow(
  workflowId: string,
  destination: string,
  rowLimit: number
) {
  // Create workflow
  const createRes = await axios.post(
    "http://localhost:8000/workflows",
    {
      workflow_id: workflowId,
      config: { source: "snowflake", destination },
    }
  );

  // Create and execute activation
  const activationId = `${workflowId}_to_${destination}`;
  const activateRes = await axios.post(
    `http://localhost:8000/activations/${activationId}/execute`,
    { limit: rowLimit }
  );

  return activateRes.data;
}
```

---

### Apache Airflow (Python DAGs)

```python
from airflow import DAG
from airflow.operators.python import PythonOperator
from datetime import datetime
import requests

def create_workflow(workflow_id, source, table, **context):
    response = requests.post(
        "http://localhost:8000/workflows",
        json={
            "workflow_id": workflow_id,
            "config": {"source": source, "table": table}
        },
    )
    return response.json()

def execute_sync(activation_id, row_limit, **context):
    response = requests.post(
        f"http://localhost:8000/activations/{activation_id}/execute",
        json={"limit": row_limit},
    )
    return response.json()

with DAG(
    "data_activation_pipeline",
    start_date=datetime(2024, 1, 1),
    schedule_interval="daily",
) as dag:
    setup = PythonOperator(
        task_id="setup_workflow",
        python_callable=create_workflow,
        op_kwargs={
            "workflow_id": "ltv_sync",
            "source": "snowflake",
            "table": "customers",
        },
    )

    sync = PythonOperator(
        task_id="execute_sync",
        python_callable=execute_sync,
        op_kwargs={
            "activation_id": "ltv_to_sf",
            "row_limit": 5000,
        },
    )

    setup >> sync
```

---

## Bash/Shell Integration

```bash
#!/bin/bash

WORKFLOW_ID="ltv_sync"
ACTIVATION_ID="ltv_to_sf"
DESTINATION="salesforce"
ROW_LIMIT=5000

# Create workflow
echo "Creating workflow..."
pyreverseetl create-workflow $WORKFLOW_ID "LTV Sync" snowflake customers

# Create activation
echo "Creating activation..."
pyreverseetl create-activation $ACTIVATION_ID $WORKFLOW_ID $DESTINATION incremental

# Execute sync
echo "Executing sync..."
RESULT=$(pyreverseetl execute $ACTIVATION_ID $ROW_LIMIT)
RUN_ID=$(echo $RESULT | jq -r '.run_id')

# Check status
echo "Checking status..."
sleep 5
pyreverseetl status $RUN_ID | jq '.sync_status'

# Get metrics
pyreverseetl metrics $ACTIVATION_ID | jq '.total_rows_synced'
```

---

## API Endpoints Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/workflows` | Create a new workflow |
| GET | `/workflows` | List all workflows |
| POST | `/activations` | Create an activation |
| GET | `/activations` | List all activations |
| POST | `/activations/<id>/execute` | Execute activation |
| GET | `/runs/<run_id>` | Get run status |
| GET | `/metrics` | Get all metrics |
| GET | `/metrics?activation_id=X` | Get activation metrics |
| GET | `/health` | Health check |

---

## Docker Deployment

```dockerfile
FROM python:3.11-slim

WORKDIR /app
RUN pip install pyreverseetl flask

COPY . .

EXPOSE 8000

CMD ["python", "-m", "pyreverseetl.server"]
```

**docker-compose.yml:**
```yaml
version: '3.8'
services:
  pyreverseetl:
    build: .
    ports:
      - "8000:8000"
    environment:
      - FLASK_ENV=production
    restart: unless-stopped
```

---

## Integration Patterns

### Quality Gates with StatGuardian
```bash
# Check data quality before activating
if statguardian validate source snowflake.customers; then
  pyreverseetl execute ltv_to_sf 5000
else
  echo "Data quality check failed"
fi
```

### Intelligent Retrieval with PyStreamMCP
```bash
# Get optimized context for activation
CONTEXT=$(pystreammcp query "customer context for activation" retrieve)

# Create workflow with optimized context
pyreverseetl create-workflow ltv_sync "LTV Sync" snowflake customers
```

### Activation Observability
```bash
# Monitor activation metrics
while true; do
  METRICS=$(pyreverseetl metrics ltv_to_sf)
  echo "Rows synced: $(echo $METRICS | jq '.total_rows_synced')"
  sleep 10
done
```

---

## Support

- Issues: https://github.com/Mullassery/PyReverseETL/issues
- Discussions: https://github.com/Mullassery/PyReverseETL/discussions
