//! Tests for the top-level key dispatcher.
//!
//! These exercise `handle_key_event` and `handle_command_palette_input`
//! against a `Model` placed in each of the states the dispatcher checks,
//! always with the default keybindings so expectations never depend on the
//! developer's own `keybindings.toml`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use taskflow::app::{
    FocusPane, Message, Model, NavigationMessage, SystemMessage, TaskMessage, UiMessage, ViewId,
};
use taskflow::config::Keybindings;
use taskflow::domain::Task;
use taskflow::ui::InputMode;

use super::handle_key_event;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ch(c: char) -> KeyEvent {
    key(KeyCode::Char(c))
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn alt(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)
}

/// Dispatch `key` with the default keybindings.
fn dispatch(model: &mut Model, key: KeyEvent) -> Message {
    let keybindings = Keybindings::default();
    handle_key_event(key, model, &keybindings)
}

fn model_in_view(view: ViewId) -> Model {
    let mut model = Model::new();
    model.current_view = view;
    model.focus_pane = FocusPane::TaskList;
    model
}

// ============================================================================
// Command palette
// ============================================================================

#[test]
fn palette_esc_hides() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideCommandPalette)
    ));
}

#[test]
fn palette_enter_dispatches_selected_action_and_closes() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    model.command_palette.query = "quit".to_string();
    model.command_palette.selected = 0;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Enter)),
        Message::System(SystemMessage::Quit)
    ));
    assert!(!model.command_palette.visible);
}

#[test]
fn palette_enter_with_no_match_just_hides() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    model.command_palette.query = "zzqxv-no-such-command".to_string();
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Enter)),
        Message::Ui(UiMessage::HideCommandPalette)
    ));
}

#[test]
fn palette_navigation_and_editing_keys() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Up)),
        Message::Ui(UiMessage::CommandPaletteUp)
    ));
    assert!(matches!(
        dispatch(&mut model, ctrl('k')),
        Message::Ui(UiMessage::CommandPaletteUp)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Down)),
        Message::Ui(UiMessage::CommandPaletteDown)
    ));
    assert!(matches!(
        dispatch(&mut model, ctrl('j')),
        Message::Ui(UiMessage::CommandPaletteDown)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Backspace)),
        Message::Ui(UiMessage::CommandPaletteBackspace)
    ));
    // Plain k/j are typed into the query, not navigation
    assert!(matches!(
        dispatch(&mut model, ch('k')),
        Message::Ui(UiMessage::CommandPaletteInput('k'))
    ));
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Ui(UiMessage::CommandPaletteInput('j'))
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Tab)),
        Message::None
    ));
}

#[test]
fn palette_takes_priority_over_other_dialogs() {
    let mut model = Model::new();
    model.command_palette.visible = true;
    model.show_confirm_delete = true;
    model.show_help = true;
    assert!(matches!(
        dispatch(&mut model, ch('y')),
        Message::Ui(UiMessage::CommandPaletteInput('y'))
    ));
}

// ============================================================================
// Confirmation dialogs
// ============================================================================

#[test]
fn delete_confirmation_keys() {
    let mut model = Model::new();
    model.show_confirm_delete = true;
    for k in [ch('y'), ch('Y')] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::Ui(UiMessage::ConfirmDelete)
        ));
    }
    for k in [ch('n'), ch('N'), key(KeyCode::Esc)] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::Ui(UiMessage::CancelDelete)
        ));
    }
    assert!(matches!(dispatch(&mut model, ch('j')), Message::None));
}

#[test]
fn generate_config_prompt_keys() {
    let mut model = Model::new();
    model.show_generate_config_prompt = true;
    for k in [ch('y'), ch('Y')] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::Ui(UiMessage::ConfirmGenerateConfig)
        ));
    }
    for k in [ch('n'), ch('N'), key(KeyCode::Esc)] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::Ui(UiMessage::CancelGenerateConfig)
        ));
    }
    assert!(matches!(dispatch(&mut model, ch('j')), Message::None));
}

