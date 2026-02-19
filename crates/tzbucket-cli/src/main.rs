use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::ExitCode;

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use clap::{Parser, Subcommand};
use serde::Serialize;
use tzbucket_core::tz::parse_tz;
use tzbucket_core::{
    AmbiguousPolicy, BucketResult, Interval, NonexistentPolicy, TimestampFormat, WeekStart,
    compute_bucket, parse_timestamp,
};

/// DST-safe time bucketing tool
#[derive(Parser, Debug)]
#[command(name = "tzbucket")]
#[command(about = "DST-safe time bucketing tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compute time buckets for timestamps
    Bucket(BucketArgs),
    /// Generate all buckets in a time range
    Range(RangeArgs),
    /// Explain local time resolution (DST handling)
    Explain(ExplainArgs),
}

#[derive(clap::Args, Debug)]
pub struct BucketArgs {
    /// IANA timezone (e.g., Europe/Berlin)
    #[arg(short, long, default_value = "UTC")]
    tz: String,

    /// Bucket interval: day, week, month
    #[arg(short = 'i', long, default_value = "day")]
    interval: String,

    /// Week start day: monday or sunday (for week interval)
    #[arg(long, default_value = "monday")]
    week_start: String,

    /// Input format: epoch_ms, epoch_s, rfc3339
    #[arg(short = 'f', long, default_value = "epoch_ms")]
    format: String,

    /// Output format: json, text
    #[arg(long, default_value = "text")]
    output_format: String,

    /// Input file path (use - for stdin)
    #[arg(long, default_value = "-")]
    input: String,

    /// Read from stdin
    #[arg(long)]
    stdin: bool,
}

#[derive(clap::Args, Debug)]
pub struct RangeArgs {
    /// IANA timezone
    #[arg(short, long)]
    tz: String,

    /// Bucket interval: day, week, month
    #[arg(short = 'i', long, default_value = "day")]
    interval: String,

    /// Week start day
    #[arg(long, default_value = "monday")]
    week_start: String,

    /// Start timestamp (RFC3339)
    #[arg(long)]
    start: String,

    /// End timestamp (RFC3339)
    #[arg(long)]
    end: String,

    /// Output format: json, text
    #[arg(long, default_value = "json")]
    output_format: String,
}

#[derive(clap::Args, Debug)]
pub struct ExplainArgs {
    /// IANA timezone
    #[arg(short, long)]
    tz: String,

    /// Local time string (without offset, e.g., 2026-03-29T02:30:00)
    #[arg(long)]
    local: String,

    /// Policy for nonexistent times: error, shift_forward
    #[arg(long, default_value = "error")]
    policy_nonexistent: String,

    /// Policy for ambiguous times: error, first, second
    #[arg(long, default_value = "error")]
    policy_ambiguous: String,

    /// Output format: json, text
    #[arg(long, default_value = "json")]
    output_format: String,
}

/// Exit codes
const EXIT_SUCCESS: u8 = 0;
const EXIT_INPUT_ERROR: u8 = 2;
const EXIT_RUNTIME_ERROR: u8 = 3;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            // Determine exit code based on error type
            let err_str = e.to_string();
            // Input/policy errors: exit code 2
            // Runtime errors: exit code 3
            if err_str.contains("parse")
                || err_str.contains("invalid")
                || err_str.contains("Invalid")
                || err_str.contains("Nonexistent time")
                || err_str.contains("Ambiguous time")
            {
                ExitCode::from(EXIT_INPUT_ERROR)
            } else {
                ExitCode::from(EXIT_RUNTIME_ERROR)
            }
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Bucket(args) => run_bucket(args),
        Commands::Range(args) => run_range(args),
        Commands::Explain(args) => run_explain(args),
    }
}

fn parse_interval(s: &str) -> Result<Interval> {
    match s.to_lowercase().as_str() {
        "day" => Ok(Interval::Day),
        "week" => Ok(Interval::Week),
        "month" => Ok(Interval::Month),
        _ => anyhow::bail!("Invalid interval '{}'. Expected: day, week, month", s),
    }
}

fn parse_week_start(s: &str) -> Result<WeekStart> {
    match s.to_lowercase().as_str() {
        "monday" => Ok(WeekStart::Monday),
        "sunday" => Ok(WeekStart::Sunday),
        _ => anyhow::bail!("Invalid week_start '{}'. Expected: monday, sunday", s),
    }
}

fn parse_format(s: &str) -> Result<TimestampFormat> {
    match s.to_lowercase().as_str() {
        "epoch_ms" => Ok(TimestampFormat::EpochMs),
        "epoch_s" => Ok(TimestampFormat::EpochS),
        "rfc3339" => Ok(TimestampFormat::Rfc3339),
        _ => anyhow::bail!(
            "Invalid format '{}'. Expected: epoch_ms, epoch_s, rfc3339",
            s
        ),
    }
}

