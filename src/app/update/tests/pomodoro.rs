//! Pomodoro timer tests.

use crate::app::{update::update, Message, Model, PomodoroMessage};
use crate::domain::PomodoroPhase;

use super::create_test_model_with_tasks;

#[test]
fn test_pomodoro_start() {
    let mut model = create_test_model_with_tasks();

    assert!(model.pomodoro_session.is_none());
    assert!(!model.focus_mode);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert!(model.pomodoro_session.is_some());
    assert!(model.focus_mode);

    let session = model.pomodoro_session.as_ref().unwrap();
    assert_eq!(session.session_goal, 4);
    assert_eq!(session.cycles_completed, 0);
    assert!(!session.paused);
}

#[test]
fn test_pomodoro_pause_resume() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert!(!model.pomodoro_session.as_ref().unwrap().paused);

    update(&mut model, Message::Pomodoro(PomodoroMessage::Pause));
    assert!(model.pomodoro_session.as_ref().unwrap().paused);

    update(&mut model, Message::Pomodoro(PomodoroMessage::Resume));
    assert!(!model.pomodoro_session.as_ref().unwrap().paused);
}

#[test]
fn test_pomodoro_toggle_pause() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert!(!model.pomodoro_session.as_ref().unwrap().paused);

    update(&mut model, Message::Pomodoro(PomodoroMessage::TogglePause));
    assert!(model.pomodoro_session.as_ref().unwrap().paused);

    update(&mut model, Message::Pomodoro(PomodoroMessage::TogglePause));
    assert!(!model.pomodoro_session.as_ref().unwrap().paused);
}

#[test]
fn test_pomodoro_stop() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert!(model.pomodoro_session.is_some());

    update(&mut model, Message::Pomodoro(PomodoroMessage::Stop));
    assert!(model.pomodoro_session.is_none());
}

#[test]
fn test_pomodoro_tick_decrements_time() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    let initial_remaining = model.pomodoro_session.as_ref().unwrap().remaining_secs;

    update(&mut model, Message::Pomodoro(PomodoroMessage::Tick));

    assert_eq!(
        model.pomodoro_session.as_ref().unwrap().remaining_secs,
        initial_remaining - 1
    );
}

#[test]
fn test_pomodoro_tick_paused_no_decrement() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );
    update(&mut model, Message::Pomodoro(PomodoroMessage::Pause));

    let initial_remaining = model.pomodoro_session.as_ref().unwrap().remaining_secs;

    update(&mut model, Message::Pomodoro(PomodoroMessage::Tick));

    // Time should not decrement when paused
    assert_eq!(
        model.pomodoro_session.as_ref().unwrap().remaining_secs,
        initial_remaining
    );
}

#[test]
fn test_pomodoro_skip_phase() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    // Should be in Work phase
    assert_eq!(
        model.pomodoro_session.as_ref().unwrap().phase,
        PomodoroPhase::Work
    );

    // Skip to break
    update(&mut model, Message::Pomodoro(PomodoroMessage::Skip));

    // Should now be in ShortBreak phase and cycle completed
    assert_eq!(
        model.pomodoro_session.as_ref().unwrap().phase,
        PomodoroPhase::ShortBreak
    );
    assert_eq!(model.pomodoro_session.as_ref().unwrap().cycles_completed, 1);
}

#[test]
fn test_pomodoro_goal_adjustment() {
    let mut model = create_test_model_with_tasks();
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
    );

    assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 4);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::IncrementGoal),
    );
    assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 5);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 4);

    // Cannot go below 1
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::DecrementGoal),
    );
    assert_eq!(model.pomodoro_session.as_ref().unwrap().session_goal, 1);
}

#[test]
fn test_pomodoro_config_changes() {
    let mut model = Model::new();

    assert_eq!(model.pomodoro_config.work_duration_mins, 25);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::SetWorkDuration(30)),
    );
    assert_eq!(model.pomodoro_config.work_duration_mins, 30);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::SetShortBreak(10)),
    );
    assert_eq!(model.pomodoro_config.short_break_mins, 10);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::SetLongBreak(20)),
    );
    assert_eq!(model.pomodoro_config.long_break_mins, 20);

    update(
        &mut model,
        Message::Pomodoro(PomodoroMessage::SetCyclesBeforeLongBreak(3)),
    );
    assert_eq!(model.pomodoro_config.cycles_before_long_break, 3);
}
