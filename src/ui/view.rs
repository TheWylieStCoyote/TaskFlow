//! Main view rendering module.
//!
//! This module contains the primary [`view`] function that renders the entire
//! application UI based on the current model state. It composes the various
//! UI components (sidebar, task list, modals, etc.) into a cohesive layout.
//!
//! # Layout Structure
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              Header (title)             │
//! ├──────────┬──────────────────────────────┤
//! │          │                              │
//! │ Sidebar  │       Main Content           │
//! │          │    (view-dependent)          │
//! │          │                              │
//! ├──────────┴──────────────────────────────┤
//! │              Footer (status)            │
//! └─────────────────────────────────────────┘
//! ```

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::Model;
use crate::config::Theme;

use crate::app::ViewId;

use super::components::{
    centered_rect, centered_rect_fixed_height, Burndown, Calendar, ConfirmDialog, DailyReview,
    Dashboard, DescriptionEditor, Eisenhower, FocusView, Forecast, HabitAnalyticsPopup, HabitsView,
    Heatmap, HelpPopup, InputDialog, InputMode, InputTarget, Kanban, KeybindingsEditor, Network,
    OverdueAlert, QuickCaptureDialog, ReportsView, SavedFilterPicker, Sidebar, StorageErrorAlert,
    TaskList, TemplatePicker, TimeLogEditor, Timeline, WeeklyPlanner, WeeklyReview, WorkLogEditor,
};

/// Main view function - renders the entire UI based on model state
pub fn view(model: &Model, frame: &mut Frame<'_>, theme: &Theme) {
    let area = frame.area();

    // Main layout: header, content, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Render header
    render_header(frame, chunks[0], theme);

    // Render main content
    render_content(model, frame, chunks[1], theme);

    // Render footer
    render_footer(model, frame, chunks[2], theme);

    // Render popups
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
            InputTarget::EditDependencies(_) => "Blocked by (task numbers, comma-separated)",
            InputTarget::EditRecurrence(_) => {
                "Recurrence (d=daily, w=weekly, m=monthly, y=yearly, 0=none)"
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
    if model.alerts.show_overdue {
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

    // Render habit analytics popup
    if model.habit_view.show_analytics {
        let popup_area = centered_rect_fixed_height(50, 12, area);
        frame.render_widget(HabitAnalyticsPopup::new(model, theme), popup_area);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " TaskFlow ",
            Style::default().fg(theme.colors.accent.to_color()),
        ),
        Span::raw("- Project Management TUI"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color())),
    );

    frame.render_widget(title, area);
}

fn render_content(model: &Model, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    // Clear layout cache at start of render
    model.layout_cache.clear();

    // Focus mode takes over the entire content area
    if model.focus_mode {
        let focus_view = FocusView::new(model, theme);
        frame.render_widget(focus_view, area);
        return;
    }

    if model.show_sidebar {
        // Split into sidebar and main content
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25), // Sidebar
                Constraint::Min(0),     // Main content
            ])
            .split(area);

        // Cache sidebar area for mouse hit-testing
        model.layout_cache.set_sidebar_area(chunks[0]);

        // Render sidebar
        frame.render_widget(Sidebar::new(model, theme), chunks[0]);

        // Cache main area and render content
        model.layout_cache.set_main_area(chunks[1]);
        render_main_content(model, frame, chunks[1], theme);
    } else {
        // No sidebar, full width content
        model.layout_cache.set_main_area(area);
        render_main_content(model, frame, area, theme);
    }
}