fn parse_nonexistent_policy(s: &str) -> Result<NonexistentPolicy> {
    match s.to_lowercase().as_str() {
        "error" => Ok(NonexistentPolicy::Error),
        "shift_forward" => Ok(NonexistentPolicy::ShiftForward),
        _ => anyhow::bail!(
            "Invalid policy_nonexistent '{}'. Expected: error, shift_forward",
            s
        ),
    }
}

fn parse_ambiguous_policy(s: &str) -> Result<AmbiguousPolicy> {
    match s.to_lowercase().as_str() {
        "error" => Ok(AmbiguousPolicy::Error),
        "first" => Ok(AmbiguousPolicy::First),
        "second" => Ok(AmbiguousPolicy::Second),
        _ => anyhow::bail!(
            "Invalid policy_ambiguous '{}'. Expected: error, first, second",
            s
        ),
    }
}

fn run_bucket(args: BucketArgs) -> Result<ExitCode> {
    let tz =
        parse_tz(&args.tz).map_err(|e| anyhow::anyhow!("Invalid timezone '{}': {}", args.tz, e))?;
    let interval = parse_interval(&args.interval)?;
    let week_start = parse_week_start(&args.week_start)?;
    let format = parse_format(&args.format)?;

    // Determine input source
    let reader: Box<dyn BufRead> = if args.stdin || args.input == "-" {
        Box::new(io::stdin().lock())
    } else {
        let file = File::open(&args.input)
            .with_context(|| format!("Failed to open file: {}", args.input))?;
        Box::new(BufReader::new(file))
    };

    // Process line by line (streaming)
    for line in reader.lines() {
        let line = line.context("Failed to read line")?;
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        match process_bucket_line(trimmed, &tz, interval, week_start, format) {
            Ok(result) => match args.output_format.as_str() {
                "json" => {
                    let json =
                        serde_json::to_string(&result).context("Failed to serialize JSON")?;
                    println!("{}", json);
                }
                "text" => {
                    println!(
                        "{} -> {} to {}",
                        result.bucket.key, result.bucket.start_local, result.bucket.end_local
                    );
                }
                _ => anyhow::bail!("Invalid output_format. Expected: json, text"),
            },
            Err(e) => {
                eprintln!("Error processing '{}': {}", trimmed, e);
                return Ok(ExitCode::from(EXIT_INPUT_ERROR));
            }
        }
    }

    Ok(ExitCode::from(EXIT_SUCCESS))
}

fn process_bucket_line(
    input: &str,
    tz: &Tz,
    interval: Interval,
    week_start: WeekStart,
    format: TimestampFormat,
) -> Result<BucketResult> {
    let instant = parse_timestamp(input, format).map_err(|e| anyhow::anyhow!("{}", e))?;

    let bucket = compute_bucket(instant, *tz, interval, Some(week_start));

    Ok(BucketResult {
        input: tzbucket_core::InputTimestamp {
            ts: input.to_string(),
            epoch_ms: instant.timestamp_millis(),
        },
        tz: tz.to_string(),
        interval,
        bucket,
    })
}

fn run_range(args: RangeArgs) -> Result<ExitCode> {
    let tz =
        parse_tz(&args.tz).map_err(|e| anyhow::anyhow!("Invalid timezone '{}': {}", args.tz, e))?;
    let interval = parse_interval(&args.interval)?;
    let week_start = parse_week_start(&args.week_start)?;

    // Parse start and end as RFC3339
    let start_utc = parse_timestamp(&args.start, TimestampFormat::Rfc3339)
        .map_err(|e| anyhow::anyhow!("Invalid start timestamp: {}", e))?;
    let end_utc = parse_timestamp(&args.end, TimestampFormat::Rfc3339)
        .map_err(|e| anyhow::anyhow!("Invalid end timestamp: {}", e))?;

    // Generate buckets
    let buckets = generate_buckets_in_range(start_utc, end_utc, tz, interval, week_start)?;

    match args.output_format.as_str() {
        "json" => {
            let json =
                serde_json::to_string_pretty(&buckets).context("Failed to serialize JSON")?;
            println!("{}", json);
        }
        "text" => {
            for bucket in buckets {
                println!(
                    "{}: {} to {}",
                    bucket.key, bucket.start_local, bucket.end_local
                );
            }
        }
        _ => anyhow::bail!("Invalid output_format. Expected: json, text"),
    }

    Ok(ExitCode::from(EXIT_SUCCESS))
}

