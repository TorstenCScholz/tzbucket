use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use similar::{ChangeTag, TextDiff};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_dir() -> PathBuf {
    project_root().join("fixtures")
}

fn golden_dir() -> PathBuf {
    project_root().join("golden")
}

fn update_golden() -> bool {
    std::env::var("UPDATE_GOLDEN").is_ok()
}

fn diff_strings(expected: &str, actual: &str) -> String {
    let diff = TextDiff::from_lines(expected, actual);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        out.push_str(&format!("{sign}{change}"));
    }
    out
}

/// Run the tzbucket CLI with the given arguments
fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tzbucket"))
        .args(args)
        .output()
        .expect("Failed to run tzbucket")
}

/// Compare two JSON strings for equality (ignoring whitespace differences)
fn assert_json_eq(actual: &str, expected: &str) {
    let actual_json: serde_json::Value =
        serde_json::from_str(actual).expect("Invalid JSON in actual");
    let expected_json: serde_json::Value =
        serde_json::from_str(expected).expect("Invalid JSON in expected");

    assert_eq!(actual_json, expected_json, "JSON output mismatch");
}

/// Compare JSON lines output (one JSON object per line)
fn assert_json_lines_eq(actual: &str, expected: &str) {
    let actual_lines: Vec<&str> = actual.lines().filter(|l| !l.trim().is_empty()).collect();
    let expected_lines: Vec<&str> = expected.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(
        actual_lines.len(),
        expected_lines.len(),
        "Different number of JSON lines: got {} expected {}",
        actual_lines.len(),
        expected_lines.len()
    );

    for (i, (actual_line, expected_line)) in
        actual_lines.iter().zip(expected_lines.iter()).enumerate()
    {
        let actual_json: serde_json::Value =
            serde_json::from_str(actual_line).unwrap_or_else(|e| {
                panic!(
                    "Invalid JSON on line {}: {}\nContent: {}",
                    i + 1,
                    e,
                    actual_line
                )
            });
        let expected_json: serde_json::Value =
            serde_json::from_str(expected_line).unwrap_or_else(|e| {
                panic!(
                    "Invalid JSON on line {}: {}\nContent: {}",
                    i + 1,
                    e,
                    expected_line
                )
            });

        assert_eq!(
            actual_json,
            expected_json,
            "JSON mismatch on line {}",
            i + 1
        );
    }
}

// =============================================================================
// Berlin DST Tests
// =============================================================================

