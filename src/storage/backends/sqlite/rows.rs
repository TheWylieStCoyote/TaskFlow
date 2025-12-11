//! Row parsing helpers for SQLite backend.
//!
//! This module provides utility functions for parsing values from SQLite rows
//! into domain types. All parsing functions log warnings on failure rather
//! than panicking.

use chrono::{DateTime, NaiveDate, Utc};
use tracing::warn;

use crate::domain::{
    Goal, GoalId, GoalStatus, Habit, HabitFrequency, HabitId, KeyResult, KeyResultId,
    KeyResultStatus, Priority, Project, ProjectId, ProjectStatus, Task, TaskId, TaskStatus,
    TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};

// ============================================================================
// Generic Parsing Helpers
// ============================================================================

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

/// Parse an RFC3339 datetime string, defaulting to now on failure.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc))
}

/// Parse an optional RFC3339 datetime string.
fn parse_optional_datetime(s: Option<String>) -> Option<DateTime<Utc>> {
    s.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    })
}

/// Parse a date string in YYYY-MM-DD format.
fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Parse an optional date string in YYYY-MM-DD format.
fn parse_optional_date(s: Option<String>) -> Option<NaiveDate> {
    s.and_then(|s| parse_date(&s))
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
        status: TaskStatus::from_str_lossy(&status_str),
        priority: Priority::from_str_lossy(&priority_str),
        project_id: project_id.map(|s| ProjectId(parse_uuid(&s, "task.project_id"))),
        parent_task_id: parent_task_id.map(|s| TaskId(parse_uuid(&s, "task.parent_task_id"))),
        tags: parse_json(&tags_json, "task.tags"),
        dependencies: parse_json::<Vec<String>>(&deps_json, "task.dependencies")
            .into_iter()
            .map(|s| TaskId(parse_uuid(&s, "task.dependencies[]")))
            .collect(),
        created_at: parse_datetime(&created_at),
        updated_at: parse_datetime(&updated_at),
        due_date: parse_optional_date(due_date),
        scheduled_date: parse_optional_date(scheduled_date),
        completed_at: parse_optional_datetime(completed_at),
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
        snooze_until: parse_optional_date(
            row.get::<_, Option<String>>("snooze_until").ok().flatten(),
        ),
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
    // Optional field - may not exist in older databases until migration runs
    let estimation_multiplier: Option<f64> = row.get("estimation_multiplier").ok().flatten();

    Ok(Project {
        id: ProjectId(parse_uuid(&id, "project.id")),
        name: row.get("name")?,
        description: row.get("description")?,
        status: ProjectStatus::from_str_lossy(&status_str),
        parent_id: parent_id.map(|s| ProjectId(parse_uuid(&s, "project.parent_id"))),
        color: row.get("color")?,
        icon: row.get("icon")?,
        created_at: parse_datetime(&created_at),
        updated_at: parse_datetime(&updated_at),
        start_date: parse_optional_date(start_date),
        due_date: parse_optional_date(due_date),
        default_tags: parse_json(&default_tags_json, "project.default_tags"),
        custom_fields: parse_json(&custom_fields_json, "project.custom_fields"),
        estimation_multiplier,
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
        started_at: parse_datetime(&started_at),
        ended_at: parse_optional_datetime(ended_at),
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
        created_at: parse_datetime(&created_at),
        updated_at: parse_datetime(&updated_at),
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
        start_date: parse_date(&start_date).unwrap_or_else(|| Utc::now().date_naive()),
        end_date: parse_optional_date(end_date),
        check_ins: std::collections::HashMap::new(), // Populated separately
        color: row.get("color")?,
        icon: row.get("icon")?,
        tags: parse_json(&tags_json, "habit.tags"),
        archived: archived != 0,
        created_at: parse_datetime(&created_at),
        updated_at: parse_datetime(&updated_at),
    })
}

/// Parse a Goal from a SQLite row.
pub(crate) fn goal_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Goal> {
    let id: String = row.get("id")?;
    let status_str: String = row.get("status")?;
    let start_date: Option<String> = row.get("start_date")?;
    let due_date: Option<String> = row.get("due_date")?;
    let quarter_json: Option<String> = row.get("quarter")?;
    let manual_progress: Option<i32> = row.get("manual_progress")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(Goal {
        id: GoalId(parse_uuid(&id, "goal.id")),
        name: row.get("name")?,
        description: row.get("description")?,
        status: GoalStatus::from_str_lossy(&status_str),
        start_date: parse_optional_date(start_date),
        due_date: parse_optional_date(due_date),
        quarter: quarter_json.and_then(|s| serde_json::from_str(&s).ok()),
        manual_progress: manual_progress.map(|p| p.clamp(0, 100) as u8),
        color: row.get("color")?,
        icon: row.get("icon")?,
        created_at: parse_datetime(&created_at),
        updated_at: parse_datetime(&updated_at),
    })
}

/// Parse a KeyResult from a SQLite row.
pub(crate) fn key_result_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeyResult> {
    let id: String = row.get("id")?;
    let goal_id: String = row.get("goal_id")?;
    let status_str: String = row.get("status")?;
    let manual_progress: Option<i32> = row.get("manual_progress")?;
    let linked_project_ids_json: String = row.get("linked_project_ids")?;
    let linked_task_ids_json: String = row.get("linked_task_ids")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(KeyResult {
        id: KeyResultId(parse_uuid(&id, "key_result.id")),
        goal_id: GoalId(parse_uuid(&goal_id, "key_result.goal_id")),
        name: row.get("name")?,
        description: row.get("description")?,
        status: KeyResultStatus::from_str_lossy(&status_str),
        target_value: row.get("target_value")?,
        current_value: row.get("current_value")?,
        unit: row.get("unit")?,
        manual_progress: manual_progress.map(|p| p.clamp(0, 100) as u8),
        linked_project_ids: parse_json::<Vec<String>>(
            &linked_project_ids_json,
            "key_result.linked_project_ids",
        )
        .into_iter()
        .map(|s| ProjectId(parse_uuid(&s, "key_result.linked_project_ids[]")))
        .collect(),
        linked_task_ids: parse_json::<Vec<String>>(
            &linked_task_ids_json,
            "key_result.linked_task_ids",
        )
        .into_iter()
        .map(|s| TaskId(parse_uuid(&s, "key_result.linked_task_ids[]")))
        .collect(),
        created_at: parse_datetime(&created_at),
        updated_at: parse_datetime(&updated_at),
    })
}
