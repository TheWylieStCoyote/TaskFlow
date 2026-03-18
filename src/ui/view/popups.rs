//! Popup and modal rendering for the view module.
//!
//! This module handles rendering of all popup dialogs, modals, and overlays
//! including help, input dialogs, confirmation dialogs, editors, and alerts.

use ratatui::{layout::Rect, Frame};

use crate::app::Model;
use crate::config::Theme;

use crate::ui::components::{
    centered_rect, centered_rect_fixed_height, CommandPalette, ConfirmDialog, DailyReview,
    DescriptionEditor, EveningReview, HabitAnalyticsPopup, HelpPopup, InputDialog, InputMode,
    InputTarget, KeybindingsEditor, OverdueAlert, QuickCaptureDialog, SavedFilterPicker,
    StorageErrorAlert, TaskDetail, TemplatePicker, TimeLogEditor, WeeklyReview, WorkLogEditor,
};

/// Renders all popup dialogs and overlays based on model state
pub(super) fn render_popups(model: &Model, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    // Render help popup
    if model.show_help {
        let popup_area = centered_rect(50, 70, area);
        frame.render_widget(HelpPopup::new(&model.keybindings, theme), popup_area);
    }

    // Render input dialog if in editing mode
    if model.input.mode == InputMode::Editing {
        // Height: 3 rows (top border, text line, bottom border)
        let input_area = centered_rect_fixed_height(60, 3, area);
        let title = match &model.input.target {
            InputTarget::Task => "New Task",
            InputTarget::Subtask(_) => "New Subtask",
            InputTarget::EditTask(_) => "Edit Task",
            InputTarget::EditDueDate(_) => "Due Date (YYYY-MM-DD, empty to clear)",
            InputTarget::EditScheduledDate(_) => "Scheduled Date (YYYY-MM-DD, empty to clear)",
            InputTarget::EditScheduledTime(_) => "Time Block (e.g., 9:00-11:00, 9am-11am)",
            InputTarget::EditTags(_) => "Tags (comma-separated)",
            InputTarget::EditDescription(_) => "Description (empty to clear)",
            InputTarget::EditEstimate(_) => "Time Estimate (e.g., 30m, 1h, 1h30m)",
            InputTarget::Project => "New Project",
            InputTarget::EditProject(_) => "Rename Project",
            InputTarget::Search => "Search (Ctrl+L to clear)",
            InputTarget::MoveToProject(_) => "Move to Project (enter number)",
            InputTarget::FilterByTag => "Filter by Tag (comma-separated, Ctrl+T to clear)",
            InputTarget::BulkMoveToProject => "Move Selected to Project (enter number)",
            InputTarget::BulkSetStatus => "Set Status for Selected (enter number)",
            InputTarget::BulkSetPriority => {
                "Set Priority for Selected (0=None 1=Low 2=Med 3=High 4=Urgent)"
            }
            InputTarget::BulkAddTags => "Tags for Selected (comma-separated, prefix - to remove)",
            InputTarget::BulkSetDueDate => "Due Date for Selected (YYYY-MM-DD, empty to clear)",
            InputTarget::BulkSnooze => "Snooze Selected Until (YYYY-MM-DD, empty to clear)",
            InputTarget::EditDependencies(_) => "Blocked by (task numbers, comma-separated)",
            InputTarget::EditRecurrence(_) => {
                "Recurrence (d[N], w[N], m[day], y, 0=none; end=YYYY-MM-DD, max=N)"
            }
            InputTarget::LinkTask(_) => "Link to next task (task number or title)",
            InputTarget::ImportFilePath(format) => match format {
                crate::storage::ImportFormat::Csv => "Import CSV: Enter file path",
                crate::storage::ImportFormat::Ics => "Import ICS: Enter file path",
            },
            InputTarget::SavedFilterName => "Save Filter As (enter name)",
            InputTarget::SnoozeTask(_) => "Snooze Until (YYYY-MM-DD)",
            InputTarget::NewHabit => "New Habit",
            InputTarget::EditHabit(_) => "Edit Habit",
            InputTarget::QuickCapture => "Quick Capture",
            InputTarget::GoalName => "New Goal",
            InputTarget::EditGoalName(_) => "Edit Goal",
            InputTarget::KeyResultName(_) => "New Key Result",
        };

        // QuickCapture gets a special larger dialog with syntax hints
        if model.input.target == InputTarget::QuickCapture {
            // Height: 9 rows (input + hints area)
            let quick_area = centered_rect_fixed_height(70, 9, area);
            frame.render_widget(
                QuickCaptureDialog::new(&model.input.buffer, model.input.cursor, theme),
                quick_area,
            );
        } else {
            frame.render_widget(
                InputDialog::new(title, &model.input.buffer, model.input.cursor, theme),
                input_area,
            );
        }
    }

    // Render delete confirmation dialog
    if model.show_confirm_delete {
        // Height: 5 rows (border, message, blank, y/n prompt, border)
        let confirm_area = centered_rect_fixed_height(50, 5, area);
        let task_name = model
            .selected_task()
            .map_or("this task", |t| t.title.as_str());
        frame.render_widget(
            ConfirmDialog::new("Delete Task", &format!("Delete \"{task_name}\"?"), theme),
            confirm_area,
        );
    }

    // Render config file generation prompt
    if model.show_generate_config_prompt {
        let confirm_area = centered_rect_fixed_height(55, 5, area);
        frame.render_widget(
            ConfirmDialog::new(
                "Generate Config",
                "No config file found. Generate default config?",
                theme,
            ),
            confirm_area,
        );
    }

    // Render import preview dialog
    if model.import.show_preview {
        if let Some(ref result) = model.import.pending {
            let confirm_area = centered_rect_fixed_height(60, 7, area);
            let message = format!(
                "Tasks to import: {}\nSkipped: {}\nErrors: {}",
                result.imported.len(),
                result.skipped.len(),
                result.errors.len()
            );
            frame.render_widget(
                ConfirmDialog::new("Import Preview", &message, theme),
                confirm_area,
            );
        }
    }

    // Render template picker
    if model.template_picker.visible {
        // Height depends on number of templates, min 4, max 15
        let height = (model.template_manager.len() as u16 + 2).clamp(4, 15);
        let picker_area = centered_rect_fixed_height(60, height, area);
        frame.render_widget(
            TemplatePicker::new(
                &model.template_manager,
                model.template_picker.selected,
                theme,
            ),
            picker_area,
        );
    }

    // Render saved filter picker
    if model.saved_filter_picker.visible {
        // Get sorted filter list for display
        let mut filter_list: Vec<_> = model.saved_filters.values().collect();
        filter_list.sort_by(|a, b| a.name.cmp(&b.name));

        // Get active filter name for highlighting
        let active_name = model
            .active_saved_filter
            .as_ref()
            .and_then(|id| model.saved_filters.get(id))
            .map(|f| f.name.as_str());

        // Height depends on number of filters, min 4, max 15
        let height = (filter_list.len() as u16 + 2).clamp(4, 15);
        let picker_area = centered_rect_fixed_height(60, height, area);
        frame.render_widget(
            SavedFilterPicker::new(
                filter_list,
                model.saved_filter_picker.selected,
                active_name,
                theme,
            ),
            picker_area,
        );
    }

    // Render keybindings editor
    if model.keybindings_editor.visible {
        // Height depends on number of bindings, min 10, max 30
        let bindings_count = model.keybindings.sorted_bindings().len() as u16;
        let height = (bindings_count + 2).clamp(10, 30);
        let editor_area = centered_rect_fixed_height(70, height, area);
        frame.render_widget(
            KeybindingsEditor::new(
                &model.keybindings,
                model.keybindings_editor.selected,
                model.keybindings_editor.capturing,
                theme,
            ),
            editor_area,
        );
    }

    // Render time log editor
    if model.time_log.visible {
        if let Some(task_id) = model.visible_tasks.get(model.selected_index) {
            let entries = model.time_entries_for_task(task_id);
            // Height: min 5, max 15 depending on entries
            let height = (entries.len() as u16 + 4).clamp(5, 15);
            let editor_area = centered_rect_fixed_height(70, height, area);
            frame.render_widget(
                TimeLogEditor::new(
                    entries,
                    model.time_log.selected,
                    model.time_log.mode,
                    &model.time_log.buffer,
                    theme,
                ),
                editor_area,
            );
        }
    }

    // Render work log editor
    if model.work_log_editor.visible {
        if let Some(task_id) = model.visible_tasks.get(model.selected_index) {
            let all_entries = model.work_logs_for_task(task_id);

            // Filter entries based on search query
            let entries: Vec<_> = if model.work_log_editor.search_query.is_empty() {
                all_entries
            } else {
                let query = model.work_log_editor.search_query.to_lowercase();
                all_entries
                    .into_iter()
                    .filter(|e| e.content.to_lowercase().contains(&query))
                    .collect()
            };

            // Height: min 6, max 20 depending on entries and mode
            let height = match model.work_log_editor.mode {
                crate::ui::WorkLogMode::Browse => (entries.len() as u16 + 4).clamp(6, 15),
                crate::ui::WorkLogMode::View | crate::ui::WorkLogMode::ConfirmDelete => 15,
                crate::ui::WorkLogMode::Add | crate::ui::WorkLogMode::Edit => {
                    (model.work_log_editor.buffer.len() as u16 + 4).clamp(10, 20)
                }
                crate::ui::WorkLogMode::Search => 15,
            };
            let editor_area = centered_rect_fixed_height(70, height, area);
            frame.render_widget(
                WorkLogEditor::new(
                    entries,
                    model.work_log_editor.selected,
                    model.work_log_editor.mode,
                    &model.work_log_editor.buffer,
                    model.work_log_editor.cursor_line,
                    model.work_log_editor.cursor_col,
                    &model.work_log_editor.search_query,
                    theme,
                ),
                editor_area,
            );
        }
    }

    // Render description editor (multi-line)
    if model.description_editor.visible {
        // Height: min 10, max 20 depending on buffer lines
        let height = (model.description_editor.buffer.len() as u16 + 4).clamp(10, 20);
        let editor_area = centered_rect_fixed_height(70, height, area);
        frame.render_widget(
            DescriptionEditor::new(
                &model.description_editor.buffer,
                model.description_editor.cursor_line,
                model.description_editor.cursor_col,
                theme,
            ),
            editor_area,
        );
    }

    // Render overdue alert popup (shown at startup if there are overdue tasks)
    // Don't show if config prompt is visible (config prompt takes priority)
    if model.alerts.show_overdue && !model.show_generate_config_prompt {
        let (count, overdue_tasks) = model.overdue_summary();
        let task_titles: Vec<String> = overdue_tasks.iter().map(|t| t.title.clone()).collect();
        // Height: 4 + min(5, count) + 2 for header/footer
        let height = (6 + count.min(5)) as u16;
        let alert_area = centered_rect_fixed_height(50, height.max(7), area);
        frame.render_widget(OverdueAlert::new(count, task_titles, theme), alert_area);
    }

    // Render storage error alert popup (shown at startup if data couldn't be loaded)
    if model.alerts.show_storage_error {
        if let Some(ref error) = model.alerts.storage_error {
            let alert_area = centered_rect_fixed_height(60, 10, area);
            frame.render_widget(StorageErrorAlert::new(error, theme), alert_area);
        }
    }

    // Render daily review mode (full screen overlay)
    if model.daily_review.visible {
        // Use centered area for the review dialog
        let review_area = centered_rect(70, 70, area);
        frame.render_widget(
            DailyReview::new(
                model,
                theme,
                model.daily_review.phase,
                model.daily_review.selected,
            ),
            review_area,
        );
    }

    // Render weekly review mode (full screen overlay)
    if model.weekly_review.visible {
        // Use centered area for the review dialog
        let review_area = centered_rect(75, 75, area);
        frame.render_widget(
            WeeklyReview::new(
                model,
                theme,
                model.weekly_review.phase,
                model.weekly_review.selected,
            ),
            review_area,
        );
    }

    // Render evening review mode (full screen overlay)
    if model.evening_review.visible {
        // Use centered area for the review dialog
        let review_area = centered_rect(70, 70, area);
        frame.render_widget(EveningReview::new(model, theme), review_area);
    }

    // Render habit analytics popup
    if model.habit_view.show_analytics {
        let popup_area = centered_rect_fixed_height(50, 12, area);
        frame.render_widget(HabitAnalyticsPopup::new(model, theme), popup_area);
    }

    // Render task detail modal
    if model.task_detail.visible {
        let popup_area = centered_rect(80, 80, area);
        frame.render_widget(
            TaskDetail::new(model, theme, model.task_detail.scroll),
            popup_area,
        );
    }

    // Render command palette (high priority - renders over other popups)
    if model.command_palette.visible {
        let palette_area = centered_rect_fixed_height(60, 15, area);
        frame.render_widget(
            CommandPalette::new(
                &model.command_palette.query,
                model.command_palette.cursor,
                model.command_palette.selected,
                &model.keybindings,
                theme,
            ),
            palette_area,
        );
    }
}