#[test]
fn import_preview_keys() {
    let mut model = Model::new();
    model.import.show_preview = true;
    for k in [key(KeyCode::Enter), ch('y'), ch('Y')] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::System(SystemMessage::ConfirmImport)
        ));
    }
    for k in [key(KeyCode::Esc), ch('n'), ch('N')] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::System(SystemMessage::CancelImport)
        ));
    }
    assert!(matches!(dispatch(&mut model, ch('j')), Message::None));
}

// ============================================================================
// Text input mode
// ============================================================================

#[test]
fn editing_mode_routes_to_input_messages() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Enter)),
        Message::Ui(UiMessage::SubmitInput)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::CancelInput)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Backspace)),
        Message::Ui(UiMessage::InputBackspace)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Delete)),
        Message::Ui(UiMessage::InputDelete)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Left)),
        Message::Ui(UiMessage::InputCursorLeft)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Right)),
        Message::Ui(UiMessage::InputCursorRight)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Home)),
        Message::Ui(UiMessage::InputCursorStart)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::End)),
        Message::Ui(UiMessage::InputCursorEnd)
    ));
    // 'q' would quit in normal mode; while editing it is just text
    assert!(matches!(
        dispatch(&mut model, ch('q')),
        Message::Ui(UiMessage::InputChar('q'))
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Tab)),
        Message::None
    ));
}

#[test]
fn editing_mode_beats_alerts() {
    let mut model = Model::new();
    model.input.mode = InputMode::Editing;
    model.alerts.show_overdue = true;
    assert!(matches!(
        dispatch(&mut model, ch('a')),
        Message::Ui(UiMessage::InputChar('a'))
    ));
}

// ============================================================================
// Alerts and help
// ============================================================================

#[test]
fn storage_error_alert_is_dismissed_by_any_key() {
    let mut model = Model::new();
    model.alerts.show_storage_error = true;
    model.alerts.show_overdue = true;
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Ui(UiMessage::DismissStorageErrorAlert)
    ));
}

#[test]
fn overdue_alert_is_dismissed_by_any_key() {
    let mut model = Model::new();
    model.alerts.show_overdue = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Enter)),
        Message::Ui(UiMessage::DismissOverdueAlert)
    ));
}

#[test]
fn help_scrolls_and_closes() {
    let mut model = Model::new();
    model.show_help = true;
    for k in [ch('j'), key(KeyCode::Down)] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::Ui(UiMessage::HelpScrollDown)
        ));
    }
    for k in [ch('k'), key(KeyCode::Up)] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::Ui(UiMessage::HelpScrollUp)
        ));
    }
    assert!(matches!(
        dispatch(&mut model, ctrl('d')),
        Message::Ui(UiMessage::HelpPageDown)
    ));
    assert!(matches!(
        dispatch(
            &mut model,
            KeyEvent::new(KeyCode::PageDown, KeyModifiers::CONTROL)
        ),
        Message::Ui(UiMessage::HelpPageDown)
    ));
    assert!(matches!(
        dispatch(&mut model, ctrl('u')),
        Message::Ui(UiMessage::HelpPageUp)
    ));
    assert!(matches!(
        dispatch(
            &mut model,
            KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL)
        ),
        Message::Ui(UiMessage::HelpPageUp)
    ));
    // Plain PageDown/PageUp must page too, not close the popup.
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::PageDown)),
        Message::Ui(UiMessage::HelpPageDown)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::PageUp)),
        Message::Ui(UiMessage::HelpPageUp)
    ));
    for k in [ch('?'), ch('q'), key(KeyCode::Esc)] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::Ui(UiMessage::HideHelp)
        ));
    }
}

// ============================================================================
// Focus mode
// ============================================================================

#[test]
fn focus_mode_special_keys_and_fall_through() {
    let mut model = Model::new();
    model.focus_mode = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::ToggleFocusMode)
    ));
    assert!(matches!(
        dispatch(&mut model, ch(']')),
        Message::Ui(UiMessage::ChainNext)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('[')),
        Message::Ui(UiMessage::ChainPrev)
    ));
    // Other keys still reach the global keybindings
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Navigation(NavigationMessage::Down)
    ));
}

// ============================================================================
// Modal delegation
// ============================================================================

#[test]
fn template_picker_is_delegated() {
    let mut model = Model::new();
    model.template_picker.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideTemplates)
    ));
}

