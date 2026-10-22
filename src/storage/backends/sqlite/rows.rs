//! Row parsing helpers for SQLite backend.
//!
//! Every `*_from_row` function turns one SQLite row into a domain value. A
//! column that is present but cannot be parsed (a timestamp, UUID, date or
//! JSON value) is a *row-level* error: the function returns `Err` rather than
//! inventing a replacement such as "now" or the nil UUID. Enumerations stored
//! as text (`status`, `priority`) stay lossy because each has a sensible
//! default and a typo there does not misrepresent the row.
//!
//! [`collect_rows`] runs a query and turns those per-row errors into a
//! `tracing::warn!` naming the table, the row id and the cause, so a corrupt
//! row is skipped visibly instead of silently.

use std::fmt;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rusqlite::types::{Type, ValueRef};
use rusqlite::Row;
use tracing::warn;

use crate::domain::{
    Goal, GoalId, GoalStatus, Habit, HabitCheckIn, HabitFrequency, HabitId, KeyResult, KeyResultId,
    KeyResultStatus, Priority, Project, ProjectId, ProjectStatus, Task, TaskId, TaskStatus,
    TimeEntry, TimeEntryId, WorkLogEntry, WorkLogEntryId,
};
use crate::storage::StorageResult;

// ============================================================================
// Errors
// ============================================================================

/// Longest column value quoted in an error message; the rest is elided.
const MAX_QUOTED_VALUE: usize = 80;

/// A column whose stored text could not be turned into a domain value.
#[derive(Debug)]
struct ColumnParseError {
    column: &'static str,
    value: String,
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl fmt::Display for ColumnParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shown: String = self.value.chars().take(MAX_QUOTED_VALUE).collect();
        let ellipsis = if shown.len() < self.value.len() {
            "..."
        } else {
            ""
        };
        write!(
            f,
            "column `{}` has unparsable value {:?}{}: {}",
            self.column, shown, ellipsis, self.source
        )
    }
}

impl std::error::Error for ColumnParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Build the `rusqlite` error returned when `column` holds `value` that `source` rejected.
fn column_error(
    row: &Row<'_>,
    column: &'static str,
    value: &str,
    source: impl std::error::Error + Send + Sync + 'static,
) -> rusqlite::Error {
    let index = row.as_ref().column_index(column).unwrap_or(0);
    rusqlite::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        Box::new(ColumnParseError {
            column,
            value: value.to_string(),
            source: Box::new(source),
        }),
    )
}

/// Human-readable cause of a row parse failure, without the `rusqlite` wrapper noise.
fn describe(error: &rusqlite::Error) -> String {
    match error {
        rusqlite::Error::FromSqlConversionFailure(_, _, inner) => inner.to_string(),
        other => other.to_string(),
    }
}

/// The first column of `row`, rendered for a log line. Every table in this
/// backend puts its primary key first.
fn row_id(row: &Row<'_>) -> String {
    match row.get_ref(0) {
        Ok(ValueRef::Text(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
        Ok(ValueRef::Integer(i)) => i.to_string(),
        Ok(ValueRef::Null) => "<null>".to_string(),
        Ok(other) => format!("{other:?}"),
        Err(_) => "<unknown>".to_string(),
    }
}

// ============================================================================
// Collecting rows
// ============================================================================

/// Run `stmt` with `params` and parse each row with `parse`, skipping rows
/// that do not parse.
///
/// A row whose stored values cannot be turned into a domain value is logged
/// with `tracing::warn!` (fields `table`, `row_id`, `error`) and left out of
/// the result; every other row still loads. Errors that mean the *query*
/// failed rather than one row being corrupt (a SQLite step error, a column
/// missing from the schema) are returned instead.
pub(crate) fn collect_rows<T, P>(
    stmt: &mut rusqlite::Statement<'_>,
    params: P,
    table: &'static str,
    parse: impl Fn(&Row<'_>) -> rusqlite::Result<T>,
) -> StorageResult<Vec<T>>
where
    P: rusqlite::Params,
{
    let rows = stmt.query_map(params, |row| Ok((row_id(row), parse(row))))?;
    let mut out = Vec::new();
    for item in rows {
        let (id, parsed) = item?;
        match parsed {
            Ok(value) => out.push(value),
            Err(
                e
                @ (rusqlite::Error::InvalidColumnName(_) | rusqlite::Error::InvalidColumnIndex(_)),
            ) => return Err(e.into()),
            Err(e) => warn!(
                table,
                row_id = %id,
                error = %describe(&e),
                "Skipping unparsable SQLite row"
            ),
        }
    }
    Ok(out)
}

// ============================================================================
// Column parsing helpers
// ============================================================================

/// Parse an RFC 3339 timestamp into UTC. Pure: the row-aware wrapper is
/// [`parse_datetime`].
pub(super) fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}

/// Parse a required RFC 3339 timestamp column.
fn parse_datetime(row: &Row<'_>, column: &'static str, s: &str) -> rusqlite::Result<DateTime<Utc>> {
    parse_rfc3339(s).map_err(|e| column_error(row, column, s, e))
}

/// Parse an optional RFC 3339 timestamp column. NULL and empty are `None`;
/// a present but unparsable value is an error.
fn parse_optional_datetime(
    row: &Row<'_>,
    column: &'static str,
    s: Option<String>,
) -> rusqlite::Result<Option<DateTime<Utc>>> {
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => parse_datetime(row, column, s).map(Some),
    }
}

/// Parse a required `YYYY-MM-DD` date column.
fn parse_date(row: &Row<'_>, column: &'static str, s: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| column_error(row, column, s, e))
}