#[test]
fn test_berlin_dst_start_2026() {
    let fixture_path = fixture_dir().join("berlin_dst_start_2026.txt");
    let output = run_cli(&[
        "bucket",
        "--tz",
        "Europe/Berlin",
        "--interval",
        "day",
        "--format",
        "rfc3339",
        "--output-format",
        "json",
        "--input",
        fixture_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");
    let expected = fs::read_to_string(golden_dir().join("berlin_dst_start_2026.json"))
        .expect("Failed to read golden file");

    assert_json_lines_eq(&actual, &expected);

    // Verify 23-hour day on DST transition
    // March 29: start_utc 23:00Z (28th), end_utc 22:00Z (29th) = 23 hours
    // This is verified by the golden file content
}

#[test]
fn test_berlin_dst_end_2026() {
    let fixture_path = fixture_dir().join("berlin_dst_end_2026.txt");
    let output = run_cli(&[
        "bucket",
        "--tz",
        "Europe/Berlin",
        "--interval",
        "day",
        "--format",
        "rfc3339",
        "--output-format",
        "json",
        "--input",
        fixture_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");
    let expected = fs::read_to_string(golden_dir().join("berlin_dst_end_2026.json"))
        .expect("Failed to read golden file");

    assert_json_lines_eq(&actual, &expected);

    // Verify 25-hour day on DST transition
    // October 25: start_utc 22:00Z (24th), end_utc 23:00Z (25th) = 25 hours
    // This is verified by the golden file content
}

// =============================================================================
// New York DST Tests
// =============================================================================

#[test]
fn test_newyork_dst_start_2026() {
    let fixture_path = fixture_dir().join("newyork_dst_start_2026.txt");
    let output = run_cli(&[
        "bucket",
        "--tz",
        "America/New_York",
        "--interval",
        "day",
        "--format",
        "rfc3339",
        "--output-format",
        "json",
        "--input",
        fixture_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");
    let expected = fs::read_to_string(golden_dir().join("newyork_dst_start_2026.json"))
        .expect("Failed to read golden file");

    assert_json_lines_eq(&actual, &expected);

    // Verify 23-hour day on DST transition
    // March 8: start_utc 05:00Z, end_utc 04:00Z (next day) = 23 hours
}

#[test]
fn test_newyork_dst_end_2026() {
    let fixture_path = fixture_dir().join("newyork_dst_end_2026.txt");
    let output = run_cli(&[
        "bucket",
        "--tz",
        "America/New_York",
        "--interval",
        "day",
        "--format",
        "rfc3339",
        "--output-format",
        "json",
        "--input",
        fixture_path.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");
    let expected = fs::read_to_string(golden_dir().join("newyork_dst_end_2026.json"))
        .expect("Failed to read golden file");

    assert_json_lines_eq(&actual, &expected);

    // Verify 25-hour day on DST transition
    // November 1: start_utc 04:00Z, end_utc 05:00Z (next day) = 25 hours
}

// =============================================================================
// Range Tests
// =============================================================================

#[test]
fn test_range_berlin_march_2026() {
    let output = run_cli(&[
        "range",
        "--tz",
        "Europe/Berlin",
        "--interval",
        "day",
        "--start",
        "2026-03-27T00:00:00Z",
        "--end",
        "2026-03-31T00:00:00Z",
        "--output-format",
        "json",
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");
    let expected = fs::read_to_string(golden_dir().join("range_berlin_march_2026.json"))
        .expect("Failed to read golden file");

    assert_json_eq(&actual, &expected);
}

// =============================================================================
// Explain Tests - Nonexistent Time
// =============================================================================

#[test]
fn test_explain_nonexistent_shift() {
    let output = run_cli(&[
        "explain",
        "--tz",
        "Europe/Berlin",
        "--local",
        "2026-03-29T02:30:00",
        "--policy-nonexistent",
        "shift_forward",
        "--output-format",
        "json",
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");
    let expected = fs::read_to_string(golden_dir().join("explain_nonexistent_shift.json"))
        .expect("Failed to read golden file");

    assert_json_eq(&actual, &expected);
}

#[test]
fn test_explain_nonexistent_error() {
    let output = run_cli(&[
        "explain",
        "--tz",
        "Europe/Berlin",
        "--local",
        "2026-03-29T02:30:00",
        "--policy-nonexistent",
        "error",
        "--output-format",
        "json",
    ]);

    // Should exit with code 2 for policy error
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 for nonexistent time error, got {:?}",
        output.status.code()
    );

    // Verify error output
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Nonexistent time"),
        "Expected error message about nonexistent time, got: {}",
        stderr
    );
}

// =============================================================================
// Explain Tests - Ambiguous Time
// =============================================================================

#[test]
fn test_explain_ambiguous_first() {
    let output = run_cli(&[
        "explain",
        "--tz",
        "Europe/Berlin",
        "--local",
        "2026-10-25T02:30:00",
        "--policy-ambiguous",
        "first",
        "--output-format",
        "json",
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");
    let expected = fs::read_to_string(golden_dir().join("explain_ambiguous_first.json"))
        .expect("Failed to read golden file");

    assert_json_eq(&actual, &expected);
}

#[test]
fn test_explain_ambiguous_second() {
    let output = run_cli(&[
        "explain",
        "--tz",
        "Europe/Berlin",
        "--local",
        "2026-10-25T02:30:00",
        "--policy-ambiguous",
        "second",
        "--output-format",
        "json",
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");

    // Parse and verify the result
    let json: serde_json::Value = serde_json::from_str(&actual).expect("Invalid JSON output");

    assert_eq!(json["status"], "ambiguous");
    assert_eq!(json["resolution"]["policy"], "second");
    // The second occurrence should have +01:00 offset (after DST ends)
    assert!(
        json["resolution"]["result"]
            .as_str()
            .unwrap()
            .contains("+01:00")
    );
}

#[test]
fn test_explain_ambiguous_error() {
    let output = run_cli(&[
        "explain",
        "--tz",
        "Europe/Berlin",
        "--local",
        "2026-10-25T02:30:00",
        "--policy-ambiguous",
        "error",
        "--output-format",
        "json",
    ]);

    // Should exit with code 2 for policy error
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 for ambiguous time error, got {:?}",
        output.status.code()
    );

    // Verify error output
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Ambiguous time"),
        "Expected error message about ambiguous time, got: {}",
        stderr
    );
}

// =============================================================================
// Explain Tests - Normal Time
// =============================================================================

#[test]
fn test_explain_normal_time() {
    let output = run_cli(&[
        "explain",
        "--tz",
        "Europe/Berlin",
        "--local",
        "2026-06-15T14:30:00",
        "--output-format",
        "json",
    ]);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");
    let json: serde_json::Value = serde_json::from_str(&actual).expect("Invalid JSON output");

    assert_eq!(json["status"], "normal");
    assert!(json["resolution"].is_null());
}

// =============================================================================
// Legacy Golden Test (for backwards compatibility)
// Processes all DST-related fixture files
// =============================================================================

#[test]
fn golden_json_output() {
    let fixtures = fixture_dir();
    let golden = golden_dir();

    // Only process DST-related fixture files
    let dst_fixtures = [
        "berlin_dst_start_2026.txt",
        "berlin_dst_end_2026.txt",
        "newyork_dst_start_2026.txt",
        "newyork_dst_end_2026.txt",
    ];

    for fixture_name in &dst_fixtures {
        let fixture_path = fixtures.join(fixture_name);
        let stem = fixture_path.file_stem().unwrap().to_str().unwrap();
        let golden_path = golden.join(format!("{stem}.json"));

        // Determine timezone from fixture name
        let tz = if stem.contains("berlin") {
            "Europe/Berlin"
        } else if stem.contains("newyork") {
            "America/New_York"
        } else {
            "UTC"
        };

        let output = Command::new(env!("CARGO_BIN_EXE_tzbucket"))
            .args([
                "bucket",
                "--tz",
                tz,
                "--interval",
                "day",
                "--format",
                "rfc3339",
                "--output-format",
                "json",
                "--input",
                fixture_path.to_str().unwrap(),
            ])
            .output()
            .expect("Failed to execute tzbucket");

        assert!(
            output.status.success(),
            "tzbucket failed for {}: {}",
            stem,
            String::from_utf8_lossy(&output.stderr)
        );

        let actual = String::from_utf8(output.stdout).expect("Output is not valid UTF-8");

        if update_golden() {
            fs::create_dir_all(&golden).ok();
            fs::write(&golden_path, &actual)
                .unwrap_or_else(|e| panic!("Failed to write golden file {golden_path:?}: {e}"));
            eprintln!("Updated golden file: {golden_path:?}");
            continue;
        }

        let expected = fs::read_to_string(&golden_path).unwrap_or_else(|e| {
            panic!(
                "Golden file {golden_path:?} not found: {e}\n\
                 Hint: Run with UPDATE_GOLDEN=1 to generate golden files"
            )
        });

        if actual != expected {
            let diff = diff_strings(&expected, &actual);
            panic!(
                "Golden test mismatch for {stem}:\n\n\
                 {diff}\n\n\
                 Run with UPDATE_GOLDEN=1 to refresh snapshots"
            );
        }
    }
}
