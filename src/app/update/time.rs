//! Time tracking and Pomodoro message handlers
//!
//! Handles all time-related messages including:
//! - Starting/stopping time tracking
//! - Pomodoro timer control (start, pause, resume, skip, stop)
//! - Pomodoro configuration changes

use crate::app::{Model, PomodoroMessage, TimeMessage, UndoAction};
use crate::domain::{PomodoroPhase, PomodoroSession, TaskId};

/// Handle time tracking messages
pub fn handle_time(model: &mut Model, msg: TimeMessage) {
    match msg {
        TimeMessage::StartTracking => {
            if let Some(task_id) = model.selected_task_id() {
                start_tracking_with_undo(model, task_id);
            }
        }
        TimeMessage::StopTracking => {
            stop_tracking_with_undo(model);
        }
        TimeMessage::ToggleTracking => {
            if let Some(task_id) = model.selected_task_id() {
                if model.is_tracking_task(&task_id) {
                    stop_tracking_with_undo(model);
                } else {
                    start_tracking_with_undo(model, task_id);
                }
            }
        }
    }

    model.invalidate_report_cache();
}

/// Start the timer on `task_id` and record an undo action.
///
/// Any running timer on another task is stopped first (recorded as a switch).
pub(crate) fn start_tracking_with_undo(model: &mut Model, task_id: TaskId) {
    let (new_entry, stopped_entry) = model.start_time_tracking(task_id);
    if let Some((before, after)) = stopped_entry {
        model.undo_stack.push(UndoAction::TimerSwitched {
            stopped_entry_before: Box::new(before),
            stopped_entry_after: Box::new(after),
            started_entry: Box::new(new_entry),
        });
    } else {
        model
            .undo_stack
            .push(UndoAction::TimeEntryStarted(Box::new(new_entry)));
    }
}

/// Stop the running timer, if any, and record an undo action.
pub(crate) fn stop_tracking_with_undo(model: &mut Model) {
    if let Some((before, after)) = model.stop_time_tracking() {
        model.undo_stack.push(UndoAction::TimeEntryStopped {
            before: Box::new(before),
            after: Box::new(after),
        });
    }
}

/// Keep the task timer in step with the Pomodoro session.
///
/// Work phases are real tracked time: while a work phase is running (not
/// paused) the session's task has a timer going; during breaks, pauses and
/// after the session ends, a timer that *this* session owns is stopped. A
/// timer the user started manually on a different task is left alone.
fn sync_timer_with_pomodoro(model: &mut Model) {
    let Some((task_id, working)) = model
        .pomodoro
        .session
        .as_ref()
        .map(|s| (s.task_id, s.phase == PomodoroPhase::Work && !s.paused))
    else {
        // Session ended: stop the timer only if it is on the task we were timing.
        if let Some(task_id) = model.pomodoro.timed_task.take() {
            if model.is_tracking_task(&task_id) {
                stop_tracking_with_undo(model);
            }
        }
        return;
    };

    if working {
        if !model.is_tracking_task(&task_id) {
            start_tracking_with_undo(model, task_id);
        }
        model.pomodoro.timed_task = Some(task_id);
    } else if model.pomodoro.timed_task.take().is_some() && model.is_tracking_task(&task_id) {
        stop_tracking_with_undo(model);
    }
    model.invalidate_report_cache();
}

pub fn handle_pomodoro(model: &mut Model, msg: PomodoroMessage) {
    handle_pomodoro_inner(model, msg);
    sync_timer_with_pomodoro(model);
}

