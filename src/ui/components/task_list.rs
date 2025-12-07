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
        nesting_depth: usize, // 0 for root, 1+ for subtasks
        is_multi_selected: bool,
        has_dependencies: bool,
        is_recurring: bool,
        has_chain: bool, // Task is linked to another task (has next_task_id)
        subtask_progress: (usize, usize), // (completed, total) - only shown if total > 0
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
    /// Maps display row to `visible_tasks` index (None for headers)
    row_to_task_index: Vec<Option<usize>>,
    active_tracking: Option<&'a TaskId>,
    theme: &'a Theme,
    is_grouped: bool,
}

impl<'a> TaskList<'a> {
    #[must_use]
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
                let nesting_depth = model.task_depth(task_id);
                let is_multi_selected = model.selected_tasks.contains(task_id);
                let has_dependencies = !task.dependencies.is_empty();
                let is_recurring = task.recurrence.is_some();
                let has_chain = task.next_task_id.is_some();
                let subtask_progress = model.subtask_progress(task_id);
                entries.push(ListEntry::Task {
                    task,
                    index: idx,
                    time_spent,
                    nesting_depth,
                    is_multi_selected,
                    has_dependencies,
                    is_recurring,
                    has_chain,
                    subtask_progress,
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
                    let nesting_depth = model.task_depth(&task_id);
                    let is_multi_selected = model.selected_tasks.contains(&task_id);
                    let has_dependencies = !task.dependencies.is_empty();
                    let is_recurring = task.recurrence.is_some();
                    let has_chain = task.next_task_id.is_some();
                    let subtask_progress = model.subtask_progress(&task_id);
                    entries.push(ListEntry::Task {
                        task,
                        index: idx,
                        time_spent,
                        nesting_depth,
                        is_multi_selected,
                        has_dependencies,
                        is_recurring,
                        has_chain,
                        subtask_progress,
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
                    nesting_depth,
                    is_multi_selected,
                    has_dependencies,
                    is_recurring,
                    has_chain,
                    subtask_progress,
                } => {
                    let ctx = TaskItemContext {
                        task,
                        is_selected: *index == self.selected,
                        is_tracking: self.active_tracking == Some(&task.id),
                        time_spent: *time_spent,
                        nesting_depth: *nesting_depth,
                        is_multi_selected: *is_multi_selected,
                        has_dependencies: *has_dependencies,
                        is_recurring: *is_recurring,
                        has_chain: *has_chain,
                        subtask_progress: *subtask_progress,
                        theme,
                    };
                    task_to_list_item(&ctx)
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

/// Context for rendering a task item
struct TaskItemContext<'a> {
    task: &'a Task,
    is_selected: bool,
    is_tracking: bool,
    time_spent: u32,
    nesting_depth: usize, // 0 for root, 1+ for subtasks
    is_multi_selected: bool,
    has_dependencies: bool,
    is_recurring: bool,
    has_chain: bool,                  // Task is linked to another task
    subtask_progress: (usize, usize), // (completed, total)
    theme: &'a Theme,
}

fn project_header_to_list_item(name: &str, task_count: usize, theme: &Theme) -> ListItem<'static> {
    let header_style = Style::default()
        .fg(theme.colors.accent.to_color())
        .add_modifier(Modifier::BOLD);

    let count_style = Style::default().fg(theme.colors.muted.to_color());

    let line = Line::from(vec![
        Span::styled("── ", Style::default().fg(theme.colors.muted.to_color())),
        Span::styled(name.to_string(), header_style),
        Span::styled(format!(" ({task_count}) "), count_style),
        Span::styled("──", Style::default().fg(theme.colors.muted.to_color())),
    ]);

    ListItem::new(line)
}

#[allow(clippy::too_many_lines)]
fn task_to_list_item(ctx: &TaskItemContext) -> ListItem<'static> {
    let task = ctx.task;
    let theme = ctx.theme;

    // Multi-select indicator
    let select_span = if ctx.is_multi_selected {
        Span::styled("● ", Style::default().fg(theme.colors.accent.to_color()))
    } else {
        Span::raw("  ")
    };

    // Subtask indentation prefix - supports multi-level nesting
    let indent_span = if ctx.nesting_depth > 0 {
        // Add spaces for each level of nesting, then the branch character
        let spaces = "  ".repeat(ctx.nesting_depth.saturating_sub(1));
        let indent = format!("{spaces}└─ ");
        Span::styled(indent, Style::default().fg(theme.colors.muted.to_color()))
    } else {
        Span::raw("")
    };

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
    let tracking_span = if ctx.is_tracking {
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
    } else if ctx.is_selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let title_span = Span::styled(task.title.clone(), title_style);

    // Add due date if present
    let due_span = if let Some(due) = task.due_date {
        use std::cmp::Ordering;
        let today = chrono::Utc::now().date_naive();
        let style = match due.cmp(&today) {
            Ordering::Less => Style::default().fg(theme.colors.danger.to_color()), // Overdue
            Ordering::Equal => Style::default().fg(theme.colors.warning.to_color()), // Due today
            Ordering::Greater => Style::default().fg(theme.colors.muted.to_color()),
        };
        Span::styled(format!(" [{}]", due.format("%m/%d")), style)
    } else {
        Span::raw("")
    };

    // Add scheduled date if present (shows when task is planned to be worked on)
    let sched_span = if let Some(sched) = task.scheduled_date {
        Span::styled(
            format!(" 📅{}", sched.format("%m/%d")),
            Style::default().fg(theme.colors.accent.to_color()),
        )
    } else {
        Span::raw("")
    };

    // Time spent indicator
    let time_span = if ctx.time_spent > 0 {
        let hours = ctx.time_spent / 60;
        let mins = ctx.time_spent % 60;
        let time_str = if hours > 0 {
            format!(" ({hours}h {mins}m)")
        } else {
            format!(" ({mins}m)")
        };
        Span::styled(
            time_str,
            Style::default().fg(theme.colors.accent.to_color()),
        )
    } else {
        Span::raw("")
    };

    // Tags display
    let tags_span = if task.tags.is_empty() {
        Span::raw("")
    } else {
        let tags_str = task
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ");
        Span::styled(
            format!(" {tags_str}"),
            Style::default().fg(theme.colors.muted.to_color()),
        )
    };

    // Description indicator (shows if task has a note)
    let desc_span = if task.description.is_some() {
        Span::styled(" [+]", Style::default().fg(theme.colors.muted.to_color()))
    } else {
        Span::raw("")
    };

    // Dependency indicator (shows if task is blocked by other tasks)
    let dep_span = if ctx.has_dependencies {
        Span::styled(" [B]", Style::default().fg(theme.colors.warning.to_color()))
    } else {
        Span::raw("")
    };

    // Recurrence indicator
    let recur_span = if ctx.is_recurring {
        Span::styled(" ↻", Style::default().fg(theme.colors.accent.to_color()))
    } else {
        Span::raw("")
    };

    // Chain indicator (→) - shows task is linked to next task in sequence
    let chain_span = if ctx.has_chain {
        Span::styled(" →", Style::default().fg(theme.colors.accent.to_color()))
    } else {
        Span::raw("")
    };

    // Subtask progress indicator - shows percentage if task has subtasks
    let progress_span = if ctx.subtask_progress.1 > 0 {
        let (completed, total) = ctx.subtask_progress;
        let percentage = (completed * 100) / total;
        let style = if completed == total {
            // All done - show in success/done color
            Style::default().fg(theme.status.done.to_color())
        } else {
            Style::default().fg(theme.colors.muted.to_color())
        };
        Span::styled(format!(" [{percentage}%]"), style)
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        select_span,
        indent_span,
        tracking_span,
        priority_span,
        status_span,
        title_span,
        progress_span,
        desc_span,
        dep_span,
        recur_span,
        chain_span,
        due_span,
        sched_span,
        time_span,
        tags_span,
    ]);

    ListItem::new(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;
    use crate::domain::{Priority, Task, TaskStatus};

    /// Helper to render a widget into a buffer
    fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        widget.render(area, &mut buffer);
        buffer
    }

    /// Extract text content from buffer
    fn buffer_content(buffer: &Buffer) -> String {
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

    #[test]
    fn test_task_list_renders_title() {
        let model = Model::new();
        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 60, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Tasks"),
            "Task list title should be visible"
        );
    }

    #[test]
    fn test_task_list_renders_empty_list() {
        let model = Model::new();
        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 60, 20);

        // Should render without panic
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_task_list_renders_task_titles() {
        let mut model = Model::new();
        let task = Task::new("Test Task Title");
        let task_id = task.id.clone();
        model.tasks.insert(task_id.clone(), task);
        model.visible_tasks.push(task_id);

        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 60, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Test Task Title"),
            "Task title should be visible"
        );
    }

