//! System message handlers
//!
//! Handles system-level messages including:
//! - Quit, save, resize
//! - Undo/redo operations
//! - Import/export operations

use crate::app::{Model, RunningState, SystemMessage, UndoAction};
use crate::ui::{InputMode, InputTarget};

/// Handle system messages
pub fn handle_system(model: &mut Model, msg: SystemMessage) {
    match msg {
        SystemMessage::Quit => {
            // Don't stop time tracking - let it persist across app restarts
            // The running entry will be restored when the app reopens
            model.running = RunningState::Quitting;
        }
        SystemMessage::Save => {
            let _ = model.save();
        }
        SystemMessage::Undo => {
            handle_undo(model);
        }
        SystemMessage::Redo => {
            handle_redo(model);
        }
        SystemMessage::Resize { width, height } => {
            model.terminal_size = (width, height);
        }
        SystemMessage::Tick => {
            // Handle periodic updates (e.g., timer display)

            // Clear status message after timeout (3 seconds)
            if model.alerts.status_message.is_some() {
                if let Some(set_at) = model.alerts.status_message_set_at {
                    if set_at.elapsed().as_secs() >= 3 {
                        model.alerts.status_message = None;
                        model.alerts.status_message_set_at = None;
                    }
                } else {
                    // Message exists but no timestamp - set it now
                    model.alerts.status_message_set_at = Some(std::time::Instant::now());
                }
            }
        }
        SystemMessage::ExportCsv => {
            handle_export_csv(model);
        }
        SystemMessage::ExportIcs => {
            handle_export_ics(model);
        }
        SystemMessage::ExportChainsDot => {
            handle_export_chains_dot(model);
        }
        SystemMessage::ExportChainsMermaid => {
            handle_export_chains_mermaid(model);
        }
        SystemMessage::ExportReportMarkdown => {
            handle_export_report_markdown(model);
        }
        SystemMessage::ExportReportHtml => {
            handle_export_report_html(model);
        }
        SystemMessage::StartImportCsv => {
            handle_start_import(model, crate::storage::ImportFormat::Csv);
        }
        SystemMessage::StartImportIcs => {
            handle_start_import(model, crate::storage::ImportFormat::Ics);
        }
        SystemMessage::ExecuteImport => {
            handle_execute_import(model);
        }
        SystemMessage::ConfirmImport => {
            handle_confirm_import(model);
        }
        SystemMessage::CancelImport => {
            handle_cancel_import(model);
        }
        SystemMessage::RefreshStorage => {
            let changes = model.refresh_storage();
            if changes > 0 {
                model.alerts.status_message = Some(format!("Refreshed: {changes} change(s) detected"));
            } else {
                model.alerts.status_message = Some("No external changes detected".to_string());
            }
        }
    }
}

/// Get a human-readable description of an undo action
fn action_description(action: &UndoAction) -> &'static str {
    match action {
        UndoAction::TaskCreated(_) => "task creation",
        UndoAction::TaskDeleted { .. } => "task deletion",
        UndoAction::TaskModified { .. } => "task change",
        UndoAction::ProjectCreated(_) => "project creation",
        UndoAction::ProjectDeleted(_) => "project deletion",
        UndoAction::ProjectModified { .. } => "project change",
        UndoAction::TimeEntryStarted(_) => "timer start",
        UndoAction::TimeEntryStopped { .. } => "timer stop",
        UndoAction::TimeEntryDeleted(_) => "time entry deletion",
        UndoAction::TimeEntryModified { .. } => "time entry change",
        UndoAction::TimerSwitched { .. } => "timer switch",
        UndoAction::WorkLogCreated(_) => "work log creation",
        UndoAction::WorkLogDeleted(_) => "work log deletion",
        UndoAction::WorkLogModified { .. } => "work log change",
    }
}

/// Direction for undo/redo operations
#[derive(Clone, Copy, PartialEq, Eq)]
enum UndoDirection {
    Undo,
    Redo,
}

