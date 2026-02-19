# tzbucket Architecture

## Overview

`tzbucket` is a Rust workspace with two crates:

- `tzbucket-core`: DST-safe bucketing logic and parsing utilities
- `tzbucket-cli`: command-line interface and output/error rendering

## High-Level Flow

1. Parse CLI args (`clap`)
2. Parse timezone + timestamps
3. Compute bucket boundaries in local calendar time
4. Convert boundaries to UTC
5. Emit deterministic JSON/text output

## Workspace Layout

### `crates/tzbucket-core`

- `src/lib.rs`: public exports and prelude
- `src/models.rs`: `Interval`, `WeekStart`, policy and output structs
- `src/parse.rs`: timestamp parsing (`epoch_ms`, `epoch_s`, `rfc3339`)
- `src/tz.rs`: timezone parsing + conversion helpers
- `src/compute.rs`: bucket computation for day/week/month
- `src/error.rs`: core error enum

### `crates/tzbucket-cli`

- `src/main.rs`: command dispatch
- `src/cli.rs`: subcommand and argument definitions
- `src/error.rs`: CLI error typing, exit-code mapping, error envelopes
- `src/shared.rs`: shared parsing/format helpers for CLI modules
- `src/bucket_cmd.rs`: `bucket` execution path
- `src/range_cmd.rs`: `range` execution path
- `src/explain_cmd.rs`: `explain` execution path

## Key Design Decisions

### 1) Local-calendar boundary computation

Buckets are aligned to local calendar boundaries (`00:00` for day, configured week start, first day of month). This is what analytics/reporting systems usually need.

### 2) Independent boundary conversion

Start and end boundaries are converted to UTC independently. This correctly handles offset changes during DST transitions and yields valid 23h/25h UTC spans.

### 3) Deterministic serialization

Output structs are serialized with stable field order. `range` results are sorted by `start_utc`.

### 4) Explicit CLI error model

CLI maps errors to stable categories:

- input/policy => exit code `2`
- runtime => exit code `3`

In JSON mode, errors are emitted as structured JSON to `stderr`.

## Command Responsibilities

### `bucket`

- Input: UTC timestamps from stdin/file
- Output: one bucket result per input line (NDJSON in JSON mode)
- Behavior: streaming line-by-line processing

### `range`

- Input: `start`, `end`, timezone, interval
- Semantics: half-open range `[start, end)` with overlap inclusion
- Output: ordered bucket list

### `explain`

- Input: local time without offset + timezone
- Output: DST classification (`normal`, `nonexistent`, `ambiguous`)
- Policies: resolve nonexistent/ambiguous cases or return policy errors

## Testing Strategy

- `tzbucket-core` unit tests for parsing/conversion/bucket logic
- CLI integration tests with fixtures + golden JSON
- Regression tests for DST edge cases and boundary semantics