#[test]
fn keybindings_editor_is_delegated() {
    let mut model = Model::new();
    model.keybindings_editor.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideKeybindingsEditor)
    ));
}

#[test]
fn time_log_is_delegated() {
    let mut model = Model::new();
    model.time_log.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideTimeLog)
    ));
}

#[test]
fn work_log_is_delegated() {
    let mut model = Model::new();
    model.work_log_editor.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideWorkLog)
    ));
}

#[test]
fn description_editor_is_delegated() {
    let mut model = Model::new();
    model.description_editor.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideDescriptionEditor)
    ));
}

#[test]
fn daily_weekly_evening_reviews_are_delegated() {
    let mut model = Model::new();
    model.daily_review.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideDailyReview)
    ));
    model.daily_review.visible = false;

    model.weekly_review.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideWeeklyReview)
    ));
    model.weekly_review.visible = false;

    model.evening_review.visible = true;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Esc)),
        Message::Ui(UiMessage::HideEveningReview)
    ));
}

#[test]
fn task_detail_is_delegated() {
    let mut model = Model::new();
    model.task_detail.visible = true;
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Ui(UiMessage::TaskDetailScrollDown)
    ));
}

#[test]
fn habit_analytics_overlay_keys() {
    let mut model = Model::new();
    model.habit_view.show_analytics = true;
    for k in [key(KeyCode::Esc), ch('q'), ch('a')] {
        assert!(matches!(
            dispatch(&mut model, k),
            Message::Ui(UiMessage::HideHabitAnalytics)
        ));
    }
    assert!(matches!(dispatch(&mut model, ch('j')), Message::None));
}

#[test]
fn pending_macro_slot_is_delegated() {
    let mut model = Model::new();
    model.macro_state.pending_slot = Some(0);
    assert!(matches!(
        dispatch(&mut model, ch('3')),
        Message::Ui(UiMessage::StartRecordMacro)
    ));
    assert_eq!(model.macro_state.pending_slot, Some(3));
}

// ============================================================================
// Multi-select
// ============================================================================

#[test]
fn multi_select_space_toggles_selection() {
    let mut model = Model::new();
    model.multi_select.mode = true;
    assert!(matches!(
        dispatch(&mut model, ch(' ')),
        Message::Ui(UiMessage::ToggleTaskSelection)
    ));
}

#[test]
fn multi_select_bulk_keys_require_a_selection() {
    let mut model = Model::new();
    model.multi_select.mode = true;

    // Nothing selected: the keys keep their single-task meanings
    assert!(matches!(
        dispatch(&mut model, ch('p')),
        Message::Task(TaskMessage::CyclePriority)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('T')),
        Message::Ui(UiMessage::StartEditTags)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('D')),
        Message::Ui(UiMessage::StartEditDueDate)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('z')),
        Message::Ui(UiMessage::StartSnoozeTask)
    ));

    let task = Task::new("Selected");
    let id = task.id;
    model.tasks.insert(id, task);
    model.multi_select.selected.insert(id);

    assert!(matches!(
        dispatch(&mut model, ch('p')),
        Message::Ui(UiMessage::StartBulkSetPriority)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('T')),
        Message::Ui(UiMessage::StartBulkAddTags)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('D')),
        Message::Ui(UiMessage::StartBulkSetDueDate)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('z')),
        Message::Ui(UiMessage::StartBulkSnooze)
    ));
}

// ============================================================================
// View-specific dispatch
// ============================================================================

#[test]
fn calendar_view_only_handles_keys_when_task_list_pane_focused() {
    let mut model = model_in_view(ViewId::Calendar);
    model.calendar_state.focus_task_list = false;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Left)),
        Message::Ui(UiMessage::CalendarPrevDay)
    ));
    // Keys the calendar does not claim fall through to the global map
    assert!(matches!(
        dispatch(&mut model, ch('q')),
        Message::System(SystemMessage::Quit)
    ));

    model.focus_pane = FocusPane::Sidebar;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Left)),
        Message::Navigation(NavigationMessage::FocusSidebar)
    ));
}

