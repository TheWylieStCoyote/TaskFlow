//! Time tracking and Pomodoro message handlers
//!
//! Handles all time-related messages including:
//! - Starting/stopping time tracking
//! - Pomodoro timer control (start, pause, resume, skip, stop)
//! - Pomodoro configuration changes

use crate::app::{Model, PomodoroMessage, TimeMessage, UndoAction};
use crate::domain::{PomodoroPhase, PomodoroSession};

/// Handle time tracking messages
pub fn handle_time(model: &mut Model, msg: TimeMessage) {
    match msg {
        TimeMessage::StartTracking => {
            if let Some(task_id) = model.selected_task_id() {
                let (new_entry, stopped_entry) = model.start_time_tracking(task_id);

                if let Some((before, after)) = stopped_entry {
                    // Timer switch: use composite action for single undo
                    model.undo_stack.push(UndoAction::TimerSwitched {
                        stopped_entry_before: Box::new(before),
                        stopped_entry_after: Box::new(after),
                        started_entry: Box::new(new_entry),
                    });
                } else {
                    // Fresh start: use simple action
                    model
                        .undo_stack
                        .push(UndoAction::TimeEntryStarted(Box::new(new_entry)));
                }
            }
        }
        TimeMessage::StopTracking => {
            if let Some((before, after)) = model.stop_time_tracking() {
                model.undo_stack.push(UndoAction::TimeEntryStopped {
                    before: Box::new(before),
                    after: Box::new(after),
                });
            }
        }
        TimeMessage::ToggleTracking => {
            if let Some(task_id) = model.selected_task_id() {
                if model.is_tracking_task(&task_id) {
                    // Stop tracking current task
                    if let Some((before, after)) = model.stop_time_tracking() {
                        model.undo_stack.push(UndoAction::TimeEntryStopped {
                            before: Box::new(before),
                            after: Box::new(after),
                        });
                    }
                } else {
                    // Start tracking new task (may switch from another task)
                    let (new_entry, stopped_entry) = model.start_time_tracking(task_id);

                    if let Some((before, after)) = stopped_entry {
                        // Timer switch: use composite action for single undo
                        model.undo_stack.push(UndoAction::TimerSwitched {
                            stopped_entry_before: Box::new(before),
                            stopped_entry_after: Box::new(after),
                            started_entry: Box::new(new_entry),
                        });
                    } else {
                        // Fresh start: use simple action
                        model
                            .undo_stack
                            .push(UndoAction::TimeEntryStarted(Box::new(new_entry)));
                    }
                }
            }
        }
    }
}

/// Handle Pomodoro timer messages
pub fn handle_pomodoro(model: &mut Model, msg: PomodoroMessage) {
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
                model.status_message = Some(format!("Pomodoro started: {goal_cycles} cycle goal"));
            } else {
                model.status_message = Some("Select a task to start Pomodoro".to_string());
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
                model.status_message = Some("Pomodoro session stopped".to_string());
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
                .pomodoro.stats
                .record_cycle(model.pomodoro.config.work_duration_mins);
        }

        session.phase = next_phase;
        session.cycles_completed = cycles_completed;
        // Reset phase timing (sets remaining_secs, phase_started_at, clears pause state)
        session.reset_phase_timing(next_remaining);

        // Check if goal reached
        if session.goal_reached() && next_phase == PomodoroPhase::Work {
            model.status_message = Some(format!(
                "🎊 Goal reached! {} cycles completed. Keep going or stop.",
                session.cycles_completed
            ));
        } else {
            model.status_message = Some(message);
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
        assert!(model.status_message.is_some());
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
}
