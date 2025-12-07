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
    centered_rect, centered_rect_fixed_height, Calendar, ConfirmDialog, Dashboard, FocusView,
    HelpPopup, InputDialog, InputMode, InputTarget, KeybindingsEditor, OverdueAlert, ReportsView,
    Sidebar, TaskList, TemplatePicker,
};

/// Main view function - renders the entire UI based on model state
pub fn view(model: &Model, frame: &mut Frame, theme: &Theme) {
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
        frame.render_widget(HelpPopup::new(), popup_area);
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
            InputTarget::Project => "New Project",
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
            .map(|t| t.title.as_str())
            .unwrap_or("this task");
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
    if model.show_templates {
        // Height depends on number of templates, min 4, max 15
        let height = (model.template_manager.len() as u16 + 2).clamp(4, 15);
        let picker_area = centered_rect_fixed_height(60, height, area);
        frame.render_widget(
            TemplatePicker::new(&model.template_manager, model.template_selected, theme),
            picker_area,
        );
    }

    // Render keybindings editor
    if model.show_keybindings_editor {
        // Height depends on number of bindings, min 10, max 30
        let bindings_count = model.keybindings.sorted_bindings().len() as u16;
        let height = (bindings_count + 2).clamp(10, 30);
        let editor_area = centered_rect_fixed_height(70, height, area);
        frame.render_widget(
            KeybindingsEditor::new(
                &model.keybindings,
                model.keybinding_selected,
                model.keybinding_capturing,
                theme,
            ),
            editor_area,
        );
    }

    // Render overdue alert popup (shown at startup if there are overdue tasks)
    if model.show_overdue_alert {
        let (count, overdue_tasks) = model.overdue_summary();
        let task_titles: Vec<String> = overdue_tasks.iter().map(|t| t.title.clone()).collect();
        // Height: 4 + min(5, count) + 2 for header/footer
        let height = (6 + count.min(5)) as u16;
        let alert_area = centered_rect_fixed_height(50, height.max(7), area);
        frame.render_widget(OverdueAlert::new(count, task_titles), alert_area);
    }
}

fn render_header(frame: &mut Frame, area: Rect, theme: &Theme) {
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

fn render_content(model: &Model, frame: &mut Frame, area: Rect, theme: &Theme) {
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

        // Render sidebar
        frame.render_widget(Sidebar::new(model, theme), chunks[0]);

        // Render main content based on current view
        render_main_content(model, frame, chunks[1], theme);
    } else {
        // No sidebar, full width content
        render_main_content(model, frame, area, theme);
    }
}

fn render_main_content(model: &Model, frame: &mut Frame, area: Rect, theme: &Theme) {
    match model.current_view {
        ViewId::Calendar => {
            let calendar = Calendar::new(model, theme);
            frame.render_widget(calendar, area);
        }
        ViewId::Dashboard => {
            let dashboard = Dashboard::new(model, theme);
            frame.render_widget(dashboard, area);
        }
        ViewId::Reports => {
            let reports = ReportsView::new(model, model.report_panel);
            frame.render_widget(reports, area);
        }
        _ => {
            let task_list = TaskList::new(model, theme);
            frame.render_widget(task_list, area);
        }
    }
}

fn render_footer(model: &Model, frame: &mut Frame, area: Rect, theme: &Theme) {
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

    // Calculate counts
    let task_count = model.visible_tasks.len();
    let completed = model
        .tasks
        .values()
        .filter(|t| t.status.is_complete())
        .count();
    let overdue = model.tasks.values().filter(|t| t.is_overdue()).count();
    let due_today = model
        .tasks
        .values()
        .filter(|t| t.is_due_today() && !t.status.is_complete())
        .count();

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
