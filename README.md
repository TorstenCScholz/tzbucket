# tzbucket

> DST-safe time bucketing for analytics and ETL pipelines.

[![CI](https://github.com/TorstenCScholz/tzbucket.git/actions/workflows/ci.yml/badge.svg)](https://github.com/TorstenCScholz/tzbucket.git/actions/workflows/ci.yml)

## What is tzbucket?

`tzbucket` is a CLI tool that assigns UTC timestamps to time buckets (day, hour, etc.) in a specific timezone. Unlike simple date grouping, tzbucket correctly handles Daylight Saving Time (DST) transitions, ensuring your analytics and ETL pipelines produce accurate results.

## Why does this matter?

Calendar-based bucketing is common in data pipelines—grouping events by day, calculating daily aggregates, or generating reports. But DST makes this tricky:

- **23-hour days**: When clocks spring forward, a "day" has only 23 hours
- **25-hour days**: When clocks fall back, a "day" has 25 hours
- **Nonexistent times**: 02:30 might not exist on DST transition days
- **Ambiguous times**: 02:30 might occur twice on fall-back days

Simple approaches like `date_trunc('day', ts AT TIME ZONE 'Europe/Berlin')` produce wrong bucket boundaries during DST transitions. tzbucket gets this right.

## Key Features

- **IANA timezone support**: Use any timezone like `Europe/Berlin`, `America/New_York`, `Asia/Tokyo`
- **DST-aware bucketing**: Correctly handles 23-hour and 25-hour days
- **Deterministic output**: Same input always produces same output
- **Multiple output formats**: Human-readable or JSON for pipeline integration
- **Three commands**: `bucket` for timestamps, `range` for bucket sequences, `explain` for DST debugging

## Install

```bash
cargo install --path crates/tzbucket-cli
```

Or download a pre-built binary from [Releases](https://github.com/TorstenCScholz/tzbucket.git/releases).

## Quickstart

### Bucket timestamps

Assign timestamps to day buckets in a timezone:

```bash
# From a file
tzbucket bucket --tz Europe/Berlin --interval day --format rfc3339 --input timestamps.txt

# From stdin
echo "2026-03-29T00:15:00Z" | tzbucket bucket --tz Europe/Berlin --format rfc3339

# JSON output for pipelines
tzbucket bucket --tz America/New_York --interval hour --input events.txt --output-format json
```

### Generate bucket ranges

Generate all buckets in a time range:

```bash
tzbucket range --tz Europe/Berlin --interval day --start 2026-03-01T00:00:00Z --end 2026-04-01T00:00:00Z
```

### Explain local times

Debug DST issues with local times:

```bash
# Check if a local time exists
tzbucket explain --tz Europe/Berlin --local 2026-03-29T02:30:00

# Resolve nonexistent times (spring forward)
tzbucket explain --tz Europe/Berlin --local 2026-03-29T02:30:00 --policy-nonexistent shift_forward

# Resolve ambiguous times (fall back)
tzbucket explain --tz Europe/Berlin --local 2026-10-25T02:30:00 --policy-ambiguous first
```

## Output Contract

### Bucket Key Formats

Each bucket is identified by a unique key:

| Interval | Key Format | Example | Description |
|----------|------------|---------|-------------|
| `day` | `YYYY-MM-DD` | `2026-03-29` | Calendar date in local timezone |
| `week` | `YYYY-MM-DD` | `2026-03-23` | Week starting date (Monday or Sunday per `--week-start`) |
| `month` | `YYYY-MM` | `2026-03` | Year and month in local timezone |

**Note:** Week keys use the week starting date, not ISO week numbers. This makes sorting and date range calculations straightforward.

### bucket command

Each input timestamp produces a JSON object with:

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

| Field | Type | Description |
|-------|------|-------------|
| `input.ts` | string | Original timestamp in RFC3339 format |
| `input.epoch_ms` | integer | Unix timestamp in milliseconds |
| `tz` | string | IANA timezone identifier |
| `interval` | string | Bucket interval (`day`, `week`, `month`) |
| `bucket.key` | string | Bucket identifier (see Key Formats above) |
| `bucket.start_local` | string | Bucket start in local time with offset |
| `bucket.end_local` | string | Bucket end in local time with offset |
| `bucket.start_utc` | string | Bucket start in UTC |
| `bucket.end_utc` | string | Bucket end in UTC |

### range command

Produces a JSON array of bucket objects:

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

### explain command

**Normal time** (exists and is unambiguous):

```json
{
  "local_time": "2026-03-15T14:30:00",
  "tz": "Europe/Berlin",
  "status": "normal",
  "resolution": {
    "utc_time": "2026-03-15T13:30:00Z"
  }
}
```

**Nonexistent time** (skipped during spring forward):

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

**Ambiguous time** (occurs twice during fall back):

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

**Error output** (no policy specified):

```json
{
  "error": "Nonexistent time '2026-03-29T02:30:00' in timezone 'Europe/Berlin'. Skipped due to DST spring forward. Use --policy-nonexistent=shift_forward to resolve.",
  "status": "nonexistent",
  "exit_code": 2
}
```

## DST Notes

### The Problem with Simple Grouping

Consider grouping UTC timestamps by "day in Berlin" during DST transitions:

**March 29, 2026 (Spring Forward in Berlin)**

At 02:00, clocks jump to 03:00. The day has only 23 hours:

| Bucket | Start UTC | End UTC | Duration |
|--------|-----------|---------|----------|
| 2026-03-28 | 2026-03-27T23:00:00Z | 2026-03-28T23:00:00Z | 24 hours |
| 2026-03-29 | 2026-03-28T23:00:00Z | 2026-03-29T22:00:00Z | **23 hours** |
| 2026-03-30 | 2026-03-29T22:00:00Z | 2026-03-30T22:00:00Z | 24 hours |

**October 25, 2026 (Fall Back in Berlin)**

At 03:00, clocks fall back to 02:00. The day has 25 hours:

| Bucket | Start UTC | End UTC | Duration |
|--------|-----------|---------|----------|
| 2026-10-25 | 2026-10-24T22:00:00Z | 2026-10-25T23:00:00Z | **25 hours** |

### Nonexistent Times

During spring forward, local times in the skipped hour don't exist:

- In Berlin on March 29, 2026: 02:00–02:59:59 never occurs
- `2026-03-29T02:30:00` is **nonexistent**

The `explain` command detects this. Use `--policy-nonexistent` to resolve:
- `error` (default): Return error
- `shift_forward`: Use the later time (02:30 → 03:30)
- `shift_backward`: Use the earlier time (02:30 → 01:30)

### Ambiguous Times

During fall back, local times in the repeated hour occur twice:

- In Berlin on October 25, 2026: 02:00–02:59:59 occurs twice
- `2026-10-25T02:30:00` is **ambiguous** (02:30 CEST or 02:30 CET?)

The `explain` command detects this. Use `--policy-ambiguous` to resolve:
- `error` (default): Return error
- `first`: Use the first occurrence (summer time, +02:00)
- `second`: Use the second occurrence (winter time, +01:00)

### How tzbucket Handles DST

**bucket command**: Only accepts UTC instants, so no ambiguity. Each timestamp maps to exactly one bucket.

**range command**: Generates buckets with correct UTC boundaries, including 23-hour and 25-hour days.

**explain command**: Handles local times with explicit policies for nonexistent and ambiguous cases.

For detailed DST documentation, see [`docs/dst_notes.md`](docs/dst_notes.md).

## Development

```bash
# Run tests
cargo test --all

# Update golden files after changing output
UPDATE_GOLDEN=1 cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt --all
```

## Documentation

- [`docs/dst_notes.md`](docs/dst_notes.md) - Detailed DST transition documentation
- [`docs/output_format.md`](docs/output_format.md) - Complete output format specification
- [`docs/architecture.md`](docs/architecture.md) - Internal architecture documentation

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
