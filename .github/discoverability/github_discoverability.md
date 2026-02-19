# GitHub Discoverability Playbook

This page defines repository metadata and content conventions to improve findability.

## Repository Description (Suggested)

`DST-safe timezone-aware time bucketing for ETL and analytics (Spark, SQL, dbt). Handles 23-hour/25-hour days and DST ambiguous/nonexistent local times.`

## Topics (Suggested)

Add these repository topics in GitHub settings:

- `timezone`
- `daylight-saving-time`
- `dst`
- `etl`
- `analytics`
- `data-engineering`
- `spark`
- `sql`
- `rust`
- `cli`

## Release Notes Template

Include user-problem phrasing in release notes:

- "Fix Spark timezone day grouping DST drift"
- "Prevent daily aggregate errors on 23-hour/25-hour days"
- "Improve handling of ambiguous/nonexistent local times"

## README/Docs Linking Rules

- Every problem page links to integration and output-contract pages.
- README links directly to problem pages.
- Use exact query-like headings (for example: "Spark timezone day grouping DST wrong").

## Community Surfaces

Use Discussions/Issues labels for problem phrases:

- `spark-dst`
- `sql-date-trunc`
- `ambiguous-time`
- `nonexistent-time`

These labels help both support and search relevance.
