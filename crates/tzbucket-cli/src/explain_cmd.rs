use std::process::ExitCode;

use chrono::{DateTime, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use serde::Serialize;
use tzbucket_core::{AmbiguousPolicy, NonexistentPolicy};

use crate::cli::ExplainArgs;
use crate::error::{CliError, CliResult, EXIT_SUCCESS, OutputFormat};
use crate::shared::{
    format_rfc3339, parse_ambiguous_policy, parse_nonexistent_policy, parse_tz_or_input_error,
};

pub fn run_explain(args: ExplainArgs, output_format: OutputFormat) -> CliResult<ExitCode> {
    let tz = parse_tz_or_input_error(&args.tz)?;
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
