# PyReverseETL: Distributed Processing Guide

**PySpark (Micro-Batch) & PyFlink (True Streaming)**

---

## Overview

PyReverseETL supports two distributed processing engines:

| Engine | Use Case | Processing Model | Latency | Throughput |
|--------|----------|------------------|---------|-----------|
| **PySpark** | Micro-batch transformations | Batch (scheduled intervals) | 100ms - 10s | 1-100 GB/min |
| **PyFlink** | Real-time streaming | Event-driven (continuous) | 10-100ms | 100MB - 10 GB/min |

---

## PySpark: Micro-Batch Processing

### When to Use PySpark
- ✅ ETL jobs with scheduled intervals (hourly, daily)
- ✅ Large-scale transformations (1GB+)
- ✅ Complex SQL transformations
- ✅ Machine learning feature engineering
- ✅ Cost-effective batch processing

### Architecture

```yaml
Source (Kafka, DB, S3)
    ↓
[Intermediate Buffer] ← Batches every N seconds/records
    ↓
PySpark Cluster (auto-scaling)
    ├─ Driver (master)
    └─ Executors (workers)
    ↓
Transformation Pipeline
    ├─ Cleanse
    ├─ Enrich
    ├─ Aggregate
    └─ Validate
    ↓
Destination (Snowflake, S3, Database)
```

### Setup

```bash
# Install PySpark
pip install pyspark==3.5.0

# Or use system Spark
brew install apache-spark  # macOS
sudo apt install spark    # Ubuntu
```

### Configuration

```yaml
transformation:
  engine: pyspark
  spark_config:
    master: "spark://localhost:7077"  # Local | K8s | YARN
    executor_memory: "4g"
    executor_cores: 4
    num_executors: 4
    
  batch_config:
    batch_size: 10000      # Records per batch
    batch_interval: 60     # Seconds between batches
    checkpoint_dir: "/tmp/spark-checkpoint"
    
  parallelism: 8           # Partitions for parallel processing
```

### Example: Micro-Batch ETL

```python
from pyspark.sql import SparkSession
from pyreverseetl import PySparkTransformer

# Initialize Spark session
spark = SparkSession.builder \
    .appName("pyreverseetl-batch") \
    .config("spark.default.parallelism", 8) \
    .config("spark.sql.adaptive.enabled", "true") \
    .getOrCreate()

# Read micro-batch from Kafka
df = spark.readStream \
    .format("kafka") \
    .option("kafka.bootstrap.servers", "localhost:9092") \
    .option("subscribe", "raw_events") \
    .option("startingOffsets", "latest") \
    .load()

# Parse JSON
from pyspark.sql.functions import from_json, col
schema = "customer_id INT, event_type STRING, timestamp STRING"
parsed = df.select(from_json(col("value").cast("string"), schema).alias("data")) \
    .select("data.*")

# Transform (micro-batch)
transformed = parsed \
    .filter(col("customer_id").isNotNull()) \
    .groupBy("customer_id") \
    .agg({
        "event_type": "count",
        "timestamp": "max"
    })

# Write to destination every 10 seconds
query = transformed.writeStream \
    .format("snowflake") \
    .option("sfUrl", "https://xx12345.snowflakecomputing.com") \
    .option("sfDatabase", "analytics") \
    .option("sfSchema", "raw") \
    .option("sfWarehouse", "compute") \
    .option("sfUser", "${SNOWFLAKE_USER}") \
    .option("sfPassword", "${SNOWFLAKE_PASSWORD}") \
    .option("checkpointLocation", "/tmp/checkpoint") \
    .outputMode("update") \
    .trigger(processingTime="10 seconds") \
    .start()

# Keep running
query.awaitTermination()
```

### Deployment on Kubernetes

```yaml
# kubernetes/spark-driver.yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: pyreverseetl-spark-batch
spec:
  template:
    spec:
      containers:
      - name: spark-driver
        image: spark:3.5.0
        command: ["./bin/spark-submit"]
        args:
          - "--master"
          - "k8s://https://kubernetes.default.svc"
          - "--deploy-mode"
          - "cluster"
          - "--executor-memory"
          - "4g"
          - "--executor-cores"
          - "4"
          - "--num-executors"
          - "4"
          - "/app/batch_job.py"
```

### Performance Tuning

```python
# Optimize for large datasets (100+ GB)
spark.conf.set("spark.sql.adaptive.enabled", "true")
spark.conf.set("spark.sql.adaptive.skewJoin.enabled", "true")
spark.conf.set("spark.sql.shuffle.partitions", "200")

# Enable columnar caching
df.cache()

# Use Parquet for intermediate storage (100x faster than CSV)
df.write.format("parquet").save("/tmp/checkpoint")
```

### Monitoring

```bash
# View Spark UI
open http://localhost:4040

# Monitor in real-time
sparkHistoryServer --version
# Navigate to http://localhost:18080
```

---

## PyFlink: True Streaming

