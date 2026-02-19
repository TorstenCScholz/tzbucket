# Migrate from Naive Time Grouping to DST-Safe Buckets

This guide helps teams move from naive timezone grouping to explicit DST-safe bucketing.

## Before (Typical)

- Inline timezone conversion in Spark/SQL
- Group by truncated local time expression
- No explicit bucket boundary table

## After (Recommended)

- Generate canonical boundaries with `tzbucket range`
- Persist dimension table (`dim_time_bucket`)
- Join facts by `[start_utc, end_utc)`
- Group by bucket key

## Migration Steps

1. Pick scope: timezone + interval (`day`, `week`, `month`).
2. Generate historical + future buckets:

```bash
tzbucket range --tz Europe/Berlin --interval day --start 2025-01-01T00:00:00Z --end 2027-01-01T00:00:00Z --output-format json > buckets.json
```

3. Load `buckets.json` into warehouse/Spark table.
4. Switch aggregations to interval join.
5. Backtest around DST boundaries and compare with previous logic.
6. Cut over and monitor.

## Backtest Dates (Examples)

- Europe/Berlin: March 29, 2026 and October 25, 2026
- America/New_York: March 8, 2026 and November 1, 2026

## Rollout Tips

- Run old/new in parallel for one cycle.
- Document expected deltas around DST dates.
- Communicate that post-migration values are the correct local-calendar interpretation.
