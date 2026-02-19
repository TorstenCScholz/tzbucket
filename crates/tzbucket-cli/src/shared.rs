use chrono::{DateTime, TimeZone};

use crate::error::{CliError, CliResult};
use chrono_tz::Tz;
use tzbucket_core::{AmbiguousPolicy, Interval, NonexistentPolicy, TimestampFormat, WeekStart};

pub fn parse_interval(s: &str) -> CliResult<Interval> {
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

pub fn parse_week_start(s: &str) -> CliResult<WeekStart> {
    match s.to_lowercase().as_str() {
        "monday" => Ok(WeekStart::Monday),
        "sunday" => Ok(WeekStart::Sunday),
        _ => Err(CliError::input(format!(
            "Invalid week_start '{}'. Expected: monday, sunday",
            s
        ))),
    }
}

pub fn parse_format(s: &str) -> CliResult<TimestampFormat> {
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

pub fn parse_nonexistent_policy(s: &str) -> CliResult<NonexistentPolicy> {
    match s.to_lowercase().as_str() {
        "error" => Ok(NonexistentPolicy::Error),
        "shift_forward" => Ok(NonexistentPolicy::ShiftForward),
        _ => Err(CliError::input(format!(
            "Invalid policy_nonexistent '{}'. Expected: error, shift_forward",
            s
        ))),
    }
}

pub fn parse_ambiguous_policy(s: &str) -> CliResult<AmbiguousPolicy> {
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

pub fn parse_rfc3339_to_utc(s: &str) -> CliResult<DateTime<chrono::Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| CliError::runtime(format!("Failed to parse RFC3339 '{}': {}", s, e)))
}

pub fn format_rfc3339<T: TimeZone>(dt: &DateTime<T>) -> String
where
    T::Offset: std::fmt::Display,
{
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

pub fn parse_tz_or_input_error(name: &str) -> CliResult<Tz> {
    tzbucket_core::tz::parse_tz(name)
        .map_err(|e| CliError::input(format!("Invalid timezone '{}': {}", name, e)))
}