#[derive(Debug, Serialize)]
struct RangeBucket {
    key: String,
    start_local: String,
    end_local: String,
    start_utc: String,
    end_utc: String,
}

fn generate_buckets_in_range(
    start_utc: DateTime<Utc>,
    end_utc: DateTime<Utc>,
    tz: Tz,
    interval: Interval,
    week_start: WeekStart,
) -> Result<Vec<RangeBucket>> {
    let mut buckets = Vec::new();

    // Convert to local time to find starting bucket
    let start_local = start_utc.with_timezone(&tz);
    let end_local = end_utc.with_timezone(&tz);

    // Generate buckets based on interval
    match interval {
        Interval::Day => {
            let mut current_date = start_local.date_naive();
            let end_date = end_local.date_naive();

            while current_date <= end_date {
                let bucket = compute_bucket_for_date(current_date, tz, interval, week_start)?;

                // Only include if bucket overlaps with range
                let bucket_start_utc = parse_rfc3339_to_utc(&bucket.start_utc)?;
                let bucket_end_utc = parse_rfc3339_to_utc(&bucket.end_utc)?;

                if bucket_start_utc <= end_utc && bucket_end_utc > start_utc {
                    buckets.push(bucket);
                }

                current_date += chrono::Duration::days(1);
            }
        }
        Interval::Week => {
            let mut current_date = start_local.date_naive();
            let end_date = end_local.date_naive();

            // Adjust to week start
            let weekday = current_date.weekday();
            let days_from_week_start = match week_start {
                WeekStart::Monday => weekday.num_days_from_monday() as i64,
                WeekStart::Sunday => weekday.num_days_from_sunday() as i64,
            };
            current_date -= chrono::Duration::days(days_from_week_start);

            while current_date <= end_date {
                let bucket = compute_bucket_for_date(current_date, tz, interval, week_start)?;

                let bucket_start_utc = parse_rfc3339_to_utc(&bucket.start_utc)?;
                let bucket_end_utc = parse_rfc3339_to_utc(&bucket.end_utc)?;

                if bucket_start_utc <= end_utc && bucket_end_utc > start_utc {
                    // Avoid duplicates
                    if !buckets.iter().any(|b: &RangeBucket| b.key == bucket.key) {
                        buckets.push(bucket);
                    }
                }

                current_date += chrono::Duration::weeks(1);
            }
        }
        Interval::Month => {
            let mut current_date = start_local.date_naive();
            let end_date = end_local.date_naive();

            // Adjust to month start
            current_date =
                chrono::NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 1)
                    .unwrap();

            while current_date <= end_date {
                let bucket = compute_bucket_for_date(current_date, tz, interval, week_start)?;

                let bucket_start_utc = parse_rfc3339_to_utc(&bucket.start_utc)?;
                let bucket_end_utc = parse_rfc3339_to_utc(&bucket.end_utc)?;

                if bucket_start_utc <= end_utc && bucket_end_utc > start_utc {
                    // Avoid duplicates
                    if !buckets.iter().any(|b: &RangeBucket| b.key == bucket.key) {
                        buckets.push(bucket);
                    }
                }

                // Move to next month
                let year = current_date.year();
                let month = current_date.month();
                current_date = if month == 12 {
                    chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
                } else {
                    chrono::NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
                };
            }
        }
    }

    // Sort by start_utc
    buckets.sort_by(|a, b| a.start_utc.cmp(&b.start_utc));

    Ok(buckets)
}

fn compute_bucket_for_date(
    date: chrono::NaiveDate,
    tz: Tz,
    interval: Interval,
    week_start: WeekStart,
) -> Result<RangeBucket> {
    // Create a UTC instant at the start of this date in local time
    let midnight = date.and_hms_opt(0, 0, 0).unwrap();

    // Convert local midnight to UTC (handling DST)
    let instant = match tz.from_local_datetime(&midnight).single() {
        Some(dt) => dt.with_timezone(&Utc),
        None => {
            // Handle nonexistent time (DST spring forward)
            // Use the earliest possible time
            tz.from_local_datetime(&midnight)
                .earliest()
                .map(|dt| dt.with_timezone(&Utc))
                .ok_or_else(|| anyhow::anyhow!("Could not resolve local midnight"))?
        }
    };

    let bucket = compute_bucket(instant, tz, interval, Some(week_start));

    Ok(RangeBucket {
        key: bucket.key,
        start_local: bucket.start_local,
        end_local: bucket.end_local,
        start_utc: bucket.start_utc,
        end_utc: bucket.end_utc,
    })
}

fn parse_rfc3339_to_utc(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| anyhow::anyhow!("Failed to parse RFC3339 '{}': {}", s, e))
}

