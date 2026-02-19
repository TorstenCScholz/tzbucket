use std::fmt;
use std::process::ExitCode;

use serde::Serialize;

pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_INPUT_ERROR: u8 = 2;
pub const EXIT_RUNTIME_ERROR: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Input,
    Runtime,
}

#[derive(Debug)]
pub struct CliError {
    kind: ErrorKind,
    message: String,
    status: Option<&'static str>,
}

impl CliError {
    pub fn input(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Input,
            message: message.into(),
            status: None,
        }
    }

    pub fn policy(message: impl Into<String>, status: &'static str) -> Self {
        Self {
            kind: ErrorKind::Input,
            message: message.into(),
            status: Some(status),
        }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Runtime,
            message: message.into(),
            status: None,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self.kind {
            ErrorKind::Input => EXIT_INPUT_ERROR,
            ErrorKind::Runtime => EXIT_RUNTIME_ERROR,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

pub type CliResult<T> = std::result::Result<T, CliError>;

#[derive(Debug, Serialize)]
struct ErrorOutput {
    error: String,
    exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

pub fn render_error(err: &CliError, output_format: OutputFormat) -> ExitCode {
    match output_format {
        OutputFormat::Json => {
            let envelope = ErrorOutput {
                error: err.message.clone(),
                exit_code: err.exit_code(),
                status: err.status.map(str::to_string),
            };

            match serde_json::to_string_pretty(&envelope) {
                Ok(json) => eprintln!("{}", json),
                Err(_) => eprintln!("Error: {}", err.message),
            }
        }
        OutputFormat::Text => {
            eprintln!("Error: {}", err.message);
        }
    }

    ExitCode::from(err.exit_code())
}

pub fn output_format_hint(s: &str) -> OutputFormat {
    if s.eq_ignore_ascii_case("json") {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

pub fn parse_output_format(s: &str) -> CliResult<OutputFormat> {
    match s.to_lowercase().as_str() {
        "json" => Ok(OutputFormat::Json),
        "text" => Ok(OutputFormat::Text),
        _ => Err(CliError::input(format!(
            "Invalid output_format '{}'. Expected: json, text",
            s
        ))),
    }
}
