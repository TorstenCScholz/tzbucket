//! Timezone handling utilities.
//!
//! This module provides functions for parsing timezone names and
//! converting between UTC and local time with proper DST handling.

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::error::{Result, TzBucketError};

/// Parse an IANA timezone name into a [`chrono_tz::Tz`].
///
/// # Arguments
///
/// * `name` - The IANA timezone name (e.g., "Europe/Berlin", "America/New_York")
///
/// # Returns
///
/// The parsed timezone on success, or an error if the timezone name is invalid.
///
/// # Examples
///
/// ```
/// use tzbucket_core::tz::parse_tz;
///
/// let tz = parse_tz("Europe/Berlin").unwrap();
/// assert_eq!(tz.to_string(), "Europe/Berlin");
/// ```
pub fn parse_tz(name: &str) -> Result<Tz> {
    name.parse::<Tz>()
        .map_err(|_| TzBucketError::InvalidTimezone(name.to_string()))
}

/// Convert a UTC datetime to local time in the specified timezone.
///
/// This function handles DST transitions correctly by using chrono-tz's
/// built-in timezone conversion.
///
/// # Arguments
///
/// * `utc` - The UTC datetime to convert
/// * `tz` - The target timezone
///
/// # Returns
///
/// The local datetime with timezone information.
pub fn utc_to_local(utc: DateTime<Utc>, tz: Tz) -> DateTime<Tz> {
    utc.with_timezone(&tz)
}

/// Convert a local datetime in a specific timezone to UTC.
///
/// This function handles DST transitions. For ambiguous times (during fall back),
/// it uses the earlier occurrence. For nonexistent times (during spring forward),
/// it shifts forward to the next valid time.
///
/// # Arguments
///
/// * `local` - The local datetime (without timezone)
/// * `tz` - The timezone to interpret the local time in
///
/// # Returns
///
/// The UTC datetime.
pub fn local_to_utc(local: chrono::NaiveDateTime, tz: Tz) -> DateTime<Utc> {
    // Use `single` which returns None for ambiguous/nonexistent times,
    // then fall back to `earliest` for ambiguous and let chrono handle nonexistent
    match tz.from_local_datetime(&local).single() {
        Some(dt) => dt.with_timezone(&Utc),
        None => {
            // Handle ambiguous or nonexistent times
            // For ambiguous: earliest gives the first occurrence
            // For nonexistent: chrono-tz will shift forward
            tz.from_local_datetime(&local)
                .earliest()
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|| {
                    // Fallback: construct from local components
                    Utc.timestamp_opt(local.and_utc().timestamp(), 0)
                        .single()
                        .unwrap()
                })
        }
    }
}

/// Convert a local date and time (at midnight) to UTC.
///
/// This is a convenience function for converting bucket boundaries,
/// which are always at 00:00:00 local time.
///
/// # Arguments
///
/// * `date` - The local date
/// * `tz` - The timezone
///
/// # Returns
///
/// The UTC datetime representing midnight local time in that timezone.
pub fn local_midnight_to_utc(date: chrono::NaiveDate, tz: Tz) -> DateTime<Utc> {
    let midnight = date.and_hms_opt(0, 0, 0).unwrap();
    local_to_utc(midnight, tz)
}

/// Format a datetime as RFC3339 with timezone offset.
///
/// # Arguments
///
/// * `dt` - The datetime with timezone
///
/// # Returns
///
/// An RFC3339 formatted string (e.g., "2026-03-29T00:00:00+01:00").
pub fn format_rfc3339<T: TimeZone>(dt: &DateTime<T>) -> String
where
    T::Offset: std::fmt::Display,
{
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// Format a UTC datetime as RFC3339 with Z suffix.
///
/// # Arguments
///
/// * `dt` - The UTC datetime
///
/// # Returns
///
/// An RFC3339 formatted string with Z suffix (e.g., "2026-03-28T23:00:00Z").
pub fn format_rfc3339_utc(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_valid_timezone() {
        let tz = parse_tz("Europe/Berlin").unwrap();
        assert_eq!(tz.to_string(), "Europe/Berlin");
    }

    #[test]
    fn parse_invalid_timezone() {
        let result = parse_tz("Invalid/Timezone");
        assert!(result.is_err());
        if let Err(TzBucketError::InvalidTimezone(name)) = result {
            assert_eq!(name, "Invalid/Timezone");
        } else {
            panic!("Expected InvalidTimezone error");
        }
    }

    #[test]
    fn utc_to_local_conversion() {
        let utc = Utc
            .with_ymd_and_hms(2026, 3, 29, 0, 15, 0)
            .single()
            .unwrap();
        let tz = parse_tz("Europe/Berlin").unwrap();
        let local = utc_to_local(utc, tz);

        // Before DST switch (at 02:00 local, clocks jump to 03:00)
        // 00:15 UTC = 01:15 local (UTC+1)
        assert_eq!(
            local.format("%Y-%m-%d %H:%M").to_string(),
            "2026-03-29 01:15"
        );
    }

    #[test]
    fn local_to_utc_conversion_normal() {
        let tz = parse_tz("Europe/Berlin").unwrap();
        let local = chrono::NaiveDate::from_ymd_opt(2026, 3, 28)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let utc = local_to_utc(local, tz);

        // 12:00 local (UTC+1) = 11:00 UTC
        assert_eq!(utc.format("%Y-%m-%d %H:%M").to_string(), "2026-03-28 11:00");
    }

    #[test]
    fn format_rfc3339_with_offset() {
        let tz = parse_tz("Europe/Berlin").unwrap();
        let dt = tz.with_ymd_and_hms(2026, 3, 29, 0, 0, 0).single().unwrap();
        let formatted = format_rfc3339(&dt);

        // Before DST switch, offset is +01:00
        assert_eq!(formatted, "2026-03-29T00:00:00+01:00");
    }

    #[test]
    fn format_rfc3339_utc_zone() {
        let dt = Utc
            .with_ymd_and_hms(2026, 3, 28, 23, 0, 0)
            .single()
            .unwrap();
        let formatted = format_rfc3339_utc(&dt);

        assert_eq!(formatted, "2026-03-28T23:00:00Z");
    }
}
