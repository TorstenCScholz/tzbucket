use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::ExitCode;

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

    /// Start of range (inclusive, RFC3339)
    #[arg(long)]
    start: String,

    /// End of range (exclusive, RFC3339)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorKind {
    Input,
    Runtime,
}

#[derive(Debug)]
struct CliError {
    kind: ErrorKind,
    message: String,
    status: Option<&'static str>,
}

impl CliError {
    fn input(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Input,
            message: message.into(),
            status: None,
        }
    }

    fn policy(message: impl Into<String>, status: &'static str) -> Self {
        Self {
            kind: ErrorKind::Input,
            message: message.into(),
            status: Some(status),
        }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Runtime,
            message: message.into(),
            status: None,
        }
    }

    fn exit_code(&self) -> u8 {
        match self.kind {
            ErrorKind::Input => EXIT_INPUT_ERROR,
            ErrorKind::Runtime => EXIT_RUNTIME_ERROR,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

type CliResult<T> = std::result::Result<T, CliError>;

#[derive(Debug, Serialize)]
struct ErrorOutput {
    error: String,
    exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

fn main() -> ExitCode {
    run()
}

fn run() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Bucket(args) => execute_bucket(args),
        Commands::Range(args) => execute_range(args),
        Commands::Explain(args) => execute_explain(args),
    }
}

fn execute_bucket(args: BucketArgs) -> ExitCode {
    let fallback = output_format_hint(&args.output_format);
    let output_format = match parse_output_format(&args.output_format) {
        Ok(format) => format,
        Err(err) => return render_error(&err, fallback),
    };

    match run_bucket(args, output_format) {
        Ok(code) => code,
        Err(err) => render_error(&err, output_format),
    }
}

fn execute_range(args: RangeArgs) -> ExitCode {
    let fallback = output_format_hint(&args.output_format);
    let output_format = match parse_output_format(&args.output_format) {
        Ok(format) => format,
        Err(err) => return render_error(&err, fallback),
    };

    match run_range(args, output_format) {
        Ok(code) => code,
        Err(err) => render_error(&err, output_format),
    }
}

fn execute_explain(args: ExplainArgs) -> ExitCode {
    let fallback = output_format_hint(&args.output_format);
    let output_format = match parse_output_format(&args.output_format) {
        Ok(format) => format,
        Err(err) => return render_error(&err, fallback),
    };

    match run_explain(args, output_format) {
        Ok(code) => code,
        Err(err) => render_error(&err, output_format),
    }
}

fn render_error(err: &CliError, output_format: OutputFormat) -> ExitCode {
    match output_format {
        OutputFormat::Json => {
            let envelope = ErrorOutput {
                error: err.message.clone(),
                exit_code: err.exit_code(),
                status: err.status.map(str::to_string),
            };

            match serde_json::to_string_pretty(&envelope) {
                Ok(json) => eprintln!("{}", json),
                Err(_) => eprintln!("Error: {}", err.message),
            }
        }
        OutputFormat::Text => {
            eprintln!("Error: {}", err.message);
        }
    }

    ExitCode::from(err.exit_code())
}

fn output_format_hint(s: &str) -> OutputFormat {
    if s.eq_ignore_ascii_case("json") {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

fn parse_output_format(s: &str) -> CliResult<OutputFormat> {
    match s.to_lowercase().as_str() {
        "json" => Ok(OutputFormat::Json),
        "text" => Ok(OutputFormat::Text),
        _ => Err(CliError::input(format!(
            "Invalid output_format '{}'. Expected: json, text",
            s
        ))),
    }
}

fn parse_interval(s: &str) -> CliResult<Interval> {
    match s.to_lowercase().as_str() {
        "day" => Ok(Interval::Day),
        "week" => Ok(Interval::Week),
        "month" => Ok(Interval::Month),
        _ => Err(CliError::input(format!(
            "Invalid interval '{}'. Expected: day, week, month",
            s
        ))),
    }
}

fn parse_week_start(s: &str) -> CliResult<WeekStart> {
    match s.to_lowercase().as_str() {
        "monday" => Ok(WeekStart::Monday),
        "sunday" => Ok(WeekStart::Sunday),
        _ => Err(CliError::input(format!(
            "Invalid week_start '{}'. Expected: monday, sunday",
            s
        ))),
    }
}

fn parse_format(s: &str) -> CliResult<TimestampFormat> {
    match s.to_lowercase().as_str() {
        "epoch_ms" => Ok(TimestampFormat::EpochMs),
        "epoch_s" => Ok(TimestampFormat::EpochS),
        "rfc3339" => Ok(TimestampFormat::Rfc3339),
        _ => Err(CliError::input(format!(
            "Invalid format '{}'. Expected: epoch_ms, epoch_s, rfc3339",
            s
        ))),
    }
}

fn parse_nonexistent_policy(s: &str) -> CliResult<NonexistentPolicy> {
    match s.to_lowercase().as_str() {
        "error" => Ok(NonexistentPolicy::Error),
        "shift_forward" => Ok(NonexistentPolicy::ShiftForward),
        _ => Err(CliError::input(format!(
            "Invalid policy_nonexistent '{}'. Expected: error, shift_forward",
            s
        ))),
    }
}

fn parse_ambiguous_policy(s: &str) -> CliResult<AmbiguousPolicy> {
    match s.to_lowercase().as_str() {
        "error" => Ok(AmbiguousPolicy::Error),
        "first" => Ok(AmbiguousPolicy::First),
        "second" => Ok(AmbiguousPolicy::Second),
        _ => Err(CliError::input(format!(
            "Invalid policy_ambiguous '{}'. Expected: error, first, second",
            s
        ))),
    }
}

fn run_bucket(args: BucketArgs, output_format: OutputFormat) -> CliResult<ExitCode> {
    let tz = parse_tz(&args.tz)
        .map_err(|e| CliError::input(format!("Invalid timezone '{}': {}", args.tz, e)))?;
    let interval = parse_interval(&args.interval)?;
    let week_start = parse_week_start(&args.week_start)?;
    let format = parse_format(&args.format)?;

    let reader: Box<dyn BufRead> = if args.stdin || args.input == "-" {
        Box::new(io::stdin().lock())
    } else {
        let file = File::open(&args.input).map_err(|e| {
            CliError::runtime(format!("Failed to open file '{}': {}", args.input, e))
        })?;
        Box::new(BufReader::new(file))
    };

    for line in reader.lines() {
        let line = line.map_err(|e| CliError::runtime(format!("Failed to read line: {}", e)))?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        let result = process_bucket_line(trimmed, &tz, interval, week_start, format)
            .map_err(|e| CliError::input(format!("Error processing '{}': {}", trimmed, e)))?;

        match output_format {
            OutputFormat::Json => {
                let json = serde_json::to_string(&result)
                    .map_err(|e| CliError::runtime(format!("Failed to serialize JSON: {}", e)))?;
                println!("{}", json);
            }
            OutputFormat::Text => {
                println!(
                    "{} -> {} to {}",
                    result.bucket.key, result.bucket.start_local, result.bucket.end_local
                );
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
) -> CliResult<BucketResult> {
    let instant = parse_timestamp(input, format).map_err(|e| CliError::input(e.to_string()))?;

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

fn run_range(args: RangeArgs, output_format: OutputFormat) -> CliResult<ExitCode> {
    let tz = parse_tz(&args.tz)
        .map_err(|e| CliError::input(format!("Invalid timezone '{}': {}", args.tz, e)))?;
    let interval = parse_interval(&args.interval)?;
    let week_start = parse_week_start(&args.week_start)?;

    let start_utc = parse_timestamp(&args.start, TimestampFormat::Rfc3339)
        .map_err(|e| CliError::input(format!("Invalid start timestamp: {}", e)))?;
    let end_utc = parse_timestamp(&args.end, TimestampFormat::Rfc3339)
        .map_err(|e| CliError::input(format!("Invalid end timestamp: {}", e)))?;

    if start_utc >= end_utc {
        return Err(CliError::input(format!(
            "Invalid range: start '{}' must be earlier than end '{}'",
            args.start, args.end
        )));
    }

    let buckets = generate_buckets_in_range(start_utc, end_utc, tz, interval, week_start)?;

    match output_format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&buckets)
                .map_err(|e| CliError::runtime(format!("Failed to serialize JSON: {}", e)))?;
            println!("{}", json);
        }
        OutputFormat::Text => {
            for bucket in buckets {
                println!(
                    "{}: {} to {}",
                    bucket.key, bucket.start_local, bucket.end_local
                );
            }
        }
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
) -> CliResult<Vec<RangeBucket>> {
    let mut buckets = Vec::new();

    let start_local = start_utc.with_timezone(&tz);
    let end_local = end_utc.with_timezone(&tz);

    match interval {
        Interval::Day => {
            let mut current_date = start_local.date_naive();
            let end_date = end_local.date_naive();

            while current_date <= end_date {
                let bucket = compute_bucket_for_date(current_date, tz, interval, week_start)?;
                let bucket_start_utc = parse_rfc3339_to_utc(&bucket.start_utc)?;
                let bucket_end_utc = parse_rfc3339_to_utc(&bucket.end_utc)?;

                if bucket_start_utc < end_utc && bucket_end_utc > start_utc {
                    buckets.push(bucket);
                }

                current_date += chrono::Duration::days(1);
            }
        }
        Interval::Week => {
            let mut current_date = start_local.date_naive();
            let end_date = end_local.date_naive();

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

                if bucket_start_utc < end_utc
                    && bucket_end_utc > start_utc
                    && !buckets.iter().any(|b: &RangeBucket| b.key == bucket.key)
                {
                    buckets.push(bucket);
                }

                current_date += chrono::Duration::weeks(1);
            }
        }
        Interval::Month => {
            let mut current_date = start_local.date_naive();
            let end_date = end_local.date_naive();

            current_date =
                chrono::NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 1)
                    .ok_or_else(|| CliError::runtime("Could not construct month start date"))?;

            while current_date <= end_date {
                let bucket = compute_bucket_for_date(current_date, tz, interval, week_start)?;
                let bucket_start_utc = parse_rfc3339_to_utc(&bucket.start_utc)?;
                let bucket_end_utc = parse_rfc3339_to_utc(&bucket.end_utc)?;

                if bucket_start_utc < end_utc
                    && bucket_end_utc > start_utc
                    && !buckets.iter().any(|b: &RangeBucket| b.key == bucket.key)
                {
                    buckets.push(bucket);
                }

                let year = current_date.year();
                let month = current_date.month();
                current_date = if month == 12 {
                    chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
                        .ok_or_else(|| CliError::runtime("Could not construct next month date"))?
                } else {
                    chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
                        .ok_or_else(|| CliError::runtime("Could not construct next month date"))?
                };
            }
        }
    }

    buckets.sort_by(|a, b| a.start_utc.cmp(&b.start_utc));

    Ok(buckets)
}

fn compute_bucket_for_date(
    date: chrono::NaiveDate,
    tz: Tz,
    interval: Interval,
    week_start: WeekStart,
) -> CliResult<RangeBucket> {
    let midnight = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
        CliError::runtime(format!("Could not construct midnight for date {}", date))
    })?;

    let local_result = tz.from_local_datetime(&midnight);
    let instant = local_result
        .single()
        .or_else(|| local_result.earliest())
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| {
            CliError::runtime(format!(
                "Could not resolve local midnight for date {}",
                date
            ))
        })?;

    let bucket = compute_bucket(instant, tz, interval, Some(week_start));

    Ok(RangeBucket {
        key: bucket.key,
        start_local: bucket.start_local,
        end_local: bucket.end_local,
        start_utc: bucket.start_utc,
        end_utc: bucket.end_utc,
    })
}