/// Parse an optional `YYYY-MM-DD` date column (NULL/empty is `None`).
fn parse_optional_date(
    row: &Row<'_>,
    column: &'static str,
    s: Option<String>,
) -> rusqlite::Result<Option<NaiveDate>> {
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => parse_date(row, column, s).map(Some),
    }
}

/// Parse an optional `HH:MM:SS` time column (NULL/empty is `None`).
fn parse_optional_time(
    row: &Row<'_>,
    column: &'static str,
    s: Option<String>,
) -> rusqlite::Result<Option<NaiveTime>> {
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => NaiveTime::parse_from_str(s, "%H:%M:%S")
            .map(Some)
            .map_err(|e| column_error(row, column, s, e)),
    }
}

/// Parse a required UUID column.
fn parse_uuid(row: &Row<'_>, column: &'static str, s: &str) -> rusqlite::Result<uuid::Uuid> {
    uuid::Uuid::parse_str(s).map_err(|e| column_error(row, column, s, e))
}

/// Parse an optional UUID column (NULL/empty is `None`).
fn parse_optional_uuid(
    row: &Row<'_>,
    column: &'static str,
    s: Option<String>,
) -> rusqlite::Result<Option<uuid::Uuid>> {
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => parse_uuid(row, column, s).map(Some),
    }
}

/// Parse a JSON column.
fn parse_json<T: serde::de::DeserializeOwned>(
    row: &Row<'_>,
    column: &'static str,
    s: &str,
) -> rusqlite::Result<T> {
    serde_json::from_str(s).map_err(|e| column_error(row, column, s, e))
}

/// Parse an optional JSON column (NULL/empty is `None`).
fn parse_optional_json<T: serde::de::DeserializeOwned>(
    row: &Row<'_>,
    column: &'static str,
    s: Option<String>,
) -> rusqlite::Result<Option<T>> {
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => parse_json(row, column, s).map(Some),
    }
}

/// Parse a JSON array of UUID strings.
fn parse_uuid_list(
    row: &Row<'_>,
    column: &'static str,
    s: &str,
) -> rusqlite::Result<Vec<uuid::Uuid>> {
    parse_json::<Vec<String>>(row, column, s)?
        .iter()
        .map(|item| parse_uuid(row, column, item))
        .collect()
}