/// Apply an undo action in the specified direction.
///
/// For most actions, Undo reverses the operation and Redo reapplies it.
/// Note: Modified actions on the redo stack store the inverse (before/after swapped),
/// so "before" always contains the target state.
fn apply_undo_action(model: &mut Model, action: UndoAction, dir: UndoDirection) {
    use UndoDirection::{Redo, Undo};

    match (action, dir) {
        // Task operations
        (UndoAction::TaskCreated(task), Undo) => {
            model.delete_task_from_storage(&task.id);
            model.tasks.remove(&task.id);
        }
        (UndoAction::TaskCreated(task), Redo) => {
            model.sync_task(&task);
            model.tasks.insert(task.id, *task);
        }
        (UndoAction::TaskDeleted { task, time_entries }, Undo) => {
            model.sync_task(&task);
            model.tasks.insert(task.id, *task);
            for entry in time_entries {
                model.restore_time_entry(entry);
            }
        }
        (UndoAction::TaskDeleted { task, time_entries }, Redo) => {
            for entry in &time_entries {
                model.delete_time_entry(&entry.id);
            }
            model.delete_task_from_storage(&task.id);
            model.tasks.remove(&task.id);
        }
        (UndoAction::TaskModified { before, .. }, _) => {
            model.sync_task(&before);
            model.tasks.insert(before.id, *before);
        }

        // Project operations
        (UndoAction::ProjectCreated(project), Undo)
        | (UndoAction::ProjectDeleted(project), Redo) => {
            model.projects.remove(&project.id);
            model.dirty = true;
        }
        (UndoAction::ProjectCreated(project), Redo)
        | (UndoAction::ProjectDeleted(project), Undo) => {
            model.sync_project(&project);
            model.projects.insert(project.id, *project);
        }
        (UndoAction::ProjectModified { before, .. }, _) => {
            model.sync_project(&before);
            model.projects.insert(before.id, *before);
        }

        // Time entry operations
        (UndoAction::TimeEntryStarted(entry), Undo)
        | (UndoAction::TimeEntryDeleted(entry), Redo) => {
            model.delete_time_entry(&entry.id);
        }
        (UndoAction::TimeEntryStarted(entry), Redo)
        | (UndoAction::TimeEntryDeleted(entry), Undo) => {
            model.restore_time_entry(*entry);
        }
        (
            UndoAction::TimeEntryStopped { before, .. }
            | UndoAction::TimeEntryModified { before, .. },
            _,
        ) => {
            model.restore_time_entry(*before);
        }

        // Timer switch (unique: involves two entries)
        (
            UndoAction::TimerSwitched {
                stopped_entry_before,
                started_entry,
                ..
            },
            Undo,
        ) => {
            model.delete_time_entry(&started_entry.id);
            model.restore_time_entry(*stopped_entry_before);
        }
        (
            UndoAction::TimerSwitched {
                stopped_entry_after,
                started_entry,
                ..
            },
            Redo,
        ) => {
            model.active_time_entry = None;
            model.restore_time_entry(*stopped_entry_after);
            model.restore_time_entry(*started_entry);
        }

        // Work log operations
        (UndoAction::WorkLogCreated(entry), Undo) | (UndoAction::WorkLogDeleted(entry), Redo) => {
            model.delete_work_log_from_storage(&entry.id);
            model.work_logs.remove(&entry.id);
        }
        (UndoAction::WorkLogCreated(entry), Redo) | (UndoAction::WorkLogDeleted(entry), Undo) => {
            model.sync_work_log(&entry);
            model.work_logs.insert(entry.id, *entry);
        }
        (UndoAction::WorkLogModified { before, .. }, _) => {
            model.sync_work_log(&before);
            model.work_logs.insert(before.id, *before);
        }
    }
}

fn handle_undo(model: &mut Model) {
    if let Some(action) = model.undo_stack.pop_for_undo() {
        let description = action_description(&action);
        apply_undo_action(model, action, UndoDirection::Undo);
        model.refresh_visible_tasks();
        model.alerts.status_message = Some(format!("Undone: {description}"));
    }
}

fn handle_redo(model: &mut Model) {
    if let Some(action) = model.undo_stack.pop_for_redo() {
        let description = action_description(&action);
        apply_undo_action(model, action, UndoDirection::Redo);
        model.refresh_visible_tasks();
        model.alerts.status_message = Some(format!("Redone: {description}"));
    }
}

