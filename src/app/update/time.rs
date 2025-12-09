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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
            if let Some(task_id) = model.visible_tasks.get(model.selected_index).copied() {
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
                model.pomodoro_session = Some(PomodoroSession::new(
                    task_id,
                    &model.pomodoro_config,
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
            if let Some(ref mut session) = model.pomodoro_session {
                if !session.paused {
                    session.paused = true;
                    session.paused_at = Some(chrono::Utc::now());
                }
            }
        }
        PomodoroMessage::Resume => {
            if let Some(ref mut session) = model.pomodoro_session {
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
            if let Some(ref mut session) = model.pomodoro_session {
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
            if model.pomodoro_session.is_some() {
                transition_pomodoro_phase(model);
            }
        }
        PomodoroMessage::Stop => {
            if model.pomodoro_session.is_some() {
                model.pomodoro_session = None;
                model.status_message = Some("Pomodoro session stopped".to_string());
            }
        }
        PomodoroMessage::Tick => {
            let should_transition = if let Some(ref mut session) = model.pomodoro_session {
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
            model.pomodoro_config.work_duration_mins = mins.max(1);
        }
        PomodoroMessage::SetShortBreak(mins) => {
            model.pomodoro_config.short_break_mins = mins.max(1);
        }
        PomodoroMessage::SetLongBreak(mins) => {
            model.pomodoro_config.long_break_mins = mins.max(1);
        }
        PomodoroMessage::SetCyclesBeforeLongBreak(cycles) => {
            model.pomodoro_config.cycles_before_long_break = cycles.max(1);
        }
        PomodoroMessage::IncrementGoal => {
            if let Some(ref mut session) = model.pomodoro_session {
                session.session_goal += 1;
            }
        }
        PomodoroMessage::DecrementGoal => {
            if let Some(ref mut session) = model.pomodoro_session {
                if session.session_goal > 1 {
                    session.session_goal -= 1;
                }
            }
        }
    }
}

/// Transition to the next Pomodoro phase
fn transition_pomodoro_phase(model: &mut Model) {
    let (next_phase, next_remaining, cycles_completed, message) = {
        let Some(session) = model.pomodoro_session.as_ref() else {
            return;
        };

        match session.phase {
            PomodoroPhase::Work => {
                // Record the completed work cycle
                let new_cycles = session.cycles_completed + 1;

                // Determine if long break or short break
                if new_cycles > 0
                    && new_cycles % model.pomodoro_config.cycles_before_long_break == 0
                {
                    (
                        PomodoroPhase::LongBreak,
                        model.pomodoro_config.long_break_mins * 60,
                        new_cycles,
                        format!("🎉 Cycle {new_cycles} complete! Time for a long break."),
                    )
                } else {
                    (
                        PomodoroPhase::ShortBreak,
                        model.pomodoro_config.short_break_mins * 60,
                        new_cycles,
                        format!("🍅 Cycle {new_cycles} complete! Take a short break."),
                    )
                }
            }
            PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => (
                PomodoroPhase::Work,
                model.pomodoro_config.work_duration_mins * 60,
                session.cycles_completed,
                "☕ Break over! Back to work.".to_string(),
            ),
        }
    };

    // Update session
    if let Some(ref mut session) = model.pomodoro_session {
        // Record stats when completing a work phase
        if session.phase == PomodoroPhase::Work {
            model
                .pomodoro_stats
                .record_cycle(model.pomodoro_config.work_duration_mins);
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
