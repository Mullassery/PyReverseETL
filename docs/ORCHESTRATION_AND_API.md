# PyReverseETL Orchestration & API

**Integrate with Airflow, Kubernetes, and bash scripts. REST API for all operations.**

---

## Bash Command Line

Run syncs directly from bash:

```bash
# Execute a sync
pyreverseetl sync run customer_sync.yaml

# List all configured syncs
pyreverseetl sync list

# Show sync status
pyreverseetl sync status customer_sync

# Get last run details
pyreverseetl sync last-run customer_sync

# Dry-run (validate without executing)
pyreverseetl sync run customer_sync.yaml --dry-run

# Run with custom parameters
pyreverseetl sync run customer_sync.yaml \
  --param DB_HOST=prod.db.com \
  --param SYNC_DATE=2024-01-15

# Run with parallelism
pyreverseetl sync run customer_sync.yaml --workers=4

# Monitor in real-time
pyreverseetl sync run customer_sync.yaml --follow

# Export results
pyreverseetl sync run customer_sync.yaml --output-format=json > results.json
```

### Batch Operations

Run multiple syncs:

```bash
# Run all syncs in a directory
pyreverseetl sync run-all ./syncs/

# Run only production syncs
pyreverseetl sync run-all ./syncs/ --tag=production

# Schedule a recurring sync
pyreverseetl sync schedule customer_sync.yaml \
  --frequency=hourly \
  --timezone=America/New_York

# Cancel a running sync
pyreverseetl sync cancel customer_sync:run-12345

# List running syncs
pyreverseetl sync list-running

# Get historical sync results
pyreverseetl sync history customer_sync --days=30
```

---

## REST API

Full HTTP API for integration with Airflow, Kubernetes, and custom applications.

### Base URL
```
http://localhost:8080/api/v1
```

### Authentication
```bash
# Bearer token auth
curl -H "Authorization: Bearer $PYREVERSEETL_TOKEN" \
  http://localhost:8080/api/v1/syncs
```

### Core Endpoints

#### List Syncs
```bash
GET /syncs

# With filters
GET /syncs?tag=production&status=success
```

Response:
```json
{
  "syncs": [
    {
      "id": "cust_sync_001",
      "name": "customer_sync",
      "source": "postgres",
      "destination": "snowflake",
      "status": "idle",
      "last_run": "2024-01-15T10:30:00Z",
      "next_run": "2024-01-15T11:30:00Z"
    }
  ]
}
```

#### Get Sync Details
```bash
GET /syncs/{sync_id}
```

Response:
```json
{
  "id": "cust_sync_001",
  "name": "customer_sync",
  "description": "Sync customer data to warehouse",
  "source": {
    "type": "postgres",
    "host": "db.example.com",
    "database": "analytics"
  },
  "destination": {
    "type": "snowflake",
    "warehouse": "compute"
  },
  "schedule": {
    "frequency": "hourly",
    "timezone": "America/New_York"
  }
}
```

#### Trigger Sync (Airflow Integration)
```bash
POST /syncs/{sync_id}/trigger

# With parameters
POST /syncs/{sync_id}/trigger
Content-Type: application/json

{
  "parameters": {
    "DB_HOST": "prod.db.com",
    "SYNC_DATE": "2024-01-15"
  },
  "tags": ["airflow", "daily_batch"]
}
```

Response:
```json
{
  "run_id": "cust_sync_001:run-12345",
  "status": "running",
  "started_at": "2024-01-15T10:30:00Z",
  "estimated_duration": 120
}
```

#### Get Sync Run Status
```bash
GET /syncs/{sync_id}/runs/{run_id}
```

Response:
```json
{
  "run_id": "cust_sync_001:run-12345",
  "status": "running",
  "started_at": "2024-01-15T10:30:00Z",
  "progress": {
    "records_read": 150000,
    "records_written": 145000,
    "errors": 5,
    "percent_complete": 75
  }
}
```

#### Wait for Sync Completion
```bash
GET /syncs/{sync_id}/runs/{run_id}/wait?timeout=3600

# Returns immediately if sync is complete, waits up to 1 hour
```

