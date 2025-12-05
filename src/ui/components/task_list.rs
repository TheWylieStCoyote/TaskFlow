use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

// Note: Color is still used for highlight_style background

use crate::app::{Model, ViewId};
use crate::config::Theme;
use crate::domain::{Priority, Task, TaskId, TaskStatus};

/// Represents an item in the task list (either a task or a project header)
enum ListEntry<'a> {
    Task {
        task: &'a Task,
        index: usize, // Index in visible_tasks for selection tracking
        time_spent: u32,
    },
    ProjectHeader {
        name: String,
        task_count: usize,
    },
}

/// Task list widget
pub struct TaskList<'a> {
    entries: Vec<ListEntry<'a>>,
    selected: usize,
    /// Maps display row to visible_tasks index (None for headers)
    row_to_task_index: Vec<Option<usize>>,
    active_tracking: Option<&'a TaskId>,
    theme: &'a Theme,
    is_grouped: bool,
}

impl<'a> TaskList<'a> {
    pub fn new(model: &'a Model, theme: &'a Theme) -> Self {
        let active_tracking = model.active_time_entry().map(|e| &e.task_id);
        let is_grouped = model.current_view == ViewId::Projects;

        if is_grouped {
            Self::new_grouped(model, theme, active_tracking)
        } else {
            Self::new_flat(model, theme, active_tracking)
        }
    }

    fn new_flat(model: &'a Model, theme: &'a Theme, active_tracking: Option<&'a TaskId>) -> Self {
        let mut entries = Vec::new();
        let mut row_to_task_index = Vec::new();

        for (idx, task_id) in model.visible_tasks.iter().enumerate() {
            if let Some(task) = model.tasks.get(task_id) {
                let time_spent = model.total_time_for_task(task_id);
                entries.push(ListEntry::Task {
                    task,
                    index: idx,
                    time_spent,
                });
                row_to_task_index.push(Some(idx));
            }
        }

        Self {
            entries,
            selected: model.selected_index,
            row_to_task_index,
            active_tracking,
            theme,
            is_grouped: false,
        }
    }

    fn new_grouped(
        model: &'a Model,
        theme: &'a Theme,
        active_tracking: Option<&'a TaskId>,
    ) -> Self {
        let grouped = model.get_tasks_grouped_by_project();
        let mut entries = Vec::new();
        let mut row_to_task_index = Vec::new();

        for (_project_id, project_name, task_ids) in grouped {
            // Add project header
            entries.push(ListEntry::ProjectHeader {
                name: project_name,
                task_count: task_ids.len(),
            });
            row_to_task_index.push(None); // Headers are not selectable

            // Add tasks under this project
            for task_id in task_ids {
                if let Some(task) = model.tasks.get(&task_id) {
                    // Find the index in visible_tasks
                    let idx = model
                        .visible_tasks
                        .iter()
                        .position(|id| id == &task_id)
                        .unwrap_or(0);
                    let time_spent = model.total_time_for_task(&task_id);
                    entries.push(ListEntry::Task {
                        task,
                        index: idx,
                        time_spent,
                    });
                    row_to_task_index.push(Some(idx));
                }
            }
        }

        Self {
            entries,
            selected: model.selected_index,
            row_to_task_index,
            active_tracking,
            theme,
            is_grouped: true,
        }
    }

    /// Find the display row for the currently selected task index
    fn selected_row(&self) -> Option<usize> {
        self.row_to_task_index
            .iter()
            .position(|idx| *idx == Some(self.selected))
    }
}

impl Widget for TaskList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let selected_row = self.selected_row();

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| match entry {
                ListEntry::Task {
                    task,
                    index,
                    time_spent,
                } => {
                    let is_selected = *index == self.selected;
                    let is_tracking = self.active_tracking == Some(&task.id);
                    task_to_list_item(task, is_selected, is_tracking, *time_spent, theme)
                }
                ListEntry::ProjectHeader { name, task_count } => {
                    project_header_to_list_item(name, *task_count, theme)
                }
            })
            .collect();

        let title = if self.is_grouped {
            " Tasks (by Project) "
        } else {
            " Tasks "
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(theme.colors.border.to_color())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(selected_row);

        StatefulWidget::render(list, area, buf, &mut state);
    }
}

