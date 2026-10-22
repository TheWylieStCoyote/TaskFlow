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
        frame.render_widget(
            HelpPopup::new(&model.keybindings, theme, model.help_scroll),
            popup_area,
        );
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

#[cfg(test)]
mod tests {
    use super::render_popups;
    use crate::app::Model;
    use crate::config::Theme;
    use crate::domain::{today, Habit, SavedFilter, Task, TimeEntry, WorkLogEntry};
    use crate::storage::{ImportFormat, ImportResult};
    use crate::ui::components::{InputMode, InputTarget};
    use crate::ui::WorkLogMode;
    use chrono::Duration;
    use ratatui::{backend::TestBackend, layout::Rect, Terminal};

    fn render(model: &Model) -> String {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 40);
                render_popups(model, frame, area, &theme);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                content.push(
                    buffer
                        .cell((x, y))
                        .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
                );
            }
            content.push('\n');
        }
        content
    }

    fn model_with_task() -> (Model, crate::domain::TaskId) {
        let mut model = Model::new();
        let task = Task::new("Popup task");
        let id = task.id;
        model.tasks.insert(id, task);
        model.refresh_visible_tasks();
        model.selected_index = 0;
        (model, id)
    }

    #[test]
    fn test_no_popups_renders_blank_frame() {
        let model = Model::new();
        let content = render(&model);
        assert!(content.trim().is_empty());
    }

    #[test]
    fn test_help_popup() {
        let mut model = Model::new();
        model.show_help = true;
        let content = render(&model);
        assert!(content.contains("Help (j/k scroll"));
    }

    #[test]
    fn test_input_dialog_titles_follow_target() {
        let (mut model, id) = model_with_task();
        model.input.mode = InputMode::Editing;
        model.input.buffer = "typed text".to_string();
        // Cursor at the end so the cursor glyph does not replace a character
        model.input.cursor = model.input.buffer.len();

        let cases: Vec<(InputTarget, &str)> = vec![
            (InputTarget::Task, "New Task"),
            (InputTarget::Subtask(id), "New Subtask"),
            (InputTarget::EditTask(id), "Edit Task"),
            (InputTarget::EditDueDate(id), "Due Date (YYYY-MM-DD"),
            (InputTarget::EditScheduledDate(id), "Scheduled Date"),
            (InputTarget::EditScheduledTime(id), "Time Block"),
            (InputTarget::EditTags(id), "Tags (comma-separated)"),
            (
                InputTarget::EditDescription(id),
                "Description (empty to clear)",
            ),
            (InputTarget::EditEstimate(id), "Time Estimate"),
            (InputTarget::Project, "New Project"),
            (
                InputTarget::EditProject(crate::domain::ProjectId::new()),
                "Rename Project",
            ),
            (InputTarget::Search, "Search (Ctrl+L to clear)"),
            (InputTarget::MoveToProject(id), "Move to Project"),
            (InputTarget::FilterByTag, "Filter by Tag"),
            (InputTarget::BulkMoveToProject, "Move Selected to Project"),
            (InputTarget::BulkSetStatus, "Set Status for Selected"),
            (InputTarget::BulkSetPriority, "Set Priority for Selected"),
            (InputTarget::BulkAddTags, "Tags for Selected"),
            (InputTarget::BulkSetDueDate, "Due Date for Selected"),
            (InputTarget::BulkSnooze, "Snooze Selected Until"),
            (InputTarget::EditDependencies(id), "Blocked by"),
            (InputTarget::EditRecurrence(id), "Recurrence (d[N]"),
            (InputTarget::LinkTask(id), "Link to next task"),
            (
                InputTarget::ImportFilePath(ImportFormat::Csv),
                "Import CSV: Enter file path",
            ),
            (
                InputTarget::ImportFilePath(ImportFormat::Ics),
                "Import ICS: Enter file path",
            ),
            (InputTarget::SavedFilterName, "Save Filter As"),
            (InputTarget::SnoozeTask(id), "Snooze Until (YYYY-MM-DD)"),
            (InputTarget::NewHabit, "New Habit"),
            (
                InputTarget::EditHabit(crate::domain::HabitId::new()),
                "Edit Habit",
            ),
            (InputTarget::GoalName, "New Goal"),
            (
                InputTarget::EditGoalName(crate::domain::GoalId::new()),
                "Edit Goal",
            ),
            (
                InputTarget::KeyResultName(crate::domain::GoalId::new()),
                "New Key Result",
            ),
        ];

        for (target, title) in cases {
            model.input.target = target.clone();
            let content = render(&model);
            assert!(content.contains(title), "{target:?} should show {title:?}");
            assert!(
                content.contains("typed text"),
                "{target:?} shows the buffer"
            );
        }
    }

    #[test]
    fn test_quick_capture_uses_dedicated_dialog() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::QuickCapture;
        model.input.buffer = "capture me".to_string();
        model.input.cursor = model.input.buffer.len();
        let content = render(&model);
        assert!(content.contains("Quick Capture"));
        assert!(content.contains("capture me▌"));
        assert!(content.contains("#tag"), "syntax hints are shown");
    }

    #[test]
    fn test_delete_confirmation_names_selected_task() {
        let (mut model, _) = model_with_task();
        model.show_confirm_delete = true;
        let content = render(&model);
        assert!(content.contains("Delete Task"));
        assert!(content.contains("Delete \"Popup task\"?"));
    }

    #[test]
    fn test_delete_confirmation_without_task_uses_fallback() {
        let mut model = Model::new();
        model.show_confirm_delete = true;
        let content = render(&model);
        assert!(content.contains("Delete \"this task\"?"));
    }

    #[test]
    fn test_generate_config_prompt() {
        let mut model = Model::new();
        model.show_generate_config_prompt = true;
        let content = render(&model);
        assert!(content.contains("Generate Config"));
        assert!(content.contains("No config file found. Generate default config?"));
    }

    #[test]
    fn test_import_preview_counts() {
        let mut model = Model::new();
        model.import.show_preview = true;
        model.import.pending = Some(ImportResult {
            imported: vec![Task::new("A"), Task::new("B")],
            imported_events: vec![],
            skipped: vec![],
            errors: vec![],
        });
        let content = render(&model);
        assert!(content.contains("Import Preview"));
        assert!(content.contains("Tasks to import: 2"));
        assert!(content.contains("Skipped: 0"));
        assert!(content.contains("Errors: 0"));
    }

    #[test]
    fn test_import_preview_without_pending_result_is_hidden() {
        let mut model = Model::new();
        model.import.show_preview = true;
        model.import.pending = None;
        let content = render(&model);
        assert!(!content.contains("Import Preview"));
    }

    #[test]
    fn test_template_picker() {
        let mut model = Model::new();
        model.template_picker.visible = true;
        let content = render(&model);
        assert!(content.contains("New Task from Template"));
    }

    #[test]
    fn test_saved_filter_picker_lists_filters() {
        let mut model = Model::new();
        let filter = SavedFilter::new(
            "Weekend chores",
            model.filtering.filter.clone(),
            model.filtering.sort.clone(),
        );
        model.active_saved_filter = Some(filter.id.clone());
        model.saved_filters.insert(filter.id.clone(), filter);
        model.saved_filter_picker.visible = true;
        let content = render(&model);
        assert!(content.contains("Saved Filters"));
        assert!(content.contains("Weekend chores"));
    }

    #[test]
    fn test_keybindings_editor() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        let content = render(&model);
        assert!(content.contains("Keybindings"));
    }

    #[test]
    fn test_time_log_editor_requires_selected_task() {
        let (mut model, id) = model_with_task();
        model.time_log.visible = true;
        let mut entry = TimeEntry::start(id);
        entry.stop();
        model.time_entries.insert(entry.id, entry);
        model.refresh_visible_tasks();
        assert!(render(&model).contains("Time Log"));

        let mut empty = Model::new();
        empty.time_log.visible = true;
        assert!(!render(&empty).contains("Time Log"));
    }

    #[test]
    fn test_work_log_editor_modes_and_search_filter() {
        let (mut model, id) = model_with_task();
        let keep = WorkLogEntry::new(id, "Keep this entry");
        let drop = WorkLogEntry::new(id, "Drop this one");
        model.work_logs.insert(keep.id, keep);
        model.work_logs.insert(drop.id, drop);
        model.refresh_visible_tasks();
        model.work_log_editor.visible = true;

        for (mode, title) in [
            (WorkLogMode::Browse, "Work Log (a=add"),
            (WorkLogMode::View, "View Entry"),
            (WorkLogMode::ConfirmDelete, "Confirm Delete"),
            (WorkLogMode::Add, "Add Entry"),
            (WorkLogMode::Edit, "Edit Entry"),
            (WorkLogMode::Search, "Search Work Log"),
        ] {
            model.work_log_editor.mode = mode;
            let content = render(&model);
            assert!(content.contains(title), "{mode:?} should show {title:?}");
        }

        model.work_log_editor.mode = WorkLogMode::Browse;
        model.work_log_editor.search_query = "keep".to_string();
        let content = render(&model);
        assert!(content.contains("Keep this entry"));
        assert!(!content.contains("Drop this one"));

        let mut empty = Model::new();
        empty.work_log_editor.visible = true;
        assert!(!render(&empty).contains("Work Log"));
    }

    #[test]
    fn test_description_editor() {
        let mut model = Model::new();
        model.description_editor.visible = true;
        model.description_editor.buffer = vec!["first line of text".to_string()];
        let content = render(&model);
        assert!(content.contains("Edit Description"));
        assert!(content.contains("first line of text"));
    }

    #[test]
    fn test_overdue_alert_lists_tasks_unless_config_prompt_shown() {
        let mut model = Model::new();
        let mut late = Task::new("Very late task");
        late.due_date = Some(today() - Duration::days(2));
        model.tasks.insert(late.id, late);
        model.refresh_visible_tasks();
        model.alerts.show_overdue = true;

        let content = render(&model);
        assert!(content.contains("Overdue Tasks"));
        assert!(content.contains("Very late task"));

        model.show_generate_config_prompt = true;
        let content = render(&model);
        assert!(!content.contains("Overdue Tasks"));
        assert!(content.contains("Generate Config"));
    }

    #[test]
    fn test_storage_error_alert_requires_message() {
        let mut model = Model::new();
        model.alerts.show_storage_error = true;
        model.alerts.storage_error = Some("could not read tasks.json".to_string());
        let content = render(&model);
        assert!(content.contains("Storage Error"));
        assert!(content.contains("could not read tasks.json"));

        model.alerts.storage_error = None;
        assert!(!render(&model).contains("Storage Error"));
    }

    #[test]
    fn test_review_overlays() {
        let mut model = Model::new();
        model.daily_review.visible = true;
        assert!(render(&model).contains("Daily Review (1/5)"));
        model.daily_review.visible = false;

        model.weekly_review.visible = true;
        assert!(render(&model).contains("Weekly Review (1/6)"));
        model.weekly_review.visible = false;

        model.evening_review.visible = true;
        assert!(render(&model).contains("Evening Review (1/6)"));
    }

    #[test]
    fn test_habit_analytics_popup() {
        let mut model = Model::new();
        let habit = Habit::new("Morning run");
        model.habits.insert(habit.id, habit);
        model.refresh_visible_habits();
        model.habit_view.show_analytics = true;
        let content = render(&model);
        assert!(content.contains("Morning run - Analytics"));
    }

    #[test]
    fn test_task_detail_modal() {
        let (mut model, _) = model_with_task();
        model.task_detail.visible = true;
        let content = render(&model);
        assert!(content.contains("Task Details"));
        assert!(content.contains("Popup task"));
    }

    #[test]
    fn test_command_palette_shows_query() {
        let mut model = Model::new();
        model.command_palette.visible = true;
        model.command_palette.query = "undo".to_string();
        model.command_palette.cursor = 4;
        let content = render(&model);
        assert!(content.contains("Command Palette"));
        assert!(content.contains("undo"));
    }
}
