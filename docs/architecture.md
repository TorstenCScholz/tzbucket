# tzbucket Architecture Design

## Overview

`tzbucket` is a Rust CLI tool that deterministically assigns timestamps to calendar-based buckets (day/week/month) in an IANA timezone, with explicit DST handling. The tool consists of two crates: `tzbucket-core` (library) and `tzbucket-cli` (binary).

---

## High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              tzbucket-cli                                    │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────────────────────────┐  │
│  │   CLI Args  │───▶│  Subcommand  │───▶│         Output Formatter       │  │
│  │   (clap)    │    │   Handler    │    │    (JSON/Text, deterministic)  │  │
│  └─────────────┘    └──────┬───────┘    └────────────────────────────────┘  │
│                            │                                                │
└────────────────────────────┼────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              tzbucket-core                                   │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────────────────────────┐  │
│  │   Models    │───▶│   Bucket     │───▶│      Timezone Engine           │  │
│  │             │    │   Compute    │    │    (chrono + chrono-tz)        │  │
│  │ - Interval  │    │              │    │                                │  │
│  │ - WeekStart │    │              │    │ - UTC ↔ Local conversion       │  │
│  │ - Policy    │    │              │    │ - DST boundary detection       │  │
│  │ - Bucket    │    │              │    │ - Ambiguous/Nonexistent handle │  │
│  └─────────────┘    └──────────────┘    └────────────────────────────────┘  │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         Error Types                                  │    │
│  │  - ParseError (exit 2)                                               │    │
│  │  - PolicyError (exit 2)                                              │    │
│  │  - RuntimeError (exit 3)                                             │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Module Structure

### tzbucket-core Crate

```
crates/tzbucket-core/src/
├── lib.rs              # Public API exports
├── models/
│   ├── mod.rs          # Module exports
│   ├── interval.rs     # Interval enum (Day/Week/Month)
│   ├── week_start.rs   # WeekStart enum (Monday/Sunday)
│   ├── policy.rs       # Policy struct for DST handling
│   └── bucket.rs       # Bucket struct with start/end times
├── compute/
│   ├── mod.rs          # Module exports
│   ├── bucket.rs       # Bucket computation algorithm
│   └── range.rs        # Range bucket generation
├── timezone/
│   ├── mod.rs          # Module exports
│   ├── conversion.rs   # UTC ↔ Local conversion utilities
│   └── dst.rs          # DST detection and handling
├── parse/
│   ├── mod.rs          # Module exports
│   └── timestamp.rs    # Timestamp parsing (epoch_ms, epoch_s, rfc3339)
└── error.rs            # Error types and Result alias
```

### tzbucket-cli Crate

```
crates/tzbucket-cli/src/
├── main.rs             # Entry point, CLI setup
├── cli/
│   ├── mod.rs          # Module exports
│   ├── args.rs         # clap struct definitions
│   ├── bucket.rs       # bucket subcommand handler
│   ├── range.rs        # range subcommand handler
│   └── explain.rs      # explain subcommand handler
├── output/
│   ├── mod.rs          # Module exports
│   ├── json.rs         # JSON output formatting
│   └── text.rs         # Text output formatting
└── exit_code.rs        # Exit code constants
```

---

## Key Types and Responsibilities

### Core Models

| Type | Responsibility |
|------|----------------|
| `Interval` | Enum representing bucket granularity: `Day`, `Week`, `Month` |
| `WeekStart` | Enum for week boundary: `Monday`, `Sunday` |
| `Policy` | Configuration for DST edge cases: `nonexistent` and `ambiguous` handling |
| `Bucket` | Represents a calendar bucket with `key`, `start_local`, `end_local`, `start_utc`, `end_utc` |
| `Timestamp` | Parsed input timestamp with both RFC3339 representation and epoch_ms |
| `BucketResult` | Complete output for a single bucket operation including input, tz, interval, and bucket |

### Policy Configuration

```
Policy
├── nonexistent: NonexistentPolicy
│   ├── Error      # Return error for nonexistent local times
│   └── ShiftForward  # Map to next valid time
└── ambiguous: AmbiguousPolicy
    ├── Error      # Return error for ambiguous local times
    ├── First      # Use first occurrence (earlier offset)
    └── Second     # Use second occurrence (later offset)
```

### Error Types

