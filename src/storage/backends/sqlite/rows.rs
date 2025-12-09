//! Row parsing helpers for SQLite backend.

use tracing::warn;

use crate::domain::{
    Habit, HabitFrequency, HabitId, Priority, Project, ProjectId, ProjectStatus, Task, TaskId,
    TaskStatus, TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};

/// Parse a UUID string, logging a warning if invalid.
fn parse_uuid(s: &str, field_name: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(s).unwrap_or_else(|e| {
        warn!(field = field_name, value = s, error = %e, "Invalid UUID in SQLite row");
        uuid::Uuid::nil()
    })
}

/// Parse JSON, logging a warning if invalid.
fn parse_json<T: serde::de::DeserializeOwned + Default>(s: &str, field_name: &str) -> T {
    serde_json::from_str(s).unwrap_or_else(|e| {
        warn!(field = field_name, error = %e, "Invalid JSON in SQLite row");
        T::default()
    })
}

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
        id: TaskId(parse_uuid(&id, "task.id")),
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
        project_id: project_id.map(|s| ProjectId(parse_uuid(&s, "task.project_id"))),
        parent_task_id: parent_task_id.map(|s| TaskId(parse_uuid(&s, "task.parent_task_id"))),
        tags: parse_json(&tags_json, "task.tags"),
        dependencies: parse_json::<Vec<String>>(&deps_json, "task.dependencies")
            .into_iter()
            .map(|s| TaskId(parse_uuid(&s, "task.dependencies[]")))
            .collect(),
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
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
        custom_fields: parse_json(&custom_fields_json, "task.custom_fields"),
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
        id: ProjectId(parse_uuid(&id, "project.id")),
        name: row.get("name")?,
        description: row.get("description")?,
        status: match status_str.as_str() {
            "on_hold" => ProjectStatus::OnHold,
            "completed" => ProjectStatus::Completed,
            "archived" => ProjectStatus::Archived,
            _ => ProjectStatus::Active,
        },
        parent_id: parent_id.map(|s| ProjectId(parse_uuid(&s, "project.parent_id"))),
        color: row.get("color")?,
        icon: row.get("icon")?,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
        start_date: start_date.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        due_date: due_date.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        default_tags: parse_json(&default_tags_json, "project.default_tags"),
        custom_fields: parse_json(&custom_fields_json, "project.custom_fields"),
    })
}

/// Parse a TimeEntry from a SQLite row.
pub(crate) fn time_entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimeEntry> {
    let id: String = row.get("id")?;
    let task_id: String = row.get("task_id")?;
    let started_at: String = row.get("started_at")?;
    let ended_at: Option<String> = row.get("ended_at")?;

    Ok(TimeEntry {
        id: TimeEntryId(parse_uuid(&id, "time_entry.id")),
        task_id: TaskId(parse_uuid(&task_id, "time_entry.task_id")),
        description: row.get("description")?,
        started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
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
        id: WorkLogEntryId(parse_uuid(&id, "work_log.id")),
        task_id: TaskId(parse_uuid(&task_id, "work_log.task_id")),
        content: row.get("content")?,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
    })
}

/// Parse a Habit from a SQLite row (without check-ins).
pub(crate) fn habit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Habit> {
    let id: String = row.get("id")?;
    let frequency_json: String = row.get("frequency")?;
    let start_date: String = row.get("start_date")?;
    let end_date: Option<String> = row.get("end_date")?;
    let tags_json: String = row.get("tags")?;
    let archived: i32 = row.get("archived")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(Habit {
        id: HabitId(parse_uuid(&id, "habit.id")),
        name: row.get("name")?,
        description: row.get("description")?,
        frequency: serde_json::from_str(&frequency_json).unwrap_or(HabitFrequency::Daily),
        start_date: chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Utc::now().date_naive()),
        end_date: end_date.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        check_ins: std::collections::HashMap::new(), // Populated separately
        color: row.get("color")?,
        icon: row.get("icon")?,
        tags: parse_json(&tags_json, "habit.tags"),
        archived: archived != 0,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc)),
    })
}
