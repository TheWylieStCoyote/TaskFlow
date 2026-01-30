//! Pipe interface for scripting integration.
//!
//! This module provides a stdin/stdout interface for external tools to interact
//! with TaskFlow. Commands are sent as JSON objects on stdin, and responses are
//! written to stdout in JSON, YAML, or CSV format.
//!
//! # Usage
//!
//! ```bash
//! # List all tasks
//! echo '{"operation":"list","entity":"task"}' | taskflow pipe
//!
//! # Create a task
//! echo '{"operation":"create","entity":"task","data":{"title":"Buy milk"}}' | taskflow pipe
//!
//! # Export all data as YAML
//! echo '{"operation":"export","entity":"task"}' | taskflow pipe --format yaml
//! ```

pub mod format;
pub mod handlers;
pub mod types;

#[cfg(test)]
mod tests;

use std::io::{BufRead, Write};

use tracing::warn;

use crate::cli::Cli;
use crate::load_model_for_cli;

use types::{OutputFormat, PipeError, PipeRequest, PipeResponse, ResponseMetadata};

/// Run the pipe interface.
///
/// Reads JSON commands from stdin line by line, processes them, and writes
/// responses to stdout in the specified format.
pub fn run_pipe(cli: &Cli, format_str: &str) -> anyhow::Result<()> {
    let format = match format_str.to_lowercase().as_str() {
        "json" => OutputFormat::Json,
        "yaml" | "yml" => OutputFormat::Yaml,
        "csv" => OutputFormat::Csv,
        _ => {
            warn!(format = %format_str, "Unknown output format, using JSON");
            OutputFormat::Json
        }
    };

    let mut model = load_model_for_cli(cli)?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                let response =
                    create_error_response("IO_ERROR", format!("Failed to read stdin: {e}"));
                write_response(&mut stdout, &response, format)?;
                continue;
            }
        };

        // Skip empty lines
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse the request
        let request: PipeRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let response =
                    create_error_response("PARSE_ERROR", format!("Failed to parse request: {e}"));
                write_response(&mut stdout, &response, format)?;
                continue;
            }
        };

        // Process the request
        let response = process_request(&mut model, request);
        write_response(&mut stdout, &response, format)?;
    }

    // Save any changes
    if let Err(e) = model.save() {
        warn!(error = %e, "Failed to save model after pipe operations");
    }

    Ok(())
}

/// Process a single request and return a response.
fn process_request(
    model: &mut taskflow::app::Model,
    request: PipeRequest,
) -> PipeResponse<serde_json::Value> {
    match handlers::dispatch(model, &request) {
        Ok(data) => {
            // Extract metadata if present
            let metadata = extract_metadata(&data);
            PipeResponse {
                success: true,
                data: Some(data),
                error: None,
                metadata,
            }
        }
        Err(error) => PipeResponse {
            success: false,
            data: None,
            error: Some(error),
            metadata: None,
        },
    }
}

/// Extract metadata from a response data object.
fn extract_metadata(data: &serde_json::Value) -> Option<ResponseMetadata> {
    let total = data
        .get("total")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let offset = data
        .get("offset")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let limit = data
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    if total.is_some() || offset.is_some() || limit.is_some() {
        Some(ResponseMetadata {
            total,
            offset,
            limit,
        })
    } else {
        None
    }
}

/// Create an error response.
fn create_error_response(code: &str, message: String) -> PipeResponse<serde_json::Value> {
    PipeResponse {
        success: false,
        data: None,
        error: Some(PipeError::new(code, message)),
        metadata: None,
    }
}

/// Write a response to stdout.
fn write_response<W: Write>(
    writer: &mut W,
    response: &PipeResponse<serde_json::Value>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let output = format::serialize_response(response, format)?;
    writeln!(writer, "{}", output)?;
    writer.flush()?;
    Ok(())
}
