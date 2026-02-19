# tzbucket

> DST-safe time bucketing for analytics and ETL pipelines.

[![CI](https://github.com/TorstenCScholz/tzbucket/actions/workflows/ci.yml/badge.svg)](https://github.com/TorstenCScholz/tzbucket/actions/workflows/ci.yml)

## What is tzbucket?

`tzbucket` is a CLI tool that assigns UTC timestamps to calendar buckets (`day`, `week`, `month`) in a specific IANA timezone. It handles DST transitions correctly, so your analytics and ETL boundaries stay accurate.

## Why this matters

Calendar grouping is common in data pipelines, but DST creates edge cases:

- **23-hour days** on spring-forward transitions
- **25-hour days** on fall-back transitions
- **Nonexistent local times** (skipped hour)
- **Ambiguous local times** (repeated hour)

Naive local grouping often gets these boundaries wrong. `tzbucket` computes them explicitly.

## Key Features

- IANA timezone support (via `chrono-tz`)
- DST-aware day/week/month bucketing
- Deterministic output
- JSON and text output modes
- Three subcommands: `bucket`, `range`, `explain`

## Install

```bash
cargo install --path crates/tzbucket-cli
```

Or download a pre-built binary from [Releases](https://github.com/TorstenCScholz/tzbucket/releases).

## Quickstart

### Bucket timestamps

```bash
# From a file
tzbucket bucket --tz Europe/Berlin --interval day --format rfc3339 --input timestamps.txt

# From stdin
echo "2026-03-29T00:15:00Z" | tzbucket bucket --tz Europe/Berlin --format rfc3339

# JSON output for pipelines
tzbucket bucket --tz America/New_York --interval day --format rfc3339 --input events.txt --output-format json
```

### Generate bucket ranges

```bash
tzbucket range --tz Europe/Berlin --interval day --start 2026-03-01T00:00:00Z --end 2026-04-01T00:00:00Z --output-format json
```

### Explain local times

```bash
# Check DST status for a local time
tzbucket explain --tz Europe/Berlin --local 2026-03-29T02:30:00 --output-format json

# Resolve nonexistent times (spring forward)
tzbucket explain --tz Europe/Berlin --local 2026-03-29T02:30:00 --policy-nonexistent shift_forward --output-format json

# Resolve ambiguous times (fall back)
tzbucket explain --tz Europe/Berlin --local 2026-10-25T02:30:00 --policy-ambiguous first --output-format json
```

## Output Contract

### Bucket keys

| Interval | Key Format | Example |
|----------|------------|---------|
| `day` | `YYYY-MM-DD` | `2026-03-29` |
| `week` | `YYYY-MM-DD` | `2026-03-23` |
| `month` | `YYYY-MM` | `2026-03` |

Week keys use the week start date (not ISO week number).

### `bucket` output (NDJSON)

```json
{
  "input": {
    "ts": "2026-03-29T00:30:00Z",
    "epoch_ms": 1774744200000
  },
  "tz": "Europe/Berlin",
  "interval": "day",
  "bucket": {
    "key": "2026-03-29",
    "start_local": "2026-03-29T00:00:00+01:00",
    "end_local": "2026-03-30T00:00:00+02:00",
    "start_utc": "2026-03-28T23:00:00Z",
    "end_utc": "2026-03-29T22:00:00Z"
  }
}
```

### `range` output (JSON array)

```json
[
  {
    "key": "2026-03-29",
    "start_local": "2026-03-29T00:00:00+01:00",
    "end_local": "2026-03-30T00:00:00+02:00",
    "start_utc": "2026-03-28T23:00:00Z",
    "end_utc": "2026-03-29T22:00:00Z"
  }
]
```

### `explain` output examples

Normal local time:

```json
{
  "local_time": "2026-03-15T14:30:00",
  "tz": "Europe/Berlin",
  "status": "normal"
}
```

Nonexistent local time with policy:

```json
{
  "local_time": "2026-03-29T02:30:00",
  "tz": "Europe/Berlin",
  "status": "nonexistent",
  "resolution": {
    "policy": "shift_forward",
    "result": "2026-03-29T03:30:00+02:00"
  }
}
```

Ambiguous local time with policy:

```json
{
  "local_time": "2026-10-25T02:30:00",
  "tz": "Europe/Berlin",
  "status": "ambiguous",
  "resolution": {
    "policy": "first",
    "result": "2026-10-25T02:30:00+02:00"
  }
}
```

Error in JSON mode (`--output-format json`):

```json
{
  "error": "Nonexistent time '2026-03-29T02:30:00' in timezone 'Europe/Berlin'. Skipped due to DST spring forward. Use --policy-nonexistent=shift_forward to resolve.",
  "exit_code": 2,
  "status": "nonexistent"
}
```

Note: JSON errors are emitted to **stderr**.

## DST Notes

For detailed DST examples and behavior notes, see [`docs/dst_notes.md`](docs/dst_notes.md).

## ETL Integration

For practical, real-world pipeline patterns (Spark, Airflow, dbt/warehouse, Kubernetes batch), see [`docs/etl_integration.md`](docs/etl_integration.md).

## Practical Use Cases

- Spark timezone day grouping DST errors: [`docs/spark_dst_bucketing.md`](docs/spark_dst_bucketing.md)
- SQL `date_trunc` timezone DST pitfalls: [`docs/sql_date_trunc_dst_pitfalls.md`](docs/sql_date_trunc_dst_pitfalls.md)
- ETL DST readiness checklist: [`docs/dst_etl_checklist.md`](docs/dst_etl_checklist.md)
- Migration guide from naive grouping: [`docs/migrate_from_naive_grouping.md`](docs/migrate_from_naive_grouping.md)

## Development

```bash
# Run tests
cargo test --all

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt --all
```

## Documentation

- [`docs/dst_notes.md`](docs/dst_notes.md) - DST transition behavior
- [`docs/output_format.md`](docs/output_format.md) - Output and error contract
- [`docs/architecture.md`](docs/architecture.md) - Project architecture
- [`docs/etl_integration.md`](docs/etl_integration.md) - Integration patterns for ETL pipelines
- [`docs/spark_dst_bucketing.md`](docs/spark_dst_bucketing.md) - Spark DST grouping fixes
- [`docs/sql_date_trunc_dst_pitfalls.md`](docs/sql_date_trunc_dst_pitfalls.md) - SQL timezone truncation pitfalls
- [`docs/dst_etl_checklist.md`](docs/dst_etl_checklist.md) - Production DST checklist
- [`docs/migrate_from_naive_grouping.md`](docs/migrate_from_naive_grouping.md) - Migration playbook

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
