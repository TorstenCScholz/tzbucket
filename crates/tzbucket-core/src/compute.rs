//! Bucket computation logic.
//!
//! This module provides the core bucket computation algorithm that
//! correctly handles DST transitions by computing boundaries in local
//! time and converting each boundary independently to UTC.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;

use crate::models::{Bucket, BucketResult, InputTimestamp, Interval, WeekStart};
use crate::parse::{TimestampFormat, parse_timestamp};
use crate::tz::{
    format_rfc3339, format_rfc3339_utc, local_midnight_to_utc, parse_tz, utc_to_local,
};

/// Compute a time bucket for a given UTC instant.
///
/// This function computes bucket boundaries by:
/// 1. Converting the UTC instant to local time in the specified timezone
/// 2. Finding the bucket start and end in local time (at 00:00:00)
/// 3. Converting each boundary independently back to UTC
///
/// This approach correctly handles DST transitions, resulting in
/// 23-hour buckets on spring-forward days and 25-hour buckets on
/// fall-back days.
///
/// # Arguments
///
/// * `instant` - The UTC instant to bucket
/// * `tz` - The timezone for bucket computation
/// * `interval` - The bucket granularity (day/week/month)
/// * `week_start` - The week start day (only used for Week interval)
///
/// # Returns
///
/// A [`Bucket`] with the computed boundaries.
///
/// # Examples
///
/// ```
/// use tzbucket_core::compute::compute_bucket;
/// use tzbucket_core::models::{Interval, WeekStart};
/// use tzbucket_core::tz::parse_tz;
/// use chrono::{TimeZone, Utc};
///
/// let instant = Utc.with_ymd_and_hms(2026, 3, 29, 0, 15, 0).single().unwrap();
/// let tz = parse_tz("Europe/Berlin").unwrap();
/// let bucket = compute_bucket(instant, tz, Interval::Day, None);
///
/// assert_eq!(bucket.key, "2026-03-29");
/// ```
pub fn compute_bucket(
    instant: DateTime<Utc>,
    tz: Tz,
    interval: Interval,
    week_start: Option<WeekStart>,
) -> Bucket {
    // Convert to local time
    let local = utc_to_local(instant, tz);

    // Compute bucket boundaries based on interval
    let (start_local_date, end_local_date, key) = match interval {
        Interval::Day => compute_day_bucket(&local),
        Interval::Week => compute_week_bucket(&local, week_start.unwrap_or_default()),
        Interval::Month => compute_month_bucket(&local),
    };

    // Convert boundaries to UTC (independently, to handle DST correctly)
    let start_utc = local_midnight_to_utc(start_local_date, tz);
    let end_utc = local_midnight_to_utc(end_local_date, tz);

    // Format local boundaries from the resolved UTC instants.
    // This avoids panicking in zones where local midnight can be nonexistent.
    let start_local_dt = start_utc.with_timezone(&tz);
    let end_local_dt = end_utc.with_timezone(&tz);

    Bucket {
        key,
        start_local: format_rfc3339(&start_local_dt),
        end_local: format_rfc3339(&end_local_dt),
        start_utc: format_rfc3339_utc(&start_utc),
        end_utc: format_rfc3339_utc(&end_utc),
    }
}

/// Compute day bucket boundaries.
fn compute_day_bucket(local: &DateTime<Tz>) -> (NaiveDate, NaiveDate, String) {
    let date = local.date_naive();
    let next_date = date + chrono::Duration::days(1);
    let key = format!("{}", date.format("%Y-%m-%d"));
    (date, next_date, key)
}

/// Compute week bucket boundaries.
///
/// The bucket key uses the week starting date in `YYYY-MM-DD` format.
/// This works for both Monday and Sunday week starts.
fn compute_week_bucket(
    local: &DateTime<Tz>,
    week_start: WeekStart,
) -> (NaiveDate, NaiveDate, String) {
    let date = local.date_naive();
    let weekday = date.weekday();

    // Calculate days since week start
    let days_from_week_start = match week_start {
        WeekStart::Monday => weekday.num_days_from_monday() as i64,
        WeekStart::Sunday => {
            // Sunday = 0, Monday = 1, ..., Saturday = 6
            weekday.num_days_from_sunday() as i64
        }
    };

    // Find the start of the week
    let week_start_date = date - chrono::Duration::days(days_from_week_start);
    let week_end_date = week_start_date + chrono::Duration::weeks(1);

    // Use week starting date as the key (YYYY-MM-DD format)
    let key = format!("{}", week_start_date.format("%Y-%m-%d"));
    (week_start_date, week_end_date, key)
}

