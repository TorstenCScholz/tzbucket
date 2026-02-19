use clap::{Parser, Subcommand};

/// DST-safe time bucketing tool
#[derive(Parser, Debug)]
#[command(name = "tzbucket")]
#[command(about = "DST-safe time bucketing tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Compute time buckets for timestamps
    Bucket(BucketArgs),
    /// Generate all buckets in a time range
    Range(RangeArgs),
    /// Explain local time resolution (DST handling)
    Explain(ExplainArgs),
}

#[derive(clap::Args, Debug)]
pub struct BucketArgs {
    /// IANA timezone (e.g., Europe/Berlin)
    #[arg(short, long, default_value = "UTC")]
    pub tz: String,

    /// Bucket interval: day, week, month
    #[arg(short = 'i', long, default_value = "day")]
    pub interval: String,

    /// Week start day: monday or sunday (for week interval)
    #[arg(long, default_value = "monday")]
    pub week_start: String,

    /// Input format: epoch_ms, epoch_s, rfc3339
    #[arg(short = 'f', long, default_value = "epoch_ms")]
    pub format: String,

    /// Output format: json, text
    #[arg(long, default_value = "text")]
    pub output_format: String,

    /// Input file path (use - for stdin)
    #[arg(long, default_value = "-")]
    pub input: String,

    /// Read from stdin
    #[arg(long)]
    pub stdin: bool,
}

#[derive(clap::Args, Debug)]
pub struct RangeArgs {
    /// IANA timezone
    #[arg(short, long)]
    pub tz: String,

    /// Bucket interval: day, week, month
    #[arg(short = 'i', long, default_value = "day")]
    pub interval: String,

    /// Week start day
    #[arg(long, default_value = "monday")]
    pub week_start: String,

    /// Start of range (inclusive, RFC3339)
    #[arg(long)]
    pub start: String,

    /// End of range (exclusive, RFC3339)
    #[arg(long)]
    pub end: String,

    /// Output format: json, text
    #[arg(long, default_value = "json")]
    pub output_format: String,
}

#[derive(clap::Args, Debug)]
pub struct ExplainArgs {
    /// IANA timezone
    #[arg(short, long)]
    pub tz: String,

    /// Local time string (without offset, e.g., 2026-03-29T02:30:00)
    #[arg(long)]
    pub local: String,

    /// Policy for nonexistent times: error, shift_forward
    #[arg(long, default_value = "error")]
    pub policy_nonexistent: String,

    /// Policy for ambiguous times: error, first, second
    #[arg(long, default_value = "error")]
    pub policy_ambiguous: String,

    /// Output format: json, text
    #[arg(long, default_value = "json")]
    pub output_format: String,
}
