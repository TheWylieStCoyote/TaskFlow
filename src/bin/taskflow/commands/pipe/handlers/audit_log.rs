//! Read-only access to the task audit log for the pipe interface.
//!
//! The audit log is written by the application whenever a task is created,
//! modified, completed, uncompleted or deleted. Scripts can list and fetch
//! entries but never create, change or remove them.

use chrono::NaiveDate;
use uuid::Uuid;

use taskflow::app::Model;
use taskflow::domain::{AuditLogEntry, AuditLogEntryId, TaskId};

use super::HandlerResult;
use crate::commands::pipe::types::{FilterParams, Operation, PipeError, PipeRequest};

/// Handle audit log operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_audit_logs(model, request.filters.as_ref()),
        Operation::Get => get_audit_log(model, request.id.as_deref()),
        Operation::Create | Operation::Update | Operation::Delete => Err(PipeError::new(
            "READ_ONLY",
            "audit_log is append-only and written by the application",
        )),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

/// List audit log entries, oldest first.
///
/// Filters come from the shared [`FilterParams`]:
///
/// - `task_id` is the UUID of the task whose history is wanted.
/// - `status` is a list of actions (`created`, `modified`, `completed`,
///   `uncompleted`, `deleted`); an entry matches when its action is listed.
/// - `due_after` / `due_before` are inclusive `YYYY-MM-DD` bounds on the
///   UTC date of `changed_at`.
/// - `limit` (default 100) and `offset` page backwards from the newest entry,
///   so `limit: 5` returns the five most recent entries.
/// - `sort_order` orders the returned page: `asc` (default, oldest first) or
///   `desc` (newest first).
fn list_audit_logs(model: &Model, filters: Option<&FilterParams>) -> HandlerResult {
    let mut entries: Vec<&AuditLogEntry> = model.audit_logs.values().collect();

    if let Some(f) = filters {
        if let Some(ref task_id_str) = f.task_id {
            if let Ok(uuid) = Uuid::parse_str(task_id_str) {
                let task_id = TaskId(uuid);
                entries.retain(|e| e.task_id == task_id);
            }
        }

        if let Some(ref actions) = f.status {
            let wanted: Vec<String> = actions.iter().map(|a| a.to_lowercase()).collect();
            entries.retain(|e| wanted.contains(&e.action.to_string()));
        }

        if let Some(ref after) = f.due_after {
            if let Ok(date) = NaiveDate::parse_from_str(after, "%Y-%m-%d") {
                entries.retain(|e| e.changed_at.date_naive() >= date);
            }
        }
        if let Some(ref before) = f.due_before {
            if let Ok(date) = NaiveDate::parse_from_str(before, "%Y-%m-%d") {
                entries.retain(|e| e.changed_at.date_naive() <= date);
            }
        }
    }

    // Newest first while paging so `limit` keeps the most recent entries; the
    // id tie-break keeps the order stable for entries sharing a timestamp.
    entries.sort_by(|a, b| {
        b.changed_at
            .cmp(&a.changed_at)
            .then_with(|| b.id.0.cmp(&a.id.0))
    });

    let total = entries.len();
    let offset = filters.and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.and_then(|f| f.limit).unwrap_or(100);
    let mut page: Vec<AuditLogEntry> = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    let descending = filters.and_then(|f| f.sort_order.as_deref()) == Some("desc");
    if !descending {
        page.reverse();
    }

    Ok(serde_json::json!({
        "audit_logs": page,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

fn get_audit_log(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Audit log entry ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let entry_id = AuditLogEntryId(uuid);
    model
        .audit_logs
        .get(&entry_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Audit log entry not found: {id}")))
        .and_then(|e| serde_json::to_value(e).map_err(PipeError::serialization))
}