/// Compute month bucket boundaries.
fn compute_month_bucket(local: &DateTime<Tz>) -> (NaiveDate, NaiveDate, String) {
    let date = local.date_naive();
    let year = date.year();
    let month = date.month();

    // First day of current month
    let month_start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();

    // First day of next month
    let month_end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };

    let key = format!("{}", date.format("%Y-%m"));
    (month_start, month_end, key)
}

/// Compute a bucket result from a timestamp string.
///
/// This is a convenience function that parses the timestamp, computes the bucket,
/// and returns a complete [`BucketResult`].
///
/// # Arguments
///
/// * `input` - The timestamp string to parse
/// * `format` - The timestamp format
/// * `tz_name` - The IANA timezone name
/// * `interval` - The bucket granularity
/// * `week_start` - The week start day (optional)
///
/// # Returns
///
/// A [`BucketResult`] on success, or an error if parsing fails.
pub fn compute_bucket_from_string(
    input: &str,
    format: TimestampFormat,
    tz_name: &str,
    interval: Interval,
    week_start: Option<WeekStart>,
) -> crate::error::Result<BucketResult> {
    // Parse timezone
    let tz = parse_tz(tz_name)?;

    // Parse timestamp
    let instant = parse_timestamp(input, format)?;

    // Compute bucket
    let bucket = compute_bucket(instant, tz, interval, week_start);

    // Create input timestamp
    let input_ts = InputTimestamp {
        ts: input.trim().to_string(),
        epoch_ms: instant.timestamp_millis(),
    };

    Ok(BucketResult {
        input: input_ts,
        tz: tz_name.to_string(),
        interval,
        bucket,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn get_berlin_tz() -> Tz {
        parse_tz("Europe/Berlin").unwrap()
    }

    #[test]
    fn day_bucket_normal_day() {
        // 2026-03-28 12:00 UTC = 2026-03-28 13:00 Berlin (before DST)
        let instant = Utc
            .with_ymd_and_hms(2026, 3, 28, 12, 0, 0)
            .single()
            .unwrap();
        let tz = get_berlin_tz();
        let bucket = compute_bucket(instant, tz, Interval::Day, None);

        assert_eq!(bucket.key, "2026-03-28");
        assert_eq!(bucket.start_local, "2026-03-28T00:00:00+01:00");
        assert_eq!(bucket.end_local, "2026-03-29T00:00:00+01:00");
        assert_eq!(bucket.start_utc, "2026-03-27T23:00:00Z");
        assert_eq!(bucket.end_utc, "2026-03-28T23:00:00Z");
    }

    #[test]
    fn day_bucket_dst_spring_forward() {
        // DST switch in Berlin: 2026-03-29 02:00 -> 03:00
        // 2026-03-29 00:15 UTC = 2026-03-29 01:15 Berlin (before DST switch)
        let instant = Utc
            .with_ymd_and_hms(2026, 3, 29, 0, 15, 0)
            .single()
            .unwrap();
        let tz = get_berlin_tz();
        let bucket = compute_bucket(instant, tz, Interval::Day, None);

        assert_eq!(bucket.key, "2026-03-29");
        // Start: 2026-03-29 00:00 local (before DST, +01:00)
        assert_eq!(bucket.start_local, "2026-03-29T00:00:00+01:00");
        // End: 2026-03-30 00:00 local (after DST, +02:00)
        assert_eq!(bucket.end_local, "2026-03-30T00:00:00+02:00");
        // Start UTC: 2026-03-28 23:00Z
        assert_eq!(bucket.start_utc, "2026-03-28T23:00:00Z");
        // End UTC: 2026-03-29 22:00Z (23-hour day!)
        assert_eq!(bucket.end_utc, "2026-03-29T22:00:00Z");
    }

    #[test]
    fn day_bucket_dst_fall_back() {
        // DST switch in Berlin: 2026-10-25 03:00 -> 02:00
        // 2026-10-25 01:00 UTC = 2026-10-25 03:00 Berlin (after DST switch back)
        let instant = Utc
            .with_ymd_and_hms(2026, 10, 25, 1, 0, 0)
            .single()
            .unwrap();
        let tz = get_berlin_tz();
        let bucket = compute_bucket(instant, tz, Interval::Day, None);

        assert_eq!(bucket.key, "2026-10-25");
        // Start: 2026-10-25 00:00 local (before DST switch back, +02:00)
        assert_eq!(bucket.start_local, "2026-10-25T00:00:00+02:00");
        // End: 2026-10-26 00:00 local (after DST switch back, +01:00)
        assert_eq!(bucket.end_local, "2026-10-26T00:00:00+01:00");
        // Start UTC: 2026-10-24 22:00Z
        assert_eq!(bucket.start_utc, "2026-10-24T22:00:00Z");
        // End UTC: 2026-10-25 23:00Z (25-hour day!)
        assert_eq!(bucket.end_utc, "2026-10-25T23:00:00Z");
    }

    #[test]
    fn week_bucket_monday_start() {
        // 2026-03-29 is a Sunday
        // With Monday start, week should be 2026-03-23 (Mon) to 2026-03-30 (Mon)
        let instant = Utc
            .with_ymd_and_hms(2026, 3, 29, 12, 0, 0)
            .single()
            .unwrap();
        let tz = get_berlin_tz();
        let bucket = compute_bucket(instant, tz, Interval::Week, Some(WeekStart::Monday));

        // Key is the week starting date
        assert_eq!(bucket.key, "2026-03-23");
        assert!(bucket.start_local.starts_with("2026-03-23"));
        assert!(bucket.end_local.starts_with("2026-03-30"));
    }

    #[test]
    fn week_bucket_sunday_start() {
        // 2026-03-29 is a Sunday
        // With Sunday start, this should be the start of a new week
        let instant = Utc
            .with_ymd_and_hms(2026, 3, 29, 12, 0, 0)
            .single()
            .unwrap();
        let tz = get_berlin_tz();
        let bucket = compute_bucket(instant, tz, Interval::Week, Some(WeekStart::Sunday));

        // Key is the week starting date (Sunday 2026-03-29)
        assert_eq!(bucket.key, "2026-03-29");
        assert!(bucket.start_local.starts_with("2026-03-29"));
        assert!(bucket.end_local.starts_with("2026-04-05"));
    }

    #[test]
    fn month_bucket() {
        let instant = Utc
            .with_ymd_and_hms(2026, 3, 15, 12, 0, 0)
            .single()
            .unwrap();
        let tz = get_berlin_tz();
        let bucket = compute_bucket(instant, tz, Interval::Month, None);

        assert_eq!(bucket.key, "2026-03");
        assert!(bucket.start_local.starts_with("2026-03-01"));
        assert!(bucket.end_local.starts_with("2026-04-01"));
    }

    #[test]
    fn month_bucket_december() {
        let instant = Utc
            .with_ymd_and_hms(2026, 12, 15, 12, 0, 0)
            .single()
            .unwrap();
        let tz = get_berlin_tz();
        let bucket = compute_bucket(instant, tz, Interval::Month, None);

        assert_eq!(bucket.key, "2026-12");
        assert!(bucket.start_local.starts_with("2026-12-01"));
        assert!(bucket.end_local.starts_with("2027-01-01"));
    }

    #[test]
    fn compute_bucket_from_string_epoch_ms() {
        // 2026-03-29 00:15:00 UTC in epoch milliseconds
        // Calculate: 2026-03-29 00:15:00 UTC
        let instant = Utc
            .with_ymd_and_hms(2026, 3, 29, 0, 15, 0)
            .single()
            .unwrap();
        let epoch_ms = instant.timestamp_millis();

        let result = compute_bucket_from_string(
            &epoch_ms.to_string(),
            TimestampFormat::EpochMs,
            "Europe/Berlin",
            Interval::Day,
            None,
        )
        .unwrap();

        assert_eq!(result.bucket.key, "2026-03-29");
        assert_eq!(result.tz, "Europe/Berlin");
    }

    #[test]
    fn compute_bucket_from_string_rfc3339() {
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
    }
}