#### Get Historical Runs
```bash
GET /syncs/{sync_id}/runs?limit=10&status=success
```

#### Cancel Running Sync
```bash
DELETE /syncs/{sync_id}/runs/{run_id}
```

---

## Airflow Integration

### Using Airflow Operators

#### Via PythonOperator
```python
from airflow import DAG
from airflow.operators.python import PythonOperator
from pyreverseetl import Connector, SyncPipeline
from datetime import datetime

def run_customer_sync(**context):
    source = Connector.postgres(
        host="db.example.com",
        database="analytics",
        query="SELECT * FROM customers"
    )
    
    dest = Connector.snowflake(
        account="xy12345",
        warehouse="compute",
        database="analytics",
        table="customers"
    )
    
    pipeline = SyncPipeline(source, dest)
    result = pipeline.execute()
    
    context['task_instance'].xcom_push(
        key='rows_synced',
        value=result.rows_written
    )

with DAG('customer_sync', start_date=datetime(2024, 1, 1), schedule='@hourly'):
    sync_task = PythonOperator(
        task_id='sync_customers',
        python_callable=run_customer_sync,
    )
```

#### Via HTTP Operator (Using REST API)
```python
from airflow import DAG
from airflow.operators.http import SimpleHttpOperator
from datetime import datetime

with DAG('customer_sync_api', start_date=datetime(2024, 1, 1), schedule='@hourly'):
    trigger_sync = SimpleHttpOperator(
        task_id='trigger_sync',
        method='POST',
        http_conn_id='pyreverseetl_api',
        endpoint='/api/v1/syncs/cust_sync_001/trigger',
        data='{"parameters": {}}',
        xcom_push=True,
    )
    
    wait_sync = SimpleHttpOperator(
        task_id='wait_sync',
        method='GET',
        http_conn_id='pyreverseetl_api',
        endpoint='/api/v1/syncs/cust_sync_001/runs/{{ ti.xcom_pull(task_ids="trigger_sync", key="run_id") }}/wait',
        xcom_push=True,
    )
    
    trigger_sync >> wait_sync
```

#### Via Bash Operator
```python
from airflow import DAG
from airflow.operators.bash import BashOperator
from datetime import datetime

with DAG('customer_sync_bash', start_date=datetime(2024, 1, 1), schedule='@hourly'):
    sync_task = BashOperator(
        task_id='sync_customers',
        bash_command='pyreverseetl sync run customer_sync.yaml',
    )
    
    check_task = BashOperator(
        task_id='check_results',
        bash_command='''
        RESULT=$(pyreverseetl sync last-run customer_sync --output-format=json)
        ROWS=$(echo $RESULT | jq '.rows_written')
        echo "Synced $ROWS records"
        ''',
    )
    
    sync_task >> check_task
```

---

## Kubernetes Integration

### StatefulSet Deployment
```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: pyreverseetl
spec:
  replicas: 3
  selector:
    matchLabels:
      app: pyreverseetl
  template:
    metadata:
      labels:
        app: pyreverseetl
    spec:
      containers:
      - name: pyreverseetl
        image: pyreverseetl:v2.0.1
        ports:
        - containerPort: 8080
          name: api
        - containerPort: 9090
          name: metrics
        env:
        - name: PYREVERSEETL_API_HOST
          value: "0.0.0.0"
        - name: PYREVERSEETL_API_PORT
          value: "8080"
        volumeMounts:
        - name: configs
          mountPath: /etc/pyreverseetl
        - name: state
          mountPath: /var/lib/pyreverseetl
      volumes:
      - name: configs
        configMap:
          name: pyreverseetl-syncs
      - name: state
        persistentVolumeClaim:
          claimName: pyreverseetl-state
```

### CronJob for Scheduled Syncs
```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: customer-sync
spec:
  schedule: "0 * * * *"  # Every hour
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: pyreverseetl
            image: pyreverseetl:v2.0.1
            command: ["pyreverseetl", "sync", "run", "customer_sync.yaml"]
          restartPolicy: OnFailure
```

