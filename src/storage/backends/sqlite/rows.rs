//! Row parsing helpers for SQLite backend.

use crate::domain::{
    Priority, Project, ProjectId, ProjectStatus, Task, TaskId, TaskStatus, TimeEntry, TimeEntryId,
    WorkLogEntry, WorkLogEntryId,
};

/// Parse a Task from a SQLite row.
pub(crate) fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    let id: String = row.get("id")?;
    let status_str: String = row.get("status")?;
    let priority_str: String = row.get("priority")?;
    let project_id: Option<String> = row.get("project_id")?;
    let parent_task_id: Option<String> = row.get("parent_task_id")?;
    let tags_json: String = row.get("tags")?;
    let deps_json: String = row.get("dependencies")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;
    let due_date: Option<String> = row.get("due_date")?;
    let scheduled_date: Option<String> = row.get("scheduled_date")?;
    let completed_at: Option<String> = row.get("completed_at")?;
    let recurrence_json: Option<String> = row.get("recurrence")?;
    let custom_fields_json: String = row.get("custom_fields")?;

    Ok(Task {
        id: TaskId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
        title: row.get("title")?,
        description: row.get("description")?,
        status: match status_str.as_str() {
            "in_progress" => TaskStatus::InProgress,
            "blocked" => TaskStatus::Blocked,
            "done" => TaskStatus::Done,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Todo,
        },
        priority: match priority_str.as_str() {
            "low" => Priority::Low,
            "medium" => Priority::Medium,
            "high" => Priority::High,
            "urgent" => Priority::Urgent,
            _ => Priority::None,
        },
        project_id: project_id.map(|s| ProjectId(uuid::Uuid::parse_str(&s).unwrap_or_default())),
        parent_task_id: parent_task_id
            .map(|s| TaskId(uuid::Uuid::parse_str(&s).unwrap_or_default())),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        dependencies: serde_json::from_str::<Vec<String>>(&deps_json)
            .unwrap_or_default()
            .into_iter()
            .map(|s| TaskId(uuid::Uuid::parse_str(&s).unwrap_or_default()))
            .collect(),
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        due_date: due_date.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        scheduled_date: scheduled_date
            .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        completed_at: completed_at.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        }),
        recurrence: recurrence_json.and_then(|s| serde_json::from_str(&s).ok()),
        estimated_minutes: row.get("estimated_minutes")?,
        actual_minutes: row.get::<_, i32>("actual_minutes")? as u32,
        sort_order: row.get("sort_order").ok().flatten(),
        next_task_id: row
            .get::<_, Option<String>>("next_task_id")
            .ok()
            .flatten()
            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
            .map(TaskId),
        custom_fields: serde_json::from_str(&custom_fields_json).unwrap_or_default(),
        snooze_until: row
            .get::<_, Option<String>>("snooze_until")
            .ok()
            .flatten()
            .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
    })
}

/// Parse a Project from a SQLite row.
pub(crate) fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    let id: String = row.get("id")?;
    let status_str: String = row.get("status")?;
    let parent_id: Option<String> = row.get("parent_id")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;
    let start_date: Option<String> = row.get("start_date")?;
    let due_date: Option<String> = row.get("due_date")?;
    let default_tags_json: String = row.get("default_tags")?;
    let custom_fields_json: String = row.get("custom_fields")?;

    Ok(Project {
        id: ProjectId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
        name: row.get("name")?,
        description: row.get("description")?,
        status: match status_str.as_str() {
            "on_hold" => ProjectStatus::OnHold,
            "completed" => ProjectStatus::Completed,
            "archived" => ProjectStatus::Archived,
            _ => ProjectStatus::Active,
        },
        parent_id: parent_id.map(|s| ProjectId(uuid::Uuid::parse_str(&s).unwrap_or_default())),
        color: row.get("color")?,
        icon: row.get("icon")?,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        start_date: start_date.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        due_date: due_date.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        default_tags: serde_json::from_str(&default_tags_json).unwrap_or_default(),
        custom_fields: serde_json::from_str(&custom_fields_json).unwrap_or_default(),
    })
}

/// Parse a TimeEntry from a SQLite row.
pub(crate) fn time_entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimeEntry> {
    let id: String = row.get("id")?;
    let task_id: String = row.get("task_id")?;
    let started_at: String = row.get("started_at")?;
    let ended_at: Option<String> = row.get("ended_at")?;

    Ok(TimeEntry {
        id: TimeEntryId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
        task_id: TaskId(uuid::Uuid::parse_str(&task_id).unwrap_or_default()),
        description: row.get("description")?,
        started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        ended_at: ended_at.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        }),
        duration_minutes: row
            .get::<_, Option<i32>>("duration_minutes")?
            .map(|m| m as u32),
    })
}

/// Parse a WorkLogEntry from a SQLite row.
pub(crate) fn work_log_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkLogEntry> {
    let id: String = row.get("id")?;
    let task_id: String = row.get("task_id")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(WorkLogEntry {
        id: WorkLogEntryId(uuid::Uuid::parse_str(&id).unwrap_or_default()),
        task_id: TaskId(uuid::Uuid::parse_str(&task_id).unwrap_or_default()),
        content: row.get("content")?,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}
