//! Default keybindings.

use std::collections::HashMap;

use super::Action;

/// Create the default keybindings map.
#[must_use]
pub fn default_bindings() -> HashMap<String, Action> {
    let mut bindings = HashMap::new();

    // Navigation
    bindings.insert("j".to_string(), Action::MoveDown);
    bindings.insert("k".to_string(), Action::MoveUp);
    bindings.insert("down".to_string(), Action::MoveDown);
    bindings.insert("up".to_string(), Action::MoveUp);
    bindings.insert("g".to_string(), Action::MoveFirst);
    bindings.insert("G".to_string(), Action::MoveLast);
    bindings.insert("ctrl+u".to_string(), Action::PageUp);
    bindings.insert("ctrl+d".to_string(), Action::PageDown);
    bindings.insert("pageup".to_string(), Action::PageUp);
    bindings.insert("pagedown".to_string(), Action::PageDown);

    // Task actions
    bindings.insert("x".to_string(), Action::ToggleComplete);
    bindings.insert("space".to_string(), Action::ToggleComplete);
    bindings.insert("a".to_string(), Action::CreateTask);
    bindings.insert("A".to_string(), Action::CreateSubtask);
    bindings.insert("P".to_string(), Action::CreateProject);
    bindings.insert("E".to_string(), Action::EditProject);
    bindings.insert("X".to_string(), Action::DeleteProject);
    bindings.insert("e".to_string(), Action::EditTask);
    bindings.insert("D".to_string(), Action::EditDueDate);
    bindings.insert("S".to_string(), Action::EditScheduledDate);
    bindings.insert("T".to_string(), Action::EditTags);
    bindings.insert("n".to_string(), Action::EditDescription);
    bindings.insert("N".to_string(), Action::EditDescriptionMultiline);
    bindings.insert("d".to_string(), Action::DeleteTask);
    bindings.insert("p".to_string(), Action::CyclePriority);
    bindings.insert("m".to_string(), Action::MoveToProject);

    // Task snooze
    bindings.insert("z".to_string(), Action::SnoozeTask);
    bindings.insert("Z".to_string(), Action::ClearSnooze);

    // Time tracking
    bindings.insert("t".to_string(), Action::ToggleTimeTracking);
    bindings.insert("L".to_string(), Action::ShowTimeLog);
    bindings.insert("W".to_string(), Action::ShowWorkLog);
    bindings.insert("E".to_string(), Action::EditEstimate);

    // UI actions
    bindings.insert("b".to_string(), Action::ToggleSidebar);
    bindings.insert("c".to_string(), Action::ToggleShowCompleted);
    bindings.insert("?".to_string(), Action::ShowHelp);
    bindings.insert("f".to_string(), Action::ToggleFocusMode);
    bindings.insert("h".to_string(), Action::FocusSidebar);
    bindings.insert("l".to_string(), Action::FocusTaskList);
    bindings.insert("left".to_string(), Action::FocusSidebar);
    bindings.insert("right".to_string(), Action::FocusTaskList);
    bindings.insert("enter".to_string(), Action::Select);
    bindings.insert("/".to_string(), Action::Search);
    bindings.insert("ctrl+l".to_string(), Action::ClearSearch);
    bindings.insert("#".to_string(), Action::FilterByTag);
    bindings.insert("ctrl+t".to_string(), Action::ClearTagFilter);
    bindings.insert("s".to_string(), Action::CycleSortField);
    bindings.insert("ctrl+s".to_string(), Action::ToggleSortOrder);

    // Multi-select / Bulk operations
    bindings.insert("v".to_string(), Action::ToggleMultiSelect);
    bindings.insert("V".to_string(), Action::SelectAll);
    bindings.insert("ctrl+v".to_string(), Action::ClearSelection);

    // Dependencies
    bindings.insert("B".to_string(), Action::EditDependencies);

    // Recurrence
    bindings.insert("R".to_string(), Action::EditRecurrence);

    // Manual ordering
    bindings.insert("ctrl+up".to_string(), Action::MoveTaskUp);
    bindings.insert("ctrl+down".to_string(), Action::MoveTaskDown);

    // Task chains
    bindings.insert("ctrl+l".to_string(), Action::LinkTask);
    bindings.insert("ctrl+shift+l".to_string(), Action::UnlinkTask);

    // Calendar navigation
    bindings.insert("<".to_string(), Action::CalendarPrevMonth);
    bindings.insert(">".to_string(), Action::CalendarNextMonth);

    // System
    bindings.insert("ctrl+s".to_string(), Action::Save);
    bindings.insert("u".to_string(), Action::Undo);
    bindings.insert("ctrl+z".to_string(), Action::Undo);
    bindings.insert("ctrl+r".to_string(), Action::Redo);
    bindings.insert("U".to_string(), Action::Redo);
    bindings.insert("q".to_string(), Action::Quit);
    // bindings.insert("esc".to_string(), Action::Quit);
    bindings.insert("f5".to_string(), Action::RefreshStorage);

    // Export
    bindings.insert("ctrl+e".to_string(), Action::ExportCsv);
    bindings.insert("ctrl+i".to_string(), Action::ExportIcs);
    bindings.insert("ctrl+g".to_string(), Action::ExportChainsDot);
    bindings.insert("ctrl+m".to_string(), Action::ExportChainsMermaid);
    bindings.insert("ctrl+p".to_string(), Action::ExportReportMarkdown);
    bindings.insert("ctrl+h".to_string(), Action::ExportReportHtml);

    // Import
    bindings.insert("I".to_string(), Action::ImportCsv); // Shift+I for CSV import
    bindings.insert("alt+i".to_string(), Action::ImportIcs); // Alt+I for ICS import

    // Reports navigation (Tab/Shift+Tab or l/h when in reports view)
    bindings.insert("tab".to_string(), Action::ReportsNextPanel);
    bindings.insert("shift+tab".to_string(), Action::ReportsPrevPanel);

    // Macros - q to record, Q to stop, @0-9 to play
    bindings.insert("ctrl+q".to_string(), Action::RecordMacro);
    bindings.insert("ctrl+Q".to_string(), Action::StopRecordMacro);
    bindings.insert("@0".to_string(), Action::PlayMacro0);
    bindings.insert("@1".to_string(), Action::PlayMacro1);
    bindings.insert("@2".to_string(), Action::PlayMacro2);
    bindings.insert("@3".to_string(), Action::PlayMacro3);
    bindings.insert("@4".to_string(), Action::PlayMacro4);
    bindings.insert("@5".to_string(), Action::PlayMacro5);
    bindings.insert("@6".to_string(), Action::PlayMacro6);
    bindings.insert("@7".to_string(), Action::PlayMacro7);
    bindings.insert("@8".to_string(), Action::PlayMacro8);
    bindings.insert("@9".to_string(), Action::PlayMacro9);

    // Templates
    bindings.insert("ctrl+n".to_string(), Action::ShowTemplates);

    // Keybindings editor
    bindings.insert("ctrl+k".to_string(), Action::ShowKeybindingsEditor);

    // Pomodoro timer
    bindings.insert("f5".to_string(), Action::PomodoroStart);
    bindings.insert("f6".to_string(), Action::PomodoroTogglePause);
    bindings.insert("f7".to_string(), Action::PomodoroSkip);
    bindings.insert("f8".to_string(), Action::PomodoroStop);

    // Habit tracking (work in Habits view)
    // Note: These use different keys to avoid conflicts with task actions
    // In Habits view: n=create, e=edit, d=delete, space/x=toggle, a=analytics
    // The habit-specific actions are mapped contextually in the key handler

    // Git sync (requires Markdown backend)
    bindings.insert("gs".to_string(), Action::GitStatus);
    bindings.insert("gc".to_string(), Action::GitCommit);
    bindings.insert("gp".to_string(), Action::GitPull);
    bindings.insert("gP".to_string(), Action::GitPush);
    bindings.insert("gS".to_string(), Action::GitSync);

    bindings
}
