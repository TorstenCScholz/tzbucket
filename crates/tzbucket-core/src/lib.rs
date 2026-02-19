//! # tzbucket-core
//!
//! A DST-safe time bucketing library for Rust.
//!
//! This library provides functionality for assigning timestamps to calendar-based
//! buckets (day/week/month) in an IANA timezone, with explicit DST handling.
//!
//! ## Features
//!
//! - **DST Safety**: Bucket boundaries are computed in local time and converted
//!   independently to UTC, correctly handling 23-hour and 25-hour days.
//! - **Multiple Intervals**: Support for day, week, and month buckets.
//! - **Flexible Week Start**: Configurable week start (Monday or Sunday).
//! - **Multiple Input Formats**: Parse epoch milliseconds, epoch seconds, or RFC3339.
//! - **IANA Timezones**: Full support for IANA timezone database via chrono-tz.
//!
//! ## Example
//!
//! ```rust
//! use tzbucket_core::prelude::*;
//!
//! // Parse a timestamp
//! let instant = parse_timestamp("2026-03-29T00:15:00Z", TimestampFormat::Rfc3339).unwrap();
//!
//! // Get timezone
//! let tz = parse_tz("Europe/Berlin").unwrap();
//!
//! // Compute day bucket
//! let bucket = compute_bucket(instant, tz, Interval::Day, None);
//!
//! println!("Bucket key: {}", bucket.key);
//! println!("Start (local): {}", bucket.start_local);
//! println!("End (local): {}", bucket.end_local);
//! ```

pub mod compute;
pub mod error;
pub mod models;
pub mod parse;
pub mod tz;

// Re-export commonly used types at the crate root
pub use compute::{compute_bucket, compute_bucket_from_string};
pub use error::{Result, TzBucketError};
pub use models::{
    AmbiguousPolicy, Bucket, BucketResult, InputTimestamp, Interval, NonexistentPolicy, Policy,
    WeekStart,
};
pub use parse::{TimestampFormat, parse_timestamp, parse_timestamp_auto};

/// Prelude module for convenient imports.
///
/// ```
/// use tzbucket_core::prelude::*;
/// ```
pub mod prelude {
    pub use crate::compute::{compute_bucket, compute_bucket_from_string};
    pub use crate::error::{Result, TzBucketError};
    pub use crate::models::*;
    pub use crate::parse::{TimestampFormat, parse_timestamp, parse_timestamp_auto};
    pub use crate::tz::parse_tz;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn full_workflow_day_bucket() {
        let instant = chrono::Utc
            .with_ymd_and_hms(2026, 3, 29, 0, 15, 0)
            .single()
            .unwrap();
        let tz = tz::parse_tz("Europe/Berlin").unwrap();
        let bucket = compute_bucket(instant, tz, Interval::Day, None);

        assert_eq!(bucket.key, "2026-03-29");
        // DST spring forward: 23-hour day
        assert_eq!(bucket.start_utc, "2026-03-28T23:00:00Z");
        assert_eq!(bucket.end_utc, "2026-03-29T22:00:00Z");
    }

    #[test]
    fn full_workflow_with_string_input() {
        let result = compute_bucket_from_string(
            "2026-03-29T00:15:00Z",
            TimestampFormat::Rfc3339,
            "Europe/Berlin",
            Interval::Day,
            None,
        )
        .unwrap();

        assert_eq!(result.bucket.key, "2026-03-29");
        assert_eq!(result.input.ts, "2026-03-29T00:15:00Z");
        assert_eq!(result.tz, "Europe/Berlin");
    }

    #[test]
    fn prelude_exports() {
        // Test that prelude exports work
        use crate::prelude::*;

        let _tz = parse_tz("UTC").unwrap();
        let _format = TimestampFormat::EpochMs;
        let _interval = Interval::Day;
    }
}