fn handle_pomodoro_inner(model: &mut Model, msg: PomodoroMessage) {
    match msg {
        PomodoroMessage::Start { goal_cycles } => {
            // Start a new session for the selected task
            if let Some(task) = model.selected_task() {
                let task_id = task.id;
                model.pomodoro.session = Some(PomodoroSession::new(
                    task_id,
                    &model.pomodoro.config,
                    goal_cycles,
                ));
                // Automatically enter focus mode
                model.focus_mode = true;
                model.alerts.status_message =
                    Some(format!("Pomodoro started: {goal_cycles} cycle goal"));
            } else {
                model.alerts.status_message = Some("Select a task to start Pomodoro".to_string());
            }
        }
        PomodoroMessage::Pause => {
            if let Some(ref mut session) = model.pomodoro.session {
                if !session.paused {
                    session.paused = true;
                    session.paused_at = Some(chrono::Utc::now());
                }
            }
        }
        PomodoroMessage::Resume => {
            if let Some(ref mut session) = model.pomodoro.session {
                if session.paused {
                    // Add elapsed pause time to total paused duration
                    if let Some(pause_start) = session.paused_at {
                        let pause_duration =
                            (chrono::Utc::now() - pause_start).num_seconds().max(0) as u32;
                        session.paused_duration_secs += pause_duration;
                    }
                    session.paused = false;
                    session.paused_at = None;
                }
            }
        }
        PomodoroMessage::TogglePause => {
            if let Some(ref mut session) = model.pomodoro.session {
                if session.paused {
                    // Resuming - add elapsed pause time
                    if let Some(pause_start) = session.paused_at {
                        let pause_duration =
                            (chrono::Utc::now() - pause_start).num_seconds().max(0) as u32;
                        session.paused_duration_secs += pause_duration;
                    }
                    session.paused = false;
                    session.paused_at = None;
                } else {
                    // Pausing - record pause start
                    session.paused = true;
                    session.paused_at = Some(chrono::Utc::now());
                }
            }
        }
        PomodoroMessage::Skip => {
            if model.pomodoro.session.is_some() {
                transition_pomodoro_phase(model);
            }
        }
        PomodoroMessage::Stop => {
            if model.pomodoro.session.is_some() {
                model.pomodoro.session = None;
                model.alerts.status_message = Some("Pomodoro session stopped".to_string());
            }
        }
        PomodoroMessage::Tick => {
            let should_transition = if let Some(ref mut session) = model.pomodoro.session {
                if !session.paused && session.remaining_secs > 0 {
                    session.remaining_secs -= 1;
                }
                session.remaining_secs == 0
            } else {
                false
            };

            if should_transition {
                transition_pomodoro_phase(model);
            }
        }
        PomodoroMessage::SetWorkDuration(mins) => {
            model.pomodoro.config.work_duration_mins = mins.max(1);
        }
        PomodoroMessage::SetShortBreak(mins) => {
            model.pomodoro.config.short_break_mins = mins.max(1);
        }
        PomodoroMessage::SetLongBreak(mins) => {
            model.pomodoro.config.long_break_mins = mins.max(1);
        }
        PomodoroMessage::SetCyclesBeforeLongBreak(cycles) => {
            model.pomodoro.config.cycles_before_long_break = cycles.max(1);
        }
        PomodoroMessage::IncrementGoal => {
            if let Some(ref mut session) = model.pomodoro.session {
                session.session_goal += 1;
            }
        }
        PomodoroMessage::DecrementGoal => {
            if let Some(ref mut session) = model.pomodoro.session {
                if session.session_goal > 1 {
                    session.session_goal -= 1;
                }
            }
        }
    }
}

