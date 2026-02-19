# SEO Keyword Clusters

Use this map to align docs, release notes, and examples with real search intent.

## Cluster 1: Spark + DST Grouping Errors

Primary queries:

- `spark timezone day grouping dst wrong`
- `pyspark group by local day daylight saving time`
- `spark daily aggregates wrong around dst`

Intent:

- Fix wrong daily counts in Spark when local timezone has DST transitions.

Suggested landing pages:

- `docs/spark_dst_bucketing.md`
- `docs/etl_integration.md`

## Cluster 2: SQL `date_trunc` Timezone Pitfalls

Primary queries:

- `sql date_trunc timezone dst bug`
- `postgres date_trunc day timezone daylight saving`
- `warehouse daily metrics dst issue`

Intent:

- Understand why local-day SQL grouping drifts on DST boundaries.

Suggested landing pages:

- `docs/sql_date_trunc_dst_pitfalls.md`
- `docs/migrate_from_naive_grouping.md`

## Cluster 3: General ETL DST Reliability

Primary queries:

- `how to handle dst in etl pipelines`
- `23 hour day 25 hour day analytics`
- `timezone-aware bucketing etl`

Intent:

- Get architecture pattern for reliable time bucketing in pipelines.

Suggested landing pages:

- `docs/dst_etl_checklist.md`
- `docs/etl_integration.md`

## Cluster 4: Local-Time Ambiguity Handling

Primary queries:

- `nonexistent local time spring forward handling`
- `ambiguous local time fall back handling`
- `resolve daylight saving ambiguous timestamp`

Intent:

- Learn policy options for local time resolution.

Suggested landing pages:

- `docs/output_format.md`
- `docs/dst_notes.md`

## Cluster 5: Category/Tool Discovery

Primary queries:

- `dst safe time bucketing`
- `timezone-aware bucketing cli`
- `event timestamp bucketing by timezone`

Intent:

- Find a tool to plug into existing ETL/analytics stacks.

Suggested landing pages:

- `README.md`
- `docs/etl_integration.md`