fn parse_rfc3339_to_utc(s: &str) -> CliResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| CliError::runtime(format!("Failed to parse RFC3339 '{}': {}", s, e)))
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

fn run_explain(args: ExplainArgs, output_format: OutputFormat) -> CliResult<ExitCode> {
    let tz = parse_tz(&args.tz)
        .map_err(|e| CliError::input(format!("Invalid timezone '{}': {}", args.tz, e)))?;
    let nonexistent_policy = parse_nonexistent_policy(&args.policy_nonexistent)?;
    let ambiguous_policy = parse_ambiguous_policy(&args.policy_ambiguous)?;
    let local = parse_local_time(&args.local)?;

    let result = explain_local_time(local, tz, nonexistent_policy, ambiguous_policy)?;

    match output_format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&result)
                .map_err(|e| CliError::runtime(format!("Failed to serialize JSON: {}", e)))?;
            println!("{}", json);
        }
        OutputFormat::Text => {
            println!("Local time: {}", result.local_time);
            println!("Timezone: {}", result.tz);
            println!("Status: {}", result.status);
            if let Some(resolution) = result.resolution {
                println!("Resolution: {} -> {}", resolution.policy, resolution.result);
            }
        }
    }

    Ok(ExitCode::from(EXIT_SUCCESS))
}

