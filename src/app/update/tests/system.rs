//! System message tests (quit, resize).

use crate::app::{update::update, Message, Model, RunningState, SystemMessage, TimeMessage};

use super::create_test_model_with_tasks;

#[test]
fn test_system_quit_preserves_timer() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    model.start_time_tracking(task_id);

    assert!(model.active_time_entry.is_some());

    update(&mut model, Message::System(SystemMessage::Quit));

    // Timer should persist across app restarts (not stopped on quit)
    assert!(model.active_time_entry.is_some());
    assert_eq!(model.running, RunningState::Quitting);
}

#[test]
fn test_system_resize() {
    let mut model = Model::new();

    update(
        &mut model,
        Message::System(SystemMessage::Resize {
            width: 120,
            height: 40,
        }),
    );

    assert_eq!(model.terminal_size, (120, 40));
}

#[test]
fn test_time_toggle_tracking_start() {
    let mut model = create_test_model_with_tasks();
    assert!(model.active_time_entry.is_none());

    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    assert!(model.active_time_entry.is_some());
}

#[test]
fn test_time_toggle_tracking_stop() {
    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    model.start_time_tracking(task_id);

    update(&mut model, Message::Time(TimeMessage::ToggleTracking));

    assert!(model.active_time_entry.is_none());
}

// ============================================================================
// Undo/Redo Tests
// ============================================================================

#[test]
fn test_undo_task_created() {
    use crate::app::TaskMessage;

    let mut model = Model::new();
    assert_eq!(model.tasks.len(), 0);

    // Create a task
    update(
        &mut model,
        Message::Task(TaskMessage::Create("Test task".into())),
    );
    assert_eq!(model.tasks.len(), 1);

    // Undo should remove the task
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.len(), 0);
    assert!(model
        .alerts
        .status_message
        .as_ref()
        .is_some_and(|m| m.contains("Undone")));

    // Redo should restore it
    update(&mut model, Message::System(SystemMessage::Redo));
    assert_eq!(model.tasks.len(), 1);
    assert!(model
        .alerts
        .status_message
        .as_ref()
        .is_some_and(|m| m.contains("Redone")));
}

#[test]
fn test_undo_task_deleted() {
    use crate::app::TaskMessage;

    let mut model = create_test_model_with_tasks();
    let initial_count = model.tasks.len();
    let task_id = model.visible_tasks[0];

    // Delete a task
    update(&mut model, Message::Task(TaskMessage::Delete(task_id)));
    assert_eq!(model.tasks.len(), initial_count - 1);

    // Undo should restore it
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.tasks.len(), initial_count);
    assert!(model.tasks.contains_key(&task_id));
}

#[test]
fn test_undo_task_modified() {
    use crate::app::TaskMessage;
    use crate::domain::Priority;

    let mut model = create_test_model_with_tasks();
    let task_id = model.visible_tasks[0];
    let original_priority = model.tasks.get(&task_id).unwrap().priority;

    // Modify the task priority
    update(
        &mut model,
        Message::Task(TaskMessage::SetPriority(task_id, Priority::High)),
    );
    assert_eq!(model.tasks.get(&task_id).unwrap().priority, Priority::High);

    // Undo should restore original priority
    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(
        model.tasks.get(&task_id).unwrap().priority,
        original_priority
    );
}

#[test]
fn test_undo_empty_stack() {
    let mut model = Model::new();
    let initial_status = model.alerts.status_message.clone();

    // Undo on empty stack should do nothing (no panic)
    update(&mut model, Message::System(SystemMessage::Undo));

    // No "Undone" message should appear
    assert_eq!(model.alerts.status_message, initial_status);
}

#[test]
fn test_redo_empty_stack() {
    let mut model = Model::new();
    let initial_status = model.alerts.status_message.clone();

    // Redo on empty stack should do nothing (no panic)
    update(&mut model, Message::System(SystemMessage::Redo));

    // No "Redone" message should appear
    assert_eq!(model.alerts.status_message, initial_status);
}

