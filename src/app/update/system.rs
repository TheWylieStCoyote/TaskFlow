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
            // Stop any running timer before quitting
            model.stop_time_tracking();
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
            // Clear status message after a tick
            model.status_message = None;
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
    }
}

fn handle_undo(model: &mut Model) {
    if let Some(action) = model.undo_stack.pop_for_undo() {
        match action {
            UndoAction::TaskCreated(task) => {
                // Undo create by deleting the task
                model.delete_task_from_storage(&task.id);
                model.tasks.remove(&task.id);
            }
            UndoAction::TaskDeleted(task) => {
                // Undo delete by restoring the task
                model.sync_task(&task);
                model.tasks.insert(task.id.clone(), *task);
            }
            UndoAction::TaskModified { before, after: _ } => {
                // Undo modify by restoring previous state
                model.sync_task(&before);
                model.tasks.insert(before.id.clone(), *before);
            }
            UndoAction::ProjectCreated(project) => {
                // Undo project create by removing it
                model.projects.remove(&project.id);
                model.dirty = true;
            }
            UndoAction::ProjectDeleted(project) => {
                // Undo project delete by restoring it
                model.sync_project(&project);
                model.projects.insert(project.id.clone(), *project);
            }
            UndoAction::ProjectModified { before, after: _ } => {
                // Undo modify by restoring previous state
                model.sync_project(&before);
                model.projects.insert(before.id.clone(), *before);
            }
        }
        model.refresh_visible_tasks();
    }
}

fn handle_redo(model: &mut Model) {
    if let Some(action) = model.undo_stack.pop_for_redo() {
        match action {
            UndoAction::TaskCreated(task) => {
                // Redo create by restoring the task
                model.sync_task(&task);
                model.tasks.insert(task.id.clone(), *task);
            }
            UndoAction::TaskDeleted(task) => {
                // Redo delete by removing the task
                model.delete_task_from_storage(&task.id);
                model.tasks.remove(&task.id);
            }
            UndoAction::TaskModified { before, after: _ } => {
                // Redo modify: the redo stack holds the inverse, so "before" is the state we want
                model.sync_task(&before);
                model.tasks.insert(before.id.clone(), *before);
            }
            UndoAction::ProjectCreated(project) => {
                // Redo project create by restoring it
                model.sync_project(&project);
                model.projects.insert(project.id.clone(), *project);
            }
            UndoAction::ProjectDeleted(project) => {
                // Redo project delete by removing it
                model.projects.remove(&project.id);
                model.dirty = true;
            }
            UndoAction::ProjectModified { before, after: _ } => {
                // Redo modify: the redo stack holds the inverse, so "before" is the state we want
                model.sync_project(&before);
                model.projects.insert(before.id.clone(), *before);
            }
        }
        model.refresh_visible_tasks();
    }
}