---

## Event-Driven Triggers

### Webhook Receiver
```bash
# Start webhook server
pyreverseetl api --webhook-port=9000

# Trigger from external system
curl -X POST http://localhost:9000/webhooks/sync_trigger \
  -H "Content-Type: application/json" \
  -d '{
    "sync_id": "customer_sync",
    "event": "data_available",
    "timestamp": "2024-01-15T10:30:00Z"
  }'
```

### Message Queue Integration

Consume from Kafka, SQS, or Pub/Sub:

```yaml
triggers:
  - name: s3_file_upload
    type: s3
    bucket: data-input
    prefix: customer_updates/
    on_file_uploaded:
      run_sync: customer_sync
      
  - name: kafka_events
    type: kafka
    topic: sync_requests
    on_message:
      sync_id_field: sync_name
      parameters_field: params
```

---

## Monitoring & Alerts

### OpenTelemetry Metrics
```python
from pyreverseetl import MetricsCollector

metrics = MetricsCollector.get_instance()

# Track sync performance
print(f"Success rate: {metrics.sync_success_rate}%")
print(f"Avg duration: {metrics.avg_sync_duration_sec}s")
print(f"Error rate: {metrics.error_rate}%")
```

### Health Check Endpoint
```bash
# Health check
curl http://localhost:8080/health

# Returns 200 if healthy
```

### Prometheus Metrics
```bash
# Prometheus-formatted metrics
curl http://localhost:8080/metrics

# Example output:
# pyreverseetl_syncs_total 42
# pyreverseetl_syncs_success_total 40
# pyreverseetl_syncs_failed_total 2
# pyreverseetl_sync_duration_seconds_bucket
```

---

## Scripting Examples

### Batch Sync with Error Handling
```bash
#!/bin/bash

set -e

SYNCS=("customer_sync" "product_sync" "order_sync")
FAILED=()

for sync in "${SYNCS[@]}"; do
    echo "Running $sync..."
    if ! pyreverseetl sync run "$sync.yaml"; then
        FAILED+=("$sync")
        echo "❌ $sync failed"
    else
        echo "✅ $sync succeeded"
    fi
done

if [ ${#FAILED[@]} -gt 0 ]; then
    echo "Failed syncs: ${FAILED[@]}"
    exit 1
fi
```

### Sync with Data Quality Checks
```bash
#!/bin/bash

# Run sync
echo "Running sync..."
RESULT=$(pyreverseetl sync run customer_sync.yaml --output-format=json)

# Extract metrics
ROWS_READ=$(echo $RESULT | jq '.rows_read')
ROWS_WRITTEN=$(echo $RESULT | jq '.rows_written')
ERROR_COUNT=$(echo $RESULT | jq '.errors')

# Quality checks
ERROR_RATE=$(echo "scale=2; $ERROR_COUNT / $ROWS_READ * 100" | bc)
if (( $(echo "$ERROR_RATE > 1" | bc -l) )); then
    echo "❌ Error rate $ERROR_RATE% exceeds threshold"
    exit 1
fi

if [ "$ROWS_WRITTEN" -lt 100 ]; then
    echo "❌ Too few rows written: $ROWS_WRITTEN"
    exit 1
fi

echo "✅ Quality checks passed"
```

---

## Architecture

PyReverseETL provides:

- **Rust core** — High-performance sync engine
- **Python bindings** — Easy integration with Python tools
- **REST API** — Language-agnostic HTTP interface
- **Bash CLI** — Quick command-line access
- **OpenTelemetry** — Production monitoring

All powered by a single deployment.

---

## Summary

PyReverseETL works with your existing tools:

- **Airflow** — Use Python operators or HTTP operators
- **Kubernetes** — StatefulSets and CronJobs
- **Bash scripts** — Direct CLI commands
- **Custom apps** — REST API integration
- **Message queues** — Event-driven triggers

One platform. All your orchestration needs.

---

**Next:** [Quality & Testing](QUALITY_AND_TESTING.md) | [Connector Ecosystem](CONNECTOR_ECOSYSTEM.md)
