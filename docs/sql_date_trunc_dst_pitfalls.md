# SQL `date_trunc` Timezone DST Bug: Why Daily Metrics Break

A common query pattern:

```sql
date_trunc('day', ts_utc AT TIME ZONE 'Europe/Berlin')
```

This looks correct but can cause subtle errors around DST transitions.

## Problem

Local calendar days do not map to fixed 24-hour UTC windows during DST changes.

- DST start day can be 23 hours.
- DST end day can be 25 hours.

If your logic assumes fixed durations, grouped results can drift.

## Safer Approach

Use explicit bucket boundaries and join by interval.

1. Generate bucket boundaries with `tzbucket range`.
2. Load them as a dimension table.
3. Join facts on:

```sql
f.event_ts_utc >= b.start_utc
AND f.event_ts_utc < b.end_utc
```

## Example SQL Pattern

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

## When to Use This

- Analytics/reporting with business-local day/week/month semantics.
- ETL where correctness on DST boundary dates matters.

## Related Docs

- `docs/migrate_from_naive_grouping.md`
- `docs/etl_integration.md`
- `docs/output_format.md`