/// Read a text column that may be missing from databases created before a
/// migration added it. A missing column reads as NULL.
fn optional_column(row: &Row<'_>, column: &str) -> rusqlite::Result<Option<String>> {
    match row.get::<_, Option<String>>(column) {
        Ok(v) => Ok(v),
        Err(rusqlite::Error::InvalidColumnName(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

// ============================================================================
// Row mappers
// ============================================================================

/// Parse a Task from a SQLite row.
pub(crate) fn task_from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
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
    let recurrence_options_json = optional_column(row, "recurrence_options")?;
    let next_task_id = optional_column(row, "next_task_id")?;
    let snooze_until = optional_column(row, "snooze_until")?;
    let scheduled_start_time = optional_column(row, "scheduled_start_time")?;
    let scheduled_end_time = optional_column(row, "scheduled_end_time")?;
    let sort_order: Option<i32> = match row.get("sort_order") {
        Ok(v) => v,
        Err(rusqlite::Error::InvalidColumnName(_)) => None,
        Err(e) => return Err(e),
    };

    Ok(Task {
        id: TaskId(parse_uuid(row, "id", &id)?),
        title: row.get("title")?,
        description: row.get("description")?,
        status: TaskStatus::from_str_lossy(&status_str),
        priority: Priority::from_str_lossy(&priority_str),
        project_id: parse_optional_uuid(row, "project_id", project_id)?.map(ProjectId),
        parent_task_id: parse_optional_uuid(row, "parent_task_id", parent_task_id)?.map(TaskId),
        tags: parse_json(row, "tags", &tags_json)?,
        dependencies: parse_uuid_list(row, "dependencies", &deps_json)?
            .into_iter()
            .map(TaskId)
            .collect(),
        created_at: parse_datetime(row, "created_at", &created_at)?,
        updated_at: parse_datetime(row, "updated_at", &updated_at)?,
        due_date: parse_optional_date(row, "due_date", due_date)?,
        scheduled_date: parse_optional_date(row, "scheduled_date", scheduled_date)?,
        completed_at: parse_optional_datetime(row, "completed_at", completed_at)?,
        recurrence: parse_optional_json(row, "recurrence", recurrence_json)?,
        recurrence_options: parse_optional_json(
            row,
            "recurrence_options",
            recurrence_options_json,
        )?
        .unwrap_or_default(),
        estimated_minutes: row.get("estimated_minutes")?,
        actual_minutes: row.get::<_, i32>("actual_minutes")? as u32,
        sort_order,
        next_task_id: parse_optional_uuid(row, "next_task_id", next_task_id)?.map(TaskId),
        custom_fields: parse_json(row, "custom_fields", &custom_fields_json)?,
        snooze_until: parse_optional_date(row, "snooze_until", snooze_until)?,
        // Time blocking
        scheduled_start_time: parse_optional_time(
            row,
            "scheduled_start_time",
            scheduled_start_time,
        )?,
        scheduled_end_time: parse_optional_time(row, "scheduled_end_time", scheduled_end_time)?,
        // Git integration is not stored in SQLite (ephemeral linking)
        git_ref: None,
    })
}

/// Parse a Project from a SQLite row.
pub(crate) fn project_from_row(row: &Row<'_>) -> rusqlite::Result<Project> {
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
    let estimation_multiplier: Option<f64> = match row.get("estimation_multiplier") {
        Ok(v) => v,
        Err(rusqlite::Error::InvalidColumnName(_)) => None,
        Err(e) => return Err(e),
    };

    Ok(Project {
        id: ProjectId(parse_uuid(row, "id", &id)?),
        name: row.get("name")?,
        description: row.get("description")?,
        status: ProjectStatus::from_str_lossy(&status_str),
        parent_id: parse_optional_uuid(row, "parent_id", parent_id)?.map(ProjectId),
        color: row.get("color")?,
        icon: row.get("icon")?,
        created_at: parse_datetime(row, "created_at", &created_at)?,
        updated_at: parse_datetime(row, "updated_at", &updated_at)?,
        start_date: parse_optional_date(row, "start_date", start_date)?,
        due_date: parse_optional_date(row, "due_date", due_date)?,
        default_tags: parse_json(row, "default_tags", &default_tags_json)?,
        custom_fields: parse_json(row, "custom_fields", &custom_fields_json)?,
        estimation_multiplier,
    })
}

/// Parse a TimeEntry from a SQLite row.
pub(crate) fn time_entry_from_row(row: &Row<'_>) -> rusqlite::Result<TimeEntry> {
    let id: String = row.get("id")?;
    let task_id: String = row.get("task_id")?;
    let started_at: String = row.get("started_at")?;
    let ended_at: Option<String> = row.get("ended_at")?;

    Ok(TimeEntry {
        id: TimeEntryId(parse_uuid(row, "id", &id)?),
        task_id: TaskId(parse_uuid(row, "task_id", &task_id)?),
        description: row.get("description")?,
        started_at: parse_datetime(row, "started_at", &started_at)?,
        ended_at: parse_optional_datetime(row, "ended_at", ended_at)?,
        duration_minutes: row
            .get::<_, Option<i32>>("duration_minutes")?
            .map(|m| m as u32),
    })
}

/// Parse a WorkLogEntry from a SQLite row.
pub(crate) fn work_log_from_row(row: &Row<'_>) -> rusqlite::Result<WorkLogEntry> {
    let id: String = row.get("id")?;
    let task_id: String = row.get("task_id")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(WorkLogEntry {
        id: WorkLogEntryId(parse_uuid(row, "id", &id)?),
        task_id: TaskId(parse_uuid(row, "task_id", &task_id)?),
        content: row.get("content")?,
        created_at: parse_datetime(row, "created_at", &created_at)?,
        updated_at: parse_datetime(row, "updated_at", &updated_at)?,
    })
}

/// Parse a Habit from a SQLite row (without check-ins).
pub(crate) fn habit_from_row(row: &Row<'_>) -> rusqlite::Result<Habit> {
    let id: String = row.get("id")?;
    let frequency_json: String = row.get("frequency")?;
    let start_date: String = row.get("start_date")?;
    let end_date: Option<String> = row.get("end_date")?;
    let tags_json: String = row.get("tags")?;
    let archived: i32 = row.get("archived")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(Habit {
        id: HabitId(parse_uuid(row, "id", &id)?),
        name: row.get("name")?,
        description: row.get("description")?,
        frequency: parse_json::<HabitFrequency>(row, "frequency", &frequency_json)?,
        start_date: parse_date(row, "start_date", &start_date)?,
        end_date: parse_optional_date(row, "end_date", end_date)?,
        check_ins: std::collections::HashMap::new(), // Populated separately
        color: row.get("color")?,
        icon: row.get("icon")?,
        tags: parse_json(row, "tags", &tags_json)?,
        archived: archived != 0,
        created_at: parse_datetime(row, "created_at", &created_at)?,
        updated_at: parse_datetime(row, "updated_at", &updated_at)?,
    })
}

/// Parse one `habit_check_ins` row selected as
/// `date, completed, note, checked_at`.
pub(crate) fn habit_check_in_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<(NaiveDate, HabitCheckIn)> {
    let date_str: String = row.get(0)?;
    let completed: i32 = row.get(1)?;
    let note: Option<String> = row.get(2)?;
    let checked_at_str: String = row.get(3)?;

    let date = parse_date(row, "date", &date_str)?;
    let checked_at = parse_datetime(row, "checked_at", &checked_at_str)?;

    Ok((
        date,
        HabitCheckIn {
            date,
            completed: completed != 0,
            note,
            checked_at,
        },
    ))
}

/// Parse a Goal from a SQLite row.
pub(crate) fn goal_from_row(row: &Row<'_>) -> rusqlite::Result<Goal> {
    let id: String = row.get("id")?;
    let status_str: String = row.get("status")?;
    let start_date: Option<String> = row.get("start_date")?;
    let due_date: Option<String> = row.get("due_date")?;
    let quarter_json: Option<String> = row.get("quarter")?;
    let manual_progress: Option<i32> = row.get("manual_progress")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(Goal {
        id: GoalId(parse_uuid(row, "id", &id)?),
        name: row.get("name")?,
        description: row.get("description")?,
        status: GoalStatus::from_str_lossy(&status_str),
        start_date: parse_optional_date(row, "start_date", start_date)?,
        due_date: parse_optional_date(row, "due_date", due_date)?,
        quarter: parse_optional_json(row, "quarter", quarter_json)?,
        manual_progress: manual_progress.map(|p| p.clamp(0, 100) as u8),
        color: row.get("color")?,
        icon: row.get("icon")?,
        created_at: parse_datetime(row, "created_at", &created_at)?,
        updated_at: parse_datetime(row, "updated_at", &updated_at)?,
    })
}

/// Parse a KeyResult from a SQLite row.
pub(crate) fn key_result_from_row(row: &Row<'_>) -> rusqlite::Result<KeyResult> {
    let id: String = row.get("id")?;
    let goal_id: String = row.get("goal_id")?;
    let status_str: String = row.get("status")?;
    let manual_progress: Option<i32> = row.get("manual_progress")?;
    let linked_project_ids_json: String = row.get("linked_project_ids")?;
    let linked_task_ids_json: String = row.get("linked_task_ids")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;

    Ok(KeyResult {
        id: KeyResultId(parse_uuid(row, "id", &id)?),
        goal_id: GoalId(parse_uuid(row, "goal_id", &goal_id)?),
        name: row.get("name")?,
        description: row.get("description")?,
        status: KeyResultStatus::from_str_lossy(&status_str),
        target_value: row.get("target_value")?,
        current_value: row.get("current_value")?,
        unit: row.get("unit")?,
        manual_progress: manual_progress.map(|p| p.clamp(0, 100) as u8),
        linked_project_ids: parse_uuid_list(row, "linked_project_ids", &linked_project_ids_json)?
            .into_iter()
            .map(ProjectId)
            .collect(),
        linked_task_ids: parse_uuid_list(row, "linked_task_ids", &linked_task_ids_json)?
            .into_iter()
            .map(TaskId)
            .collect(),
        created_at: parse_datetime(row, "created_at", &created_at)?,
        updated_at: parse_datetime(row, "updated_at", &updated_at)?,
    })
}