// ============================================================================
// Tick Tests
// ============================================================================

#[test]
fn test_tick_clears_old_status_message() {
    use std::time::{Duration, Instant};

    let mut model = Model::new();
    model.alerts.status_message = Some("Old message".to_string());
    // Simulate message set 4 seconds ago
    model.alerts.status_message_set_at = Instant::now().checked_sub(Duration::from_secs(4));

    update(&mut model, Message::System(SystemMessage::Tick));

    // Message should be cleared after 3 second timeout
    assert!(model.alerts.status_message.is_none());
    assert!(model.alerts.status_message_set_at.is_none());
}

#[test]
fn test_tick_keeps_recent_status_message() {
    use std::time::Instant;

    let mut model = Model::new();
    model.alerts.status_message = Some("Recent message".to_string());
    model.alerts.status_message_set_at = Some(Instant::now());

    update(&mut model, Message::System(SystemMessage::Tick));

    // Message should still be there (less than 3 seconds old)
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_tick_sets_timestamp_if_missing() {
    let mut model = Model::new();
    model.alerts.status_message = Some("Message without timestamp".to_string());
    model.alerts.status_message_set_at = None;

    update(&mut model, Message::System(SystemMessage::Tick));

    // Timestamp should be set
    assert!(model.alerts.status_message_set_at.is_some());
    // Message should still be there
    assert!(model.alerts.status_message.is_some());
}

// ============================================================================
// RefreshStorage Tests
// ============================================================================

#[test]
fn test_refresh_storage_no_changes() {
    let mut model = Model::new();

    update(&mut model, Message::System(SystemMessage::RefreshStorage));

    // Should show "no changes" message
    assert!(model
        .alerts
        .status_message
        .as_ref()
        .is_some_and(|m| m.contains("No external changes")));
}

// ============================================================================
// Save Tests
// ============================================================================

#[test]
fn test_save_does_not_panic() {
    let mut model = Model::new();

    // Save should not panic even without a storage backend
    update(&mut model, Message::System(SystemMessage::Save));
    // No assertion needed - just verify no panic
}

// ============================================================================
// Undo/Redo: remaining action kinds
// ============================================================================

mod undo_action_kinds {
    use crate::app::{update::update, Message, Model, SystemMessage, TimeMessage, UiMessage};
    use crate::domain::{Project, Task, WorkLogEntry};
    use crate::ui::{InputMode, InputTarget, WorkLogMode};

    use super::create_test_model_with_tasks;

    fn status(model: &Model) -> &str {
        model.alerts.status_message.as_deref().unwrap_or("")
    }

    #[test]
    fn test_undo_redo_timer_stop() {
        let mut model = create_test_model_with_tasks();
        update(&mut model, Message::Time(TimeMessage::ToggleTracking));
        let entry_id = model.active_time_entry.expect("timer started");

        update(&mut model, Message::Time(TimeMessage::ToggleTracking));
        assert!(model.active_time_entry.is_none());
        assert!(model.time_entries[&entry_id].ended_at.is_some());

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: timer stop");
        assert_eq!(model.active_time_entry, Some(entry_id));
        assert!(model.time_entries[&entry_id].ended_at.is_none());

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: timer stop");
        // The stored entry is stopped again. Note: `restore_time_entry` does not
        // clear `active_time_entry` for a stopped entry, so the active pointer
        // is left stale here (known gap, not asserted).
        assert!(model.time_entries[&entry_id].ended_at.is_some());
    }

    #[test]
    fn test_undo_redo_timer_start() {
        let mut model = create_test_model_with_tasks();
        update(&mut model, Message::Time(TimeMessage::ToggleTracking));
        let entry_id = model.active_time_entry.expect("timer started");

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: timer start");
        assert!(model.active_time_entry.is_none());
        assert!(!model.time_entries.contains_key(&entry_id));

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: timer start");
        assert_eq!(model.active_time_entry, Some(entry_id));
        assert!(model.time_entries.contains_key(&entry_id));
    }

    #[test]
    fn test_undo_redo_task_deleted_restores_time_entries() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        update(&mut model, Message::Time(TimeMessage::ToggleTracking));
        update(&mut model, Message::Time(TimeMessage::ToggleTracking));
        let entry_id = *model.time_entries.keys().next().expect("one entry");
        assert_eq!(model.time_entries[&entry_id].task_id, task_id);

        update(
            &mut model,
            Message::Task(crate::app::TaskMessage::Delete(task_id)),
        );
        assert!(!model.tasks.contains_key(&task_id));
        assert!(model.time_entries.is_empty());

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: task deletion");
        assert!(model.tasks.contains_key(&task_id));
        assert!(model.time_entries.contains_key(&entry_id));

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: task deletion");
        assert!(!model.tasks.contains_key(&task_id));
        assert!(model.time_entries.is_empty());
    }

    #[test]
    fn test_undo_redo_project_deleted() {
        let mut model = Model::new();
        let project = Project::new("Doomed");
        let project_id = project.id;
        model.projects.insert(project_id, project);
        let mut task = Task::new("In project");
        task.project_id = Some(project_id);
        let task_id = task.id;
        model.tasks.insert(task_id, task);
        model.refresh_visible_tasks();
        model.selected_project = Some(project_id);

        update(&mut model, Message::Ui(UiMessage::DeleteProject));
        assert!(!model.projects.contains_key(&project_id));
        assert_eq!(model.tasks[&task_id].project_id, None);

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: project deletion");
        assert_eq!(model.projects[&project_id].name, "Doomed");

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: project deletion");
        assert!(!model.projects.contains_key(&project_id));
    }

    #[test]
    fn test_undo_redo_project_modified() {
        let mut model = Model::new();
        let project = Project::new("Before");
        let project_id = project.id;
        model.projects.insert(project_id, project);
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::EditProject(project_id);
        model.input.buffer = "After".to_string();

        update(&mut model, Message::Ui(UiMessage::SubmitInput));
        assert_eq!(model.projects[&project_id].name, "After");

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: project change");
        assert_eq!(model.projects[&project_id].name, "Before");

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: project change");
        assert_eq!(model.projects[&project_id].name, "After");
    }

    #[test]
    fn test_undo_redo_project_created() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::Project;
        model.input.buffer = "Fresh".to_string();
        update(&mut model, Message::Ui(UiMessage::SubmitInput));
        assert_eq!(model.projects.len(), 1);

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: project creation");
        assert!(model.projects.is_empty());

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: project creation");
        assert_eq!(model.projects.len(), 1);
        assert_eq!(model.projects.values().next().unwrap().name, "Fresh");
    }

    fn model_with_work_log() -> (Model, crate::domain::TaskId, crate::domain::WorkLogEntryId) {
        let mut model = Model::new();
        let task = Task::new("Logged task");
        let task_id = task.id;
        model.tasks.insert(task_id, task);
        let entry = WorkLogEntry::new(task_id, "Original content");
        let entry_id = entry.id;
        model.work_logs.insert(entry_id, entry);
        model.refresh_visible_tasks();
        model.selected_index = 0;
        model.work_log_editor.visible = true;
        (model, task_id, entry_id)
    }

    #[test]
    fn test_undo_redo_work_log_created() {
        let (mut model, task_id, _) = model_with_work_log();
        model.work_log_editor.mode = WorkLogMode::Add;
        model.work_log_editor.buffer = vec!["Brand new".to_string()];
        update(&mut model, Message::Ui(UiMessage::WorkLogSubmit));
        assert_eq!(model.work_logs.len(), 2);
        let new_id = model
            .work_logs
            .values()
            .find(|e| e.content == "Brand new")
            .map(|e| e.id)
            .expect("new entry present");

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: work log creation");
        assert!(!model.work_logs.contains_key(&new_id));
        assert_eq!(model.work_logs_for_task(&task_id).len(), 1);

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: work log creation");
        assert_eq!(model.work_logs[&new_id].content, "Brand new");
    }

    #[test]
    fn test_undo_redo_work_log_deleted() {
        let (mut model, _, entry_id) = model_with_work_log();
        model.work_log_editor.mode = WorkLogMode::ConfirmDelete;
        model.work_log_editor.selected = 0;
        update(&mut model, Message::Ui(UiMessage::WorkLogDelete));
        assert!(model.work_logs.is_empty());

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: work log deletion");
        assert_eq!(model.work_logs[&entry_id].content, "Original content");

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: work log deletion");
        assert!(model.work_logs.is_empty());
    }

    #[test]
    fn test_undo_redo_work_log_modified() {
        let (mut model, _, entry_id) = model_with_work_log();
        model.work_log_editor.mode = WorkLogMode::Edit;
        model.work_log_editor.selected = 0;
        model.work_log_editor.buffer = vec!["Edited content".to_string()];
        update(&mut model, Message::Ui(UiMessage::WorkLogSubmit));
        assert_eq!(model.work_logs[&entry_id].content, "Edited content");

        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: work log change");
        assert_eq!(model.work_logs[&entry_id].content, "Original content");

        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: work log change");
        assert_eq!(model.work_logs[&entry_id].content, "Edited content");
    }

    #[test]
    fn test_undo_redo_task_modified_descriptions() {
        let mut model = create_test_model_with_tasks();
        let task_id = model.visible_tasks[0];
        update(
            &mut model,
            Message::Task(crate::app::TaskMessage::SetPriority(
                task_id,
                crate::domain::Priority::Urgent,
            )),
        );
        update(&mut model, Message::System(SystemMessage::Undo));
        assert_eq!(status(&model), "Undone: task change");
        update(&mut model, Message::System(SystemMessage::Redo));
        assert_eq!(status(&model), "Redone: task change");
        assert_eq!(
            model.tasks[&task_id].priority,
            crate::domain::Priority::Urgent
        );
    }
}