| Error Type | Exit Code | When Used |
|------------|-----------|-----------|
| `ParseError` | 2 | Invalid timestamp format, unknown timezone |
| `PolicyError` | 2 | DST policy violation (nonexistent/ambiguous with error policy) |
| `RuntimeError` | 3 | I/O errors, unexpected failures |

---

## Data Flow by Subcommand

### bucket Subcommand

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Input      │────▶│   Parse      │────▶│   Convert    │────▶│   Compute    │
│   Source     │     │   Timestamp  │     │   to Local   │     │   Bucket     │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
      │                    │                    │                    │
      ▼                    ▼                    ▼                    ▼
  stdin/file          epoch_ms/s         chrono-tz            Find bucket
  (streaming)         rfc3339            IANA TZ              boundaries
                                                                in local time
                                                                convert back
                                                                    │
                                                                    ▼
                                                            ┌──────────────┐
                                                            │   Output     │
                                                            │   JSON/Text  │
                                                            └──────────────┘
```

**Steps:**
1. Read timestamps from stdin or file (streaming, line by line)
2. Parse each timestamp according to input format (epoch_ms, epoch_s, rfc3339)
3. Convert UTC instant to local datetime in specified timezone
4. Compute bucket boundaries:
   - Day: local 00:00:00 to next day 00:00:00
   - Week: week-start local 00:00:00 to next week-start 00:00:00
   - Month: first day of month 00:00:00 to first day of next month 00:00:00
5. Convert bucket boundaries back to UTC
6. Output result in specified format

### range Subcommand

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Start/End  │────▶│   Parse      │────▶│   Generate   │────▶│   Sort &     │
│   Timestamps │     │   Range      │     │   Buckets    │     │   Output     │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
```

**Steps:**
1. Parse start and end timestamps (UTC instants)
2. Convert to local time in specified timezone
3. Generate all buckets between start and end:
   - Find first bucket boundary on or after start
   - Iterate bucket boundaries until past end
4. Sort buckets by `start_utc` (deterministic output)
5. Output as JSON array

### explain Subcommand

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Local      │────▶│   Analyze    │────▶│   Apply      │────▶│   Output     │
│   Time + TZ  │     │   DST Status │     │   Policy     │     │   Result     │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
```

**Steps:**
1. Parse local time string (no offset) and timezone
2. Analyze DST status:
   - Normal: unique mapping to UTC
   - Nonexistent: time skipped during DST spring forward
   - Ambiguous: time occurs twice during DST fall back
3. Apply policy to resolve:
   - Error policies: return error with explanation
   - Resolution policies: compute the resolved UTC instant
4. Output explanation with DST details

---

## DST Handling Strategy

### Problem Statement

Daylight Saving Time creates two categories of problematic local times:

1. **Nonexistent Times**: During spring forward, a range of local times is skipped
   - Example: In Europe/Berlin on 2026-03-29, 02:00 jumps to 03:00
   - Local times 02:00:00 through 02:59:59 do not exist

2. **Ambiguous Times**: During fall back, a range of local times occurs twice
   - Example: In Europe/Berlin on 2026-10-25, 02:00 occurs twice
   - Local times 02:00:00 through 02:59:59 are ambiguous

### Strategy for bucket Subcommand

The `bucket` command only accepts **Instants** (UTC timestamps with offset), so:
- No ambiguous/nonexistent times can be input
- DST is handled during bucket boundary computation
- Bucket boundaries are computed in local time, then converted to UTC

**Example: DST Start Day Bucket**

```
Input: 2026-03-29T00:15:00Z (UTC)
TZ: Europe/Berlin

1. Convert to local: 2026-03-29T01:15:00+01:00 (before DST switch)
2. Find day bucket start: 2026-03-29T00:00:00 local
3. Find day bucket end: 2026-03-30T00:00:00 local
4. Convert boundaries to UTC:
   - start_local 2026-03-29T00:00:00+01:00 → 2026-03-28T23:00:00Z
   - end_local 2026-03-30T00:00:00+02:00 → 2026-03-29T22:00:00Z
5. Result: 23-hour bucket (DST spring forward)
```

### Strategy for explain Subcommand

The `explain` command accepts local times without offset, requiring explicit policy:

```
Nonexistent Time Resolution:
┌─────────────────────────────────────────────────────────────┐
│ Input: 2026-03-29T02:30:00 in Europe/Berlin                 │
│ Status: Nonexistent (skipped during DST spring forward)     │
│                                                             │
│ Policy: Error → Return error with explanation               │
│ Policy: ShiftForward → Resolve to 2026-03-29T03:00:00+02:00 │
└─────────────────────────────────────────────────────────────┘

