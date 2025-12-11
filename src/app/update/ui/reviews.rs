//! Daily, weekly, and evening review handlers

use chrono::Duration;

use crate::app::{Model, UiMessage};
use crate::ui::{DailyReviewPhase, EveningReviewPhase, WeeklyReviewPhase};

/// Handle daily review UI messages
pub fn handle_ui_daily_review(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowDailyReview => {
            model.daily_review.visible = true;
            model.daily_review.phase = DailyReviewPhase::Welcome;
            model.daily_review.selected = 0;
        }
        UiMessage::HideDailyReview => {
            model.daily_review.visible = false;
        }
        UiMessage::DailyReviewNext => {
            model.daily_review.phase = model.daily_review.phase.next();
            model.daily_review.selected = 0;
        }
        UiMessage::DailyReviewPrev => {
            model.daily_review.phase = model.daily_review.phase.prev();
            model.daily_review.selected = 0;
        }
        UiMessage::DailyReviewUp => {
            if model.daily_review.selected > 0 {
                model.daily_review.selected -= 1;
            }
        }
        UiMessage::DailyReviewDown => {
            // Get the task count for current phase
            let today = chrono::Utc::now().date_naive();
            let count = match model.daily_review.phase {
                DailyReviewPhase::OverdueTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
                    .count(),
                DailyReviewPhase::TodayTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date == Some(today))
                    .count(),
                DailyReviewPhase::ScheduledTasks => model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && t.scheduled_date == Some(today)
                            && t.due_date != Some(today)
                    })
                    .count(),
                _ => 0,
            };
            if count > 0 && model.daily_review.selected < count - 1 {
                model.daily_review.selected += 1;
            }
        }
        UiMessage::DailyReviewComplete => {
            // Get the task at the current selection and toggle its completion
            let today = chrono::Utc::now().date_naive();
            let task_ids: Vec<_> = match model.daily_review.phase {
                DailyReviewPhase::OverdueTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
                    .map(|t| t.id)
                    .collect(),
                DailyReviewPhase::TodayTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date == Some(today))
                    .map(|t| t.id)
                    .collect(),
                DailyReviewPhase::ScheduledTasks => model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && t.scheduled_date == Some(today)
                            && t.due_date != Some(today)
                    })
                    .map(|t| t.id)
                    .collect(),
                _ => vec![],
            };

            if let Some(task_id) = task_ids.get(model.daily_review.selected).copied() {
                model.modify_task_with_undo(&task_id, |task| {
                    task.toggle_complete();
                });
                model.alerts.status_message = Some("Task completed!".to_string());

                // Adjust selection if we just removed an item
                let new_count = task_ids.len().saturating_sub(1);
                if model.daily_review.selected >= new_count && new_count > 0 {
                    model.daily_review.selected = new_count - 1;
                }
            }
        }
        _ => {}
    }
}

/// Handle weekly review UI messages
pub fn handle_ui_weekly_review(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowWeeklyReview => {
            model.weekly_review.visible = true;
            model.weekly_review.phase = WeeklyReviewPhase::Welcome;
            model.weekly_review.selected = 0;
        }
        UiMessage::HideWeeklyReview => {
            model.weekly_review.visible = false;
        }
        UiMessage::WeeklyReviewNext => {
            model.weekly_review.phase = model.weekly_review.phase.next();
            model.weekly_review.selected = 0;
        }
        UiMessage::WeeklyReviewPrev => {
            model.weekly_review.phase = model.weekly_review.phase.prev();
            model.weekly_review.selected = 0;
        }
        UiMessage::WeeklyReviewUp => {
            if model.weekly_review.selected > 0 {
                model.weekly_review.selected -= 1;
            }
        }
        UiMessage::WeeklyReviewDown => {
            // Get the count for current phase
            let today = chrono::Utc::now().date_naive();
            let week_ago = today - chrono::Duration::days(7);
            let week_ahead = today + chrono::Duration::days(7);

            let count = match model.weekly_review.phase {
                WeeklyReviewPhase::CompletedTasks => model
                    .tasks
                    .values()
                    .filter(|t| {
                        t.status.is_complete()
                            && t.completed_at.is_some_and(|d| d.date_naive() >= week_ago)
                    })
                    .count(),
                WeeklyReviewPhase::OverdueTasks => model
                    .tasks
                    .values()
                    .filter(|t| !t.status.is_complete() && t.due_date.is_some_and(|d| d < today))
                    .count(),
                WeeklyReviewPhase::UpcomingWeek => model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && t.due_date.is_some_and(|d| d >= today && d <= week_ahead)
                    })
                    .count(),
                WeeklyReviewPhase::StaleProjects => {
                    // Count stale projects
                    model
                        .projects
                        .iter()
                        .filter(|(id, _)| {
                            let task_count = model
                                .tasks
                                .values()
                                .filter(|t| {
                                    t.project_id.as_ref() == Some(*id) && !t.status.is_complete()
                                })
                                .count();
                            let has_recent = model.tasks.values().any(|t| {
                                t.project_id.as_ref() == Some(*id)
                                    && t.updated_at.date_naive() >= week_ago
                            });
                            task_count > 0 && !has_recent
                        })
                        .count()
                }
                _ => 0,
            };

            if count > 0 && model.weekly_review.selected < count - 1 {
                model.weekly_review.selected += 1;
            }
        }
        _ => {}
    }
}

