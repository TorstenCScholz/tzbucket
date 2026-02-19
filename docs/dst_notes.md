# DST (Daylight Saving Time) Notes

This document provides detailed information about Daylight Saving Time (DST) and how it affects time bucketing operations.

## What is DST?

Daylight Saving Time is the practice of advancing clocks during warmer months to extend evening daylight. Clocks are typically:

- **Sprung forward** in spring (lose 1 hour)
- **Fallen back** in autumn/fall (gain 1 hour)

Not all regions observe DST, and the dates vary by location. This creates complexity for time-based data processing.

## DST Transitions in 2026

### Europe/Berlin

| Event | Date | Transition | Local Time Change |
|-------|------|------------|-------------------|
| Spring Forward | March 29, 2026 | 02:00 → 03:00 | 23-hour day |
| Fall Back | October 25, 2026 | 03:00 → 02:00 | 25-hour day |

**Spring Forward Details:**

At 02:00 CET (Central European Time), clocks jump to 03:00 CEST (Central European Summer Time):
- Times 02:00:00 through 02:59:59 do not exist
- The hour after 01:59:59 is 03:00:00
- The day has only 23 hours

**Fall Back Details:**

At 03:00 CEST, clocks fall back to 02:00 CET:
- Times 02:00:00 through 02:59:59 occur twice
- The hour after 01:59:59 CEST is 02:00:00 CET
- The day has 25 hours

### America/New_York

| Event | Date | Transition | Local Time Change |
|-------|------|------------|-------------------|
| Spring Forward | March 8, 2026 | 02:00 → 03:00 | 23-hour day |
| Fall Back | November 1, 2026 | 02:00 → 01:00 | 25-hour day |

**Spring Forward Details:**

At 02:00 EST (Eastern Standard Time), clocks jump to 03:00 EDT (Eastern Daylight Time):
- Times 02:00:00 through 02:59:59 do not exist
- The hour after 01:59:59 is 03:00:00
- The day has only 23 hours

**Fall Back Details:**

At 02:00 EDT, clocks fall back to 01:00 EST:
- Times 01:00:00 through 01:59:59 occur twice
- The hour after 01:59:59 EDT is 01:00:00 EST
- The day has 25 hours

### Other Timezones

DST rules vary by region:

| Timezone | Spring Forward | Fall Back |
|----------|---------------|-----------|
| Europe/London | March 29, 2026 | October 25, 2026 |
| Europe/Paris | March 29, 2026 | October 25, 2026 |
| America/Los_Angeles | March 8, 2026 | November 1, 2026 |
| America/Chicago | March 8, 2026 | November 1, 2026 |
| Australia/Sydney | April 5, 2026 | October 4, 2026 (opposite seasons) |
| Asia/Tokyo | No DST | No DST |

## Nonexistent Times

During spring forward, certain local times don't exist because the clock jumps forward.

### Example: Berlin, March 29, 2026

```
01:59:59 CET  →  02:00:00 CEST (skipped)
                 03:00:00 CEST
```

The local time `2026-03-29T02:30:00` does not exist in Berlin.

### Handling Nonexistent Times

When you use `tzbucket explain` with a nonexistent time:

```bash
tzbucket explain --tz Europe/Berlin --local 2026-03-29T02:30:00
```

Output:
```json
{
  "error": "Nonexistent time '2026-03-29T02:30:00' in timezone 'Europe/Berlin'. Skipped due to DST spring forward. Use --policy-nonexistent=shift_forward to resolve.",
  "status": "nonexistent",
  "exit_code": 2
}
```

### Resolution Policies

Use `--policy-nonexistent` to resolve:

| Policy | Behavior | Example Result |
|--------|----------|----------------|
| `error` | Return error (default) | Exit code 2 |
| `shift_forward` | Move to later time | 02:30 → 03:30 CEST |
| `shift_backward` | Move to earlier time | 02:30 → 01:30 CET |

Example with policy:
```bash
tzbucket explain --tz Europe/Berlin --local 2026-03-29T02:30:00 --policy-nonexistent shift_forward
```

Output:
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

## Ambiguous Times

During fall back, certain local times occur twice because the clock moves backward.

### Example: Berlin, October 25, 2026

