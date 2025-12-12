//! Time entry CRUD operations for the pipe interface.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use taskflow::app::Model;
use taskflow::domain::{TaskId, TimeEntry, TimeEntryId};

use super::HandlerResult;
use crate::commands::pipe::types::{
    FilterParams, Operation, PipeError, PipeRequest, TimeEntryInput,
};

/// Handle time entry operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_time_entries(model, &request.filters),
        Operation::Get => get_time_entry(model, request.id.as_deref()),
        Operation::Create => create_time_entry(model, request.data.as_ref()),
        Operation::Update => update_time_entry(model, request.id.as_deref(), request.data.as_ref()),
        Operation::Delete => delete_time_entry(model, request.id.as_deref()),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

fn list_time_entries(model: &Model, filters: &Option<FilterParams>) -> HandlerResult {
    let mut entries: Vec<&TimeEntry> = model.time_entries.values().collect();

    // Apply filters
    if let Some(ref f) = filters {
        // Filter by task (using project_id field as task filter)
        if let Some(ref task_id_str) = f.project_id {
            if let Ok(uuid) = Uuid::parse_str(task_id_str) {
                let task_id = TaskId(uuid);
                entries.retain(|e| e.task_id == task_id);
            }
        }
    }

    // Sort by start time (newest first)
    entries.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    let total = entries.len();
    let offset = filters.as_ref().and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.as_ref().and_then(|f| f.limit).unwrap_or(100);
    let entries: Vec<TimeEntry> = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(serde_json::json!({
        "time_entries": entries,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn get_time_entry(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Time entry ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let entry_id = TimeEntryId(uuid);
    model
        .time_entries
        .get(&entry_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Time entry not found: {id}")))
        .and_then(|e| serde_json::to_value(e).map_err(PipeError::serialization))
}

fn create_time_entry(model: &mut Model, data: Option<&serde_json::Value>) -> HandlerResult {
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Time entry data is required"))?;

    let input: TimeEntryInput = serde_json::from_value(data.clone()).map_err(|e| {
        PipeError::new(
            "INVALID_DATA",
            format!("Failed to parse time entry data: {e}"),
        )
    })?;

    let task_id_str = input
        .task_id
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Task ID is required"))?;

    let task_id = Uuid::parse_str(&task_id_str)
        .map(TaskId)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid task UUID: {task_id_str}")))?;

    // Verify task exists
    if !model.tasks.contains_key(&task_id) {
        return Err(PipeError::new(
            "NOT_FOUND",
            format!("Task not found: {task_id_str}"),
        ));
    }

    let started_at = input
        .started_at
        .as_ref()
        .and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or_else(Utc::now);

    let ended_at = input.ended_at.as_ref().and_then(|s| {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.with_timezone(&Utc))
    });

    let duration_minutes = input.duration_minutes;

    let entry = TimeEntry {
        id: TimeEntryId::new(),
        task_id,
        description: input.description,
        started_at,
        ended_at,
        duration_minutes,
    };

    let entry_id = entry.id;
    model.time_entries.insert(entry_id, entry.clone());
    model.sync_time_entry(&entry);

    serde_json::to_value(&entry).map_err(PipeError::serialization)
}

fn update_time_entry(
    model: &mut Model,
    id: Option<&str>,
    data: Option<&serde_json::Value>,
) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Time entry ID is required"))?;
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Time entry data is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let entry_id = TimeEntryId(uuid);

    let entry = model
        .time_entries
        .get_mut(&entry_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Time entry not found: {id}")))?;

    if let Some(started) = data.get("started_at").and_then(|v| v.as_str()) {
        if let Ok(dt) = DateTime::parse_from_rfc3339(started) {
            entry.started_at = dt.with_timezone(&Utc);
        }
    }
    if let Some(ended) = data.get("ended_at") {
        entry.ended_at = if ended.is_null() {
            None
        } else {
            ended.as_str().and_then(|s| {
                DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
            })
        };
    }
    if let Some(duration) = data.get("duration_minutes") {
        entry.duration_minutes = if duration.is_null() {
            None
        } else {
            duration.as_u64().map(|v| v as u32)
        };
    }
    if let Some(desc) = data.get("description") {
        entry.description = if desc.is_null() {
            None
        } else {
            desc.as_str().map(|s| s.to_string())
        };
    }

    let updated_entry = entry.clone();
    model.sync_time_entry(&updated_entry);

    serde_json::to_value(&updated_entry).map_err(PipeError::serialization)
}

fn delete_time_entry(model: &mut Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Time entry ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let entry_id = TimeEntryId(uuid);

    model
        .time_entries
        .remove(&entry_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Time entry not found: {id}")))?;

    model.delete_time_entry(&entry_id);

    Ok(serde_json::json!({ "deleted": id }))
}
