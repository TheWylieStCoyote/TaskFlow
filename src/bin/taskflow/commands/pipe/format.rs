//! Output format serialization for the pipe interface.
//!
//! Supports JSON, YAML, and CSV output formats.

use serde::Serialize;

use crate::commands::pipe::types::{OutputFormat, PipeError, PipeResponse};

/// Serialize a successful response to the specified format.
pub fn serialize_response<T: Serialize>(
    response: &PipeResponse<T>,
    format: OutputFormat,
) -> Result<String, PipeError> {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(response).map_err(|e| {
            PipeError::new("SERIALIZE_ERROR", format!("JSON serialization failed: {e}"))
        }),
        OutputFormat::Yaml => serde_yaml::to_string(response).map_err(|e| {
            PipeError::new("SERIALIZE_ERROR", format!("YAML serialization failed: {e}"))
        }),
        OutputFormat::Csv => serialize_csv(response),
    }
}

/// Serialize to CSV format.
///
/// CSV only works for list operations that return arrays.
/// Falls back to JSON for non-list responses.
fn serialize_csv<T: Serialize>(response: &PipeResponse<T>) -> Result<String, PipeError> {
    // First convert to JSON value to inspect structure
    let value = serde_json::to_value(response)
        .map_err(|e| PipeError::new("SERIALIZE_ERROR", format!("Serialization failed: {e}")))?;

    // If error response, fall back to JSON
    if !response.success {
        return serde_json::to_string_pretty(response).map_err(|e| {
            PipeError::new("SERIALIZE_ERROR", format!("JSON serialization failed: {e}"))
        });
    }

    // Try to find an array in the data
    if let Some(data) = value.get("data") {
        // Check for known list fields
        let list_fields = [
            "tasks",
            "projects",
            "time_entries",
            "work_logs",
            "habits",
            "goals",
            "key_results",
            "tags",
        ];

        for field in list_fields {
            if let Some(array) = data.get(field).and_then(|v| v.as_array()) {
                return array_to_csv(array, field);
            }
        }
    }

    // If no list found, fall back to JSON
    serde_json::to_string_pretty(response)
        .map_err(|e| PipeError::new("SERIALIZE_ERROR", format!("JSON serialization failed: {e}")))
}

/// Convert a JSON array to CSV format.
fn array_to_csv(array: &[serde_json::Value], entity_type: &str) -> Result<String, PipeError> {
    if array.is_empty() {
        return Ok(String::new());
    }

    // Define columns based on entity type
    let columns: Vec<&str> = match entity_type {
        "tasks" => vec![
            "id",
            "title",
            "status",
            "priority",
            "project_id",
            "due_date",
            "scheduled_date",
            "tags",
            "estimated_minutes",
            "created_at",
            "completed_at",
        ],
        "projects" => vec![
            "id",
            "name",
            "status",
            "description",
            "parent_id",
            "color",
            "created_at",
        ],
        "time_entries" => vec![
            "id",
            "task_id",
            "description",
            "started_at",
            "ended_at",
            "duration_minutes",
        ],
        "work_logs" => vec!["id", "task_id", "content", "created_at", "updated_at"],
        "habits" => vec!["id", "name", "frequency", "start_date", "archived", "tags"],
        "goals" => vec![
            "id",
            "name",
            "status",
            "description",
            "start_date",
            "due_date",
            "manual_progress",
        ],
        "key_results" => vec![
            "id",
            "goal_id",
            "name",
            "target_value",
            "current_value",
            "unit",
            "status",
        ],
        "tags" => vec!["name", "task_count", "habit_count", "total_count"],
        _ => {
            // Auto-detect columns from first object
            if let Some(first) = array.first().and_then(|v| v.as_object()) {
                first.keys().map(|s| s.as_str()).collect()
            } else {
                return Err(PipeError::new(
                    "SERIALIZE_ERROR",
                    "Cannot determine CSV columns",
                ));
            }
        }
    };

    let mut csv = String::new();

    // Header row
    csv.push_str(&columns.join(","));
    csv.push('\n');

    // Data rows
    for item in array {
        let row: Vec<String> = columns
            .iter()
            .map(|col| item.get(*col).map(format_csv_value).unwrap_or_default())
            .collect();
        csv.push_str(&row.join(","));
        csv.push('\n');
    }

    Ok(csv)
}

/// Format a JSON value for CSV output.
fn format_csv_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => escape_csv_string(s),
        serde_json::Value::Array(arr) => {
            // Join array values with semicolons
            let items: Vec<String> = arr
                .iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    _ => v.to_string(),
                })
                .collect();
            escape_csv_string(&items.join(";"))
        }
        serde_json::Value::Object(_) => {
            // For objects, serialize to compact JSON
            escape_csv_string(&value.to_string())
        }
    }
}

/// Escape a string for CSV output.
fn escape_csv_string(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_string() {
        assert_eq!(escape_csv_string("simple"), "simple");
        assert_eq!(escape_csv_string("with,comma"), "\"with,comma\"");
        assert_eq!(escape_csv_string("with\"quote"), "\"with\"\"quote\"");
        assert_eq!(escape_csv_string("with\nnewline"), "\"with\nnewline\"");
    }

    #[test]
    fn test_format_csv_value() {
        assert_eq!(format_csv_value(&serde_json::Value::Null), "");
        assert_eq!(format_csv_value(&serde_json::json!(true)), "true");
        assert_eq!(format_csv_value(&serde_json::json!(42)), "42");
        assert_eq!(format_csv_value(&serde_json::json!("hello")), "hello");
        assert_eq!(format_csv_value(&serde_json::json!(["a", "b"])), "a;b");
    }
}