// ============================================================================
// Export handlers (file output)
// ============================================================================

mod export_files {
    use crate::app::{update::update, Message, Model, SystemMessage};
    use crate::domain::Task;
    use crate::storage::BackendType;
    use tempfile::TempDir;

    fn model_with_storage() -> (Model, TempDir, std::path::PathBuf) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("data.json");
        let mut model = Model::new()
            .with_storage(BackendType::Json, path.clone())
            .expect("storage");
        let mut first = Task::new("First exported");
        first.tags = vec!["alpha".to_string()];
        let mut second = Task::new("Second exported");
        second.dependencies = vec![first.id];
        first.next_task_id = Some(second.id);
        model.tasks.insert(first.id, first);
        model.tasks.insert(second.id, second);
        model.refresh_visible_tasks();
        (model, dir, path)
    }

    fn status(model: &Model) -> &str {
        model.alerts.status_message.as_deref().unwrap_or("")
    }

    #[test]
    fn test_export_csv_writes_file_next_to_data() {
        let (mut model, _dir, path) = model_with_storage();
        update(&mut model, Message::System(SystemMessage::ExportCsv));

        let export_path = path.with_extension("csv");
        assert!(export_path.exists(), "csv should be written");
        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(content.contains("First exported"));
        assert!(content.contains("Second exported"));
        assert!(status(&model).starts_with("Exported 2 tasks to "));
        assert!(status(&model).ends_with("data.csv"));
    }

    #[test]
    fn test_export_ics_writes_calendar() {
        let (mut model, _dir, path) = model_with_storage();
        update(&mut model, Message::System(SystemMessage::ExportIcs));

        let content = std::fs::read_to_string(path.with_extension("ics")).unwrap();
        assert!(content.contains("BEGIN:VCALENDAR"));
        assert!(content.contains("First exported"));
        assert!(status(&model).starts_with("Exported 2 tasks to "));
    }

    #[test]
    fn test_export_chains_dot_writes_graph() {
        let (mut model, _dir, path) = model_with_storage();
        update(&mut model, Message::System(SystemMessage::ExportChainsDot));

        let content = std::fs::read_to_string(path.with_extension("dot")).unwrap();
        assert!(content.contains("digraph"));
        assert!(status(&model).contains("Exported task chains to "));
        assert!(status(&model).contains("Graphviz"));
    }

    #[test]
    fn test_export_chains_mermaid_writes_diagram() {
        let (mut model, _dir, path) = model_with_storage();
        update(
            &mut model,
            Message::System(SystemMessage::ExportChainsMermaid),
        );

        let content = std::fs::read_to_string(path.with_extension("md")).unwrap();
        assert!(!content.is_empty());
        assert!(status(&model).contains("Mermaid diagram"));
    }

    #[test]
    fn test_export_report_markdown_writes_report() {
        let (mut model, _dir, path) = model_with_storage();
        update(
            &mut model,
            Message::System(SystemMessage::ExportReportMarkdown),
        );

        let export_path = path.with_extension("report.md");
        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(content.contains('#'), "markdown report has a heading");
        assert!(status(&model).starts_with("Exported analytics report to "));
        assert!(status(&model).ends_with("data.report.md"));
    }

    #[test]
    fn test_export_report_html_writes_report() {
        let (mut model, _dir, path) = model_with_storage();
        update(&mut model, Message::System(SystemMessage::ExportReportHtml));

        let export_path = path.with_extension("report.html");
        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(content.to_lowercase().contains("<html"));
        assert!(status(&model).ends_with("data.report.html"));
    }

    /// Point the data path into a directory that does not exist so that the
    /// `std::fs::write` branch of every exporter fails.
    fn model_with_unwritable_path() -> (Model, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let mut model = Model::new();
        model.storage.data_path = Some(dir.path().join("missing").join("data.json"));
        (model, dir)
    }

    #[test]
    fn test_export_failures_report_write_error() {
        for msg in [
            SystemMessage::ExportCsv,
            SystemMessage::ExportIcs,
            SystemMessage::ExportChainsDot,
            SystemMessage::ExportChainsMermaid,
            SystemMessage::ExportReportMarkdown,
            SystemMessage::ExportReportHtml,
        ] {
            let (mut model, _dir) = model_with_unwritable_path();
            let label = format!("{msg:?}");
            update(&mut model, Message::System(msg));
            assert!(
                status(&model).starts_with("Export failed: "),
                "{label} should report a write failure, got {:?}",
                status(&model)
            );
        }
    }
}

