//! Task CRUD operations for the pipe interface.

use chrono::NaiveDate;
use uuid::Uuid;

use taskflow::app::Model;
use taskflow::domain::{Priority, ProjectId, Task, TaskId, TaskStatus};

use super::HandlerResult;
use crate::commands::pipe::types::{FilterParams, Operation, PipeError, PipeRequest, TaskInput};

/// Handle task operations.
pub fn handle(model: &mut Model, request: &PipeRequest) -> HandlerResult {
    match request.operation {
        Operation::List => list_tasks(model, &request.filters),
        Operation::Get => get_task(model, request.id.as_deref()),
        Operation::Create => create_task(model, request.data.as_ref()),
        Operation::Update => update_task(model, request.id.as_deref(), request.data.as_ref()),
        Operation::Delete => delete_task(model, request.id.as_deref()),
        Operation::Export | Operation::Import => Err(PipeError::new(
            "INVALID_OPERATION",
            "Use the export/import operations at the top level",
        )),
    }
}

/// List tasks with optional filters.
fn list_tasks(model: &Model, filters: &Option<FilterParams>) -> HandlerResult {
    let mut tasks: Vec<&Task> = model.tasks.values().collect();

    // Apply filters
    if let Some(ref f) = filters {
        // Filter by project
        if let Some(ref proj_id_str) = f.project_id {
            if let Ok(uuid) = Uuid::parse_str(proj_id_str) {
                let proj_id = ProjectId(uuid);
                tasks.retain(|t| t.project_id.as_ref() == Some(&proj_id));
            }
        }

        // Filter by tags
        if let Some(ref filter_tags) = f.tags {
            let mode = f.tags_mode.as_deref().unwrap_or("all");
            tasks.retain(|t| {
                let task_tags_lower: Vec<String> =
                    t.tags.iter().map(|s| s.to_lowercase()).collect();
                if mode == "any" {
                    filter_tags
                        .iter()
                        .any(|ft| task_tags_lower.contains(&ft.to_lowercase()))
                } else {
                    filter_tags
                        .iter()
                        .all(|ft| task_tags_lower.contains(&ft.to_lowercase()))
                }
            });
        }

        // Filter by status
        if let Some(ref statuses) = f.status {
            let status_set: Vec<TaskStatus> =
                statuses.iter().filter_map(|s| parse_status(s)).collect();
            if !status_set.is_empty() {
                tasks.retain(|t| status_set.contains(&t.status));
            }
        }

        // Filter by priority
        if let Some(ref priorities) = f.priority {
            let priority_set: Vec<Priority> = priorities
                .iter()
                .filter_map(|p| parse_priority(p))
                .collect();
            if !priority_set.is_empty() {
                tasks.retain(|t| priority_set.contains(&t.priority));
            }
        }

        // Filter by search text
        if let Some(ref search) = f.search {
            let search_lower = search.to_lowercase();
            tasks.retain(|t| {
                t.title.to_lowercase().contains(&search_lower)
                    || t.tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&search_lower))
                    || t.description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&search_lower))
            });
        }

        // Filter by completion status
        if !f.include_completed.unwrap_or(false) {
            tasks.retain(|t| !t.status.is_complete());
        }

        // Filter by due date
        if let Some(ref due_before) = f.due_before {
            if let Ok(date) = NaiveDate::parse_from_str(due_before, "%Y-%m-%d") {
                tasks.retain(|t| t.due_date.is_some_and(|d| d <= date));
            }
        }
        if let Some(ref due_after) = f.due_after {
            if let Ok(date) = NaiveDate::parse_from_str(due_after, "%Y-%m-%d") {
                tasks.retain(|t| t.due_date.is_some_and(|d| d >= date));
            }
        }

        // Sort tasks
        if let Some(ref sort_by) = f.sort_by {
            let desc = f.sort_order.as_deref() == Some("desc");
            match sort_by.as_str() {
                "due_date" => {
                    tasks.sort_by(|a, b| {
                        let cmp = a.due_date.cmp(&b.due_date);
                        if desc {
                            cmp.reverse()
                        } else {
                            cmp
                        }
                    });
                }
                "priority" => {
                    tasks.sort_by(|a, b| {
                        // Higher priority first by default (Urgent=4, High=3, Medium=2, Low=1, None=0)
                        let a_val = priority_to_num(&a.priority);
                        let b_val = priority_to_num(&b.priority);
                        let cmp = b_val.cmp(&a_val);
                        if desc {
                            cmp.reverse()
                        } else {
                            cmp
                        }
                    });
                }
                "title" => {
                    tasks.sort_by(|a, b| {
                        let cmp = a.title.to_lowercase().cmp(&b.title.to_lowercase());
                        if desc {
                            cmp.reverse()
                        } else {
                            cmp
                        }
                    });
                }
                "created" | "created_at" => {
                    tasks.sort_by(|a, b| {
                        let cmp = a.created_at.cmp(&b.created_at);
                        if desc {
                            cmp.reverse()
                        } else {
                            cmp
                        }
                    });
                }
                "updated" | "updated_at" => {
                    tasks.sort_by(|a, b| {
                        let cmp = a.updated_at.cmp(&b.updated_at);
                        if desc {
                            cmp.reverse()
                        } else {
                            cmp
                        }
                    });
                }
                _ => {}
            }
        }
    }

    let total = tasks.len();

    // Apply pagination
    let offset = filters.as_ref().and_then(|f| f.offset).unwrap_or(0);
    let limit = filters.as_ref().and_then(|f| f.limit).unwrap_or(100);
    let tasks: Vec<Task> = tasks
        .into_iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(serde_json::json!({
        "tasks": tasks,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

/// Get a single task by ID.
fn get_task(model: &Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Task ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let task_id = TaskId(uuid);
    model
        .tasks
        .get(&task_id)
        .map(|t| serde_json::to_value(t).unwrap())
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Task not found: {id}")))
}

/// Create a new task.
fn create_task(model: &mut Model, data: Option<&serde_json::Value>) -> HandlerResult {
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Task data is required"))?;

    let input: TaskInput = serde_json::from_value(data.clone())
        .map_err(|e| PipeError::new("INVALID_DATA", format!("Failed to parse task data: {e}")))?;

    let title = input
        .title
        .ok_or_else(|| PipeError::new("MISSING_DATA", "Task title is required"))?;

    // Build task from input
    let mut task = Task::new(&title);

    if let Some(ref desc) = input.description {
        task.description = Some(desc.clone());
    }
    if let Some(ref status_str) = input.status {
        if let Some(status) = parse_status(status_str) {
            task.status = status;
            if status.is_complete() {
                task.completed_at = Some(chrono::Utc::now());
            }
        }
    }
    if let Some(ref priority_str) = input.priority {
        if let Some(priority) = parse_priority(priority_str) {
            task.priority = priority;
        }
    }
    if let Some(ref proj_id_str) = input.project_id {
        if let Ok(uuid) = Uuid::parse_str(proj_id_str) {
            task.project_id = Some(ProjectId(uuid));
        }
    }
    if let Some(ref tags) = input.tags {
        task.tags.clone_from(tags);
    }
    if let Some(ref due_str) = input.due_date {
        task.due_date = NaiveDate::parse_from_str(due_str, "%Y-%m-%d").ok();
    }
    if let Some(ref scheduled_str) = input.scheduled_date {
        task.scheduled_date = NaiveDate::parse_from_str(scheduled_str, "%Y-%m-%d").ok();
    }
    if let Some(minutes) = input.estimated_minutes {
        task.estimated_minutes = Some(minutes);
    }
    if let Some(ref deps) = input.dependencies {
        task.dependencies = deps
            .iter()
            .filter_map(|d| Uuid::parse_str(d).ok().map(TaskId))
            .collect();
    }

    // Insert and sync
    let task_id = task.id;
    model.tasks.insert(task_id, task.clone());
    model.sync_task(&task);

    Ok(serde_json::to_value(&task).unwrap())
}

/// Update an existing task.
fn update_task(
    model: &mut Model,
    id: Option<&str>,
    data: Option<&serde_json::Value>,
) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Task ID is required"))?;
    let data = data.ok_or_else(|| PipeError::new("MISSING_DATA", "Task data is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let task_id = TaskId(uuid);

    // Get existing task
    let task = model
        .tasks
        .get_mut(&task_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Task not found: {id}")))?;

    // Apply partial updates from data
    if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
        task.title = title.to_string();
    }
    if let Some(desc) = data.get("description") {
        task.description = if desc.is_null() {
            None
        } else {
            desc.as_str().map(|s| s.to_string())
        };
    }
    if let Some(status_str) = data.get("status").and_then(|v| v.as_str()) {
        if let Some(status) = parse_status(status_str) {
            let was_complete = task.status.is_complete();
            task.status = status;
            if status.is_complete() && !was_complete {
                task.completed_at = Some(chrono::Utc::now());
            } else if !status.is_complete() && was_complete {
                task.completed_at = None;
            }
        }
    }
    if let Some(priority_str) = data.get("priority").and_then(|v| v.as_str()) {
        if let Some(priority) = parse_priority(priority_str) {
            task.priority = priority;
        }
    }
    if let Some(proj_id) = data.get("project_id") {
        task.project_id = if proj_id.is_null() {
            None
        } else {
            proj_id
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok().map(ProjectId))
        };
    }
    if let Some(tags) = data.get("tags").and_then(|v| v.as_array()) {
        task.tags = tags
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(due) = data.get("due_date") {
        task.due_date = if due.is_null() {
            None
        } else {
            due.as_str()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        };
    }
    if let Some(scheduled) = data.get("scheduled_date") {
        task.scheduled_date = if scheduled.is_null() {
            None
        } else {
            scheduled
                .as_str()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        };
    }
    if let Some(est) = data.get("estimated_minutes") {
        task.estimated_minutes = if est.is_null() {
            None
        } else {
            est.as_u64().map(|v| v as u32)
        };
    }
    if let Some(deps) = data.get("dependencies").and_then(|v| v.as_array()) {
        task.dependencies = deps
            .iter()
            .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok().map(TaskId)))
            .collect();
    }

    task.updated_at = chrono::Utc::now();
    let updated_task = task.clone();
    model.sync_task(&updated_task);

    Ok(serde_json::to_value(&updated_task).unwrap())
}

/// Delete a task.
fn delete_task(model: &mut Model, id: Option<&str>) -> HandlerResult {
    let id = id.ok_or_else(|| PipeError::new("MISSING_ID", "Task ID is required"))?;

    let uuid = Uuid::parse_str(id)
        .map_err(|_| PipeError::new("INVALID_ID", format!("Invalid UUID format: {id}")))?;

    let task_id = TaskId(uuid);

    model
        .tasks
        .remove(&task_id)
        .ok_or_else(|| PipeError::new("NOT_FOUND", format!("Task not found: {id}")))?;

    model.delete_task_from_storage(&task_id);

    Ok(serde_json::json!({ "deleted": id }))
}

// ============================================================================
// Helper functions
// ============================================================================

fn parse_status(s: &str) -> Option<TaskStatus> {
    match s.to_lowercase().replace(['-', '_'], "").as_str() {
        "todo" => Some(TaskStatus::Todo),
        "inprogress" => Some(TaskStatus::InProgress),
        "blocked" => Some(TaskStatus::Blocked),
        "done" | "completed" => Some(TaskStatus::Done),
        "cancelled" | "canceled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "none" => Some(Priority::None),
        "low" => Some(Priority::Low),
        "medium" | "med" => Some(Priority::Medium),
        "high" => Some(Priority::High),
        "urgent" => Some(Priority::Urgent),
        _ => None,
    }
}

fn priority_to_num(p: &Priority) -> u8 {
    match p {
        Priority::None => 0,
        Priority::Low => 1,
        Priority::Medium => 2,
        Priority::High => 3,
        Priority::Urgent => 4,
    }
}
