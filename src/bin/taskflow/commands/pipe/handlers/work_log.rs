//! Work log CRUD operations for the pipe interface.

use uuid::Uuid;

use taskflow::app::Model;
use taskflow::domain::{TaskId, WorkLogEntry, WorkLogEntryId};

use super::HandlerResult;
use crate::commands::pipe::types::{FilterParams, Operation, PipeError, PipeRequest};

/// Handle work log operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_work_logs(model, &request.filters),
        Operation::Get => get_work_log(model, request.id.as_deref()),
        Operation::Create => create_work_log(model, request.data.as_ref()),
        Operation::Update => update_work_log(model, request.id.as_deref(), request.data.as_ref()),
        Operation::Delete => delete_work_log(model, request.id.as_deref()),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

fn list_work_logs(model: &Model, filters: &Option<FilterParams>) -> HandlerResult {
    let mut logs: Vec<&WorkLogEntry> = model.work_logs.values().collect();

    // Apply filters
    if let Some(ref f) = filters {
        // Filter by task (using project_id field as task filter)
        if let Some(ref task_id_str) = f.project_id {
            if let Ok(uuid) = Uuid::parse_str(task_id_str) {
                let task_id = TaskId(uuid);
                logs.retain(|l| l.task_id == task_id);
            }
        }

        // Filter by search
        if let Some(ref search) = f.search {
            let search_lower = search.to_lowercase();
            logs.retain(|l| l.content.to_lowercase().contains(&search_lower));
        }
    }

    // Sort by created_at (newest first)
    logs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = logs.len();
    let offset = filters.as_ref().and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.as_ref().and_then(|f| f.limit).unwrap_or(100);
    let logs: Vec<WorkLogEntry> = logs.into_iter().skip(offset).take(limit).cloned().collect();

    Ok(serde_json::json!({
        "work_logs": logs,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn get_work_log(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Work log ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let log_id = WorkLogEntryId(uuid);
    model
        .work_logs
        .get(&log_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Work log not found: {id}")))
        .and_then(|l| serde_json::to_value(l).map_err(PipeError::serialization))
}

fn create_work_log(model: &mut Model, data: Option<&serde_json::Value>) -> HandlerResult {
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Work log data is required"))?;

    let task_id_str = data
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Task ID is required"))?;

    let task_id = Uuid::parse_str(task_id_str)
        .map(TaskId)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid task UUID: {task_id_str}")))?;

    // Verify task exists
    if !model.tasks.contains_key(&task_id) {
        return Err(PipeError::new(
            "NOT_FOUND",
            format!("Task not found: {task_id_str}"),
        ));
    }

    let content = data
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Content is required"))?
        .to_string();

    let log = WorkLogEntry::new(task_id, content);

    let log_id = log.id;
    model.work_logs.insert(log_id, log.clone());
    model.sync_work_log(&log);

    serde_json::to_value(&log).map_err(PipeError::serialization)
}

fn update_work_log(
    model: &mut Model,
    id: Option<&str>,
    data: Option<&serde_json::Value>,
) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Work log ID is required"))?;
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Work log data is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let log_id = WorkLogEntryId(uuid);

    let log = model
        .work_logs
        .get_mut(&log_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Work log not found: {id}")))?;

    if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
        log.content = content.to_string();
    }

    log.updated_at = chrono::Utc::now();
    let updated_log = log.clone();
    model.sync_work_log(&updated_log);

    serde_json::to_value(&updated_log).map_err(PipeError::serialization)
}

fn delete_work_log(model: &mut Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Work log ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let log_id = WorkLogEntryId(uuid);

    model
        .work_logs
        .remove(&log_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Work log not found: {id}")))?;

    model.delete_work_log_from_storage(&log_id);

    Ok(serde_json::json!({ "deleted": id }))
}