fn parse_local_time(s: &str) -> CliResult<NaiveDateTime> {
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

    Err(CliError::input(format!(
        "Invalid local time format '{}'. Expected: YYYY-MM-DDTHH:MM:SS",
        s
    )))
}

fn explain_local_time(
    local: NaiveDateTime,
    tz: Tz,
    nonexistent_policy: NonexistentPolicy,
    ambiguous_policy: AmbiguousPolicy,
) -> CliResult<ExplainResult> {
    use chrono::offset::LocalResult;

    let local_result = tz.from_local_datetime(&local);

    let (status, resolution) = match local_result {
        LocalResult::Single(_dt) => ("normal".to_string(), None),
        LocalResult::Ambiguous(first, second) => match ambiguous_policy {
            AmbiguousPolicy::Error => {
                return Err(CliError::policy(
                    format!(
                        "Ambiguous time '{}' in timezone '{}'. Occurs twice due to DST fall back. \
                         Use --policy-ambiguous=first or --policy-ambiguous=second to resolve.",
                        local.format("%Y-%m-%dT%H:%M:%S"),
                        tz
                    ),
                    "ambiguous",
                ));
            }
            AmbiguousPolicy::First => (
                "ambiguous".to_string(),
                Some(Resolution {
                    policy: "first".to_string(),
                    result: format_rfc3339(&first),
                }),
            ),
            AmbiguousPolicy::Second => (
                "ambiguous".to_string(),
                Some(Resolution {
                    policy: "second".to_string(),
                    result: format_rfc3339(&second),
                }),
            ),
        },
        LocalResult::None => match nonexistent_policy {
            NonexistentPolicy::Error => {
                return Err(CliError::policy(
                    format!(
                        "Nonexistent time '{}' in timezone '{}'. Skipped due to DST spring forward. \
                         Use --policy-nonexistent=shift_forward to resolve.",
                        local.format("%Y-%m-%dT%H:%M:%S"),
                        tz
                    ),
                    "nonexistent",
                ));
            }
            NonexistentPolicy::ShiftForward => {
                let result_dt = resolve_nonexistent_shift_forward(local, tz).ok_or_else(|| {
                    CliError::runtime("Could not resolve shifted time with shift_forward policy")
                })?;

                (
                    "nonexistent".to_string(),
                    Some(Resolution {
                        policy: "shift_forward".to_string(),
                        result: format_rfc3339(&result_dt),
                    }),
                )
            }
        },
    };

    Ok(ExplainResult {
        local_time: local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        tz: tz.to_string(),
        status,
        resolution,
    })
}