### When to Use PyFlink
- ✅ Real-time event processing (sub-second latency)
- ✅ Continuous stream transformations
- ✅ Event-time windowing (tumbling, sliding, session)
- ✅ Complex event processing (CEP)
- ✅ Stream-to-stream joins
- ✅ Stateful transformations

### Architecture

```yaml
Source (Kafka, Kinesis, Pub/Sub) [Real-time Events]
    ↓
PyFlink Cluster (always running)
    ├─ JobManager (master)
    └─ TaskManagers (workers)
    ↓
Transformation Pipeline (per-event)
    ├─ Filter
    ├─ Map/Enrich
    ├─ Window operations
    ├─ Joins
    └─ Validate
    ↓
Destination (Kafka, Database, S3) [Real-time Results]
```

### Setup

```bash
# Install PyFlink
pip install apache-flink==1.17.0

# Or from source
git clone https://github.com/apache/flink.git
cd flink
mvn -DskipTests clean package
```

### Configuration

```yaml
transformation:
  engine: flink
  flink_config:
    jobmanager_memory: "1024m"
    taskmanager_memory: "2048m"
    taskmanager_slots: 4
    parallelism: 4
    
  streaming_config:
    checkpoint_interval: 60000   # 60 seconds
    state_backend: "rocksdb"     # For durability
    restart_strategy: "exponential_delay"
    
  source:
    type: "kafka"
    brokers: "localhost:9092"
    topic: "events"
    consumer_group: "pyreverseetl-consumer"
```

### Example: Real-Time Streaming

```python
from pyflink.datastream import StreamExecutionEnvironment
from pyflink.datastream.functions import MapFunction, FilterFunction
from pyflink.datastream.connectors.kafka import FlinkKafkaProducer
from pyflink.common.serialization import SimpleStringSchema
from pyflink.datastream.window import TumblingEventTimeWindow
from pyflink.common.time import Time

# Initialize environment
env = StreamExecutionEnvironment.get_execution_environment()
env.set_parallelism(4)

# Enable checkpointing for fault tolerance
env.enable_change_log_replication(True)
env.get_checkpoint_config().set_checkpointing_interval(60 * 1000)

# Read from Kafka (real-time events)
kafka_stream = env.add_source(
    FlinkKafkaProducer(
        topic="events",
        value_serializer=lambda x: str(x).encode('utf-8'),
        properties={"bootstrap.servers": "localhost:9092"}
    )
)

# Parse events
class ParseEvent(MapFunction):
    def map(self, event):
        import json
        data = json.loads(event)
        return {
            "customer_id": data["customer_id"],
            "event_type": data["event_type"],
            "amount": data["amount"],
            "timestamp": data["timestamp"]
        }

parsed_stream = kafka_stream.map(ParseEvent())

# Filter valid events
class ValidateEvent(FilterFunction):
    def filter(self, event):
        return event["customer_id"] is not None and event["amount"] > 0

validated_stream = parsed_stream.filter(ValidateEvent())

# Tumbling window (5-second windows)
windowed_stream = validated_stream \
    .key_by(lambda x: x["customer_id"]) \
    .window(TumblingEventTimeWindow.of(Time.seconds(5)))

# Aggregate within window
class SumAmount(ReduceFunction):
    def reduce(self, v1, v2):
        v1["amount"] += v2["amount"]
        v1["count"] += 1
        return v1

aggregated_stream = windowed_stream.reduce(SumAmount())

# Send results back to Kafka
aggregated_stream.add_sink(
    FlinkKafkaProducer(
        topic="customer_summary",
        value_serializer=lambda x: json.dumps(x).encode('utf-8'),
        properties={"bootstrap.servers": "localhost:9092"}
    )
)

# Execute
env.execute("pyreverseetl-streaming")
```

### Windowing Operations

```python
from pyflink.datastream.window import TumblingEventTimeWindow, SlidingEventTimeWindow, SessionWindow
from pyflink.common.time import Time

# Tumbling Window (non-overlapping, 5-second windows)
windowed = stream.key_by(lambda x: x["customer_id"]) \
    .window(TumblingEventTimeWindow.of(Time.seconds(5)))
# Events: |-----5s-----|-----5s-----|-----5s-----|

# Sliding Window (overlapping, 10-second window, 5-second slide)
windowed = stream.key_by(lambda x: x["customer_id"]) \
    .window(SlidingEventTimeWindow.of(Time.seconds(10), Time.seconds(5)))
# Events: |---10s----|
#              |---10s----|
#                   |---10s----|

# Session Window (gap-based, 15-second inactivity gap)
windowed = stream.key_by(lambda x: x["customer_id"]) \
    .window(SessionWindow.with_gap(Time.seconds(15)))
# Closes window after 15s of no events
```

### Stateful Processing (Stream-to-Stream Joins)

