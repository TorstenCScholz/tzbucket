# Output Format Specification

This document specifies the JSON output format for all tzbucket commands.

## Bucket Key Formats

Each bucket is identified by a unique key string. The format depends on the interval:

| Interval | Key Format | Example | Description |
|----------|------------|---------|-------------|
| `day` | `YYYY-MM-DD` | `2026-03-29` | Calendar date in local timezone |
| `week` | `YYYY-MM-DD` | `2026-03-23` | Week starting date (Monday or Sunday depending on `--week-start`) |
| `month` | `YYYY-MM` | `2026-03` | Year and month in local timezone |

**Week Key Format:** The week bucket key uses the week starting date, not an ISO week number. This makes it easy to:
- Sort buckets chronologically
- Determine the exact date range from the key
- Support both Monday and Sunday week starts unambiguously

Example: Week starting Monday 2026-03-23 has key `2026-03-23`, regardless of which day of the week you're bucketing.

## Common Conventions

### Timestamp Formats

All timestamps use ISO 8601 / RFC 3339 format:

| Format | Example | Description |
|--------|---------|-------------|
| UTC | `2026-03-29T00:30:00Z` | UTC time with `Z` suffix |
| With offset | `2026-03-29T00:00:00+01:00` | Local time with UTC offset |
| Local only | `2026-03-29T02:30:00` | Local time without offset (input to explain) |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (invalid input, I/O error) |
| 2 | DST resolution error (nonexistent/ambiguous time without policy) |

---

## bucket command

Assigns UTC timestamps to time buckets in a specified timezone.

### Input

- **File**: One timestamp per line (via `--input` flag)
- **Stdin**: One timestamp per line (when no `--input` specified)

Supported timestamp formats:
- RFC 3339: `2026-03-29T00:30:00Z`
- Unix epoch seconds: `1743205800`
- Unix epoch milliseconds: `1743205800000`

### Output

Each input timestamp produces one JSON object per line (newline-delimited JSON).

#### Success Output

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

#### Field Reference

| Field | Type | Description |
|-------|------|-------------|
| `input.ts` | string | Original timestamp in RFC 3339 format |
| `input.epoch_ms` | integer | Unix timestamp in milliseconds |
| `tz` | string | IANA timezone identifier (e.g., `Europe/Berlin`) |
| `interval` | string | Bucket interval: `day` or `hour` |
| `bucket.key` | string | Bucket identifier (date for day interval, datetime for hour) |
| `bucket.start_local` | string | Bucket start in local time with UTC offset |
| `bucket.end_local` | string | Bucket end in local time with UTC offset |
| `bucket.start_utc` | string | Bucket start in UTC |
| `bucket.end_utc` | string | Bucket end in UTC |

#### Error Output

```json
{
  "error": "Failed to parse timestamp: invalid",
  "exit_code": 1
}
```

### Examples

**Normal day (24 hours):**

```json
{"input":{"ts":"2026-03-28T22:30:00Z","epoch_ms":1774737000000},"tz":"Europe/Berlin","interval":"day","bucket":{"key":"2026-03-28","start_local":"2026-03-28T00:00:00+01:00","end_local":"2026-03-29T00:00:00+01:00","start_utc":"2026-03-27T23:00:00Z","end_utc":"2026-03-28T23:00:00Z"}}
```

**DST transition day (23 hours):**

```json
{"input":{"ts":"2026-03-29T00:30:00Z","epoch_ms":1774744200000},"tz":"Europe/Berlin","interval":"day","bucket":{"key":"2026-03-29","start_local":"2026-03-29T00:00:00+01:00","end_local":"2026-03-30T00:00:00+02:00","start_utc":"2026-03-28T23:00:00Z","end_utc":"2026-03-29T22:00:00Z"}}
```

Note: `end_utc` - `start_utc` = 23 hours due to spring forward.

**Fall back day (25 hours):**

```json
{"input":{"ts":"2026-10-25T00:30:00Z","epoch_ms":1792888200000},"tz":"Europe/Berlin","interval":"day","bucket":{"key":"2026-10-25","start_local":"2026-10-25T00:00:00+02:00","end_local":"2026-10-26T00:00:00+01:00","start_utc":"2026-10-24T22:00:00Z","end_utc":"2026-10-25T23:00:00Z"}}
```

Note: `end_utc` - `start_utc` = 25 hours due to fall back.

---

## range command

