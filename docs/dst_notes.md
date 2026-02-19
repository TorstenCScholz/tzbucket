# DST Notes

This document explains how DST impacts bucketing and local-time resolution in `tzbucket`.

## DST Transitions Used in Tests (2026)

### Europe/Berlin

- Spring forward: **March 29, 2026** (`02:00 -> 03:00`) => 23-hour day
- Fall back: **October 25, 2026** (`03:00 -> 02:00`) => 25-hour day

### America/New_York

- Spring forward: **March 8, 2026** (`02:00 -> 03:00`) => 23-hour day
- Fall back: **November 1, 2026** (`02:00 -> 01:00`) => 25-hour day

## Bucketing Behavior

`bucket` and `range` operate on UTC instants. Bucket boundaries are computed in local calendar time and converted back to UTC independently.

This naturally yields variable UTC durations:

- A spring-forward day bucket may be 23 hours in UTC.
- A fall-back day bucket may be 25 hours in UTC.

Example (Berlin spring forward):

- `start_local`: `2026-03-29T00:00:00+01:00`
- `end_local`: `2026-03-30T00:00:00+02:00`
- `start_utc`: `2026-03-28T23:00:00Z`
- `end_utc`: `2026-03-29T22:00:00Z` (23-hour duration)

## `explain` Behavior

`explain` analyzes local time strings without offset and classifies them as:

- `normal`
- `nonexistent`
- `ambiguous`

### Nonexistent times

A local time in the skipped hour does not exist (for example Berlin `2026-03-29T02:30:00`).

Policies:

- `error` (default): returns an error (exit code `2`)
- `shift_forward`: resolves to the next valid wall-clock equivalent

### Ambiguous times

A local time in the repeated hour occurs twice (for example Berlin `2026-10-25T02:30:00`).

Policies:

- `error` (default): returns an error (exit code `2`)
- `first`: chooses first occurrence
- `second`: chooses second occurrence

## Error Output in JSON Mode

With `--output-format json`, errors are emitted on `stderr` as JSON:

```json
{
  "error": "Nonexistent time '2026-03-29T02:30:00' in timezone 'Europe/Berlin'. Skipped due to DST spring forward. Use --policy-nonexistent=shift_forward to resolve.",
  "exit_code": 2,
  "status": "nonexistent"
}
```