// ============================================================================
// Import execution (file parsing + preview)
// ============================================================================

mod import_execution {
    use crate::app::{update::update, Message, Model, SystemMessage};
    use crate::domain::{CalendarEvent, Task};
    use crate::storage::{ImportFormat, ImportResult};
    use crate::ui::{InputMode, InputTarget};
    use tempfile::TempDir;

    fn write_file(dir: &TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write import fixture");
        path.to_string_lossy().into_owned()
    }

    fn execute(model: &mut Model, format: ImportFormat, path: String) {
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::ImportFilePath(format);
        model.input.buffer = path;
        update(model, Message::System(SystemMessage::ExecuteImport));
    }

    fn status(model: &Model) -> &str {
        model.alerts.status_message.as_deref().unwrap_or("")
    }

    #[test]
    fn test_execute_import_csv_shows_preview() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "tasks.csv",
            "title,status,priority\nWrite tests,todo,high\nShip it,done,low\n",
        );
        let mut model = Model::new();

        execute(&mut model, ImportFormat::Csv, path);

        assert!(model.import.show_preview);
        let pending = model.import.pending.as_ref().expect("pending import");
        assert_eq!(pending.imported.len(), 2);
        assert!(pending.skipped.is_empty());
        assert!(pending.errors.is_empty());
        assert_eq!(
            status(&model),
            "Preview: 2 tasks to import, 0 skipped, 0 errors. Press Enter to confirm, Esc to cancel."
        );
        // Input mode is reset once the file has been parsed
        assert_eq!(model.input.mode, InputMode::Normal);
        assert_eq!(model.input.target, InputTarget::Task);
        assert!(model.input.buffer.is_empty());
        // Nothing is applied until the import is confirmed
        assert!(model.tasks.is_empty());
    }

    #[test]
    fn test_execute_import_csv_skips_duplicates_and_counts_errors() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "tasks.csv",
            "title\nExisting task\nNew task\n   \n\"\"\n",
        );
        let mut model = Model::new();
        let existing = Task::new("Existing task");
        model.tasks.insert(existing.id, existing);

        execute(&mut model, ImportFormat::Csv, path);

        let pending = model.import.pending.as_ref().expect("pending import");
        assert_eq!(pending.imported.len(), 1);
        assert_eq!(pending.imported[0].title, "New task");
        assert_eq!(pending.skipped.len(), 1);
        assert_eq!(pending.skipped[0].0.title, "Existing task");
        assert_eq!(pending.errors.len(), 1, "empty quoted title is an error");
        assert!(status(&model).starts_with("Preview: 1 task to import, 1 skipped, 1 errors."));
    }

    #[test]
    fn test_execute_import_csv_header_only_reports_no_items() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "empty.csv", "title,status\n");
        let mut model = Model::new();

        execute(&mut model, ImportFormat::Csv, path);

        assert_eq!(status(&model), "No items found in file");
        assert!(!model.import.show_preview);
        assert!(model.import.pending.is_none());
    }

    #[test]
    fn test_execute_import_csv_without_title_column_fails() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "bad.csv", "id,status\n1,todo\n");
        let mut model = Model::new();

        execute(&mut model, ImportFormat::Csv, path);

        assert!(status(&model).starts_with("Import failed: "));
        assert!(status(&model).contains("title"));
        assert_eq!(model.input.mode, InputMode::Normal);
        assert_eq!(model.input.target, InputTarget::Task);
        assert!(model.import.pending.is_none());
    }

    #[test]
    fn test_execute_import_ics_counts_tasks_and_events() {
        let dir = TempDir::new().unwrap();
        let ics = "BEGIN:VCALENDAR\nVERSION:2.0\n\
BEGIN:VTODO\nUID:todo-1\nSUMMARY:Imported todo\nEND:VTODO\n\
BEGIN:VEVENT\nUID:event-1\nSUMMARY:Standup\nDTSTART:20251201T090000Z\nDTEND:20251201T093000Z\nEND:VEVENT\n\
BEGIN:VEVENT\nUID:event-2\nSUMMARY:Offsite\nDTSTART;VALUE=DATE:20251202\nEND:VEVENT\n\
END:VCALENDAR\n";
        let path = write_file(&dir, "cal.ics", ics);
        let mut model = Model::new();

        execute(&mut model, ImportFormat::Ics, path);

        let pending = model.import.pending.as_ref().expect("pending import");
        assert_eq!(pending.imported.len(), 1);
        assert_eq!(pending.imported[0].title, "Imported todo");
        assert_eq!(pending.imported_events.len(), 2);
        assert!(
            status(&model).starts_with("Preview: 1 task, 2 events to import, 0 skipped, 0 errors.")
        );
    }

    #[test]
    fn test_execute_import_ics_events_only() {
        let dir = TempDir::new().unwrap();
        let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:Solo event\nDTSTART:20251201T090000Z\nEND:VEVENT\nEND:VCALENDAR\n";
        let path = write_file(&dir, "events.ics", ics);
        let mut model = Model::new();

        execute(&mut model, ImportFormat::Ics, path);

        let pending = model.import.pending.as_ref().expect("pending import");
        assert!(pending.imported.is_empty());
        assert_eq!(pending.imported_events.len(), 1);
        assert!(status(&model).starts_with("Preview: 1 event to import, 0 skipped, 0 errors."));
    }

    #[test]
    fn test_execute_import_ics_event_without_start_is_error() {
        let dir = TempDir::new().unwrap();
        let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:No start\nEND:VEVENT\nEND:VCALENDAR\n";
        let path = write_file(&dir, "broken.ics", ics);
        let mut model = Model::new();

        execute(&mut model, ImportFormat::Ics, path);

        let pending = model.import.pending.as_ref().expect("pending import");
        assert!(pending.imported.is_empty());
        assert!(pending.imported_events.is_empty());
        assert_eq!(pending.errors.len(), 1);
        assert!(status(&model).starts_with("Preview:  to import, 0 skipped, 1 errors."));
    }

    #[test]
    fn test_execute_import_ignores_non_import_target() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::Task;
        model.input.buffer = "/some/file.csv".to_string();

        update(&mut model, Message::System(SystemMessage::ExecuteImport));

        assert!(model.alerts.status_message.is_none());
        assert_eq!(model.input.mode, InputMode::Editing);
        assert_eq!(model.input.buffer, "/some/file.csv");
    }

    #[test]
    fn test_execute_import_whitespace_path_resets_input() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::ImportFilePath(ImportFormat::Ics);
        model.input.buffer = "   ".to_string();

        update(&mut model, Message::System(SystemMessage::ExecuteImport));

        assert_eq!(status(&model), "No file path provided");
        assert_eq!(model.input.mode, InputMode::Normal);
        assert_eq!(model.input.target, InputTarget::Task);
    }

    #[test]
    fn test_confirm_import_adds_tasks_and_events() {
        let mut model = Model::new();
        model.import.show_preview = true;
        model.import.pending = Some(ImportResult {
            imported: vec![Task::new("Imported")],
            imported_events: vec![CalendarEvent::new("Event A"), CalendarEvent::new("Event B")],
            skipped: vec![],
            errors: vec![],
        });

        update(&mut model, Message::System(SystemMessage::ConfirmImport));

        assert_eq!(model.tasks.len(), 1);
        assert_eq!(model.calendar_events.len(), 2);
        assert!(model.storage.dirty);
        assert!(!model.import.show_preview);
        assert_eq!(status(&model), "Imported 1 task and 2 events");
    }

    #[test]
    fn test_confirm_import_events_only_message() {
        let mut model = Model::new();
        model.import.pending = Some(ImportResult {
            imported: vec![],
            imported_events: vec![CalendarEvent::new("Only event")],
            skipped: vec![],
            errors: vec![],
        });

        update(&mut model, Message::System(SystemMessage::ConfirmImport));

        assert_eq!(model.calendar_events.len(), 1);
        assert_eq!(status(&model), "Imported 1 event");
    }

    #[test]
    fn test_confirm_import_nothing_to_import_message() {
        let mut model = Model::new();
        model.import.pending = Some(ImportResult {
            imported: vec![],
            imported_events: vec![],
            skipped: vec![],
            errors: vec![],
        });

        update(&mut model, Message::System(SystemMessage::ConfirmImport));

        assert_eq!(status(&model), "No items imported");
        assert!(model.import.pending.is_none());
    }

    #[test]
    fn test_confirm_import_pluralises_tasks() {
        let mut model = Model::new();
        model.import.pending = Some(ImportResult {
            imported: vec![Task::new("A"), Task::new("B")],
            imported_events: vec![],
            skipped: vec![],
            errors: vec![],
        });

        update(&mut model, Message::System(SystemMessage::ConfirmImport));

        assert_eq!(status(&model), "Imported 2 tasks");
    }
}

