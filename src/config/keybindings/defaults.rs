//! Default keybindings.
//!
//! The defaults live in a flat slice rather than being inserted straight into a
//! `HashMap` so that a unit test can assert every key appears exactly once. A
//! duplicate key in a map silently shadows the earlier binding, which is how
//! several documented actions ended up unreachable in the past.

use std::collections::HashMap;

use super::Action;

/// Every default `(key, action)` pair. Keys must be unique.
///
/// Key strings follow the format produced by the input layer: a single
/// character (`j`, `G`, `?`), a named key (`enter`, `f5`, `pagedown`), or a
/// `ctrl+`/`alt+` prefixed key. `shift+` is only ever emitted for non-character
/// keys (`shift+tab`); for letters use the uppercase character instead.
///
/// Two-character entries beginning with a non-alphanumeric character (`@0`) are
/// chords: the first key is pressed, then the second.
pub(crate) const DEFAULT_BINDINGS: &[(&str, Action)] = &[
    // Navigation
    ("j", Action::MoveDown),
    ("J", Action::MoveDown),
    ("k", Action::MoveUp),
    ("K", Action::MoveUp),
    ("down", Action::MoveDown),
    ("up", Action::MoveUp),
    ("g", Action::MoveFirst),
    ("G", Action::MoveLast),
    ("ctrl+u", Action::PageUp),
    ("ctrl+d", Action::PageDown),
    ("pageup", Action::PageUp),
    ("pagedown", Action::PageDown),
    // Task actions
    ("x", Action::ToggleComplete),
    ("space", Action::ToggleComplete),
    ("a", Action::CreateTask),
    ("alt+a", Action::QuickCapture),
    ("A", Action::CreateSubtask),
    ("P", Action::CreateProject),
    ("E", Action::EditProject),
    ("X", Action::DeleteProject),
    ("e", Action::EditTask),
    ("D", Action::EditDueDate),
    ("S", Action::EditScheduledDate),
    ("alt+s", Action::EditScheduledTime),
    ("T", Action::EditTags),
    ("n", Action::EditDescription),
    ("N", Action::EditDescriptionMultiline),
    ("d", Action::DeleteTask),
    ("Y", Action::DuplicateTask),
    ("p", Action::CyclePriority),
    ("m", Action::MoveToProject),
    // Quick reschedule
    ("]", Action::RescheduleTomorrow),
    ("[", Action::RescheduleNextWeek),
    ("M", Action::RescheduleNextMonday),
    // Task snooze
    ("z", Action::SnoozeTask),
    ("Z", Action::ClearSnooze),
    // Time tracking
    ("t", Action::ToggleTimeTracking),
    ("L", Action::ShowTimeLog),
    ("W", Action::ShowWorkLog),
    ("alt+t", Action::EditEstimate),
    // UI actions
    ("b", Action::ToggleSidebar),
    ("c", Action::ToggleShowCompleted),
    ("?", Action::ShowHelp),
    ("f", Action::ToggleFocusMode),
    ("F", Action::ToggleFullScreenFocus),
    ("Q", Action::AddToFocusQueue),
    ("h", Action::FocusSidebar),
    ("H", Action::FocusSidebar),
    ("l", Action::FocusTaskList),
    ("left", Action::FocusSidebar),
    ("right", Action::FocusTaskList),
    ("enter", Action::Select),
    ("/", Action::Search),
    ("ctrl+l", Action::ClearSearch),
    ("#", Action::FilterByTag),
    ("ctrl+t", Action::ClearTagFilter),
    ("s", Action::CycleSortField),
    ("ctrl+o", Action::ToggleSortOrder),
    // Multi-select / Bulk operations
    ("v", Action::ToggleMultiSelect),
    ("V", Action::SelectAll),
    ("ctrl+v", Action::ClearSelection),
    // Dependencies
    ("B", Action::EditDependencies),
    // Recurrence
    ("R", Action::EditRecurrence),
    // Manual ordering
    ("ctrl+up", Action::MoveTaskUp),
    ("ctrl+down", Action::MoveTaskDown),
    // Task chains
    ("alt+l", Action::LinkTask),
    ("alt+u", Action::UnlinkTask),
    // Calendar navigation
    ("<", Action::CalendarPrevMonth),
    (">", Action::CalendarNextMonth),
    // System
    ("ctrl+s", Action::Save),
    ("u", Action::Undo),
    ("ctrl+z", Action::Undo),
    ("ctrl+r", Action::Redo),
    ("U", Action::Redo),
    ("q", Action::Quit),
    ("alt+r", Action::RefreshStorage),
    // Export
    ("ctrl+e", Action::ExportCsv),
    ("ctrl+i", Action::ExportIcs),
    ("ctrl+g", Action::ExportChainsDot),
    ("ctrl+m", Action::ExportChainsMermaid),
    ("ctrl+p", Action::ExportReportMarkdown),
    ("ctrl+h", Action::ExportReportHtml),
    // Import
    ("I", Action::ImportCsv),
    ("alt+i", Action::ImportIcs),
    // Reports navigation (Tab/Shift+Tab or l/h when in reports view)
    ("tab", Action::ReportsNextPanel),
    ("shift+tab", Action::ReportsPrevPanel),
    // Macros: ctrl+q then a digit starts (or stops) recording into that slot;
    // `@` then a digit plays the slot back. Stopping is handled by the same
    // ctrl+q flow, so StopRecordMacro needs no key of its own.
    ("ctrl+q", Action::RecordMacro),
    ("@0", Action::PlayMacro0),
    ("@1", Action::PlayMacro1),
    ("@2", Action::PlayMacro2),
    ("@3", Action::PlayMacro3),
    ("@4", Action::PlayMacro4),
    ("@5", Action::PlayMacro5),
    ("@6", Action::PlayMacro6),
    ("@7", Action::PlayMacro7),
    ("@8", Action::PlayMacro8),
    ("@9", Action::PlayMacro9),
    // Templates
    ("ctrl+n", Action::ShowTemplates),
    // Keybindings editor
    ("ctrl+k", Action::ShowKeybindingsEditor),
    // Pomodoro timer
    ("f5", Action::PomodoroStart),
    ("f6", Action::PomodoroTogglePause),
    ("f7", Action::PomodoroSkip),
    ("f8", Action::PomodoroStop),
    // Habit tracking: handled contextually in the Habits view
    // (n=create, e=edit, d=delete, space/x=toggle, a=analytics).
    // Burndown chart controls (work in Burndown view)
    ("w", Action::BurndownCycleWindow),
    ("o", Action::BurndownToggleMode),
    ("+", Action::BurndownToggleScopeCreep),
    // Duplicate detection: handled contextually in the Duplicates view
    // (D=dismiss, M=merge, r=refresh) so `D`/`M` keep their global meanings.
    // Git integration
    ("alt+g", Action::ViewGitTodos),
    ("O", Action::OpenInEditor),
    // Reviews
    ("alt+d", Action::ShowDailyReview),
    ("alt+w", Action::ShowWeeklyReview),
    ("alt+e", Action::ShowEveningReview),
    // Task detail
    ("i", Action::ShowTaskDetail),
    // Command palette
    ("alt+p", Action::ShowCommandPalette),
    (":", Action::ShowCommandPalette),
    // Subtask tree collapse/expand
    ("alt+z", Action::ToggleSubtaskCollapse),
];

/// Create the default keybindings map.
#[must_use]
pub fn default_bindings() -> HashMap<String, Action> {
    DEFAULT_BINDINGS
        .iter()
        .map(|(key, action)| ((*key).to_string(), action.clone()))
        .collect()
}
