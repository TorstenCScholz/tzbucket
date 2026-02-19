//! Input parsing for timestamps.
//!
//! This module provides functions for parsing timestamps in various formats:
//! - `epoch_ms`: Unix epoch milliseconds (default)
//! - `epoch_s`: Unix epoch seconds
//! - `rfc3339`: RFC3339 formatted strings (e.g., `2026-03-29T00:15:00Z`)

use chrono::{DateTime, TimeZone, Utc};
use std::str::FromStr;

use crate::error::{Result, TzBucketError};

/// Supported timestamp formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimestampFormat {
    /// Unix epoch milliseconds (e.g., "1793362500000")
    #[default]
    EpochMs,
    /// Unix epoch seconds (e.g., "1793362500")
    EpochS,
    /// RFC3339 format (e.g., "2026-03-29T00:15:00Z" or "2026-03-29T00:15:00+01:00")
    Rfc3339,
}

impl std::fmt::Display for TimestampFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimestampFormat::EpochMs => write!(f, "epoch_ms"),
            TimestampFormat::EpochS => write!(f, "epoch_s"),
            TimestampFormat::Rfc3339 => write!(f, "rfc3339"),
        }
    }
}

impl FromStr for TimestampFormat {
    type Err = TzBucketError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "epoch_ms" => Ok(TimestampFormat::EpochMs),
            "epoch_s" => Ok(TimestampFormat::EpochS),
            "rfc3339" => Ok(TimestampFormat::Rfc3339),
            _ => Err(TzBucketError::ParseError(format!(
                "Unknown format: '{}'. Expected 'epoch_ms', 'epoch_s', or 'rfc3339'",
                s
            ))),
        }
    }
}

/// Parse a timestamp string according to the specified format.
///
/// # Arguments
///
/// * `input` - The timestamp string to parse
/// * `format` - The format to use for parsing
///
/// # Returns
///
/// The parsed UTC datetime on success, or an error if parsing fails.
///
/// # Examples
///
/// ```
/// use tzbucket_core::parse::{parse_timestamp, TimestampFormat};
/// use chrono::{TimeZone, Utc};
///
/// // Parse epoch milliseconds
/// let dt = parse_timestamp("1793362500000", TimestampFormat::EpochMs).unwrap();
/// assert_eq!(dt, Utc.timestamp_millis_opt(1793362500000).single().unwrap());
///
/// // Parse RFC3339
/// let dt = parse_timestamp("2026-03-29T00:15:00Z", TimestampFormat::Rfc3339).unwrap();
/// ```
pub fn parse_timestamp(input: &str, format: TimestampFormat) -> Result<DateTime<Utc>> {
    let trimmed = input.trim();

    match format {
        TimestampFormat::EpochMs => parse_epoch_ms(trimmed),
        TimestampFormat::EpochS => parse_epoch_s(trimmed),
        TimestampFormat::Rfc3339 => parse_rfc3339(trimmed),
    }
}

/// Parse epoch milliseconds.
fn parse_epoch_ms(input: &str) -> Result<DateTime<Utc>> {
    let ms: i64 = input.parse().map_err(|_| {
        TzBucketError::ParseError(format!(
            "Invalid epoch milliseconds: '{}'. Expected integer value.",
            input
        ))
    })?;

    Utc.timestamp_millis_opt(ms).single().ok_or_else(|| {
        TzBucketError::ParseError(format!("Epoch milliseconds out of range: {}", ms))
    })
}

/// Parse epoch seconds.
fn parse_epoch_s(input: &str) -> Result<DateTime<Utc>> {
    let s: i64 = input.parse().map_err(|_| {
        TzBucketError::ParseError(format!(
            "Invalid epoch seconds: '{}'. Expected integer value.",
            input
        ))
    })?;

    Utc.timestamp_opt(s, 0)
        .single()
        .ok_or_else(|| TzBucketError::ParseError(format!("Epoch seconds out of range: {}", s)))
}

/// Parse RFC3339 formatted timestamp.
///
/// Supports formats like:
/// - `2026-03-29T00:15:00Z`
/// - `2026-03-29T00:15:00+01:00`
/// - `2026-03-29T00:15:00-05:00`
fn parse_rfc3339(input: &str) -> Result<DateTime<Utc>> {
    // Try parsing with various RFC3339 formats
    DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            TzBucketError::ParseError(format!(
                "Invalid RFC3339 timestamp: '{}'. Error: {}",
                input, e
            ))
        })
}