/// Transition to the next Pomodoro phase
#[allow(clippy::too_many_lines)]
fn transition_pomodoro_phase(model: &mut Model) {
    let (next_phase, next_remaining, cycles_completed, message) = {
        let Some(session) = model.pomodoro.session.as_ref() else {
            return;
        };

        match session.phase {
            PomodoroPhase::Work => {
                // Record the completed work cycle
                let new_cycles = session.cycles_completed + 1;

                // Determine if long break or short break
                if new_cycles > 0
                    && new_cycles % model.pomodoro.config.cycles_before_long_break == 0
                {
                    (
                        PomodoroPhase::LongBreak,
                        model.pomodoro.config.long_break_mins * 60,
                        new_cycles,
                        format!("🎉 Cycle {new_cycles} complete! Time for a long break."),
                    )
                } else {
                    (
                        PomodoroPhase::ShortBreak,
                        model.pomodoro.config.short_break_mins * 60,
                        new_cycles,
                        format!("🍅 Cycle {new_cycles} complete! Take a short break."),
                    )
                }
            }
            PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => (
                PomodoroPhase::Work,
                model.pomodoro.config.work_duration_mins * 60,
                session.cycles_completed,
                "☕ Break over! Back to work.".to_string(),
            ),
        }
    };

    // Update session
    if let Some(ref mut session) = model.pomodoro.session {
        // Record stats when completing a work phase
        if session.phase == PomodoroPhase::Work {
            model
                .pomodoro
                .stats
                .record_cycle(model.pomodoro.config.work_duration_mins);
        }

        session.phase = next_phase;
        session.cycles_completed = cycles_completed;
        // Reset phase timing (sets remaining_secs, phase_started_at, clears pause state)
        session.reset_phase_timing(next_remaining);

        // Check if goal reached
        if session.goal_reached() && next_phase == PomodoroPhase::Work {
            let goal_msg = format!(
                "🎊 Goal reached! {} cycles completed. Keep going or stop.",
                session.cycles_completed
            );
            crate::notifications::notify_pomodoro_phase(&goal_msg);
            model.alerts.status_message = Some(goal_msg);
        } else {
            crate::notifications::notify_pomodoro_phase(&message);
            model.alerts.status_message = Some(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Task;

    fn setup_model_with_task() -> Model {
        let mut model = Model::new();
        let task = Task::new("Test task");
        let task_id = task.id;
        model.tasks.insert(task_id, task);
        model.refresh_visible_tasks();
        model.selected_index = 0;
        model
    }

    #[test]
    fn test_start_tracking() {
        let mut model = setup_model_with_task();
        assert!(model.active_time_entry.is_none());

        handle_time(&mut model, TimeMessage::StartTracking);

        assert!(model.active_time_entry.is_some());
        assert!(!model.undo_stack.is_empty());
    }

    #[test]
    fn test_stop_tracking() {
        let mut model = setup_model_with_task();
        handle_time(&mut model, TimeMessage::StartTracking);
        assert!(model.active_time_entry.is_some());

        handle_time(&mut model, TimeMessage::StopTracking);

        assert!(model.active_time_entry.is_none());
    }

    #[test]
    fn test_toggle_tracking_start() {
        let mut model = setup_model_with_task();
        assert!(model.active_time_entry.is_none());

        handle_time(&mut model, TimeMessage::ToggleTracking);

        assert!(model.active_time_entry.is_some());
    }

    #[test]
    fn test_toggle_tracking_stop() {
        let mut model = setup_model_with_task();
        handle_time(&mut model, TimeMessage::StartTracking);
        assert!(model.active_time_entry.is_some());

        handle_time(&mut model, TimeMessage::ToggleTracking);

        assert!(model.active_time_entry.is_none());
    }

    #[test]
    fn test_pomodoro_start() {
        let mut model = setup_model_with_task();
        assert!(model.pomodoro.session.is_none());

        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });

        assert!(model.pomodoro.session.is_some());
        assert!(model.focus_mode);
        let session = model.pomodoro.session.as_ref().unwrap();
        assert_eq!(session.session_goal, 4);
    }

    #[test]
    fn test_pomodoro_start_without_task() {
        let mut model = Model::new();
        model.refresh_visible_tasks();

        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });

        assert!(model.pomodoro.session.is_none());
        assert!(model.alerts.status_message.is_some());
    }

    #[test]
    fn test_pomodoro_pause_resume() {
        let mut model = setup_model_with_task();
        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });

        handle_pomodoro(&mut model, PomodoroMessage::Pause);
        assert!(model.pomodoro.session.as_ref().unwrap().paused);

        handle_pomodoro(&mut model, PomodoroMessage::Resume);
        assert!(!model.pomodoro.session.as_ref().unwrap().paused);
    }

    #[test]
    fn test_pomodoro_toggle_pause() {
        let mut model = setup_model_with_task();
        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });

        handle_pomodoro(&mut model, PomodoroMessage::TogglePause);
        assert!(model.pomodoro.session.as_ref().unwrap().paused);

        handle_pomodoro(&mut model, PomodoroMessage::TogglePause);
        assert!(!model.pomodoro.session.as_ref().unwrap().paused);
    }

    #[test]
    fn test_pomodoro_stop() {
        let mut model = setup_model_with_task();
        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });
        assert!(model.pomodoro.session.is_some());

        handle_pomodoro(&mut model, PomodoroMessage::Stop);

        assert!(model.pomodoro.session.is_none());
    }

    #[test]
    fn test_pomodoro_tick() {
        let mut model = setup_model_with_task();
        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });

        let initial_remaining = model.pomodoro.session.as_ref().unwrap().remaining_secs;
        handle_pomodoro(&mut model, PomodoroMessage::Tick);

        assert_eq!(
            model.pomodoro.session.as_ref().unwrap().remaining_secs,
            initial_remaining - 1
        );
    }

    #[test]
    fn test_pomodoro_tick_paused() {
        let mut model = setup_model_with_task();
        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });
        handle_pomodoro(&mut model, PomodoroMessage::Pause);

        let initial_remaining = model.pomodoro.session.as_ref().unwrap().remaining_secs;
        handle_pomodoro(&mut model, PomodoroMessage::Tick);

        // Should not decrement when paused
        assert_eq!(
            model.pomodoro.session.as_ref().unwrap().remaining_secs,
            initial_remaining
        );
    }

    #[test]
    fn test_pomodoro_config_changes() {
        let mut model = Model::new();

        handle_pomodoro(&mut model, PomodoroMessage::SetWorkDuration(30));
        assert_eq!(model.pomodoro.config.work_duration_mins, 30);

        handle_pomodoro(&mut model, PomodoroMessage::SetShortBreak(10));
        assert_eq!(model.pomodoro.config.short_break_mins, 10);

        handle_pomodoro(&mut model, PomodoroMessage::SetLongBreak(20));
        assert_eq!(model.pomodoro.config.long_break_mins, 20);

        handle_pomodoro(&mut model, PomodoroMessage::SetCyclesBeforeLongBreak(6));
        assert_eq!(model.pomodoro.config.cycles_before_long_break, 6);
    }

    #[test]
    fn test_pomodoro_config_minimum_values() {
        let mut model = Model::new();

        handle_pomodoro(&mut model, PomodoroMessage::SetWorkDuration(0));
        assert_eq!(model.pomodoro.config.work_duration_mins, 1);

        handle_pomodoro(&mut model, PomodoroMessage::SetShortBreak(0));
        assert_eq!(model.pomodoro.config.short_break_mins, 1);

        handle_pomodoro(&mut model, PomodoroMessage::SetLongBreak(0));
        assert_eq!(model.pomodoro.config.long_break_mins, 1);

        handle_pomodoro(&mut model, PomodoroMessage::SetCyclesBeforeLongBreak(0));
        assert_eq!(model.pomodoro.config.cycles_before_long_break, 1);
    }

    #[test]
    fn test_pomodoro_goal_increment_decrement() {
        let mut model = setup_model_with_task();
        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });

        handle_pomodoro(&mut model, PomodoroMessage::IncrementGoal);
        assert_eq!(model.pomodoro.session.as_ref().unwrap().session_goal, 5);

        handle_pomodoro(&mut model, PomodoroMessage::DecrementGoal);
        assert_eq!(model.pomodoro.session.as_ref().unwrap().session_goal, 4);
    }

    #[test]
    fn test_pomodoro_goal_minimum() {
        let mut model = setup_model_with_task();
        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 1 });

        handle_pomodoro(&mut model, PomodoroMessage::DecrementGoal);
        // Should not go below 1
        assert_eq!(model.pomodoro.session.as_ref().unwrap().session_goal, 1);
    }

    #[test]
    fn test_pomodoro_skip_phase() {
        let mut model = setup_model_with_task();
        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 4 });

        let initial_phase = model.pomodoro.session.as_ref().unwrap().phase;
        handle_pomodoro(&mut model, PomodoroMessage::Skip);

        // Phase should have changed
        assert_ne!(
            model.pomodoro.session.as_ref().unwrap().phase,
            initial_phase
        );
    }

    fn selected_task(model: &Model) -> crate::domain::TaskId {
        model.selected_task_id().unwrap()
    }

    #[test]
    fn pomodoro_work_phase_runs_the_task_timer() {
        let mut model = setup_model_with_task();
        let task_id = selected_task(&model);

        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 2 });
        assert!(model.is_tracking_task(&task_id));
        assert_eq!(model.pomodoro.timed_task, Some(task_id));
        assert_eq!(model.time_entries.len(), 1);

        // Pausing a work phase stops the timer; resuming starts a fresh entry.
        handle_pomodoro(&mut model, PomodoroMessage::Pause);
        assert!(!model.is_tracking_task(&task_id));
        assert!(model.time_entries.values().all(|e| !e.is_running()));
        handle_pomodoro(&mut model, PomodoroMessage::Resume);
        assert!(model.is_tracking_task(&task_id));
        assert_eq!(model.time_entries.len(), 2);

        // Skipping into a break stops it; the break ending starts it again.
        handle_pomodoro(&mut model, PomodoroMessage::Skip);
        assert_eq!(
            model.pomodoro.session.as_ref().unwrap().phase,
            PomodoroPhase::ShortBreak
        );
        assert!(!model.is_tracking_task(&task_id));
        handle_pomodoro(&mut model, PomodoroMessage::Skip);
        assert_eq!(
            model.pomodoro.session.as_ref().unwrap().phase,
            PomodoroPhase::Work
        );
        assert!(model.is_tracking_task(&task_id));
        assert_eq!(model.time_entries.len(), 3);

        // Stopping the session stops the timer and forgets the owned task.
        handle_pomodoro(&mut model, PomodoroMessage::Stop);
        assert!(model.active_time_entry.is_none());
        assert_eq!(model.pomodoro.timed_task, None);
        assert_eq!(model.time_entries.len(), 3);
        assert!(model.time_entries.values().all(|e| !e.is_running()));
    }

    #[test]
    fn pomodoro_does_not_restart_a_timer_already_on_the_task() {
        let mut model = setup_model_with_task();
        handle_time(&mut model, TimeMessage::StartTracking);
        let running = model.active_time_entry.unwrap();

        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 1 });

        assert_eq!(
            model.active_time_entry,
            Some(running),
            "same entry keeps running"
        );
        assert_eq!(model.time_entries.len(), 1);
    }

    #[test]
    fn pomodoro_break_leaves_a_manual_timer_on_another_task_alone() {
        let mut model = setup_model_with_task();
        let other = Task::new("Other");
        let other_id = other.id;
        model.tasks.insert(other_id, other);

        handle_pomodoro(&mut model, PomodoroMessage::Start { goal_cycles: 1 });
        // User manually switches the timer to a different task mid-phase.
        start_tracking_with_undo(&mut model, other_id);
        assert!(model.is_tracking_task(&other_id));

        handle_pomodoro(&mut model, PomodoroMessage::Skip); // into a break
        assert!(
            model.is_tracking_task(&other_id),
            "not our timer, not stopped"
        );

        handle_pomodoro(&mut model, PomodoroMessage::Stop);
        assert!(model.is_tracking_task(&other_id));
    }
}
