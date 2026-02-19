# DST ETL Checklist

Use this checklist to avoid DST-related regressions in analytics pipelines.

## Design

- Define whether metrics follow UTC days or business-local calendar days.
- If local-calendar semantics are required, use explicit bucket boundaries.
- Use half-open intervals: `[start_utc, end_utc)`.

## Data Model

- Keep event timestamps in UTC.
- Store bucket dimensions with `key`, `start_utc`, `end_utc`, `tz`, `interval`.
- Version bucket dimensions by timezone and interval.

## Pipeline

- Generate future bucket windows with `tzbucket range`.
- Join facts against buckets using interval predicates.
- Avoid per-row ad hoc timezone conversions in transformations.

## Validation

- Add regression checks around known DST dates in your regions.
- Confirm 23-hour and 25-hour day windows appear where expected.
- Verify no event maps to multiple buckets.

## Operations

- Pin `tzbucket` version in jobs/images.
- Alert on non-zero exits: `2` (input/policy), `3` (runtime).
- Rebuild bucket dimensions on schedule.

## Local-Time Inputs

If you accept local times, enforce explicit policies for:

- nonexistent times (spring forward)
- ambiguous times (fall back)

Use `tzbucket explain` to diagnose and document policy behavior.
