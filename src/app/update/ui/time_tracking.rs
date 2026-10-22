//! Time log editor handlers.
//!
//! Every mutation goes through `Model::add_time_entry` /
//! `Model::upsert_time_entry` / `Model::delete_time_entry` so the task's
//! `actual_minutes` and the derived caches stay in step, and every edit
//! records an undo action.

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc};

use crate::app::{Model, UiMessage, UndoAction};
use crate::domain::duration::parse_minutes;
use crate::domain::{format_minutes, today, LocalDate, TaskId, TimeEntry, TimeEntryId};
use crate::ui::{day_label, TimeLogMode};

/// Shortest duration an entry can be nudged down to, in minutes.
pub const NUDGE_MIN_MINUTES: u32 = 5;

/// Syntax hint appended to add-entry parse errors.
pub const ADD_SYNTAX: &str = "[date] START[-END | +DURATION] [note]";

/// A parsed add-entry prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEntrySpec {
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub minutes: u32,
    pub description: Option<String>,
}

/// Handle time log editor messages
pub fn handle_ui_time_log(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowTimeLog => {
            // Only show if a task is selected
            if model.selected_index < model.visible_tasks.len() {
                model.time_log.visible = true;
                model.time_log.selected = 0;
                model.time_log.mode = TimeLogMode::Browse;
                model.time_log.buffer.clear();
            }
        }
        UiMessage::HideTimeLog => {
            model.time_log.visible = false;
            model.time_log.mode = TimeLogMode::Browse;
            model.time_log.buffer.clear();
        }
        UiMessage::TimeLogUp => {
            if model.time_log.selected > 0 {
                model.time_log.selected -= 1;
            }
        }
        UiMessage::TimeLogDown => {
            if let Some(task_id) = model.selected_task_id() {
                let entries = model.time_entries_for_task(&task_id);
                if model.time_log.selected < entries.len().saturating_sub(1) {
                    model.time_log.selected += 1;
                }
            }
        }
        UiMessage::TimeLogEditStart => {
            if let Some(entry) = selected_entry(model) {
                let start_time = entry
                    .started_at
                    .with_timezone(&Local)
                    .format("%H:%M")
                    .to_string();
                open_prompt(model, TimeLogMode::EditStart, start_time);
            }
        }
        UiMessage::TimeLogEditEnd => {
            if let Some(entry) = selected_entry(model) {
                // Can't edit end time of running entry
                if entry.is_running() {
                    model.alerts.status_message =
                        Some("Cannot edit end time of running entry".to_string());
                    return;
                }
                let end_time = entry
                    .ended_at
                    .map(|t| t.with_timezone(&Local).format("%H:%M").to_string())
                    .unwrap_or_default();
                open_prompt(model, TimeLogMode::EditEnd, end_time);
            }
        }
        UiMessage::TimeLogEditNote => {
            if let Some(entry) = selected_entry(model) {
                let note = entry.description.clone().unwrap_or_default();
                open_prompt(model, TimeLogMode::EditNote, note);
            }
        }
        UiMessage::TimeLogEditDate => {
            if let Some(entry) = selected_entry(model) {
                if entry.is_running() {
                    model.alerts.status_message =
                        Some("Cannot move the running entry; stop the timer first".to_string());
                    return;
                }
                let date = entry.started_at.local_date().format("%Y-%m-%d").to_string();
                open_prompt(model, TimeLogMode::EditDate, date);
            }
        }
        UiMessage::TimeLogAddEntry => {
            if model.selected_task_id().is_some() {
                open_prompt(model, TimeLogMode::Add, format!("{} ", today()));
            }
        }
        UiMessage::TimeLogNudge(delta) => nudge_selected(model, delta),
        UiMessage::TimeLogInputChar(c) => {
            if model.time_log.mode.is_text_input() {
                model.time_log.buffer.push(c);
            }
        }
        UiMessage::TimeLogInputBackspace => {
            if model.time_log.mode.is_text_input() {
                model.time_log.buffer.pop();
            }
        }
        UiMessage::TimeLogConfirmDelete => {
            model.time_log.mode = TimeLogMode::ConfirmDelete;
        }
        UiMessage::TimeLogCancel => {
            model.time_log.mode = TimeLogMode::Browse;
            model.time_log.buffer.clear();
        }
        UiMessage::TimeLogSubmit => match model.time_log.mode {
            TimeLogMode::Add => submit_add(model),
            TimeLogMode::EditStart
            | TimeLogMode::EditEnd
            | TimeLogMode::EditNote
            | TimeLogMode::EditDate => submit_edit(model),
            TimeLogMode::Browse | TimeLogMode::ConfirmDelete => {}
        },
        UiMessage::TimeLogDelete if model.time_log.mode == TimeLogMode::ConfirmDelete => {
            if let Some(task_id) = model.selected_task_id() {
                let entries = model.time_entries_for_task(&task_id);
                if let Some(entry) = entries.get(model.time_log.selected) {
                    let entry_id = entry.id;

                    // Can't delete if it's the active entry
                    if model.active_time_entry.as_ref() == Some(&entry_id) {
                        model.alerts.status_message =
                            Some("Cannot delete running time entry".to_string());
                        model.time_log.mode = TimeLogMode::Browse;
                        return;
                    }

                    if let Some(removed) = model.time_entries.get(&entry_id).cloned() {
                        model
                            .undo_stack
                            .push(UndoAction::TimeEntryDeleted(Box::new(removed)));
                        model.delete_time_entry(&entry_id);

                        // Adjust selection
                        let remaining = model.time_entries_for_task(&task_id);
                        if model.time_log.selected >= remaining.len() && !remaining.is_empty() {
                            model.time_log.selected = remaining.len() - 1;
                        }
                        model.alerts.status_message = Some("Time entry deleted".to_string());
                    }
                }
            }
            model.time_log.mode = TimeLogMode::Browse;
        }
        _ => {}
    }
}