#[test]
fn habits_view_overrides_global_keys() {
    let mut model = model_in_view(ViewId::Habits);
    assert!(matches!(
        dispatch(&mut model, ch('n')),
        Message::Ui(UiMessage::StartCreateHabit)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('q')),
        Message::System(SystemMessage::Quit)
    ));
}

#[test]
fn timeline_view_dispatch() {
    let mut model = model_in_view(ViewId::Timeline);
    assert!(matches!(
        dispatch(&mut model, ch('t')),
        Message::Navigation(NavigationMessage::TimelineGoToday)
    ));
}

#[test]
fn kanban_view_dispatch() {
    let mut model = model_in_view(ViewId::Kanban);
    assert!(matches!(
        dispatch(&mut model, ch('h')),
        Message::Navigation(NavigationMessage::KanbanLeft)
    ));
}

#[test]
fn eisenhower_view_dispatch() {
    let mut model = model_in_view(ViewId::Eisenhower);
    assert!(matches!(
        dispatch(&mut model, ch('l')),
        Message::Navigation(NavigationMessage::EisenhowerRight)
    ));
}

#[test]
fn weekly_planner_view_dispatch() {
    let mut model = model_in_view(ViewId::WeeklyPlanner);
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Navigation(NavigationMessage::WeeklyPlannerDown)
    ));
}

#[test]
fn reports_view_dispatch() {
    let mut model = model_in_view(ViewId::Reports);
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Tab)),
        Message::Navigation(NavigationMessage::ReportsNextPanel)
    ));
    // j moves the Reports → Tasks table selection rather than the task list.
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Ui(UiMessage::ReportsTasksMove(1))
    ));
}

#[test]
fn git_todos_view_rescans_on_r() {
    let mut model = model_in_view(ViewId::GitTodos);
    assert!(matches!(
        dispatch(&mut model, ch('r')),
        Message::System(SystemMessage::ScanGitTodos)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Navigation(NavigationMessage::Down)
    ));
}

#[test]
fn duplicates_view_dispatch() {
    let mut model = model_in_view(ViewId::Duplicates);
    assert!(matches!(
        dispatch(&mut model, ch('D')),
        Message::Ui(UiMessage::DismissDuplicate)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Navigation(NavigationMessage::Down)
    ));
}

#[test]
fn network_view_dispatch() {
    let mut model = model_in_view(ViewId::Network);
    assert!(matches!(
        dispatch(&mut model, ch('k')),
        Message::Navigation(NavigationMessage::NetworkUp)
    ));
}

#[test]
fn goals_view_dispatch() {
    let mut model = model_in_view(ViewId::Goals);
    assert!(matches!(
        dispatch(&mut model, ch('N')),
        Message::Ui(UiMessage::StartCreateKeyResult)
    ));
}

// ============================================================================
// Chords
// ============================================================================

#[test]
fn chord_prefix_is_remembered_and_completed() {
    let mut model = Model::new();
    assert!(matches!(dispatch(&mut model, ch('@')), Message::None));
    assert_eq!(model.pending_chord.as_deref(), Some("@"));

    assert!(matches!(
        dispatch(&mut model, ch('3')),
        Message::Ui(UiMessage::PlayMacro(3))
    ));
    assert!(model.pending_chord.is_none());
}

#[test]
fn unknown_chord_completion_yields_none_and_clears_prefix() {
    let mut model = Model::new();
    model.pending_chord = Some("@".to_string());
    assert!(matches!(dispatch(&mut model, ch('z')), Message::None));
    assert!(model.pending_chord.is_none());
}

#[test]
fn bound_single_keys_are_not_chord_prefixes() {
    let mut model = Model::new();
    assert!(matches!(
        dispatch(&mut model, ch('?')),
        Message::Ui(UiMessage::ShowHelp)
    ));
    assert!(model.pending_chord.is_none());
}

// ============================================================================
// Enter and keybinding lookup
// ============================================================================

#[test]
fn enter_opens_task_detail_when_task_list_focused() {
    let mut model = Model::new();
    model.focus_pane = FocusPane::TaskList;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Enter)),
        Message::Ui(UiMessage::ShowTaskDetail)
    ));
}

