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
    centered_rect, centered_rect_fixed_height, Calendar, ConfirmDialog, DailyReview, Dashboard,
    DescriptionEditor, Eisenhower, FocusView, HabitAnalyticsPopup, HabitsView, HelpPopup,
    InputDialog, InputMode, InputTarget, Kanban, KeybindingsEditor, OverdueAlert, ReportsView,
    SavedFilterPicker, Sidebar, StorageErrorAlert, TaskList, TemplatePicker, TimeLogEditor,
    Timeline, WeeklyPlanner, WeeklyReview, WorkLogEditor,
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
        frame.render_widget(HelpPopup::new(&model.keybindings), popup_area);
    }

    // Render input dialog if in editing mode
    if model.input_mode == InputMode::Editing {
        // Height: 3 rows (top border, text line, bottom border)
        let input_area = centered_rect_fixed_height(60, 3, area);
        let title = match &model.input_target {
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
        };
        frame.render_widget(
            InputDialog::new(title, &model.input_buffer, model.cursor_position),
            input_area,
        );
    }

    // Render delete confirmation dialog
    if model.show_confirm_delete {
        // Height: 5 rows (border, message, blank, y/n prompt, border)
        let confirm_area = centered_rect_fixed_height(50, 5, area);
        let task_name = model
            .selected_task()
            .map_or("this task", |t| t.title.as_str());
        frame.render_widget(
            ConfirmDialog::new("Delete Task", &format!("Delete \"{task_name}\"?")),
            confirm_area,
        );
    }

    // Render import preview dialog
    if model.show_import_preview {
        if let Some(ref result) = model.pending_import {
            let confirm_area = centered_rect_fixed_height(60, 7, area);
            let message = format!(
                "Tasks to import: {}\nSkipped: {}\nErrors: {}",
                result.imported.len(),
                result.skipped.len(),
                result.errors.len()
            );
            frame.render_widget(ConfirmDialog::new("Import Preview", &message), confirm_area);
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
        frame.render_widget(OverdueAlert::new(count, task_titles), alert_area);
    }

    // Render storage error alert popup (shown at startup if data couldn't be loaded)
    if model.alerts.show_storage_error {
        if let Some(ref error) = model.alerts.storage_error {
            let alert_area = centered_rect_fixed_height(60, 10, area);
            frame.render_widget(StorageErrorAlert::new(error), alert_area);
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

            let reports = ReportsView::new(model, model.report_panel);
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
    if let Some(ref msg) = model.status_message {
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
    if let Some(ref session) = model.pomodoro_session {
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

    // Add show mode and help
    spans.push(Span::styled(
        " | ",
        Style::default().fg(theme.colors.muted.to_color()),
    ));
    spans.push(Span::styled(
        if model.show_completed {
            "showing all"
        } else {
            "hiding completed"
        },
        Style::default().fg(theme.colors.muted.to_color()),
    ));
    spans.push(Span::styled(
        " | Press ? for help ",
        Style::default().fg(theme.colors.muted.to_color()),
    ));

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}