Generates a sequence of time buckets within a specified time range.

### Range Semantics

The range command uses **half-open interval** semantics: `[start, end)`

- **Start is inclusive**: Buckets that start at or after the start timestamp are included
- **End is exclusive**: Buckets that start at or after the end timestamp are excluded
- **Intersection**: Generates all buckets that **intersect** the `[start, end)` range

This means a bucket is included if any part of it overlaps with the range, even if the bucket starts before the range or ends after it. This is the most useful behavior for analytics and reporting use cases.

**Example:** If you request range `2026-03-27T00:00:00Z` to `2026-03-28T12:00:00Z` in `Europe/Berlin`:
- The bucket for `2026-03-27` (which runs from `2026-03-26T23:00:00Z` to `2026-03-27T23:00:00Z`) is included because it intersects the range
- The bucket for `2026-03-28` (which runs from `2026-03-27T23:00:00Z` to `2026-03-28T22:00:00Z`) is included because it intersects the range

### Input

- `--start`: Start of range (inclusive, RFC3339)
- `--end`: End of range (exclusive, RFC3339)
- `--tz`: IANA timezone identifier
- `--interval`: Bucket interval (`day`, `week`, or `month`)

### Output

Produces a JSON array of bucket objects.

#### Success Output

```json
[
  {
    "key": "2026-03-27",
    "start_local": "2026-03-27T00:00:00+01:00",
    "end_local": "2026-03-28T00:00:00+01:00",
    "start_utc": "2026-03-26T23:00:00Z",
    "end_utc": "2026-03-27T23:00:00Z"
  },
  {
    "key": "2026-03-28",
    "start_local": "2026-03-28T00:00:00+01:00",
    "end_local": "2026-03-29T00:00:00+01:00",
    "start_utc": "2026-03-27T23:00:00Z",
    "end_utc": "2026-03-28T23:00:00Z"
  },
  {
    "key": "2026-03-29",
    "start_local": "2026-03-29T00:00:00+01:00",
    "end_local": "2026-03-30T00:00:00+02:00",
    "start_utc": "2026-03-28T23:00:00Z",
    "end_utc": "2026-03-29T22:00:00Z"
  }
]
```

#### Field Reference

| Field | Type | Description |
|-------|------|-------------|
| `key` | string | Bucket identifier |
| `start_local` | string | Bucket start in local time with UTC offset |
| `end_local` | string | Bucket end in local time with UTC offset |
| `start_utc` | string | Bucket start in UTC |
| `end_utc` | string | Bucket end in UTC |

#### Error Output

```json
{
  "error": "Invalid timezone 'Invalid/Zone'",
  "exit_code": 1
}
```

### Example: DST Transition Period

```bash
tzbucket range --tz Europe/Berlin --interval day --start 2026-03-27T00:00:00Z --end 2026-03-31T00:00:00Z
```

Output shows the 23-hour day on March 29:

```json
[
  {
    "key": "2026-03-27",
    "start_local": "2026-03-27T00:00:00+01:00",
    "end_local": "2026-03-28T00:00:00+01:00",
    "start_utc": "2026-03-26T23:00:00Z",
    "end_utc": "2026-03-27T23:00:00Z"
  },
  {
    "key": "2026-03-28",
    "start_local": "2026-03-28T00:00:00+01:00",
    "end_local": "2026-03-29T00:00:00+01:00",
    "start_utc": "2026-03-27T23:00:00Z",
    "end_utc": "2026-03-28T23:00:00Z"
  },
  {
    "key": "2026-03-29",
    "start_local": "2026-03-29T00:00:00+01:00",
    "end_local": "2026-03-30T00:00:00+02:00",
    "start_utc": "2026-03-28T23:00:00Z",
    "end_utc": "2026-03-29T22:00:00Z"
  },
  {
    "key": "2026-03-30",
    "start_local": "2026-03-30T00:00:00+02:00",
    "end_local": "2026-03-31T00:00:00+02:00",
    "start_utc": "2026-03-29T22:00:00Z",
    "end_utc": "2026-03-30T22:00:00Z"
  },
  {
    "key": "2026-03-31",
    "start_local": "2026-03-31T00:00:00+02:00",
    "end_local": "2026-04-01T00:00:00+02:00",
    "start_utc": "2026-03-30T22:00:00Z",
    "end_utc": "2026-03-31T22:00:00Z"
  }
]
```

---

## explain command