#[derive(Debug, Serialize)]
struct ExplainResult {
    local_time: String,
    tz: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<Resolution>,
}

#[derive(Debug, Serialize)]
struct Resolution {
    policy: String,
    result: String,
}

fn run_explain(args: ExplainArgs) -> Result<ExitCode> {
    let tz =
        parse_tz(&args.tz).map_err(|e| anyhow::anyhow!("Invalid timezone '{}': {}", args.tz, e))?;
    let nonexistent_policy = parse_nonexistent_policy(&args.policy_nonexistent)?;
    let ambiguous_policy = parse_ambiguous_policy(&args.policy_ambiguous)?;

    // Parse local time (without offset)
    let local = parse_local_time(&args.local)?;

    // Determine status and resolve
    let result = explain_local_time(local, tz, nonexistent_policy, ambiguous_policy)?;

    match args.output_format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&result).context("Failed to serialize JSON")?;
            println!("{}", json);
        }
        "text" => {
            println!("Local time: {}", result.local_time);
            println!("Timezone: {}", result.tz);
            println!("Status: {}", result.status);
            if let Some(resolution) = result.resolution {
                println!("Resolution: {} -> {}", resolution.policy, resolution.result);
            }
        }
        _ => anyhow::bail!("Invalid output_format. Expected: json, text"),
    }

    Ok(ExitCode::from(EXIT_SUCCESS))
}

fn parse_local_time(s: &str) -> Result<NaiveDateTime> {
    // Try parsing as RFC3339-like without offset
    // Formats: YYYY-MM-DDTHH:MM:SS or YYYY-MM-DD HH:MM:SS
    let formats = [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ];

    for fmt in &formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(dt);
        }
    }

    anyhow::bail!(
        "Invalid local time format '{}'. Expected: YYYY-MM-DDTHH:MM:SS",
        s
    )
}

fn explain_local_time(
    local: NaiveDateTime,
    tz: Tz,
    nonexistent_policy: NonexistentPolicy,
    ambiguous_policy: AmbiguousPolicy,
) -> Result<ExplainResult> {
    use chrono::offset::LocalResult;

    let local_result = tz.from_local_datetime(&local);

    let (status, resolution) = match local_result {
        LocalResult::Single(_dt) => {
            // Normal time - unambiguous
            ("normal".to_string(), None)
        }
        LocalResult::Ambiguous(first, second) => {
            // Ambiguous time (DST fall back)
            match ambiguous_policy {
                AmbiguousPolicy::Error => {
                    return Err(anyhow::anyhow!(
                        "Ambiguous time '{}' in timezone '{}'. Occurs twice due to DST fall back. \
                         Use --policy-ambiguous=first or --policy-ambiguous=second to resolve.",
                        local.format("%Y-%m-%dT%H:%M:%S"),
                        tz
                    ));
                }
                AmbiguousPolicy::First => {
                    let result = format_rfc3339(&first);
                    (
                        "ambiguous".to_string(),
                        Some(Resolution {
                            policy: "first".to_string(),
                            result,
                        }),
                    )
                }
                AmbiguousPolicy::Second => {
                    let result = format_rfc3339(&second);
                    (
                        "ambiguous".to_string(),
                        Some(Resolution {
                            policy: "second".to_string(),
                            result,
                        }),
                    )
                }
            }
        }
        LocalResult::None => {
            // Nonexistent time (DST spring forward)
            match nonexistent_policy {
                NonexistentPolicy::Error => {
                    return Err(anyhow::anyhow!(
                        "Nonexistent time '{}' in timezone '{}'. Skipped due to DST spring forward. \
                         Use --policy-nonexistent=shift_forward to resolve.",
                        local.format("%Y-%m-%dT%H:%M:%S"),
                        tz
                    ));
                }
                NonexistentPolicy::ShiftForward => {
                    // Shift forward by the DST gap (typically 1 hour)
                    let shifted = local + chrono::Duration::hours(1);
                    let result_dt = tz
                        .from_local_datetime(&shifted)
                        .single()
                        .ok_or_else(|| anyhow::anyhow!("Could not resolve shifted time"))?;
                    let result = format_rfc3339(&result_dt);
                    (
                        "nonexistent".to_string(),
                        Some(Resolution {
                            policy: "shift_forward".to_string(),
                            result,
                        }),
                    )
                }
            }
        }
    };

    Ok(ExplainResult {
        local_time: local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        tz: tz.to_string(),
        status,
        resolution,
    })
}

fn format_rfc3339<T: TimeZone>(dt: &DateTime<T>) -> String
where
    T::Offset: std::fmt::Display,
{
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}