fn handle_export_csv(model: &mut Model) {
    use crate::storage::{export_to_string, ExportFormat};

    let tasks = model.tasks_for_export();
    match export_to_string(&tasks, ExportFormat::Csv) {
        Ok(content) => {
            // Determine export path
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("csv"))
                .unwrap_or_else(|| std::path::PathBuf::from("tasks.csv"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported {} tasks to {}",
                        tasks.len(),
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_ics(model: &mut Model) {
    use crate::storage::{export_to_string, ExportFormat};

    let tasks = model.tasks_for_export();
    match export_to_string(&tasks, ExportFormat::Ics) {
        Ok(content) => {
            // Determine export path
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("ics"))
                .unwrap_or_else(|| std::path::PathBuf::from("tasks.ics"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported {} tasks to {}",
                        tasks.len(),
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_chains_dot(model: &mut Model) {
    use crate::storage::{export_chains_to_string, ExportFormat};

    match export_chains_to_string(&model.tasks, ExportFormat::Dot) {
        Ok(content) => {
            // Determine export path
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("dot"))
                .unwrap_or_else(|| std::path::PathBuf::from("task_chains.dot"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported task chains to {} (use Graphviz to render)",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_export_chains_mermaid(model: &mut Model) {
    use crate::storage::{export_chains_to_string, ExportFormat};

    match export_chains_to_string(&model.tasks, ExportFormat::Mermaid) {
        Ok(content) => {
            // Determine export path
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("md"))
                .unwrap_or_else(|| std::path::PathBuf::from("task_chains.md"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported task chains to {} (Mermaid diagram)",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
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
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("report.md"))
                .unwrap_or_else(|| std::path::PathBuf::from("taskflow_report.md"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported analytics report to {}",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
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
            let export_path = model
                .data_path
                .as_ref()
                .map(|p| p.with_extension("report.html"))
                .unwrap_or_else(|| std::path::PathBuf::from("taskflow_report.html"));

            match std::fs::write(&export_path, content) {
                Ok(()) => {
                    model.status_message = Some(format!(
                        "Exported analytics report to {}",
                        export_path.display()
                    ));
                }
                Err(e) => {
                    model.status_message = Some(format!("Export failed: {e}"));
                }
            }
        }
        Err(e) => {
            model.status_message = Some(format!("Export failed: {e}"));
        }
    }
}

fn handle_start_import(model: &mut Model, format: crate::storage::ImportFormat) {
    model.input_mode = InputMode::Editing;
    model.input_target = InputTarget::ImportFilePath(format);
    model.input_buffer.clear();
    model.cursor_position = 0;
}

pub fn handle_execute_import(model: &mut Model) {
    use crate::storage::{
        apply_merge_strategy, import_from_csv, import_from_ics, ImportFormat, ImportOptions,
        MergeStrategy,
    };
    use std::fs::File;
    use std::io::BufReader;

    let format = match &model.input_target {
        InputTarget::ImportFilePath(fmt) => *fmt,
        _ => return,
    };

    let file_path = model.input_buffer.trim();
    if file_path.is_empty() {
        model.status_message = Some("No file path provided".to_string());
        model.input_mode = InputMode::Normal;
        model.input_target = InputTarget::Task;
        return;
    }

    // Open the file
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            model.status_message = Some(format!("Failed to open file: {e}"));
            model.input_mode = InputMode::Normal;
            model.input_target = InputTarget::Task;
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
                model.status_message = Some(format!("Import failed: {e}"));
                model.input_mode = InputMode::Normal;
                model.input_target = InputTarget::Task;
                return;
            }
        },
        ImportFormat::Ics => match import_from_ics(reader, &options) {
            Ok(r) => r,
            Err(e) => {
                model.status_message = Some(format!("Import failed: {e}"));
                model.input_mode = InputMode::Normal;
                model.input_target = InputTarget::Task;
                return;
            }
        },
    };

    // Apply duplicate detection
    apply_merge_strategy(&mut result, &model.tasks, options.merge_strategy);

    // Reset input mode
    model.input_mode = InputMode::Normal;
    model.input_target = InputTarget::Task;
    model.input_buffer.clear();

    // If there are tasks to import, show preview
    if result.imported.is_empty() && result.skipped.is_empty() && result.errors.is_empty() {
        model.status_message = Some("No tasks found in file".to_string());
        return;
    }

    // Store the result and show preview
    let import_count = result.imported.len();
    let skip_count = result.skipped.len();
    let error_count = result.errors.len();

    model.pending_import = Some(result);
    model.show_import_preview = true;
    model.status_message = Some(format!(
        "Preview: {} to import, {} skipped, {} errors. Press Enter to confirm, Esc to cancel.",
        import_count, skip_count, error_count
    ));
}

fn handle_confirm_import(model: &mut Model) {
    if let Some(result) = model.pending_import.take() {
        let count = result.imported.len();

        // Add all imported tasks
        for task in result.imported {
            model.sync_task(&task);
            model.tasks.insert(task.id.clone(), task);
        }

        model.dirty = true;
        model.show_import_preview = false;
        model.refresh_visible_tasks();
        model.status_message = Some(format!("Imported {} tasks", count));
    }
}

fn handle_cancel_import(model: &mut Model) {
    model.pending_import = None;
    model.show_import_preview = false;
    model.status_message = Some("Import cancelled".to_string());
}