/// The entry under the cursor, if any.
fn selected_entry(model: &Model) -> Option<TimeEntry> {
    let task_id = model.selected_task_id()?;
    model
        .time_entries_for_task(&task_id)
        .get(model.time_log.selected)
        .map(|e| (*e).clone())
}

fn open_prompt(model: &mut Model, mode: TimeLogMode, buffer: String) {
    model.time_log.mode = mode;
    model.time_log.buffer = buffer;
}

fn close_prompt(model: &mut Model) {
    model.time_log.mode = TimeLogMode::Browse;
    model.time_log.buffer.clear();
}

/// Move the cursor onto `entry_id` (entries are listed by day, newest day
/// first and chronological within a day, so an edited start time or date can
/// change an entry's position).
fn select_entry(model: &mut Model, task_id: &TaskId, entry_id: TimeEntryId) {
    if let Some(pos) = model
        .time_entries_for_task(task_id)
        .iter()
        .position(|e| e.id == entry_id)
    {
        model.time_log.selected = pos;
    }
}

/// Store an edited copy of an entry, recording it for undo.
fn commit_edit(model: &mut Model, before: TimeEntry, after: TimeEntry) {
    let task_id = after.task_id;
    let entry_id = after.id;
    model.undo_stack.push(UndoAction::TimeEntryModified {
        before: Box::new(before),
        after: Box::new(after.clone()),
    });
    model.upsert_time_entry(after);
    select_entry(model, &task_id, entry_id);
}

fn set_duration(entry: &mut TimeEntry, ended_at: DateTime<Utc>) {
    entry.ended_at = Some(ended_at);
    let minutes = ended_at
        .signed_duration_since(entry.started_at)
        .num_minutes();
    entry.duration_minutes = Some(u32::try_from(minutes.max(0)).unwrap_or(u32::MAX));
}