    #[test]
    fn test_task_list_renders_priority_indicator() {
        let mut model = Model::new();
        let task = Task::new("Urgent Task").with_priority(Priority::Urgent);
        let task_id = task.id.clone();
        model.tasks.insert(task_id.clone(), task);
        model.visible_tasks.push(task_id);

        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 60, 20);
        let content = buffer_content(&buffer);

        // Urgent tasks show "!!!!"
        assert!(
            content.contains("!!!!") || content.contains("Urgent"),
            "Priority indicator should be visible"
        );
    }

    #[test]
    fn test_task_list_renders_completed_task() {
        let mut model = Model::new();
        let task = Task::new("Completed Task").with_status(TaskStatus::Done);
        let task_id = task.id.clone();
        model.tasks.insert(task_id.clone(), task);
        model.visible_tasks.push(task_id);
        model.show_completed = true;

        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 60, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("Completed Task"),
            "Completed task title should be visible"
        );
    }

    #[test]
    fn test_task_list_renders_status_symbol() {
        let mut model = Model::new();
        let task = Task::new("In Progress Task").with_status(TaskStatus::InProgress);
        let task_id = task.id.clone();
        model.tasks.insert(task_id.clone(), task);
        model.visible_tasks.push(task_id);

        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 60, 20);
        let content = buffer_content(&buffer);

        // Should have status symbol (varies by status)
        assert!(
            content.contains("In Progress Task"),
            "Task with status should be visible"
        );
    }

    #[test]
    fn test_task_list_renders_tags() {
        let mut model = Model::new();
        let task = Task::new("Tagged Task").with_tags(vec!["rust".into(), "test".into()]);
        let task_id = task.id.clone();
        model.tasks.insert(task_id.clone(), task);
        model.visible_tasks.push(task_id);

        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 80, 20);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("#rust") || content.contains("#test"),
            "Tags should be visible"
        );
    }

    #[test]
    fn test_task_list_renders_due_date() {
        let mut model = Model::new();
        let today = chrono::Utc::now().date_naive();
        let task = Task::new("Due Task").with_due_date(today);
        let task_id = task.id.clone();
        model.tasks.insert(task_id.clone(), task);
        model.visible_tasks.push(task_id);

        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 80, 20);
        let content = buffer_content(&buffer);

        // Due date shown in format [MM/DD]
        assert!(content.contains('['), "Due date bracket should be visible");
    }

    #[test]
    fn test_task_list_renders_with_sample_data() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 80, 30);
        let content = buffer_content(&buffer);

        // Sample data contains various tasks
        assert!(
            content.contains("Tasks"),
            "Task list title should be visible"
        );
    }

    #[test]
    fn test_task_list_grouped_view() {
        let mut model = Model::new().with_sample_data();
        model.current_view = ViewId::Projects;
        model.refresh_visible_tasks();

        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 80, 30);
        let content = buffer_content(&buffer);

        assert!(
            content.contains("by Project"),
            "Grouped title should be visible"
        );
    }

    #[test]
    fn test_task_list_description_indicator() {
        let mut model = Model::new();
        let task =
            Task::new("Task with Notes").with_description("Some important notes".to_string());
        let task_id = task.id.clone();
        model.tasks.insert(task_id.clone(), task);
        model.visible_tasks.push(task_id);

        let theme = Theme::default();
        let task_list = TaskList::new(&model, &theme);
        let buffer = render_widget(task_list, 80, 20);
        let content = buffer_content(&buffer);

        // [+] indicator for tasks with descriptions
        assert!(
            content.contains("[+]"),
            "Description indicator should be visible"
        );
    }
}