/// Parse a timestamp string, auto-detecting the format.
///
/// This function attempts to parse the input in the following order:
/// 1. RFC3339 (if it contains 'T' or 'Z' or offset)
/// 2. Epoch milliseconds (if the number is large enough)
/// 3. Epoch seconds
///
/// # Arguments
///
/// * `input` - The timestamp string to parse
///
/// # Returns
///
/// The parsed UTC datetime on success, or an error if parsing fails.
pub fn parse_timestamp_auto(input: &str) -> Result<DateTime<Utc>> {
    let trimmed = input.trim();

    // Check if it looks like RFC3339 (contains 'T' or 'Z' or offset)
    if trimmed.contains('T')
        || trimmed.contains('Z')
        || trimmed.contains('+')
        || (trimmed.len() > 6 && trimmed.chars().nth(trimmed.len() - 6) == Some('-'))
    {
        return parse_rfc3339(trimmed);
    }

    // Try parsing as a number
    if let Ok(num) = trimmed.parse::<i64>() {
        // Heuristic: if the number is > 10^12, it's probably milliseconds
        // (year 2001 in seconds is ~10^9, year 2001 in ms is ~10^12)
        if num > 10_000_000_000 {
            return parse_epoch_ms(trimmed);
        } else {
            return parse_epoch_s(trimmed);
        }
    }

    Err(TzBucketError::ParseError(format!(
        "Could not auto-detect format for: '{}'",
        input
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Timelike};

    #[test]
    fn parse_epoch_milliseconds() {
        let dt = parse_timestamp("1793362500000", TimestampFormat::EpochMs).unwrap();
        let expected = Utc.timestamp_millis_opt(1793362500000).single().unwrap();
        assert_eq!(dt, expected);
    }

    #[test]
    fn parse_epoch_seconds() {
        let dt = parse_timestamp("1793362500", TimestampFormat::EpochS).unwrap();
        let expected = Utc.timestamp_opt(1793362500, 0).single().unwrap();
        assert_eq!(dt, expected);
    }

    #[test]
    fn parse_rfc3339_zulu() {
        let dt = parse_timestamp("2026-03-29T00:15:00Z", TimestampFormat::Rfc3339).unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 29);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 15);
    }

    #[test]
    fn parse_rfc3339_with_offset() {
        // 2026-03-29T00:15:00+01:00 = 2026-03-28T23:15:00Z
        let dt = parse_timestamp("2026-03-29T00:15:00+01:00", TimestampFormat::Rfc3339).unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 28);
        assert_eq!(dt.hour(), 23);
        assert_eq!(dt.minute(), 15);
    }

    #[test]
    fn parse_invalid_epoch_ms() {
        let result = parse_timestamp("not-a-number", TimestampFormat::EpochMs);
        assert!(result.is_err());
    }

    #[test]
    fn parse_invalid_rfc3339() {
        let result = parse_timestamp("not-a-date", TimestampFormat::Rfc3339);
        assert!(result.is_err());
    }

    #[test]
    fn format_from_str() {
        assert_eq!(
            TimestampFormat::from_str("epoch_ms").unwrap(),
            TimestampFormat::EpochMs
        );
        assert_eq!(
            TimestampFormat::from_str("epoch_s").unwrap(),
            TimestampFormat::EpochS
        );
        assert_eq!(
            TimestampFormat::from_str("rfc3339").unwrap(),
            TimestampFormat::Rfc3339
        );
        assert!(TimestampFormat::from_str("invalid").is_err());
    }

    #[test]
    fn auto_detect_rfc3339() {
        let dt = parse_timestamp_auto("2026-03-29T00:15:00Z").unwrap();
        assert_eq!(dt.year(), 2026);
    }

    #[test]
    fn auto_detect_epoch_ms() {
        let dt = parse_timestamp_auto("1793362500000").unwrap();
        let expected = Utc.timestamp_millis_opt(1793362500000).single().unwrap();
        assert_eq!(dt, expected);
    }

    #[test]
    fn auto_detect_epoch_s() {
        let dt = parse_timestamp_auto("1793362500").unwrap();
        let expected = Utc.timestamp_opt(1793362500, 0).single().unwrap();
        assert_eq!(dt, expected);
    }

    #[test]
    fn format_display() {
        assert_eq!(format!("{}", TimestampFormat::EpochMs), "epoch_ms");
        assert_eq!(format!("{}", TimestampFormat::EpochS), "epoch_s");
        assert_eq!(format!("{}", TimestampFormat::Rfc3339), "rfc3339");
    }
}