fn handle_export_csv(model: &mut Model) {
    use crate::storage::{export_to_string, ExportFormat};

    let tasks = model.tasks_for_export();
    match export_to_string(&tasks, ExportFormat::Csv) {
        Ok(content) => {
            // Determine export path
            let export_path = model.data_path.as_ref().map_or_else(
                || std::path::PathBuf::from("tasks.csv"),
                |p| p.with_extension("csv"),
            );

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.alerts.status_message = Some(format!(
                        "Exported {} tasks to {}",
                        tasks.len(),
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.alerts.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.alerts.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_ics(model: &mut Model) {
    use crate::storage::{export_to_string, ExportFormat};

    let tasks = model.tasks_for_export();
    match export_to_string(&tasks, ExportFormat::Ics) {
        Ok(content) => {
            // Determine export path
            let export_path = model.data_path.as_ref().map_or_else(
                || std::path::PathBuf::from("tasks.ics"),
                |p| p.with_extension("ics"),
            );

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.alerts.status_message = Some(format!(
                        "Exported {} tasks to {}",
                        tasks.len(),
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.alerts.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.alerts.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_chains_dot(model: &mut Model) {
    use crate::storage::{export_chains_to_string, ExportFormat};

    match export_chains_to_string(&model.tasks, ExportFormat::Dot) {
        Ok(content) => {
            // Determine export path
            let export_path = model.data_path.as_ref().map_or_else(
                || std::path::PathBuf::from("task_chains.dot"),
                |p| p.with_extension("dot"),
            );

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.alerts.status_message = Some(format!(
                        "Exported task chains to {} (use Graphviz to render)",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.alerts.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.alerts.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_chains_mermaid(model: &mut Model) {
    use crate::storage::{export_chains_to_string, ExportFormat};

    match export_chains_to_string(&model.tasks, ExportFormat::Mermaid) {
        Ok(content) => {
            // Determine export path
            let export_path = model.data_path.as_ref().map_or_else(
                || std::path::PathBuf::from("task_chains.md"),
                |p| p.with_extension("md"),
            );

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.alerts.status_message = Some(format!(
                        "Exported task chains to {} (Mermaid diagram)",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.alerts.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.alerts.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_report_markdown(model: &mut Model) {
    use crate::app::analytics::AnalyticsEngine;
    use crate::domain::analytics::ReportConfig;
    use crate::storage::export_report_to_markdown_string;

    let config = ReportConfig::last_n_days(30);
    let engine = AnalyticsEngine::new(model);
    let report = engine.generate_report(&config);

    match export_report_to_markdown_string(&report) {
        Ok(content) => {
            let export_path = model.data_path.as_ref().map_or_else(
                || std::path::PathBuf::from("taskflow_report.md"),
                |p| p.with_extension("report.md"),
            );

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.alerts.status_message = Some(format!(
                        "Exported analytics report to {}",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.alerts.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.alerts.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_report_html(model: &mut Model) {
    use crate::app::analytics::AnalyticsEngine;
    use crate::domain::analytics::ReportConfig;
    use crate::storage::export_report_to_html_string;

    let config = ReportConfig::last_n_days(30);
    let engine = AnalyticsEngine::new(model);
    let report = engine.generate_report(&config);

    match export_report_to_html_string(&report) {
        Ok(content) => {
            let export_path = model.data_path.as_ref().map_or_else(
                || std::path::PathBuf::from("taskflow_report.html"),
                |p| p.with_extension("report.html"),
            );

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.alerts.status_message = Some(format!(
                        "Exported analytics report to {}",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.alerts.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.alerts.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_start_import(model: &mut Model, format: crate::storage::ImportFormat) {
    model.input.mode = InputMode::Editing;
    model.input.target = InputTarget::ImportFilePath(format);
    model.input.buffer.clear();
    model.input.cursor = 0;
}

pub fn handle_execute_import(model: &mut Model) {
    use crate::storage::{
        apply_merge_strategy, import_from_csv, import_from_ics, ImportFormat, ImportOptions,
        MergeStrategy,
    };
    use std::fs::File;
    use std::io::BufReader;

    let format = match &model.input.target {
        InputTarget::ImportFilePath(fmt) => *fmt,
        _ => return,
    };

    let file_path = model.input.buffer.trim();
    if file_path.is_empty() {
        model.alerts.status_message = Some("No file path provided".to_string());
        model.input.mode = InputMode::Normal;
        model.input.target = InputTarget::Task;
        return;
    }

    // Open the file
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            model.alerts.status_message = Some(format!("Failed to open file: {e}"));
            model.input.mode = InputMode::Normal;
            model.input.target = InputTarget::Task;
            return;
        }
    };

    let reader = BufReader::new(file);
    let options = ImportOptions {
        merge_strategy: MergeStrategy::Skip,
        validate: true,
        dry_run: false,
    };

    // Parse the file
    let mut result = match format {
        ImportFormat::Csv => match import_from_csv(reader, &options) {
            Ok(r) => r,
            Err(e) => {
                model.alerts.status_message = Some(format!("Import failed: {e}"));
                model.input.mode = InputMode::Normal;
                model.input.target = InputTarget::Task;
                return;
            }
        },
        ImportFormat::Ics => match import_from_ics(reader, &options) {
            Ok(r) => r,
            Err(e) => {
                model.alerts.status_message = Some(format!("Import failed: {e}"));
                model.input.mode = InputMode::Normal;
                model.input.target = InputTarget::Task;
                return;
            }
        },
    };

    // Apply duplicate detection
    apply_merge_strategy(&mut result, &model.tasks, options.merge_strategy);

    // Reset input mode
    model.input.mode = InputMode::Normal;
    model.input.target = InputTarget::Task;
    model.input.buffer.clear();

    // If there are tasks to import, show preview
    if result.imported.is_empty() && result.skipped.is_empty() && result.errors.is_empty() {
        model.alerts.status_message = Some("No tasks found in file".to_string());
        return;
    }

    // Store the result and show preview
    let import_count = result.imported.len();
    let skip_count = result.skipped.len();
    let error_count = result.errors.len();

    model.pending_import = Some(result);
    model.show_import_preview = true;
    model.alerts.status_message = Some(format!(
        "Preview: {import_count} to import, {skip_count} skipped, {error_count} errors. Press Enter to confirm, Esc to cancel."
    ));
}

fn handle_confirm_import(model: &mut Model) {
    if let Some(result) = model.pending_import.take() {
        let count = result.imported.len();

        // Add all imported tasks
        for task in result.imported {
            model.sync_task(&task);
            model.tasks.insert(task.id, task);
        }

        model.dirty = true;
        model.show_import_preview = false;
        model.refresh_visible_tasks();
        model.alerts.status_message = Some(format!("Imported {count} tasks"));
    }
}

fn handle_cancel_import(model: &mut Model) {
    model.pending_import = None;
    model.show_import_preview = false;
    model.alerts.status_message = Some("Import cancelled".to_string());
}