Ambiguous Time Resolution:
┌─────────────────────────────────────────────────────────────┐
│ Input: 2026-10-25T02:30:00 in Europe/Berlin                 │
│ Status: Ambiguous (occurs twice during DST fall back)       │
│                                                             │
│ Policy: Error → Return error with explanation               │
│ Policy: First → Resolve to 2026-10-25T02:30:00+02:00 (DST)  │
│ Policy: Second → Resolve to 2026-10-25T02:30:00+01:00 (STD) │
└─────────────────────────────────────────────────────────────┘
```

### Bucket Boundary Computation Across DST

When computing bucket boundaries that span DST transitions:

1. **Compute in local time first**: Find the local calendar boundary
2. **Convert each boundary independently**: Each conversion handles its own DST context
3. **Result may have non-24-hour buckets**: This is correct and expected

```
DST Spring Forward (23-hour day):
┌──────────────────────────────────────────────────────────────┐
│ Local: 2026-03-29 00:00 → 2026-03-30 00:00                   │
│ UTC:   2026-03-28 23:00Z → 2026-03-29 22:00Z (23 hours)      │
└──────────────────────────────────────────────────────────────┘

DST Fall Back (25-hour day):
┌──────────────────────────────────────────────────────────────┐
│ Local: 2026-10-25 00:00 → 2026-10-26 00:00                   │
│ UTC:   2026-10-24 22:00Z → 2026-10-25 23:00Z (25 hours)      │
└──────────────────────────────────────────────────────────────┘
```

---

## JSON Output Contract

All output uses structs with serde serialization for deterministic field ordering:

### bucket Output

```json
{
  "input": {
    "ts": "2026-03-29T00:15:00Z",
    "epoch_ms": 1793362500000
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

### range Output

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
    "end_local": "2026-03-29T00:00:00+02:00",
    "start_utc": "2026-03-27T23:00:00Z",
    "end_utc": "2026-03-28T22:00:00Z"
  }
]
```

### explain Output

```json
{
  "input_local": "2026-03-29T02:30:00",
  "tz": "Europe/Berlin",
  "status": "nonexistent",
  "resolution": {
    "policy": "shift_forward",
    "resolved_utc": "2026-03-29T01:00:00Z",
    "resolved_local": "2026-03-29T03:00:00+02:00"
  }
}
```

---

## Dependencies

### tzbucket-core

| Crate | Purpose |
|-------|---------|
| `chrono` | Date/time primitives |
| `chrono-tz` | IANA timezone database (compile-time) |
| `serde` | Serialization for JSON output |
| `thiserror` | Error type definitions |

### tzbucket-cli

| Crate | Purpose |
|-------|---------|
| `tzbucket-core` | Core library |
| `clap` | CLI argument parsing |
| `serde_json` | JSON output |
| `tracing` | Logging/debugging |
| `tracing-subscriber` | Log output configuration |
| `anyhow` | Error handling in main |

---

## Testing Strategy

### Unit Tests (tzbucket-core)

- Interval/WeekStart/Policy model tests
- Timestamp parsing tests
- Bucket computation tests with known DST boundaries
- Timezone conversion tests

### Integration Tests (tzbucket-cli)

- Golden tests using fixtures and expected JSON output
- Exit code verification
- Streaming input tests

### Required Fixtures

1. `berlin_dst_start_2026.txt` - Timestamps around 2026-03-29 DST spring forward
2. `berlin_dst_end_2026.txt` - Timestamps around 2026-10-25 DST fall back
3. `ny_dst_2026.txt` - Smoke test for America/New_York timezone
4. `explain_nonexistent.txt` - Local time that does not exist
5. `explain_ambiguous.txt` - Local time that is ambiguous

---

## Implementation Order

1. **Core models**: `Interval`, `WeekStart`, `Policy`, `Bucket` structs
2. **Timestamp parsing**: epoch_ms, epoch_s, rfc3339 formats
3. **Bucket computation**: Day bucket first, then Week, then Month
4. **CLI bucket subcommand**: Basic functionality
5. **DST tests**: Berlin fixtures and golden files
6. **range subcommand**: Bucket list generation
7. **explain subcommand**: DST analysis and policy resolution
8. **Error handling**: Proper exit codes and messages
