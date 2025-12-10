//! Time log editor handlers

use chrono::{NaiveTime, Utc};

use crate::app::{Model, UiMessage, UndoAction};
use crate::ui::TimeLogMode;

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
            if let Some(task_id) = model.selected_task_id() {
                let entries = model.time_entries_for_task(&task_id);
                if let Some(entry) = entries.get(model.time_log.selected) {
                    let start_time = entry.started_at.format("%H:%M").to_string();
                    model.time_log.mode = TimeLogMode::EditStart;
                    model.time_log.buffer = start_time;
                }
            }
        }
        UiMessage::TimeLogEditEnd => {
            if let Some(task_id) = model.selected_task_id() {
                let entries = model.time_entries_for_task(&task_id);
                if let Some(entry) = entries.get(model.time_log.selected) {
                    // Can't edit end time of running entry
                    if entry.is_running() {
                        model.alerts.status_message =
                            Some("Cannot edit end time of running entry".to_string());
                        return;
                    }
                    let end_time = entry
                        .ended_at
                        .map(|t| t.format("%H:%M").to_string())
                        .unwrap_or_default();
                    model.time_log.mode = TimeLogMode::EditEnd;
                    model.time_log.buffer = end_time;
                }
            }
        }
        UiMessage::TimeLogConfirmDelete => {
            model.time_log.mode = TimeLogMode::ConfirmDelete;
        }
        UiMessage::TimeLogCancel => {
            model.time_log.mode = TimeLogMode::Browse;
            model.time_log.buffer.clear();
        }
        UiMessage::TimeLogSubmit => {
            if let Some(task_id) = model.selected_task_id() {
                let entries = model.time_entries_for_task(&task_id);
                if let Some(entry) = entries.get(model.time_log.selected) {
                    let entry_id = entry.id;

                    // Parse the time from buffer (HH:MM format)
                    if let Ok(time) = NaiveTime::parse_from_str(&model.time_log.buffer, "%H:%M") {
                        if let Some(entry) = model.time_entries.get_mut(&entry_id) {
                            let before = entry.clone();

                            match model.time_log.mode {
                                TimeLogMode::EditStart => {
                                    // Update start time, keeping the same date
                                    let date = entry.started_at.date_naive();
                                    if let Some(new_dt) =
                                        date.and_time(time).and_local_timezone(Utc).single()
                                    {
                                        entry.started_at = new_dt;
                                        // Recalculate duration if entry is completed
                                        if let Some(ended_at) = entry.ended_at {
                                            let duration =
                                                ended_at.signed_duration_since(entry.started_at);
                                            entry.duration_minutes =
                                                Some(duration.num_minutes().max(0) as u32);
                                        }
                                    }
                                }
                                TimeLogMode::EditEnd => {
                                    // Update end time, keeping the same date
                                    let date = entry.started_at.date_naive();
                                    if let Some(new_dt) =
                                        date.and_time(time).and_local_timezone(Utc).single()
                                    {
                                        entry.ended_at = Some(new_dt);
                                        let duration =
                                            new_dt.signed_duration_since(entry.started_at);
                                        entry.duration_minutes =
                                            Some(duration.num_minutes().max(0) as u32);
                                    }
                                }
                                _ => {}
                            }

                            let after = entry.clone();
                            model.undo_stack.push(UndoAction::TimeEntryModified {
                                before: Box::new(before),
                                after: Box::new(after.clone()),
                            });
                            model.sync_time_entry(&after);
                            model.alerts.status_message = Some("Time entry updated".to_string());
                        }
                    } else {
                        model.alerts.status_message = Some("Invalid time format. Use HH:MM".to_string());
                        return;
                    }
                }
            }
            model.time_log.mode = TimeLogMode::Browse;
            model.time_log.buffer.clear();
        }
        UiMessage::TimeLogAddEntry => {
            if let Some(task_id) = model.selected_task_id() {
                // Create a new 30-minute entry ending now
                let ended_at = Utc::now();
                let started_at = ended_at - chrono::Duration::minutes(30);

                let mut entry = crate::domain::TimeEntry::start(task_id);
                entry.started_at = started_at;
                entry.ended_at = Some(ended_at);
                entry.duration_minutes = Some(30);

                model
                    .undo_stack
                    .push(UndoAction::TimeEntryStarted(Box::new(entry.clone())));
                model.sync_time_entry(&entry);
                model.time_entries.insert(entry.id, entry);
                model.time_log.selected = 0; // New entry will be at top (sorted by date)
                model.alerts.status_message = Some("Added 30-minute time entry".to_string());
            }
        }
        UiMessage::TimeLogDelete => {
            if model.time_log.mode == TimeLogMode::ConfirmDelete {
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

                        if let Some(removed) = model.time_entries.remove(&entry_id) {
                            model
                                .undo_stack
                                .push(UndoAction::TimeEntryDeleted(Box::new(removed.clone())));
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
        }
        _ => {}
    }
}