fn submit_edit(model: &mut Model) {
    let Some(before) = selected_entry(model) else {
        close_prompt(model);
        return;
    };
    let mut after = before.clone();
    let buffer = model.time_log.buffer.trim().to_string();

    let status = match model.time_log.mode {
        TimeLogMode::EditStart => {
            let Some(time) = super::parse_single_time(&buffer) else {
                model.alerts.status_message = Some("Invalid time format. Use HH:MM".to_string());
                return;
            };
            // Keep the same local date; a start after the end shortens to zero.
            let Some(new_start) = local_to_utc(before.started_at.local_date(), time) else {
                model.alerts.status_message =
                    Some("That time does not exist locally (DST gap)".to_string());
                return;
            };
            after.started_at = new_start;
            if let Some(ended_at) = after.ended_at {
                set_duration(&mut after, ended_at);
            }
            "Time entry updated".to_string()
        }
        TimeLogMode::EditEnd => {
            let Some(time) = super::parse_single_time(&buffer) else {
                model.alerts.status_message = Some("Invalid time format. Use HH:MM".to_string());
                return;
            };
            // End on the start's local date; an end before the start rolls
            // over to the next day.
            let Some(new_end) = local_to_utc(before.started_at.local_date(), time) else {
                model.alerts.status_message =
                    Some("That time does not exist locally (DST gap)".to_string());
                return;
            };
            let new_end = if new_end < before.started_at {
                new_end + Duration::days(1)
            } else {
                new_end
            };
            set_duration(&mut after, new_end);
            "Time entry updated".to_string()
        }
        TimeLogMode::EditNote => {
            after.description = if buffer.is_empty() {
                None
            } else {
                Some(buffer)
            };
            if after.description.is_some() {
                "Note updated".to_string()
            } else {
                "Note cleared".to_string()
            }
        }
        TimeLogMode::EditDate => {
            let Some(date) = parse_date_spec(&buffer, today()) else {
                model.alerts.status_message =
                    Some("Invalid date. Use YYYY-MM-DD, today, yesterday or -N".to_string());
                return;
            };
            if before.is_running() {
                model.alerts.status_message = Some("Cannot move the running entry".to_string());
                close_prompt(model);
                return;
            }
            let clock = before.started_at.with_timezone(&Local).time();
            let Some(new_start) = local_to_utc(date, clock) else {
                model.alerts.status_message =
                    Some("That time does not exist locally on that date (DST gap)".to_string());
                return;
            };
            let shift = new_start.signed_duration_since(before.started_at);
            after.started_at = new_start;
            after.ended_at = before.ended_at.map(|e| e + shift);
            format!("Moved entry to {}", day_label(date))
        }
        TimeLogMode::Browse | TimeLogMode::Add | TimeLogMode::ConfirmDelete => return,
    };

    commit_edit(model, before, after);
    model.alerts.status_message = Some(status);
    close_prompt(model);
}

fn submit_add(model: &mut Model) {
    let Some(task_id) = model.selected_task_id() else {
        close_prompt(model);
        return;
    };
    match parse_add_entry(&model.time_log.buffer, today()) {
        Ok(spec) => {
            let mut entry = TimeEntry::start(task_id);
            entry.started_at = spec.started_at;
            entry.ended_at = Some(spec.ended_at);
            entry.duration_minutes = Some(spec.minutes);
            entry.description = spec.description;
            let entry_id = entry.id;
            let day = day_label(spec.started_at.local_date());

            model
                .undo_stack
                .push(UndoAction::TimeEntryStarted(Box::new(entry.clone())));
            model.add_time_entry(entry);
            select_entry(model, &task_id, entry_id);
            model.alerts.status_message =
                Some(format!("Added {} on {day}", format_minutes(spec.minutes)));
            close_prompt(model);
        }
        Err(reason) => {
            // Keep the prompt open so the input can be corrected.
            model.alerts.status_message = Some(format!("Add: {reason} · {ADD_SYNTAX}"));
        }
    }
}