fn project_header_to_list_item(name: &str, task_count: usize, theme: &Theme) -> ListItem<'static> {
    let header_style = Style::default()
        .fg(theme.colors.accent.to_color())
        .add_modifier(Modifier::BOLD);

    let count_style = Style::default().fg(theme.colors.muted.to_color());

    let line = Line::from(vec![
        Span::styled("── ", Style::default().fg(theme.colors.muted.to_color())),
        Span::styled(name.to_string(), header_style),
        Span::styled(format!(" ({}) ", task_count), count_style),
        Span::styled("──", Style::default().fg(theme.colors.muted.to_color())),
    ]);

    ListItem::new(line)
}

fn task_to_list_item(
    task: &Task,
    is_selected: bool,
    is_tracking: bool,
    time_spent: u32,
    theme: &Theme,
) -> ListItem<'static> {
    let status_style = match task.status {
        TaskStatus::Done => Style::default().fg(theme.status.done.to_color()),
        TaskStatus::InProgress => Style::default().fg(theme.status.in_progress.to_color()),
        TaskStatus::Blocked => Style::default().fg(theme.colors.danger.to_color()),
        TaskStatus::Cancelled => Style::default().fg(theme.status.cancelled.to_color()),
        TaskStatus::Todo => Style::default().fg(theme.status.pending.to_color()),
    };

    let priority_span = match task.priority {
        Priority::Urgent => Span::styled(
            "!!!! ",
            Style::default().fg(theme.priority.urgent.to_color()),
        ),
        Priority::High => {
            Span::styled("!!!  ", Style::default().fg(theme.priority.high.to_color()))
        }
        Priority::Medium => Span::styled(
            "!!   ",
            Style::default().fg(theme.priority.medium.to_color()),
        ),
        Priority::Low => Span::styled("!    ", Style::default().fg(theme.priority.low.to_color())),
        Priority::None => Span::raw("     "),
    };

    // Time tracking indicator
    let tracking_span = if is_tracking {
        Span::styled(
            "● ",
            Style::default()
                .fg(theme.colors.danger.to_color())
                .add_modifier(Modifier::SLOW_BLINK),
        )
    } else {
        Span::raw("  ")
    };

    let status_span = Span::styled(format!("{} ", task.status.symbol()), status_style);

    let title_style = if task.status.is_complete() {
        Style::default()
            .fg(theme.colors.muted.to_color())
            .add_modifier(Modifier::CROSSED_OUT)
    } else if is_selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let title_span = Span::styled(task.title.clone(), title_style);

    // Add due date if present
    let due_span = if let Some(due) = task.due_date {
        let today = chrono::Utc::now().date_naive();
        let style = if due < today {
            Style::default().fg(theme.colors.danger.to_color()) // Overdue
        } else if due == today {
            Style::default().fg(theme.colors.warning.to_color()) // Due today
        } else {
            Style::default().fg(theme.colors.muted.to_color())
        };
        Span::styled(format!(" [{}]", due.format("%m/%d")), style)
    } else {
        Span::raw("")
    };

    // Time spent indicator
    let time_span = if time_spent > 0 {
        let hours = time_spent / 60;
        let mins = time_spent % 60;
        let time_str = if hours > 0 {
            format!(" ({}h {}m)", hours, mins)
        } else {
            format!(" ({}m)", mins)
        };
        Span::styled(
            time_str,
            Style::default().fg(theme.colors.accent.to_color()),
        )
    } else {
        Span::raw("")
    };

    // Tags display
    let tags_span = if !task.tags.is_empty() {
        let tags_str = task
            .tags
            .iter()
            .map(|t| format!("#{}", t))
            .collect::<Vec<_>>()
            .join(" ");
        Span::styled(
            format!(" {}", tags_str),
            Style::default().fg(theme.colors.muted.to_color()),
        )
    } else {
        Span::raw("")
    };

    // Description indicator (shows if task has a note)
    let desc_span = if task.description.is_some() {
        Span::styled(" [+]", Style::default().fg(theme.colors.muted.to_color()))
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        tracking_span,
        priority_span,
        status_span,
        title_span,
        desc_span,
        due_span,
        time_span,
        tags_span,
    ]);

    ListItem::new(line)
}