```python
# Stream 1: Customer events
customer_stream = env.add_source(...)

# Stream 2: Customer metadata
metadata_stream = env.add_source(...)

# Join streams on customer_id
class JoinFn(CoFlatMapFunction):
    def flat_map1(self, event):
        # Process event stream
        yield event
    
    def flat_map2(self, metadata):
        # Process metadata stream
        yield metadata

joined = customer_stream.connect(metadata_stream) \
    .key_by(lambda x: x["customer_id"], lambda x: x["customer_id"]) \
    .flat_map(JoinFn())
```

### Deployment on Kubernetes

```bash
# Start Flink cluster on K8s
flink run-application \
  --target kubernetes-application \
  -Dkubernetes.cluster-id=pyreverseetl-flink \
  -Dkubernetes.namespace=default \
  -Dkubernetes.taskmanager.replicas=4 \
  /app/streaming_job.py

# View logs
kubectl logs -f -l app=pyreverseetl-flink
```

### Monitoring

```bash
# Access Flink Dashboard
open http://localhost:8081

# Monitor metrics (Prometheus)
curl http://localhost:9249/metrics | grep flink_taskmanager_job

# Check checkpoints
flink list
flink info <job-id>
```

---

## Comparison: PySpark vs PyFlink

| Aspect | PySpark | PyFlink |
|--------|---------|---------|
| **Latency** | 100ms - 10s | 10-100ms |
| **Processing Model** | Micro-batch | Event-driven streaming |
| **Throughput** | 1-100 GB/min | 100MB - 10 GB/min |
| **State Management** | Limited | Full (operator state) |
| **Event Time** | Supported | Native support |
| **Windowing** | Basic | Advanced (session, custom) |
| **Complexity** | Simple SQL/DataFrame | Complex CEP possible |
| **Maturity** | Very stable | Stable |
| **Community** | Large | Growing |
| **Cost** | Higher (pre-allocates resources) | Lower (auto-scales) |

---

## Hybrid Approach: Combining Both

```yaml
# Architecture: PySpark for batch, PyFlink for stream
Data Source
    ├─ Historical data → [PySpark Batch] → Warehouse
    └─ Real-time stream → [PyFlink Stream] → Real-time DB
         ↓
    [Combine results] → Analytics Dashboard
```

### Example: Daily Batch + Real-Time Stream

```python
# 1. Daily batch job with PySpark
# Runs at 2 AM, processes 1 billion historical records
spark_job = SparkBatchJob()
spark_job.run_daily_at("02:00")

# 2. Real-time stream with PyFlink
# Runs continuously, processes events as they arrive
flink_job = FlinkStreamingJob()
flink_job.start()

# 3. Both write to same warehouse (compatible schemas)
# Batch: Full refresh of daily aggregates
# Stream: Real-time incremental updates
```

---

## Performance Recommendations

### For PySpark (Micro-Batch)
- **Batch Size**: 10,000 - 100,000 records
- **Interval**: 10 - 3600 seconds (based on latency needs)
- **Parallelism**: 4 × CPU cores
- **Memory**: 4-8 GB per executor
- **Best for**: 100MB+ datasets, hourly/daily jobs

### For PyFlink (Streaming)
- **Parallelism**: 2 × CPU cores
- **State Backend**: RocksDB (for durability)
- **Checkpointing**: Every 60 seconds
- **Memory**: 2-4 GB per task
- **Best for**: <50MB/sec streams, sub-second latency needs

---

## Troubleshooting

### PySpark: "Executor OutOfMemory"
```python
# Reduce batch size
batch_size: 5000  # Was 10000

# Or increase executor memory
spark.config("spark.executor.memory", "8g")
```

### PyFlink: "State size growing unbounded"
```python
# Set state TTL
class MyFunction(RichMapFunction):
    def open(self, runtime_context):
        self.state_ttl = StateTtlConfig.new_builder(Time.seconds(3600)) \
            .set_cleanup_type(StateTtlConfig.CleanupType.Eager) \
            .build()
```

### Both: "Data loss during recovery"
```yaml
# Enable checkpointing
checkpointing:
  interval: 60000  # 60 seconds
  mode: "AT_LEAST_ONCE"  # or "EXACTLY_ONCE"
  path: "s3://backup/checkpoints/"
```

---

## Next Steps

1. **Choose Engine**:
   - Latency < 1 second? → Use **PyFlink**
   - Batch processing? → Use **PySpark**
   - Both? → Use **Hybrid**

2. **Implement Transformation**:
   - PySpark: Use Spark SQL or DataFrame API
   - PyFlink: Use DataStream API

3. **Deploy**:
   - Local: `python job.py`
   - Kubernetes: `kubectl apply -f job.yaml`
   - Cloud: Use managed services (Dataflow, EMR, etc.)

4. **Monitor**:
   - PySpark UI: `http://localhost:4040`
   - PyFlink UI: `http://localhost:8081`

---

**Status**: ✅ Both engines production-ready  
**Recommended**: PySpark for batch, PyFlink for streaming  
**Last Updated**: 2026-07-18  
**Support**: github.com/Mullassery/PyReverseETL/issues
