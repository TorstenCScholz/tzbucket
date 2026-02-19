use std::process::ExitCode;

use chrono::{DateTime, Datelike, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Serialize;
use tzbucket_core::{Interval, TimestampFormat, WeekStart, compute_bucket, parse_timestamp};

use crate::cli::RangeArgs;
use crate::error::{CliError, CliResult, EXIT_SUCCESS, OutputFormat};
use crate::shared::{
    parse_interval, parse_rfc3339_to_utc, parse_tz_or_input_error, parse_week_start,
};

pub fn run_range(args: RangeArgs, output_format: OutputFormat) -> CliResult<ExitCode> {
    let tz = parse_tz_or_input_error(&args.tz)?;
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