#[test]
fn enter_selects_sidebar_item_when_sidebar_focused() {
    let mut model = Model::new();
    model.focus_pane = FocusPane::Sidebar;
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Enter)),
        Message::Navigation(NavigationMessage::SelectSidebarItem)
    ));
}

#[test]
fn default_keybindings_are_looked_up() {
    let mut model = Model::new();
    assert!(matches!(
        dispatch(&mut model, ch('j')),
        Message::Navigation(NavigationMessage::Down)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('G')),
        Message::Navigation(NavigationMessage::Last)
    ));
    assert!(matches!(
        dispatch(&mut model, ctrl('s')),
        Message::System(SystemMessage::Save)
    ));
    assert!(matches!(
        dispatch(&mut model, alt('d')),
        Message::Ui(UiMessage::ShowDailyReview)
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::F(5))),
        Message::Pomodoro(taskflow::app::PomodoroMessage::Start { goal_cycles: 4 })
    ));
    assert!(matches!(
        dispatch(&mut model, KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT)),
        Message::Navigation(NavigationMessage::ReportsPrevPanel)
    ));
}

#[test]
fn calendar_s_schedules_to_day_while_alt_s_and_capital_s_stay_global() {
    let mut model = model_in_view(ViewId::Calendar);
    for focus_task_list in [false, true] {
        model.calendar_state.focus_task_list = focus_task_list;
        assert!(matches!(
            dispatch(&mut model, ch('s')),
            Message::Ui(UiMessage::ScheduleTaskToDay)
        ));
        assert!(matches!(
            dispatch(&mut model, alt('s')),
            Message::Ui(UiMessage::StartEditScheduledTime)
        ));
        assert!(matches!(
            dispatch(&mut model, ch('S')),
            Message::Ui(UiMessage::StartEditScheduledDate)
        ));
    }

    // Outside the calendar, s is still the sort-field cycle
    let mut model = model_in_view(ViewId::TaskList);
    assert!(matches!(
        dispatch(&mut model, ch('s')),
        Message::Ui(UiMessage::CycleSortField)
    ));
}

#[test]
fn alt_z_toggles_subtask_collapse_and_z_still_snoozes() {
    let mut model = model_in_view(ViewId::TaskList);
    assert!(matches!(
        dispatch(&mut model, alt('z')),
        Message::Ui(UiMessage::ToggleSubtaskCollapse)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('z')),
        Message::Ui(UiMessage::StartSnoozeTask)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('Z')),
        Message::Ui(UiMessage::ClearSnooze)
    ));
}

#[test]
fn unbound_keys_yield_none() {
    let mut model = Model::new();
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::F(12))),
        Message::None
    ));
    assert!(matches!(
        dispatch(&mut model, key(KeyCode::Null)),
        Message::None
    ));
    assert!(model.pending_chord.is_none());
}

#[test]
fn custom_keybindings_are_honoured() {
    let mut model = Model::new();
    let mut keybindings = Keybindings::default();
    keybindings
        .bindings
        .insert("f12".to_string(), taskflow::config::Action::Save);
    assert!(matches!(
        handle_key_event(key(KeyCode::F(12)), &mut model, &keybindings),
        Message::System(SystemMessage::Save)
    ));
}

#[test]
fn focus_mode_plus_minus_adjust_pomodoro_goal_only_during_a_session() {
    use taskflow::app::PomodoroMessage;
    use taskflow::domain::{PomodoroConfig, PomodoroSession, TaskId};

    let mut model = Model::new();
    model.focus_mode = true;

    // No session: the keys fall through to normal handling (not a Pomodoro message).
    assert!(!matches!(
        dispatch(&mut model, ch('+')),
        Message::Pomodoro(PomodoroMessage::IncrementGoal)
    ));

    model.pomodoro.session = Some(PomodoroSession::new(
        TaskId::new(),
        &PomodoroConfig::default(),
        4,
    ));
    assert!(matches!(
        dispatch(&mut model, ch('+')),
        Message::Pomodoro(PomodoroMessage::IncrementGoal)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('=')),
        Message::Pomodoro(PomodoroMessage::IncrementGoal)
    ));
    assert!(matches!(
        dispatch(&mut model, ch('-')),
        Message::Pomodoro(PomodoroMessage::DecrementGoal)
    ));
}