fn nudge_selected(model: &mut Model, delta: i32) {
    let Some(before) = selected_entry(model) else {
        return;
    };
    if before.is_running() {
        model.alerts.status_message =
            Some("Cannot nudge the running entry; stop the timer first".to_string());
        return;
    }
    let current = before.calculated_duration_minutes();
    let target = i64::from(current) + i64::from(delta);
    let new_minutes = u32::try_from(target.max(i64::from(NUDGE_MIN_MINUTES))).unwrap_or(u32::MAX);
    if new_minutes == current {
        model.alerts.status_message = Some(format!(
            "Entry is already at the {NUDGE_MIN_MINUTES}-minute minimum"
        ));
        return;
    }
    let mut after = before.clone();
    set_duration(
        &mut after,
        before.started_at + Duration::minutes(i64::from(new_minutes)),
    );
    let sign = if delta >= 0 { "+" } else { "-" };
    let status = format!(
        "Duration {} ({sign}{}m)",
        format_minutes(new_minutes),
        delta.abs()
    );
    commit_edit(model, before, after);
    model.alerts.status_message = Some(status);
}

/// Parse an add-entry prompt: `[date] START[-END | +DURATION] [note]`.
///
/// - `date`: `YYYY-MM-DD`, `today`, `yesterday` or `-N` (days ago); defaults to `today`
/// - `START`, `END`: local clock times (`HH:MM`, `9:00`, `2:30pm`)
/// - `DURATION`: `1h30m`, `90m`, `1.5h`, `2h`
/// - an `END` at or before `START` rolls over to the next day
/// - the rest of the line is the note
///
/// Examples: `09:00-10:30 standup`, `-1 14:00 +45m review`, `2026-10-20 08:15-09:00`.
///
/// # Errors
///
/// Returns a short, user-facing reason when the line cannot be parsed or
/// would produce an empty entry.
pub fn parse_add_entry(input: &str, today: NaiveDate) -> Result<NewEntrySpec, String> {
    let mut tokens = input.split_whitespace().peekable();

    let Some(&first) = tokens.peek() else {
        return Err("nothing to add".to_string());
    };
    let date = match parse_date_spec(first, today) {
        Some(d) => {
            tokens.next();
            d
        }
        None => today,
    };

    let Some(time_token) = tokens.next() else {
        return Err("missing start time (HH:MM)".to_string());
    };

    // Split "09:00-10:30" / "14:00+45m" into start and tail; a bare start
    // takes its tail from the next token(s): "14:00 +45m", "09:00 - 10:30".
    let (start_str, mut op, mut tail) = match time_token.find(['-', '+']) {
        Some(pos) => (
            &time_token[..pos],
            time_token[pos..].chars().next(),
            time_token[pos + 1..].to_string(),
        ),
        None => (time_token, None, String::new()),
    };
    if op.is_none() {
        if let Some(next) = tokens.peek() {
            if let Some(stripped) = next.strip_prefix(['-', '+']) {
                op = next.chars().next();
                tail = stripped.to_string();
                tokens.next();
            }
        }
    }
    if op.is_some() && tail.is_empty() {
        tail = tokens.next().unwrap_or_default().to_string();
    }

    let Some(start) = super::parse_single_time(start_str) else {
        return Err(format!("bad start time '{start_str}' (use HH:MM)"));
    };
    let Some(started_at) = local_to_utc(date, start) else {
        return Err("start time does not exist locally (DST gap)".to_string());
    };

    let ended_at = match op {
        Some('-') => {
            let Some(end) = super::parse_single_time(&tail) else {
                return Err(format!("bad end time '{tail}' (use HH:MM)"));
            };
            let Some(end_dt) = local_to_utc(date, end) else {
                return Err("end time does not exist locally (DST gap)".to_string());
            };
            if end_dt <= started_at {
                end_dt + Duration::days(1)
            } else {
                end_dt
            }
        }
        Some('+') => {
            let Some(minutes) = parse_minutes(&tail) else {
                return Err(format!("bad duration '{tail}' (use 1h30m, 90m, 1.5h)"));
            };
            if minutes == 0 {
                return Err("duration must be longer than 0 minutes".to_string());
            }
            started_at + Duration::minutes(i64::from(minutes))
        }
        _ => return Err("give an end time (-HH:MM) or a duration (+1h30m)".to_string()),
    };

    let minutes =
        u32::try_from(ended_at.signed_duration_since(started_at).num_minutes()).unwrap_or(u32::MAX);
    if minutes == 0 {
        return Err("entry must be longer than 0 minutes".to_string());
    }

    let note = tokens.collect::<Vec<_>>().join(" ");
    let description = if note.is_empty() { None } else { Some(note) };

    Ok(NewEntrySpec {
        started_at,
        ended_at,
        minutes,
        description,
    })
}