fn resolve_nonexistent_shift_forward(local: NaiveDateTime, tz: Tz) -> Option<DateTime<Tz>> {
    let previous = find_previous_valid_local_time(local, tz)?;
    let next = find_next_valid_local_time(local, tz)?;

    // Compute the skipped wall-clock gap and preserve the local minute/second offset.
    let gap = next.naive_local() - previous.naive_local() - chrono::Duration::seconds(1);
    let shifted_local = local + gap;
    let shifted_result = tz.from_local_datetime(&shifted_local);

    shifted_result
        .single()
        .or_else(|| shifted_result.earliest())
        .or(Some(next))
}

fn find_next_valid_local_time(local: NaiveDateTime, tz: Tz) -> Option<DateTime<Tz>> {
    // Search forward second-by-second and return the first representable local time.
    // The wide bound handles rare historical transitions with large gaps.
    let max_seconds = 2 * 24 * 60 * 60;

    for seconds in 1..=max_seconds {
        let candidate = local + chrono::Duration::seconds(i64::from(seconds));
        let local_result = tz.from_local_datetime(&candidate);

        if let Some(dt) = local_result.single().or_else(|| local_result.earliest()) {
            return Some(dt);
        }
    }

    None
}

fn find_previous_valid_local_time(local: NaiveDateTime, tz: Tz) -> Option<DateTime<Tz>> {
    let max_seconds = 2 * 24 * 60 * 60;

    for seconds in 1..=max_seconds {
        let candidate = local - chrono::Duration::seconds(i64::from(seconds));
        let local_result = tz.from_local_datetime(&candidate);

        if let Some(dt) = local_result.single().or_else(|| local_result.latest()) {
            return Some(dt);
        }
    }

    None
}

fn format_rfc3339<T: TimeZone>(dt: &DateTime<T>) -> String
where
    T::Offset: std::fmt::Display,
{
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}
