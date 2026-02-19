use std::process::ExitCode;

use clap::Parser;

mod bucket_cmd;
mod cli;
mod error;
mod explain_cmd;
mod range_cmd;
mod shared;

use bucket_cmd::run_bucket;
use cli::{Cli, Commands};
use error::{output_format_hint, parse_output_format, render_error};
use explain_cmd::run_explain;
use range_cmd::run_range;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Bucket(args) => {
            let fallback = output_format_hint(&args.output_format);
            let output_format = match parse_output_format(&args.output_format) {
                Ok(format) => format,
                Err(err) => return render_error(&err, fallback),
            };

            match run_bucket(args, output_format) {
                Ok(code) => code,
                Err(err) => render_error(&err, output_format),
            }
        }
        Commands::Range(args) => {
            let fallback = output_format_hint(&args.output_format);
            let output_format = match parse_output_format(&args.output_format) {
                Ok(format) => format,
                Err(err) => return render_error(&err, fallback),
            };

            match run_range(args, output_format) {
                Ok(code) => code,
                Err(err) => render_error(&err, output_format),
            }
        }
        Commands::Explain(args) => {
            let fallback = output_format_hint(&args.output_format);
            let output_format = match parse_output_format(&args.output_format) {
                Ok(format) => format,
                Err(err) => return render_error(&err, fallback),
            };

            match run_explain(args, output_format) {
                Ok(code) => code,
                Err(err) => render_error(&err, output_format),
            }
        }
    }
}
