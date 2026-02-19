# Output Format Specification

This document specifies runtime output for all `tzbucket` commands.

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `2` | Input/policy error |
| `3` | Runtime error |

## Common Conventions

- UTC timestamps use RFC3339 with `Z` suffix.
- Local timestamps include offset (for example `+01:00`).
- `bucket` emits **NDJSON** (one JSON object per line) in JSON mode.
- `range` emits one JSON array in JSON mode.
- `explain` emits one JSON object in JSON mode.
- On errors in JSON mode, error JSON is emitted to **stderr**.

## Bucket Key Formats

| Interval | Key Format | Example |
|----------|------------|---------|
| `day` | `YYYY-MM-DD` | `2026-03-29` |
| `week` | `YYYY-MM-DD` | `2026-03-23` |
| `month` | `YYYY-MM` | `2026-03` |

Week keys use the week start date (`--week-start monday|sunday`).

## `bucket` Command

### Success Output (JSON mode)

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

### Field Reference

| Field | Type | Description |
|-------|------|-------------|
| `input.ts` | string | Original timestamp text |
| `input.epoch_ms` | integer | Parsed Unix epoch milliseconds |
| `tz` | string | IANA timezone |
| `interval` | string | `day`, `week`, or `month` |
| `bucket.key` | string | Bucket key |
| `bucket.start_local` | string | Local bucket start with offset |
| `bucket.end_local` | string | Local bucket end with offset |
| `bucket.start_utc` | string | UTC bucket start |
| `bucket.end_utc` | string | UTC bucket end |

## `range` Command

### Range Semantics

`range` uses half-open interval semantics: `[start, end)`.

A bucket is included if and only if it intersects the requested range:

- `bucket.start_utc < end`
- `bucket.end_utc > start`

`start` must be strictly earlier than `end`.

### Success Output (JSON mode)

```json
[
  {
    "key": "2026-03-27",
    "start_local": "2026-03-27T00:00:00+01:00",
    "end_local": "2026-03-28T00:00:00+01:00",
    "start_utc": "2026-03-26T23:00:00Z",
    "end_utc": "2026-03-27T23:00:00Z"
  }
]
```

## `explain` Command

### Input Policies

- `--policy-nonexistent`: `error` or `shift_forward`
- `--policy-ambiguous`: `error`, `first`, or `second`

### Success Output (normal time)

```json
{
  "local_time": "2026-03-15T14:30:00",
  "tz": "Europe/Berlin",
  "status": "normal"
}
```

### Success Output (nonexistent/ambiguous with policy)

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

### Field Reference

| Field | Type | Description |
|-------|------|-------------|
| `local_time` | string | Input local time |
| `tz` | string | IANA timezone |
| `status` | string | `normal`, `nonexistent`, or `ambiguous` |
| `resolution.policy` | string | Policy used for resolved DST case |
| `resolution.result` | string | Resolved local time with offset |

## Error Output (JSON mode)

Errors are emitted to `stderr` as JSON:

```json
{
  "error": "Error message describing the problem",
  "exit_code": 2,
  "status": "nonexistent"
}
```

`status` is present for DST policy errors (`nonexistent`, `ambiguous`) and omitted otherwise.

### Common Errors

| Error Type | Exit Code |
|------------|-----------|
| Invalid timezone/format/timestamp/arguments | `2` |
| Nonexistent/ambiguous local time with `error` policy | `2` |
| I/O and internal runtime failures | `3` |