fn render_main_content(model: &Model, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    match model.current_view {
        ViewId::Calendar => {
            // Cache calendar area for mouse click detection
            // The calendar grid typically has a 2-row header (month title + weekday headers)
            model.layout_cache.set_calendar_area(area);
            let calendar = Calendar::new(model, theme);
            frame.render_widget(calendar, area);
        }
        ViewId::Dashboard => {
            let dashboard = Dashboard::new(model, theme);
            frame.render_widget(dashboard, area);
        }
        ViewId::Reports => {
            // Cache reports tabs area - inner area after border, 3 rows height for tabs
            // The reports view has a 1-row border, then 3 rows for tabs
            let inner_x = area.x + 1;
            let inner_y = area.y + 1;
            let inner_width = area.width.saturating_sub(2);
            let tabs_area = Rect::new(inner_x, inner_y, inner_width, 3);
            model.layout_cache.set_reports_tabs_area(tabs_area);

            // Cache individual tab positions for precise click detection
            // Tab labels: "Overview", "Velocity", "Tags", "Time", "Focus", "Insights", "Estimation"
            // Divider: " | " (3 chars)
            const TAB_WIDTHS: [u16; 7] = [8, 8, 4, 4, 5, 8, 10]; // Character widths
            const DIVIDER_WIDTH: u16 = 3;
            let mut x_pos = inner_x;
            for (i, &width) in TAB_WIDTHS.iter().enumerate() {
                let tab_rect = Rect::new(x_pos, inner_y, width, 3);
                model.layout_cache.set_reports_tab_rect(i, tab_rect);
                x_pos += width;
                if i < 6 {
                    x_pos += DIVIDER_WIDTH; // Add divider width except after last tab
                }
            }

            let reports = ReportsView::new(model, model.report_panel, theme);
            frame.render_widget(reports, area);
        }
        ViewId::Habits => {
            let habits = HabitsView::new(model, theme);
            frame.render_widget(habits, area);
        }
        ViewId::Kanban => {
            // Cache kanban column areas - divide into 4 equal columns
            let column_width = area.width / 4;
            for i in 0..4 {
                let col_area = Rect {
                    x: area.x + (i as u16 * column_width),
                    y: area.y,
                    width: column_width,
                    height: area.height,
                };
                model.layout_cache.set_kanban_column(i, col_area);
            }
            let kanban = Kanban::new(model, theme);
            frame.render_widget(kanban, area);
        }
        ViewId::Eisenhower => {
            // Cache eisenhower quadrant areas - 2x2 grid
            let half_width = area.width / 2;
            let half_height = area.height / 2;
            // Top-left (0), Top-right (1), Bottom-left (2), Bottom-right (3)
            model
                .layout_cache
                .set_eisenhower_quadrant(0, Rect::new(area.x, area.y, half_width, half_height));
            model.layout_cache.set_eisenhower_quadrant(
                1,
                Rect::new(area.x + half_width, area.y, half_width, half_height),
            );
            model.layout_cache.set_eisenhower_quadrant(
                2,
                Rect::new(area.x, area.y + half_height, half_width, half_height),
            );
            model.layout_cache.set_eisenhower_quadrant(
                3,
                Rect::new(
                    area.x + half_width,
                    area.y + half_height,
                    half_width,
                    half_height,
                ),
            );
            let eisenhower = Eisenhower::new(model, theme);
            frame.render_widget(eisenhower, area);
        }
        ViewId::WeeklyPlanner => {
            // Cache weekly planner day areas - 7 columns
            let day_width = area.width / 7;
            for i in 0..7 {
                let day_area = Rect {
                    x: area.x + (i as u16 * day_width),
                    y: area.y,
                    width: day_width,
                    height: area.height,
                };
                model.layout_cache.set_weekly_planner_day(i, day_area);
            }
            let planner = WeeklyPlanner::new(model, theme);
            frame.render_widget(planner, area);
        }
        ViewId::Timeline => {
            let timeline = Timeline::new(model, theme);
            frame.render_widget(timeline, area);
        }
        ViewId::Heatmap => {
            let heatmap = Heatmap::new(model, theme);
            frame.render_widget(heatmap, area);
        }
        ViewId::Forecast => {
            let forecast = Forecast::new(model, theme);
            frame.render_widget(forecast, area);
        }
        ViewId::Network => {
            let network = Network::new(model, theme, model.view_selection.network_task_index);
            frame.render_widget(network, area);
        }
        ViewId::Burndown => {
            let burndown = Burndown::new(model, theme);
            frame.render_widget(burndown, area);
        }
        _ => {
            // Cache task list area with header offset (border + title row = 2)
            // scroll_offset is typically 0 unless we implement virtual scrolling
            model.layout_cache.set_task_list_area(
                area,
                2,
                model
                    .selected_index
                    .saturating_sub(area.height as usize / 2),
            );
            let task_list = TaskList::new(model, theme);
            frame.render_widget(task_list, area);
        }
    }
}