// ============================================================================
// Git integration handlers
//
// These handlers operate on the process working directory, which under
// `cargo test` is the crate root (a git checkout). They skip gracefully when
// git or a base branch is unavailable, so the tests do the same.
// ============================================================================

mod git_handlers {
    use crate::app::{update::update, Message, Model, SystemMessage};
    use crate::domain::git::operations::{get_base_branch, is_branch_merged, list_branches};
    use crate::domain::git::{GitLinkType, GitRef};
    use crate::domain::{Task, TaskStatus};
    use crate::storage::BackendType;
    use tempfile::TempDir;

    fn git_available() -> bool {
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn model_with_storage() -> (Model, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let model = Model::new()
            .with_storage(BackendType::Json, dir.path().join("data.json"))
            .expect("storage");
        (model, dir)
    }

    fn status(model: &Model) -> String {
        model.alerts.status_message.clone().unwrap_or_default()
    }

    #[test]
    fn test_scan_git_todos_second_scan_reports_tracked_items() {
        if !git_available() {
            return;
        }
        let (mut model, _dir) = model_with_storage();

        update(&mut model, Message::System(SystemMessage::ScanGitTodos));

        let created: Vec<_> = model
            .tasks
            .values()
            .filter(|t| t.tags == ["git-todo"])
            .collect();
        let created_count = created.len();
        assert_eq!(
            status(&model),
            format!("Git TODOs: {created_count} created, 0 already tracked")
        );
        assert!(model.alerts.status_message_set_at.is_some());
        for task in &created {
            let desc = task.description.as_deref().unwrap_or("");
            assert!(desc.starts_with("git:"), "git location recorded: {desc}");
            assert!(
                desc.matches(':').count() >= 2,
                "git:file:line format: {desc}"
            );
        }
        if created_count > 0 {
            assert!(!model.storage.dirty, "scan saves when it creates tasks");
            assert_eq!(model.visible_tasks.len(), created_count);
        }

        // A second scan finds the same TODOs and skips all of them
        update(&mut model, Message::System(SystemMessage::ScanGitTodos));
        assert_eq!(model.tasks.len(), created_count);
        assert_eq!(
            status(&model),
            format!("Git TODOs: 0 created, {created_count} already tracked")
        );
    }

    #[test]
    fn test_check_merged_branches_ignores_unknown_branch() {
        if !git_available() {
            return;
        }
        let (mut model, _dir) = model_with_storage();
        let task = Task::new("Linked to nowhere").with_git_ref(GitRef::new(
            "taskflow-test-branch-that-does-not-exist",
            GitLinkType::Manual,
        ));
        let task_id = task.id;
        model.tasks.insert(task_id, task);
        model.refresh_visible_tasks();

        update(
            &mut model,
            Message::System(SystemMessage::CheckMergedBranches),
        );

        assert_eq!(model.tasks[&task_id].status, TaskStatus::Todo);
        assert!(model.alerts.status_message.is_none());
    }

    #[test]
    fn test_check_merged_branches_completes_linked_tasks() {
        if !git_available() {
            return;
        }
        let Ok(cwd) = std::env::current_dir() else {
            return;
        };
        let Ok(base) = get_base_branch(&cwd) else {
            return; // CI checkouts may not have main/master locally
        };
        let Ok(branches) = list_branches(&cwd) else {
            return;
        };
        let Some(merged) = branches
            .into_iter()
            .find(|b| is_branch_merged(&cwd, b, &base).unwrap_or(false))
        else {
            return;
        };

        let (mut model, _dir) = model_with_storage();
        let linked = Task::new("Merged work")
            .with_git_ref(GitRef::new(merged.clone(), GitLinkType::AutoDetected));
        let linked_id = linked.id;
        let already_done = Task::new("Already done")
            .with_status(TaskStatus::Done)
            .with_git_ref(GitRef::new(merged.clone(), GitLinkType::Manual));
        let already_done_id = already_done.id;
        let already_done_completed_at = already_done.completed_at;
        let unlinked = Task::new("No branch");
        let unlinked_id = unlinked.id;
        model.tasks.insert(linked_id, linked);
        model.tasks.insert(already_done_id, already_done);
        model.tasks.insert(unlinked_id, unlinked);
        model.refresh_visible_tasks();

        update(
            &mut model,
            Message::System(SystemMessage::CheckMergedBranches),
        );

        let completed = &model.tasks[&linked_id];
        assert_eq!(completed.status, TaskStatus::Done);
        assert!(completed.completed_at.is_some());
        assert_eq!(model.tasks[&unlinked_id].status, TaskStatus::Todo);
        assert_eq!(
            model.tasks[&already_done_id].completed_at,
            already_done_completed_at
        );
        assert_eq!(status(&model), "Auto-completed 1 task (merged branches)");
        assert!(model.storage.dirty);
    }
}
