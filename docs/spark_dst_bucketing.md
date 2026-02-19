# Fixing DST Errors in Spark Daily Aggregations

If your Spark daily aggregates are wrong around daylight saving transitions, this page is for you.

Typical symptoms:

- Daily counts drift on DST boundary dates.
- Local "day" buckets contain unexpected event windows.
- March/October (or March/November) reports differ from business expectations.

## Why It Happens

A local calendar day is not always 24 hours:

- spring forward => 23-hour day
- fall back => 25-hour day

Naive grouping with local conversions can produce wrong boundaries.

## Reliable Pattern for Spark

1. Generate canonical bucket boundaries with `tzbucket range`.
2. Load the bucket table in Spark.
3. Join events with half-open interval logic.

```bash
tzbucket range \
  --tz Europe/Berlin \
  --interval day \
  --start 2026-01-01T00:00:00Z \
  --end 2027-01-01T00:00:00Z \
  --output-format json \
  > /data/time_buckets_berlin_day_2026.json
```

```python
from pyspark.sql import functions as F

facts = spark.read.parquet("s3://analytics/events/")
buckets = (
    spark.read.json("s3://analytics/dim/time_buckets_berlin_day_2026.json")
    .withColumn("start_utc", F.to_timestamp("start_utc"))
    .withColumn("end_utc", F.to_timestamp("end_utc"))
)

joined = facts.join(
    buckets,
    (facts.event_ts_utc >= buckets.start_utc) & (facts.event_ts_utc < buckets.end_utc),
    "left",
)

daily = joined.groupBy("key").count()
```

## Why This Works

- Boundaries are computed once with explicit timezone rules.
- DST transitions are encoded in `start_utc`/`end_utc`.
- Spark does scalable joins/aggregations without custom DST logic.

## Related Docs

- `docs/etl_integration.md`
- `docs/output_format.md`
- `docs/dst_notes.md`