/// Handle evening review UI messages
pub fn handle_ui_evening_review(model: &mut Model, msg: UiMessage) {
    match msg {
        UiMessage::ShowEveningReview => {
            model.evening_review.visible = true;
            model.evening_review.phase = EveningReviewPhase::Welcome;
            model.evening_review.selected = 0;
        }
        UiMessage::HideEveningReview => {
            model.evening_review.visible = false;
        }
        UiMessage::EveningReviewNext => {
            let next_phase = model.evening_review.phase.next();

            // Auto-skip TimeReview if no time entries today
            let today = chrono::Utc::now().date_naive();
            let has_time_entries = model
                .time_entries
                .values()
                .any(|e| e.started_at.date_naive() == today);

            if next_phase == EveningReviewPhase::TimeReview && !has_time_entries {
                // Skip to Summary
                model.evening_review.phase = EveningReviewPhase::Summary;
            } else {
                model.evening_review.phase = next_phase;
            }
            model.evening_review.selected = 0;
        }
        UiMessage::EveningReviewPrev => {
            let prev_phase = model.evening_review.phase.prev();

            // Auto-skip TimeReview when going back if no time entries today
            let today = chrono::Utc::now().date_naive();
            let has_time_entries = model
                .time_entries
                .values()
                .any(|e| e.started_at.date_naive() == today);

            if prev_phase == EveningReviewPhase::TimeReview && !has_time_entries {
                // Skip back to TomorrowPreview
                model.evening_review.phase = EveningReviewPhase::TomorrowPreview;
            } else {
                model.evening_review.phase = prev_phase;
            }
            model.evening_review.selected = 0;
        }
        UiMessage::EveningReviewUp => {
            if model.evening_review.selected > 0 {
                model.evening_review.selected -= 1;
            }
        }
        UiMessage::EveningReviewDown => {
            // Get the task count for incomplete tasks phase
            if model.evening_review.phase == EveningReviewPhase::IncompleteTasks {
                let today = chrono::Utc::now().date_naive();
                let count = model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && (t.due_date == Some(today) || t.scheduled_date == Some(today))
                    })
                    .count();
                if count > 0 && model.evening_review.selected < count - 1 {
                    model.evening_review.selected += 1;
                }
            }
        }
        UiMessage::EveningReviewReschedule => {
            // Reschedule selected task to tomorrow
            if model.evening_review.phase == EveningReviewPhase::IncompleteTasks {
                let today = chrono::Utc::now().date_naive();
                let tomorrow = today + Duration::days(1);

                let task_ids: Vec<_> = model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && (t.due_date == Some(today) || t.scheduled_date == Some(today))
                    })
                    .map(|t| t.id)
                    .collect();

                if let Some(task_id) = task_ids.get(model.evening_review.selected).copied() {
                    model.modify_task_with_undo(&task_id, |task| {
                        // Clear today's scheduled_date and set to tomorrow
                        task.scheduled_date = Some(tomorrow);
                    });
                    model.alerts.status_message = Some("Task rescheduled to tomorrow".to_string());

                    // Adjust selection
                    let new_count = task_ids.len().saturating_sub(1);
                    if model.evening_review.selected >= new_count && new_count > 0 {
                        model.evening_review.selected = new_count - 1;
                    }
                }
            }
        }
        UiMessage::EveningReviewSnooze => {
            // Snooze selected task until tomorrow
            if model.evening_review.phase == EveningReviewPhase::IncompleteTasks {
                let today = chrono::Utc::now().date_naive();
                let tomorrow = today + Duration::days(1);

                let task_ids: Vec<_> = model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && (t.due_date == Some(today) || t.scheduled_date == Some(today))
                    })
                    .map(|t| t.id)
                    .collect();

                if let Some(task_id) = task_ids.get(model.evening_review.selected).copied() {
                    model.modify_task_with_undo(&task_id, |task| {
                        task.snooze_until = Some(tomorrow);
                    });
                    model.alerts.status_message = Some("Task snoozed until tomorrow".to_string());

                    // Adjust selection
                    let new_count = task_ids.len().saturating_sub(1);
                    if model.evening_review.selected >= new_count && new_count > 0 {
                        model.evening_review.selected = new_count - 1;
                    }
                }
            }
        }
        UiMessage::EveningReviewComplete => {
            // Mark selected task as complete
            if model.evening_review.phase == EveningReviewPhase::IncompleteTasks {
                let today = chrono::Utc::now().date_naive();

                let task_ids: Vec<_> = model
                    .tasks
                    .values()
                    .filter(|t| {
                        !t.status.is_complete()
                            && (t.due_date == Some(today) || t.scheduled_date == Some(today))
                    })
                    .map(|t| t.id)
                    .collect();

                if let Some(task_id) = task_ids.get(model.evening_review.selected).copied() {
                    model.modify_task_with_undo(&task_id, |task| {
                        task.toggle_complete();
                    });
                    model.alerts.status_message = Some("Task completed!".to_string());

                    // Adjust selection
                    let new_count = task_ids.len().saturating_sub(1);
                    if model.evening_review.selected >= new_count && new_count > 0 {
                        model.evening_review.selected = new_count - 1;
                    }
                }
            }
        }
        _ => {}
    }
}