fn render_footer(model: &Model, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    // Show error message if available (in red, higher priority than status)
    if let Some(ref msg) = model.alerts.error_message {
        let footer =
            Paragraph::new(msg.clone()).style(Style::default().fg(theme.colors.danger.to_color()));
        frame.render_widget(footer, area);
        return;
    }

    // Show status message if available, otherwise show normal footer
    if let Some(ref msg) = model.alerts.status_message {
        let footer =
            Paragraph::new(msg.clone()).style(Style::default().fg(theme.colors.accent.to_color()));
        frame.render_widget(footer, area);
        return;
    }

    if model.macro_state.is_recording() {
        let footer = Paragraph::new(" [REC] Recording macro... Press Ctrl+Q then 0-9 to save ")
            .style(Style::default().fg(theme.colors.danger.to_color()));
        frame.render_widget(footer, area);
        return;
    }

    // Use cached counts for performance
    let task_count = model.visible_tasks.len();
    let completed = model.footer_stats.completed_count;
    let overdue = model.footer_stats.overdue_count;
    let due_today = model.footer_stats.due_today_count;

    // Build footer with styled spans
    let mut spans = vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            format!("{task_count} tasks"),
            Style::default().fg(theme.colors.muted.to_color()),
        ),
        Span::styled(
            format!(" ({completed} completed)"),
            Style::default().fg(theme.colors.muted.to_color()),
        ),
    ];

    // Add multi-select mode indicator
    if model.multi_select.mode {
        let selected_count = model.multi_select.selected.len();
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        spans.push(Span::styled(
            format!("[MULTI-SELECT: {selected_count}]"),
            Style::default()
                .fg(theme.colors.accent.to_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add overdue indicator (red)
    if overdue > 0 {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        spans.push(Span::styled(
            format!("{overdue} overdue"),
            Style::default()
                .fg(theme.colors.danger.to_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add due today indicator (yellow)
    if due_today > 0 {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        spans.push(Span::styled(
            format!("{due_today} due today"),
            Style::default()
                .fg(theme.colors.warning.to_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add Pomodoro timer display if active
    if let Some(ref session) = model.pomodoro.session {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));

        // Show phase icon and timer
        let phase_icon = session.phase.icon();
        let time_display = session.formatted_remaining();
        let pause_indicator = if session.paused { " ⏸" } else { "" };

        // Color based on phase: work = accent, break = success
        let timer_color = if session.phase.is_break() {
            theme.colors.success.to_color()
        } else {
            theme.colors.accent.to_color()
        };

        spans.push(Span::styled(
            format!(
                "{} {} [{}/{}]{}",
                phase_icon,
                time_display,
                session.cycles_completed,
                session.session_goal,
                pause_indicator
            ),
            Style::default()
                .fg(timer_color)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Add view-specific navigation hints
    if let Some(hint) = get_view_hint(model) {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));
        spans.push(Span::styled(
            hint,
            Style::default().fg(theme.colors.accent.to_color()),
        ));
    }

    // Add show mode and help
    spans.push(Span::styled(
        " | ",
        Style::default().fg(theme.colors.muted.to_color()),
    ));
    spans.push(Span::styled(
        if model.filtering.show_completed {
            "showing all"
        } else {
            "hiding completed"
        },
        Style::default().fg(theme.colors.muted.to_color()),
    ));
    spans.push(Span::styled(
        " | ? help",
        Style::default().fg(theme.colors.muted.to_color()),
    ));

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}

/// Returns view-specific navigation hints for the footer
fn get_view_hint(model: &Model) -> Option<&'static str> {
    // Focus mode has its own hints
    if model.focus_mode {
        return Some("[/]: chain | t: timer | f: exit");
    }

    match model.current_view {
        ViewId::Kanban => Some("h/l: columns | j/k: tasks"),
        ViewId::Eisenhower => Some("h/l/j/k: quadrants"),
        ViewId::WeeklyPlanner => Some("h/l: days | j/k: tasks"),
        ViewId::Timeline => Some("h/l: scroll | </>: zoom | t: today"),
        ViewId::Network => Some("h/l/j/k: navigate"),
        ViewId::Habits => Some("n: new | Space: check-in"),
        ViewId::Calendar => Some("h/l: months | Enter: day tasks"),
        ViewId::Heatmap | ViewId::Forecast | ViewId::Burndown => None, // View-only
        _ => None, // Task list and others use default controls
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;
    use ratatui::{backend::TestBackend, Terminal};

    // =========================================================================
    // Helper functions
    // =========================================================================

    fn create_test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        Terminal::new(backend).unwrap()
    }

    fn buffer_content(terminal: &Terminal<TestBackend>) -> String {
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

    // =========================================================================
    // get_view_hint tests
    // =========================================================================

    #[test]
    fn test_get_view_hint_focus_mode() {
        let mut model = Model::new();
        model.focus_mode = true;

        let hint = get_view_hint(&model);
        assert_eq!(hint, Some("[/]: chain | t: timer | f: exit"));
    }

    #[test]
    fn test_get_view_hint_kanban() {
        let mut model = Model::new();
        model.current_view = ViewId::Kanban;

        let hint = get_view_hint(&model);
        assert_eq!(hint, Some("h/l: columns | j/k: tasks"));
    }

    #[test]
    fn test_get_view_hint_eisenhower() {
        let mut model = Model::new();
        model.current_view = ViewId::Eisenhower;

        let hint = get_view_hint(&model);
        assert_eq!(hint, Some("h/l/j/k: quadrants"));
    }

    #[test]
    fn test_get_view_hint_weekly_planner() {
        let mut model = Model::new();
        model.current_view = ViewId::WeeklyPlanner;

        let hint = get_view_hint(&model);
        assert_eq!(hint, Some("h/l: days | j/k: tasks"));
    }

    #[test]
    fn test_get_view_hint_timeline() {
        let mut model = Model::new();
        model.current_view = ViewId::Timeline;

        let hint = get_view_hint(&model);
        assert_eq!(hint, Some("h/l: scroll | </>: zoom | t: today"));
    }

    #[test]
    fn test_get_view_hint_network() {
        let mut model = Model::new();
        model.current_view = ViewId::Network;

        let hint = get_view_hint(&model);
        assert_eq!(hint, Some("h/l/j/k: navigate"));
    }

    #[test]
    fn test_get_view_hint_habits() {
        let mut model = Model::new();
        model.current_view = ViewId::Habits;

        let hint = get_view_hint(&model);
        assert_eq!(hint, Some("n: new | Space: check-in"));
    }

    #[test]
    fn test_get_view_hint_calendar() {
        let mut model = Model::new();
        model.current_view = ViewId::Calendar;

        let hint = get_view_hint(&model);
        assert_eq!(hint, Some("h/l: months | Enter: day tasks"));
    }

    #[test]
    fn test_get_view_hint_view_only_returns_none() {
        let mut model = Model::new();

        model.current_view = ViewId::Heatmap;
        assert_eq!(get_view_hint(&model), None);

        model.current_view = ViewId::Forecast;
        assert_eq!(get_view_hint(&model), None);

        model.current_view = ViewId::Burndown;
        assert_eq!(get_view_hint(&model), None);
    }

    #[test]
    fn test_get_view_hint_task_list_returns_none() {
        let mut model = Model::new();
        model.current_view = ViewId::TaskList;

        let hint = get_view_hint(&model);
        assert_eq!(hint, None);
    }

    // =========================================================================
    // view function tests
    // =========================================================================

    #[test]
    fn test_view_renders_without_panic() {
        let model = Model::new();
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_header() {
        let model = Model::new();
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("TaskFlow"),
            "Header should contain TaskFlow"
        );
    }

    #[test]
    fn test_view_renders_footer_task_count() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("tasks"), "Footer should show task count");
    }

    #[test]
    fn test_view_renders_help_popup() {
        let mut model = Model::new();
        model.show_help = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 40);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Help") || content.contains("Keybindings"),
            "Should show help popup"
        );
    }

    #[test]
    fn test_view_renders_confirm_delete_dialog() {
        let mut model = Model::new().with_sample_data();
        model.show_confirm_delete = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Delete") || content.contains("y/n"),
            "Should show delete confirmation"
        );
    }

    // =========================================================================
    // render_header tests
    // =========================================================================

    #[test]
    fn test_render_header_contains_title() {
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 3);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_header(frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("TaskFlow"));
        assert!(content.contains("Project Management"));
    }

    // =========================================================================
    // render_footer tests
    // =========================================================================

    #[test]
    fn test_render_footer_shows_task_count() {
        let model = Model::new();
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("tasks"));
        assert!(content.contains("completed"));
    }

    #[test]
    fn test_render_footer_shows_error_message() {
        let mut model = Model::new();
        model.alerts.error_message = Some("Test error message".to_string());
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Test error message"),
            "Should show error message"
        );
    }

    #[test]
    fn test_render_footer_shows_status_message() {
        let mut model = Model::new();
        model.alerts.status_message = Some("Status update".to_string());
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Status update"),
            "Should show status message"
        );
    }

    #[test]
    fn test_render_footer_shows_recording_indicator() {
        let mut model = Model::new();
        model.macro_state.start_recording(0); // Start recording in slot 0
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("REC"), "Should show recording indicator");
    }

    #[test]
    fn test_render_footer_shows_overdue_count() {
        let mut model = Model::new();
        model.footer_stats.overdue_count = 3;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("overdue"), "Should show overdue count");
    }

    #[test]
    fn test_render_footer_shows_due_today_count() {
        let mut model = Model::new();
        model.footer_stats.due_today_count = 5;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("due today"), "Should show due today count");
    }

    #[test]
    fn test_render_footer_shows_multi_select_mode() {
        let mut model = Model::new().with_sample_data();
        model.multi_select.mode = true;
        model.multi_select.selected.insert(model.visible_tasks[0]);
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("MULTI-SELECT"),
            "Should show multi-select indicator"
        );
    }

    #[test]
    fn test_render_footer_shows_view_hint() {
        let mut model = Model::new();
        model.current_view = ViewId::Kanban;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("h/l") || content.contains("columns"),
            "Should show view-specific hint"
        );
    }

    #[test]
    fn test_render_footer_shows_help_hint() {
        let model = Model::new();
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("? help"), "Should show help hint");
    }

    #[test]
    fn test_render_footer_shows_completed_visibility() {
        let mut model = Model::new();
        model.filtering.show_completed = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("showing all"),
            "Should show 'showing all' when completed visible"
        );
    }

    #[test]
    fn test_render_footer_hiding_completed() {
        let mut model = Model::new();
        model.filtering.show_completed = false;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("hiding completed"),
            "Should show 'hiding completed' when filtered"
        );
    }

    // =========================================================================
    // render_content tests
    // =========================================================================

    #[test]
    fn test_render_content_focus_mode() {
        let mut model = Model::new().with_sample_data();
        model.focus_mode = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render focus view without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_render_content_with_sidebar() {
        let mut model = Model::new().with_sample_data();
        model.show_sidebar = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should cache sidebar area
        assert!(model.layout_cache.sidebar_area().is_some());
    }

    #[test]
    fn test_render_content_without_sidebar() {
        let mut model = Model::new();
        model.show_sidebar = false;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    // =========================================================================
    // render_main_content tests for different views
    // =========================================================================

    #[test]
    fn test_render_main_content_calendar() {
        let mut model = Model::new();
        model.current_view = ViewId::Calendar;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should cache calendar area
        assert!(model.layout_cache.calendar_area().is_some());
    }

    #[test]
    fn test_render_main_content_dashboard() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Dashboard;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 30);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_render_main_content_reports() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Reports;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 30);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should cache reports tabs area
        assert!(model.layout_cache.reports_tabs_area().is_some());
    }

    #[test]
    fn test_render_main_content_habits() {
        let mut model = Model::new();
        model.current_view = ViewId::Habits;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_render_main_content_kanban() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Kanban;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should cache kanban column areas
        assert!(model.layout_cache.kanban_column(0).is_some());
        assert!(model.layout_cache.kanban_column(3).is_some());
    }

    #[test]
    fn test_render_main_content_eisenhower() {
        let mut model = Model::new();
        model.current_view = ViewId::Eisenhower;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should cache all four quadrants
        for i in 0..4 {
            assert!(
                model.layout_cache.eisenhower_quadrant(i).is_some(),
                "Quadrant {i} should be cached"
            );
        }
    }

    #[test]
    fn test_render_main_content_weekly_planner() {
        let mut model = Model::new();
        model.current_view = ViewId::WeeklyPlanner;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should cache all seven day areas
        for i in 0..7 {
            assert!(
                model.layout_cache.weekly_planner_day(i).is_some(),
                "Day {i} should be cached"
            );
        }
    }

    #[test]
    fn test_render_main_content_timeline() {
        let mut model = Model::new();
        model.current_view = ViewId::Timeline;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_render_main_content_heatmap() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Heatmap;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_render_main_content_forecast() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Forecast;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(120, 30);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 120, 30);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_render_main_content_network() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Network;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_render_main_content_burndown() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Burndown;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_render_main_content_task_list_default() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::TaskList;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 20);

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 20);
                render_main_content(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    // =========================================================================
    // Popup rendering tests
    // =========================================================================

    #[test]
    fn test_view_renders_input_dialog_for_new_task() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::Task;
        model.input.buffer = "New task title".to_string();
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("New Task") || content.contains("New task title"),
            "Should show new task input dialog"
        );
    }

    #[test]
    fn test_view_renders_input_dialog_for_search() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::Search;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Search"),
            "Should show search input dialog"
        );
    }

    #[test]
    fn test_view_renders_quick_capture_dialog() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::QuickCapture;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 30);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Quick capture has syntax hints
        let content = buffer_content(&terminal);
        assert!(
            content.contains("Capture") || content.contains("Quick"),
            "Should show quick capture dialog"
        );
    }

    #[test]
    fn test_view_renders_template_picker() {
        let mut model = Model::new();
        model.template_picker.visible = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_saved_filter_picker() {
        let mut model = Model::new();
        model.saved_filter_picker.visible = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_keybindings_editor() {
        let mut model = Model::new();
        model.keybindings_editor.visible = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 40);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_time_log_editor() {
        let mut model = Model::new().with_sample_data();
        model.time_log.visible = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_work_log_editor() {
        let mut model = Model::new().with_sample_data();
        model.work_log_editor.visible = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_description_editor() {
        let mut model = Model::new();
        model.description_editor.visible = true;
        model.description_editor.buffer = vec!["Line 1".to_string(), "Line 2".to_string()];
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_overdue_alert() {
        let mut model = Model::new().with_sample_data();
        model.alerts.show_overdue = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_storage_error_alert() {
        let mut model = Model::new();
        model.alerts.show_storage_error = true;
        model.alerts.storage_error = Some("Failed to load data".to_string());
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Failed") || content.contains("Error"),
            "Should show storage error"
        );
    }

    #[test]
    fn test_view_renders_daily_review() {
        let mut model = Model::new().with_sample_data();
        model.daily_review.visible = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 40);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_weekly_review() {
        let mut model = Model::new().with_sample_data();
        model.weekly_review.visible = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(100, 40);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_habit_analytics_popup() {
        let mut model = Model::new();
        model.habit_view.show_analytics = true;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        // Should render without panic
        assert!(terminal.backend().buffer().area.width > 0);
    }

    #[test]
    fn test_view_renders_import_preview() {
        use crate::storage::ImportResult;

        let mut model = Model::new();
        model.import.show_preview = true;
        model.import.pending = Some(ImportResult {
            imported: vec![],
            skipped: vec![],
            errors: vec![],
        });
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Import") || content.contains("Preview"),
            "Should show import preview"
        );
    }

    // =========================================================================
    // Input target title tests
    // =========================================================================

    #[test]
    fn test_input_dialog_titles_for_various_targets() {
        let theme = Theme::default();
        let targets = vec![
            (InputTarget::Task, "New Task"),
            (InputTarget::Project, "New Project"),
            (InputTarget::Search, "Search"),
            (InputTarget::FilterByTag, "Filter by Tag"),
        ];

        for (target, expected_title) in targets {
            let mut model = Model::new();
            model.input.mode = InputMode::Editing;
            model.input.target = target;
            let mut terminal = create_test_terminal(80, 24);

            terminal
                .draw(|frame| {
                    view(&model, frame, &theme);
                })
                .unwrap();

            let content = buffer_content(&terminal);
            assert!(
                content.contains(expected_title),
                "Should show title '{}' for target {:?}",
                expected_title,
                model.input.target
            );
        }
    }

    #[test]
    fn test_input_dialog_edit_task_targets() {
        use crate::domain::TaskId;

        let theme = Theme::default();
        let task_id = TaskId::new();

        let targets = vec![
            (InputTarget::EditTask(task_id), "Edit Task"),
            (InputTarget::EditDueDate(task_id), "Due Date"),
            (InputTarget::EditScheduledDate(task_id), "Scheduled Date"),
            (InputTarget::EditTags(task_id), "Tags"),
            (InputTarget::EditDescription(task_id), "Description"),
            (InputTarget::EditEstimate(task_id), "Time Estimate"),
            (InputTarget::MoveToProject(task_id), "Move to Project"),
            (InputTarget::EditDependencies(task_id), "Blocked by"),
            (InputTarget::EditRecurrence(task_id), "Recurrence"),
            (InputTarget::LinkTask(task_id), "Link to next"),
            (InputTarget::SnoozeTask(task_id), "Snooze"),
        ];

        for (target, expected_substr) in targets {
            let mut model = Model::new();
            model.input.mode = InputMode::Editing;
            model.input.target = target.clone();
            let mut terminal = create_test_terminal(80, 24);

            terminal
                .draw(|frame| {
                    view(&model, frame, &theme);
                })
                .unwrap();

            let content = buffer_content(&terminal);
            assert!(
                content.contains(expected_substr),
                "Should show '{expected_substr}' for target {target:?}"
            );
        }
    }

    #[test]
    fn test_input_dialog_subtask_target() {
        use crate::domain::TaskId;

        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::Subtask(TaskId::new());
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("Subtask"), "Should show Subtask title");
    }

    #[test]
    fn test_input_dialog_edit_project_target() {
        use crate::domain::ProjectId;

        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::EditProject(ProjectId::new());
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Rename") || content.contains("Project"),
            "Should show rename project title"
        );
    }

    #[test]
    fn test_input_dialog_bulk_targets() {
        let theme = Theme::default();

        // BulkMoveToProject
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::BulkMoveToProject;
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Move Selected"),
            "Should show bulk move title"
        );

        // BulkSetStatus
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::BulkSetStatus;
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Set Status"),
            "Should show bulk status title"
        );
    }

    #[test]
    fn test_input_dialog_import_csv() {
        use crate::storage::ImportFormat;

        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::ImportFilePath(ImportFormat::Csv);
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("CSV"), "Should show CSV import title");
    }

    #[test]
    fn test_input_dialog_import_ics() {
        use crate::storage::ImportFormat;

        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::ImportFilePath(ImportFormat::Ics);
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("ICS"), "Should show ICS import title");
    }

    #[test]
    fn test_input_dialog_saved_filter_name() {
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::SavedFilterName;
        let theme = Theme::default();
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Save Filter") || content.contains("Filter"),
            "Should show save filter title"
        );
    }

    #[test]
    fn test_input_dialog_habit_targets() {
        use crate::domain::HabitId;

        let theme = Theme::default();

        // NewHabit
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::NewHabit;
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(content.contains("New Habit"), "Should show new habit title");

        // EditHabit
        let mut model = Model::new();
        model.input.mode = InputMode::Editing;
        model.input.target = InputTarget::EditHabit(HabitId::new());
        let mut terminal = create_test_terminal(80, 24);

        terminal
            .draw(|frame| {
                view(&model, frame, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        assert!(
            content.contains("Edit Habit"),
            "Should show edit habit title"
        );
    }

    // =========================================================================
    // Pomodoro footer tests
    // =========================================================================

    #[test]
    fn test_render_footer_with_pomodoro_work_phase() {
        use crate::domain::{PomodoroSession, TaskId};

        let mut model = Model::new();
        // Create a pomodoro session directly
        let task_id = TaskId::new();
        model.pomodoro.session = Some(PomodoroSession::new(task_id, &model.pomodoro.config, 4));
        let theme = Theme::default();
        let mut terminal = create_test_terminal(120, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        // Should show timer display with work phase icon and time
        assert!(
            content.contains("25:00") || content.contains("24:") || content.contains(':'),
            "Should show work timer"
        );
    }

    #[test]
    fn test_render_footer_with_pomodoro_paused() {
        use crate::domain::{PomodoroSession, TaskId};

        let mut model = Model::new();
        let task_id = TaskId::new();
        let mut session = PomodoroSession::new(task_id, &model.pomodoro.config, 4);
        session.paused = true;
        model.pomodoro.session = Some(session);
        let theme = Theme::default();
        let mut terminal = create_test_terminal(120, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        // Should show pause indicator
        let content = buffer_content(&terminal);
        assert!(
            content.contains("⏸") || content.contains("[0/4]"),
            "Should show pause indicator or cycle count"
        );
    }

    #[test]
    fn test_render_footer_with_pomodoro_cycles() {
        use crate::domain::{PomodoroSession, TaskId};

        let mut model = Model::new();
        let task_id = TaskId::new();
        let mut session = PomodoroSession::new(task_id, &model.pomodoro.config, 4);
        session.cycles_completed = 2;
        model.pomodoro.session = Some(session);
        let theme = Theme::default();
        let mut terminal = create_test_terminal(120, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        let content = buffer_content(&terminal);
        // Should show cycle progress like [2/4]
        assert!(content.contains("[2/4]"), "Should show cycle progress");
    }

    #[test]
    fn test_render_footer_with_pomodoro_break_phase() {
        use crate::domain::{PomodoroPhase, PomodoroSession, TaskId};

        let mut model = Model::new();
        let task_id = TaskId::new();
        let mut session = PomodoroSession::new(task_id, &model.pomodoro.config, 4);
        session.phase = PomodoroPhase::ShortBreak;
        session.remaining_secs = 5 * 60; // 5 minutes
        model.pomodoro.session = Some(session);
        let theme = Theme::default();
        let mut terminal = create_test_terminal(120, 1);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_footer(&model, frame, area, &theme);
            })
            .unwrap();

        // Should render break phase (different color)
        let content = buffer_content(&terminal);
        assert!(
            content.contains("5:00") || content.contains(':'),
            "Should show break timer"
        );
    }
}
