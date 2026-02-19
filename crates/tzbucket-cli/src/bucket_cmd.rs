use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::ExitCode;

use chrono_tz::Tz;
use tzbucket_core::{BucketResult, TimestampFormat, compute_bucket, parse_timestamp};

use crate::cli::BucketArgs;
use crate::error::{CliError, CliResult, EXIT_SUCCESS, OutputFormat};
use crate::shared::{parse_format, parse_interval, parse_tz_or_input_error, parse_week_start};

pub fn run_bucket(args: BucketArgs, output_format: OutputFormat) -> CliResult<ExitCode> {
    let tz = parse_tz_or_input_error(&args.tz)?;
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
    interval: tzbucket_core::Interval,
    week_start: tzbucket_core::WeekStart,
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
