# ETL Integration Examples

This guide shows practical ways to use `tzbucket` in production ETL pipelines.

## Key Idea

`tzbucket` is a binary, but in ETL that is normal: treat it as a deterministic preprocessing step.

The most scalable pattern is:

1. Generate bucket boundaries once with `tzbucket range`
2. Store boundaries as a small dimension table
3. Join fact events to buckets using half-open interval logic

This keeps heavy work in your engine (Spark/warehouse) and keeps DST logic centralized.

## Pattern 1: Spark

### Step 1: Generate bucket dimension

```bash
./tzbucket range \
  --tz Europe/Berlin \
  --interval day \
  --start 2026-01-01T00:00:00Z \
  --end 2027-01-01T00:00:00Z \
  --output-format json \
  > /data/dim/time_buckets_berlin_day_2026.json
```

### Step 2: Load and join in PySpark

```python
from pyspark.sql import functions as F

facts = spark.read.parquet("s3://my-bucket/events/")
# expected facts schema includes: event_ts_utc (timestamp), event_id, ...

buckets = (
    spark.read.json("s3://my-bucket/dim/time_buckets_berlin_day_2026.json")
    .withColumn("start_utc", F.to_timestamp("start_utc"))
    .withColumn("end_utc", F.to_timestamp("end_utc"))
    .select("key", "start_utc", "end_utc")
)

joined = (
    facts.alias("f")
    .join(
        buckets.alias("b"),
        (F.col("f.event_ts_utc") >= F.col("b.start_utc"))
        & (F.col("f.event_ts_utc") < F.col("b.end_utc")),
        "left"
    )
)

daily = (
    joined.groupBy("b.key")
    .agg(F.count("*").alias("event_count"))
)
```

### Step 3: Operationalize

- Rebuild dimension daily/weekly for future horizon.
- Version by timezone+interval (for example `time_buckets_berlin_day_v1`).
- Broadcast the bucket table in Spark if it is small.

## Pattern 2: Airflow + Spark/SQL

Use one task to generate/update buckets and downstream tasks to consume.

```python
from airflow import DAG
from airflow.operators.bash import BashOperator
from datetime import datetime

with DAG("tzbucket_daily", start_date=datetime(2026, 1, 1), schedule="@daily", catchup=False) as dag:
    generate_buckets = BashOperator(
        task_id="generate_buckets",
        bash_command=(
            "tzbucket range --tz Europe/Berlin --interval day "
            "--start {{ ds }}T00:00:00Z --end {{ next_ds }}T00:00:00Z "
            "--output-format json > /tmp/buckets_{{ ds }}.json"
        ),
    )

    # next tasks: upload to object store, load temp table, run Spark/dbt job
```

Recommended: generate wider windows (for example full month) to avoid day-by-day churn.

## Pattern 3: dbt / Warehouse-first

Precompute buckets and load into a warehouse dimension table (for example `dim_time_bucket`).

Join pattern in SQL:

```sql
select
  b.key as berlin_day,
  count(*) as event_count
from fact_events f
join dim_time_bucket b
  on f.event_ts_utc >= b.start_utc
 and f.event_ts_utc <  b.end_utc
where b.tz = 'Europe/Berlin'
  and b.interval = 'day'
group by 1
order by 1;
```

This is engine-agnostic and preserves DST correctness.

## Pattern 4: Kubernetes Batch Job

Run `tzbucket` as a scheduled job that refreshes bucket dimensions.

- Build a small image containing the `tzbucket` binary.
- CronJob writes JSON to object storage.
- Spark/warehouse jobs read that artifact.

This keeps runtime dependencies simple and reproducible.

## When to use `bucket` directly

`bucket` is useful for smaller streams or preprocessing files line-by-line:

```bash
cat events_rfc3339.txt | tzbucket bucket --tz Europe/Berlin --interval day --format rfc3339 --output-format json
```

For large distributed datasets, prefer the dimension-join pattern.

## Binary Concern: Is This "Difficult"?

In practice, no. ETL platforms routinely use external binaries for deterministic transforms.

What matters is:

- stable output contract
- deterministic behavior
- explicit failure modes

`tzbucket` gives those. Treat it as a boundary generator, not a row-by-row Spark UDF replacement.

## Production Checklist

- Pin `tzbucket` version in image/deployment.
- Store generated bucket dimensions with partitioning and metadata (`tz`, `interval`, `generated_at`).
- Monitor error exit codes (`2` input/policy, `3` runtime).
- Add a regression test around known DST dates relevant to your business regions.