Analyzes a local time in a timezone and resolves DST issues.

### Input

- `--local`: Local time without offset (e.g., `2026-03-29T02:30:00`)
- `--tz`: IANA timezone identifier
- `--policy-nonexistent`: Policy for nonexistent times (`error`, `shift_forward`, `shift_backward`)
- `--policy-ambiguous`: Policy for ambiguous times (`error`, `first`, `second`)

### Output

Produces a single JSON object.

#### Normal Time

When the local time exists and is unambiguous:

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

#### Nonexistent Time (without policy)

When the local time doesn't exist due to spring forward:

```json
{
  "error": "Nonexistent time '2026-03-29T02:30:00' in timezone 'Europe/Berlin'. Skipped due to DST spring forward. Use --policy-nonexistent=shift_forward to resolve.",
  "status": "nonexistent",
  "exit_code": 2
}
```

#### Nonexistent Time (with policy)

When resolved with a policy:

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

#### Ambiguous Time (without policy)

When the local time occurs twice due to fall back:

```json
{
  "error": "Ambiguous time '2026-10-25T02:30:00' in timezone 'Europe/Berlin'. Occurs twice due to DST fall back. Use --policy-ambiguous=first or --policy-ambiguous=second to resolve.",
  "status": "ambiguous",
  "exit_code": 2
}
```

#### Ambiguous Time (with policy)

When resolved with a policy:

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

#### Field Reference

| Field | Type | Description |
|-------|------|-------------|
| `local_time` | string | Input local time |
| `tz` | string | IANA timezone identifier |
| `status` | string | `normal`, `nonexistent`, or `ambiguous` |
| `resolution.utc_time` | string | (normal) UTC equivalent |
| `resolution.policy` | string | (nonexistent/ambiguous) Policy used |
| `resolution.result` | string | (nonexistent/ambiguous) Resolved time with offset |
| `error` | string | (error cases) Error message |
| `exit_code` | integer | (error cases) Exit code |

### Policy Reference

#### Nonexistent Time Policies

| Policy | Description | Example (02:30 on spring forward) |
|--------|-------------|-----------------------------------|
| `error` | Return error (default) | Exit code 2 |
| `shift_forward` | Move to later time | 02:30 → 03:30 |
| `shift_backward` | Move to earlier time | 02:30 → 01:30 |

#### Ambiguous Time Policies

| Policy | Description | Example (02:30 on fall back) |
|--------|-------------|-------------------------------|
| `error` | Return error (default) | Exit code 2 |
| `first` | First occurrence (summer time) | 02:30+02:00 CEST |
| `second` | Second occurrence (winter time) | 02:30+01:00 CET |

---

## Error Handling

### Error Output Format

All errors are returned as JSON:

```json
{
  "error": "Error message describing the problem",
  "exit_code": 1
}
```

### Common Errors

| Error | Exit Code | Cause |
|-------|-----------|-------|
| Invalid timestamp format | 1 | Timestamp doesn't match expected format |
| Invalid timezone | 1 | Unknown IANA timezone identifier |
| Nonexistent time | 2 | Local time skipped during spring forward (no policy) |
| Ambiguous time | 2 | Local time occurs twice during fall back (no policy) |
| I/O error | 1 | File not found, permission denied, etc. |

### Exit Code Summary

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (invalid input, I/O error) |
| 2 | DST resolution error (requires policy) |

---

## Integration Tips

### Parsing JSON Lines (bucket)

The `bucket` command outputs newline-delimited JSON (NDJSON). Parse each line as a separate JSON object:

```python
import json

with open('output.jsonl') as f:
    for line in f:
        record = json.loads(line)
        print(record['bucket']['key'])
```

### Parsing JSON Array (range)

The `range` command outputs a JSON array:

```python
import json

with open('output.json') as f:
    buckets = json.load(f)
    for bucket in buckets:
        print(f"{bucket['key']}: {bucket['start_utc']} to {bucket['end_utc']}")
```

### Handling Errors

Check `exit_code` in error responses:

```bash
result=$(tzbucket explain --tz Europe/Berlin --local 2026-03-29T02:30:00 2>&1)
if echo "$result" | jq -e '.exit_code == 2' > /dev/null 2>&1; then
    echo "DST issue detected, applying policy..."
    tzbucket explain --tz Europe/Berlin --local 2026-03-29T02:30:00 --policy-nonexistent shift_forward
fi
```