/// Parse a date token: `YYYY-MM-DD`, `today`, `yesterday` or `-N` (N days ago).
#[must_use]
pub fn parse_date_spec(token: &str, today: NaiveDate) -> Option<NaiveDate> {
    let token = token.trim();
    match token.to_ascii_lowercase().as_str() {
        "today" => return Some(today),
        "yesterday" => return today.checked_sub_signed(Duration::days(1)),
        _ => {}
    }
    if let Some(days) = token.strip_prefix('-') {
        if let Ok(n) = days.parse::<u32>() {
            return today.checked_sub_signed(Duration::days(i64::from(n)));
        }
    }
    NaiveDate::parse_from_str(token, "%Y-%m-%d").ok()
}

/// Interpret a wall-clock time on a local calendar date and convert it to UTC.
///
/// Returns `None` for times that do not exist locally (DST gap).
fn local_to_utc(date: NaiveDate, time: NaiveTime) -> Option<DateTime<Utc>> {
    Local
        .from_local_datetime(&date.and_time(time))
        .single()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_add_entry_unit_cases() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();

        let spec = parse_add_entry("09:00-10:30 standup", today).unwrap();
        assert_eq!(spec.minutes, 90);
        assert_eq!(spec.started_at.local_date(), today);
        assert_eq!(spec.description.as_deref(), Some("standup"));

        let spec = parse_add_entry("-3 14:00 +45m", today).unwrap();
        assert_eq!(spec.started_at.local_date(), today - Duration::days(3));
        assert_eq!(spec.minutes, 45);
        assert_eq!(spec.description, None);

        // spaced forms
        assert_eq!(parse_add_entry("09:00 - 10:00", today).unwrap().minutes, 60);
        assert_eq!(parse_add_entry("09:00 + 90m", today).unwrap().minutes, 90);
        // 12-hour clock and multi-word note
        let spec = parse_add_entry("2pm-3:15pm pair programming", today).unwrap();
        assert_eq!(spec.minutes, 75);
        assert_eq!(spec.description.as_deref(), Some("pair programming"));
        // equal start and end rolls a full day
        assert_eq!(parse_add_entry("09:00-09:00", today).unwrap().minutes, 1440);

        assert!(parse_add_entry("2026-13-40 09:00-10:00", today).is_err());
        assert!(parse_add_entry("25:00-26:00", today).is_err());
        assert!(parse_add_entry("   ", today).is_err());
    }

    #[test]
    fn parse_date_spec_forms() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
        assert_eq!(parse_date_spec("today", today), Some(today));
        assert_eq!(
            parse_date_spec("Yesterday", today),
            Some(today - Duration::days(1))
        );
        assert_eq!(parse_date_spec("-0", today), Some(today));
        assert_eq!(
            parse_date_spec("-7", today),
            Some(today - Duration::days(7))
        );
        assert_eq!(
            parse_date_spec("2026-01-05", today),
            NaiveDate::from_ymd_opt(2026, 1, 5)
        );
        assert_eq!(parse_date_spec("09:00", today), None);
        assert_eq!(parse_date_spec("tomorrow", today), None);
        assert_eq!(parse_date_spec("-x", today), None);
    }
}
