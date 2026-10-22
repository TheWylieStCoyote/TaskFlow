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
            "audit_logs",
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
        "audit_logs" => vec!["id", "task_id", "changed_at", "action", "changes"],
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
            .map(|col| {
                item.get(*col)
                    .map(|value| format_csv_cell(entity_type, col, value))
                    .unwrap_or_default()
            })
            .collect();
        csv.push_str(&row.join(","));
        csv.push('\n');
    }

    Ok(csv)
}

/// Format one cell, applying entity-specific flattening where the generic
/// rendering would be unreadable.
fn format_csv_cell(entity_type: &str, column: &str, value: &serde_json::Value) -> String {
    match (entity_type, column) {
        ("audit_logs", "changes") => format_field_changes(value),
        _ => format_csv_value(value),
    }
}

/// Flatten an audit entry's `changes` array to `field: old -> new; ...`.
///
/// Items that are not field-change objects fall back to their JSON text so
/// malformed data still produces a cell.
fn format_field_changes(value: &serde_json::Value) -> String {
    let Some(changes) = value.as_array() else {
        return format_csv_value(value);
    };
    let items: Vec<String> = changes
        .iter()
        .map(|change| {
            let text = |key: &str| change.get(key).and_then(|v| v.as_str());
            match (text("field"), text("old_value"), text("new_value")) {
                (Some(field), Some(old), Some(new)) => format!("{field}: {old} -> {new}"),
                _ => change.to_string(),
            }
        })
        .collect();
    escape_csv_string(&items.join("; "))
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
    use serde_json::json;

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

    fn ok(data: serde_json::Value) -> PipeResponse<serde_json::Value> {
        PipeResponse::success(data)
    }

    #[test]
    fn test_format_csv_value_nested_values() {
        // Objects serialize to compact JSON and are quoted because of the
        // commas and quotes they contain.
        let obj = json!({"a": 1, "b": "x"});
        assert_eq!(
            format_csv_value(&obj),
            "\"{\"\"a\"\":1,\"\"b\"\":\"\"x\"\"}\""
        );

        // Non-string array items use their JSON representation.
        assert_eq!(
            format_csv_value(&json!([1, true, null, "s"])),
            "1;true;null;s"
        );

        // Array items containing commas are escaped after joining.
        assert_eq!(format_csv_value(&json!(["a,b", "c"])), "\"a,b;c\"");

        // Floats keep their representation.
        assert_eq!(format_csv_value(&json!(2.5)), "2.5");
    }

    #[test]
    fn test_serialize_response_json_pretty() {
        let out = serialize_response(&ok(json!({"deleted": "abc"})), OutputFormat::Json).unwrap();
        assert!(out.starts_with("{\n"));
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"]["deleted"], "abc");
        assert!(parsed.get("error").is_none());
        assert!(parsed.get("metadata").is_none());
    }

    #[test]
    fn test_serialize_response_yaml() {
        let out = serialize_response(&ok(json!({"deleted": "abc"})), OutputFormat::Yaml).unwrap();
        assert!(out.contains("success: true"));
        assert!(out.contains("deleted: abc"));
        assert!(!out.contains("error:"));
    }

    #[test]
    fn test_serialize_response_csv_delegates() {
        let data = json!({"tasks": [{"id": "1", "title": "A"}], "total": 1});
        let out = serialize_response(&ok(data), OutputFormat::Csv).unwrap();
        assert!(out.starts_with("id,title,status,"));
    }

    #[test]
    fn test_serialize_csv_error_response_falls_back_to_json() {
        let response = PipeResponse::error("NOT_FOUND", "Task not found");
        let out = serialize_csv(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "NOT_FOUND");
        assert_eq!(parsed["error"]["message"], "Task not found");
    }

    #[test]
    fn test_serialize_csv_non_array_payload_falls_back_to_json() {
        let out = serialize_csv(&ok(json!({"deleted": "abc"}))).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["data"]["deleted"], "abc");

        // A scalar payload has no "data" object to inspect.
        let out = serialize_csv(&ok(json!(42))).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["data"], 42);

        // A list field that is not an array is ignored.
        let out = serialize_csv(&ok(json!({"tasks": "not-an-array"}))).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["data"]["tasks"], "not-an-array");
    }

    #[test]
    fn test_serialize_csv_empty_array_is_empty_string() {
        let out = serialize_csv(&ok(json!({"tasks": [], "total": 0}))).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn test_serialize_csv_tasks_rows_and_quoting() {
        let data = json!({
            "tasks": [
                {
                    "id": "t1",
                    "title": "Buy milk, eggs",
                    "status": "todo",
                    "priority": "high",
                    "project_id": null,
                    "due_date": "2030-01-02",
                    "scheduled_date": null,
                    "tags": ["home", "errand"],
                    "estimated_minutes": 15,
                    "created_at": "2030-01-01T00:00:00Z",
                    "completed_at": null,
                    "ignored_extra_field": "nope"
                },
                {
                    "id": "t2",
                    "title": "Say \"hi\"\nthen leave",
                    "status": "done",
                    "priority": "none",
                    "tags": [],
                    "created_at": "2030-01-01T00:00:00Z"
                }
            ],
            "total": 2
        });
        let out = serialize_csv(&ok(data)).unwrap();
        let mut lines = out.split_terminator('\n');
        assert_eq!(
            lines.next().unwrap(),
            "id,title,status,priority,project_id,due_date,scheduled_date,tags,estimated_minutes,created_at,completed_at"
        );
        assert_eq!(
            lines.next().unwrap(),
            "t1,\"Buy milk, eggs\",todo,high,,2030-01-02,,home;errand,15,2030-01-01T00:00:00Z,"
        );
        // The embedded newline splits the second row across two physical lines.
        assert_eq!(lines.next().unwrap(), "t2,\"Say \"\"hi\"\"");
        assert_eq!(
            lines.next().unwrap(),
            "then leave\",done,none,,,,,,2030-01-01T00:00:00Z,"
        );
        assert!(lines.next().is_none());
        assert!(!out.contains("ignored_extra_field"));
    }

    #[test]
    fn test_serialize_csv_picks_first_known_list_field() {
        // "projects" comes after "tasks" in the known list, so tasks wins.
        let data = json!({
            "projects": [{"id": "p1", "name": "P"}],
            "tasks": [{"id": "t1", "title": "T"}]
        });
        let out = serialize_csv(&ok(data)).unwrap();
        assert!(out.starts_with("id,title,status"));
        assert!(out.contains("t1,T,"));
        assert!(!out.contains("p1"));
    }

    #[test]
    fn test_array_to_csv_known_entity_headers() {
        let cases = [
            (
                "projects",
                "id,name,status,description,parent_id,color,created_at",
            ),
            (
                "time_entries",
                "id,task_id,description,started_at,ended_at,duration_minutes",
            ),
            ("work_logs", "id,task_id,content,created_at,updated_at"),
            ("habits", "id,name,frequency,start_date,archived,tags"),
            (
                "goals",
                "id,name,status,description,start_date,due_date,manual_progress",
            ),
            (
                "key_results",
                "id,goal_id,name,target_value,current_value,unit,status",
            ),
            ("tags", "name,task_count,habit_count,total_count"),
        ];
        for (entity, header) in cases {
            let array = vec![json!({"id": "x", "name": "n"})];
            let out = array_to_csv(&array, entity).unwrap();
            let first = out.lines().next().unwrap();
            assert_eq!(first, header, "header for {entity}");
            assert_eq!(out.lines().count(), 2, "one data row for {entity}");
        }
    }

    #[test]
    fn test_array_to_csv_row_values_by_entity() {
        let tags = vec![json!({
            "name": "work",
            "task_count": 3,
            "habit_count": 1,
            "total_count": 4
        })];
        assert_eq!(
            array_to_csv(&tags, "tags").unwrap(),
            "name,task_count,habit_count,total_count\nwork,3,1,4\n"
        );

        let habits = vec![json!({
            "id": "h1",
            "name": "Run",
            "frequency": {"type": "daily"},
            "start_date": "2030-01-01",
            "archived": false,
            "tags": ["health"]
        })];
        assert_eq!(
            array_to_csv(&habits, "habits").unwrap(),
            "id,name,frequency,start_date,archived,tags\nh1,Run,\"{\"\"type\"\":\"\"daily\"\"}\",2030-01-01,false,health\n"
        );
    }

    #[test]
    fn test_array_to_csv_unknown_entity_autodetects_columns() {
        let array = vec![
            json!({"alpha": 1, "beta": "two"}),
            json!({"alpha": 3, "gamma": "ignored"}),
        ];
        let out = array_to_csv(&array, "widgets").unwrap();
        // Columns come from the first object's keys only (sorted by serde_json's map).
        assert_eq!(out, "alpha,beta\n1,two\n3,\n");
    }

    #[test]
    fn test_array_to_csv_unknown_entity_non_object_errors() {
        let array = vec![json!("just a string"), json!(2)];
        let err = array_to_csv(&array, "widgets").unwrap_err();
        assert_eq!(err.code, "SERIALIZE_ERROR");
        assert_eq!(err.message, "Cannot determine CSV columns");
    }

    #[test]
    fn test_array_to_csv_empty_unknown_entity() {
        assert_eq!(array_to_csv(&[], "widgets").unwrap(), "");
    }
}
