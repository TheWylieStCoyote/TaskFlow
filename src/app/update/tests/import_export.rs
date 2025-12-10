//! Import/export and reports tests.

use crate::app::{
    update::update, FocusPane, Message, Model, NavigationMessage, SystemMessage, UiMessage, ViewId,
};
use crate::domain::Task;
use crate::storage::{ImportFormat, ImportResult};
use crate::ui::{InputMode, InputTarget, ReportPanel};

#[test]
fn test_start_import_csv_sets_input_mode() {
    let mut model = Model::new();

    update(&mut model, Message::System(SystemMessage::StartImportCsv));

    assert_eq!(model.input.mode, InputMode::Editing);
    assert!(matches!(
        model.input.target,
        InputTarget::ImportFilePath(ImportFormat::Csv)
    ));
    assert!(model.input.buffer.is_empty());
}

#[test]
fn test_start_import_ics_sets_input_mode() {
    let mut model = Model::new();

    update(&mut model, Message::System(SystemMessage::StartImportIcs));

    assert_eq!(model.input.mode, InputMode::Editing);
    assert!(matches!(
        model.input.target,
        InputTarget::ImportFilePath(ImportFormat::Ics)
    ));
}

#[test]
fn test_cancel_import_resets_state() {
    let mut model = Model::new();

    // Set up pending import state
    model.show_import_preview = true;
    model.pending_import = Some(ImportResult {
        imported: vec![],
        skipped: vec![],
        errors: vec![],
    });

    update(&mut model, Message::System(SystemMessage::CancelImport));

    assert!(!model.show_import_preview);
    assert!(model.pending_import.is_none());
    assert!(model.status_message.is_some());
    assert!(model.status_message.as_ref().unwrap().contains("cancelled"));
}

#[test]
fn test_confirm_import_adds_tasks() {
    let mut model = Model::new();

    // Create a task to import
    let task = Task::new("Imported Task");

    model.show_import_preview = true;
    model.pending_import = Some(ImportResult {
        imported: vec![task.clone()],
        skipped: vec![],
        errors: vec![],
    });

    update(&mut model, Message::System(SystemMessage::ConfirmImport));

    assert!(!model.show_import_preview);
    assert!(model.pending_import.is_none());
    assert_eq!(model.tasks.len(), 1);
    assert!(model.tasks.values().any(|t| t.title == "Imported Task"));
    assert!(model.status_message.is_some());
    assert!(model
        .status_message
        .as_ref()
        .unwrap()
        .contains("Imported 1"));
}

#[test]
fn test_confirm_import_multiple_tasks() {
    let mut model = Model::new();

    // Create multiple tasks to import
    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");
    let task3 = Task::new("Task 3");

    model.show_import_preview = true;
    model.pending_import = Some(ImportResult {
        imported: vec![task1, task2, task3],
        skipped: vec![],
        errors: vec![],
    });

    update(&mut model, Message::System(SystemMessage::ConfirmImport));

    assert_eq!(model.tasks.len(), 3);
    assert!(model
        .status_message
        .as_ref()
        .unwrap()
        .contains("Imported 3"));
}

#[test]
fn test_import_empty_path_shows_error() {
    let mut model = Model::new();

    // Set up for file path input
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::ImportFilePath(ImportFormat::Csv);
    model.input.buffer = "   ".to_string(); // Whitespace only

    // Submit the input
    update(&mut model, Message::Ui(UiMessage::SubmitInput));

    // Should show error, not crash
    assert!(model.status_message.is_some());
    assert!(model
        .status_message
        .as_ref()
        .unwrap()
        .contains("No file path"));
}

#[test]
fn test_reports_panel_navigation() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::Reports;
    assert_eq!(model.report_panel, ReportPanel::Overview);

    // Navigate to next panel
    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsNextPanel),
    );
    assert_eq!(model.report_panel, ReportPanel::Velocity);

    // Navigate to next panel again
    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsNextPanel),
    );
    assert_eq!(model.report_panel, ReportPanel::Tags);

    // Navigate back
    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsPrevPanel),
    );
    assert_eq!(model.report_panel, ReportPanel::Velocity);
}

#[test]
fn test_reports_navigation_only_works_in_reports_view() {
    let mut model = Model::new().with_sample_data();
    model.current_view = ViewId::TaskList; // Not in reports view
    assert_eq!(model.report_panel, ReportPanel::Overview);

    // Try to navigate - should have no effect
    update(
        &mut model,
        Message::Navigation(NavigationMessage::ReportsNextPanel),
    );
    assert_eq!(model.report_panel, ReportPanel::Overview); // Unchanged
}

#[test]
fn test_sidebar_select_reports_view() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 7; // Reports view index

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert_eq!(model.current_view, ViewId::Reports);
    assert_eq!(model.focus_pane, FocusPane::TaskList);
}