```
01:59:59 CEST  →  02:00:00 CEST (first occurrence)
02:59:59 CEST  →  02:00:00 CET (clock falls back)
                  02:00:00 CET (second occurrence)
```

The local time `2026-10-25T02:30:00` occurs twice in Berlin:
1. 02:30 CEST (UTC+02:00) - summer time
2. 02:30 CET (UTC+01:00) - winter time

### Handling Ambiguous Times

When you use `tzbucket explain` with an ambiguous time:

```bash
tzbucket explain --tz Europe/Berlin --local 2026-10-25T02:30:00
```

Output:
```json
{
  "error": "Ambiguous time '2026-10-25T02:30:00' in timezone 'Europe/Berlin'. Occurs twice due to DST fall back. Use --policy-ambiguous=first or --policy-ambiguous=second to resolve.",
  "status": "ambiguous",
  "exit_code": 2
}
```

### Resolution Policies

Use `--policy-ambiguous` to resolve:

| Policy | Behavior | Example Result |
|--------|----------|----------------|
| `error` | Return error (default) | Exit code 2 |
| `first` | Use first occurrence (summer time) | 02:30 CEST (+02:00) |
| `second` | Use second occurrence (winter time) | 02:30 CET (+01:00) |

Example with policy:
```bash
tzbucket explain --tz Europe/Berlin --local 2026-10-25T02:30:00 --policy-ambiguous first
```

Output:
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

## How tzbucket Handles DST

### bucket command

The `bucket` command only accepts UTC timestamps, so there's no ambiguity:

- Each UTC instant maps to exactly one bucket
- Bucket boundaries are computed correctly for DST transitions
- 23-hour and 25-hour days are handled automatically

Example showing a 23-hour day bucket:

```json
{
  "bucket": {
    "key": "2026-03-29",
    "start_local": "2026-03-29T00:00:00+01:00",
    "end_local": "2026-03-30T00:00:00+02:00",
    "start_utc": "2026-03-28T23:00:00Z",
    "end_utc": "2026-03-29T22:00:00Z"
  }
}
```

Note: `end_utc` - `start_utc` = 23 hours (2026-03-29T22:00:00Z - 2026-03-28T23:00:00Z = 23 hours).

Example showing a 25-hour day bucket:

```json
{
  "bucket": {
    "key": "2026-10-25",
    "start_local": "2026-10-25T00:00:00+02:00",
    "end_local": "2026-10-26T00:00:00+01:00",
    "start_utc": "2026-10-24T22:00:00Z",
    "end_utc": "2026-10-25T23:00:00Z"
  }
}
```

Note: `end_utc` - `start_utc` = 25 hours (2026-10-25T23:00:00Z - 2026-10-24T22:00:00Z = 25 hours).

### range command

The `range` command generates bucket sequences with correct UTC boundaries:

```bash
tzbucket range --tz Europe/Berlin --interval day --start 2026-03-27T00:00:00Z --end 2026-03-31T00:00:00Z
```

This produces buckets including the DST transition day with correct 23-hour duration.

### explain command

The `explain` command handles local times with explicit policies:

- Detects nonexistent times (spring forward)
- Detects ambiguous times (fall back)
- Provides clear error messages with resolution suggestions
- Applies policies when specified

## Best Practices

### For Data Pipelines

1. **Store timestamps in UTC**: Always store and process timestamps in UTC to avoid ambiguity.

2. **Use bucket for grouping**: The `bucket` command handles DST correctly because it works with UTC instants.

3. **Be careful with local times**: When accepting local time input, always handle nonexistent and ambiguous cases explicitly.

### For User Interfaces

1. **Display in local time**: Convert UTC to local time for display, but keep the original UTC.

2. **Warn about DST issues**: When users input local times near DST transitions, warn them about potential issues.

3. **Use explain for debugging**: The `explain` command is useful for understanding why a particular local time behaves unexpectedly.

## Further Reading

- [IANA Time Zone Database](https://www.iana.org/time-zones) - The source of timezone data
- [Wikipedia: Daylight saving time](https://en.wikipedia.org/wiki/Daylight_saving_time) - General information
- [RFC 5545: DATE-TIME](https://tools.ietf.org/html/rfc5545#section-3.3.5) - Handling time in calendaring
