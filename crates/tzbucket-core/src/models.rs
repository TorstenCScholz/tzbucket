//! Core data types for tzbucket.
//!
//! This module defines the primary types used throughout the library:
//! - [`Interval`] - Bucket granularity (day/week/month)
//! - [`WeekStart`] - Week boundary configuration
//! - [`NonexistentPolicy`] - How to handle nonexistent local times
//! - [`AmbiguousPolicy`] - How to handle ambiguous local times
//! - [`Policy`] - Combined DST handling policy
//! - [`Bucket`] - A computed time bucket
//! - [`InputTimestamp`] - Parsed input timestamp
//! - [`BucketResult`] - Complete result for a bucket operation

use serde::Serialize;

/// Bucket granularity interval.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    /// Daily bucket (00:00:00 to next day 00:00:00 in local time)
    #[default]
    Day,
    /// Weekly bucket (week start 00:00:00 to next week start 00:00:00)
    Week,
    /// Monthly bucket (1st day 00:00:00 to 1st of next month 00:00:00)
    Month,
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interval::Day => write!(f, "day"),
            Interval::Week => write!(f, "week"),
            Interval::Month => write!(f, "month"),
        }
    }
}

/// Week start day configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart {
    /// Week starts on Monday (ISO 8601)
    #[default]
    Monday,
    /// Week starts on Sunday
    Sunday,
}

impl std::fmt::Display for WeekStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeekStart::Monday => write!(f, "monday"),
            WeekStart::Sunday => write!(f, "sunday"),
        }
    }
}

/// Policy for handling nonexistent local times.
///
/// Nonexistent times occur during DST spring forward when a range
/// of local times is skipped (e.g., 02:00-02:59 in Europe/Berlin).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NonexistentPolicy {
    /// Return an error for nonexistent times.
    #[default]
    Error,
    /// Shift forward to the next valid local time.
    ShiftForward,
}

/// Policy for handling ambiguous local times.
///
/// Ambiguous times occur during DST fall back when a range
/// of local times occurs twice (e.g., 02:00-02:59 in Europe/Berlin).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AmbiguousPolicy {
    /// Return an error for ambiguous times.
    #[default]
    Error,
    /// Use the first occurrence (earlier offset, still in DST).
    First,
    /// Use the second occurrence (later offset, back to standard time).
    Second,
}

/// Combined DST handling policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Policy {
    /// How to handle nonexistent local times.
    pub nonexistent: NonexistentPolicy,
    /// How to handle ambiguous local times.
    pub ambiguous: AmbiguousPolicy,
}

/// A computed time bucket with boundaries in both local and UTC time.
#[derive(Debug, Clone, Serialize)]
pub struct Bucket {
    /// Bucket key (format depends on interval):
    /// - Day: `YYYY-MM-DD`
    /// - Week: `YYYY-WXX` where XX is week number
    /// - Month: `YYYY-MM`
    pub key: String,
    /// Bucket start in local time with offset (RFC3339 format).
    pub start_local: String,
    /// Bucket end in local time with offset (RFC3339 format).
    pub end_local: String,
    /// Bucket start in UTC (RFC3339 format with Z suffix).
    pub start_utc: String,
    /// Bucket end in UTC (RFC3339 format with Z suffix).
    pub end_utc: String,
}

/// Parsed input timestamp.
#[derive(Debug, Clone, Serialize)]
pub struct InputTimestamp {
    /// Original input string.
    pub ts: String,
    /// Epoch milliseconds (UTC).
    pub epoch_ms: i64,
}

/// Complete result of a bucket computation.
#[derive(Debug, Clone, Serialize)]
pub struct BucketResult {
    /// The input timestamp that was processed.
    pub input: InputTimestamp,
    /// The timezone used for bucket computation.
    pub tz: String,
    /// The interval (granularity) used.
    pub interval: Interval,
    /// The computed bucket.
    pub bucket: Bucket,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_default_is_day() {
        assert_eq!(Interval::default(), Interval::Day);
    }

    #[test]
    fn interval_display() {
        assert_eq!(format!("{}", Interval::Day), "day");
        assert_eq!(format!("{}", Interval::Week), "week");
        assert_eq!(format!("{}", Interval::Month), "month");
    }

    #[test]
    fn week_start_default_is_monday() {
        assert_eq!(WeekStart::default(), WeekStart::Monday);
    }

    #[test]
    fn week_start_display() {
        assert_eq!(format!("{}", WeekStart::Monday), "monday");
        assert_eq!(format!("{}", WeekStart::Sunday), "sunday");
    }

    #[test]
    fn policy_default_is_error_error() {
        let policy = Policy::default();
        assert_eq!(policy.nonexistent, NonexistentPolicy::Error);
        assert_eq!(policy.ambiguous, AmbiguousPolicy::Error);
    }

    #[test]
    fn interval_serialization() {
        assert_eq!(serde_json::to_string(&Interval::Day).unwrap(), "\"day\"");
        assert_eq!(serde_json::to_string(&Interval::Week).unwrap(), "\"week\"");
        assert_eq!(
            serde_json::to_string(&Interval::Month).unwrap(),
            "\"month\""
        );
    }

    #[test]
    fn week_start_serialization() {
        assert_eq!(
            serde_json::to_string(&WeekStart::Monday).unwrap(),
            "\"monday\""
        );
        assert_eq!(
            serde_json::to_string(&WeekStart::Sunday).unwrap(),
            "\"sunday\""
        );
    }
}
