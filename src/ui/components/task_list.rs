use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::app::Model;
use crate::domain::{Priority, Task, TaskId, TaskStatus};

/// Task list widget
pub struct TaskList<'a> {
    tasks: Vec<&'a Task>,
    selected: usize,
    active_tracking: Option<&'a TaskId>,
    time_for_tasks: Vec<u32>,
}

impl<'a> TaskList<'a> {
    pub fn new(model: &'a Model) -> Self {
        let tasks: Vec<&Task> = model
            .visible_tasks
            .iter()
            .filter_map(|id| model.tasks.get(id))
            .collect();

        let active_tracking = model.active_time_entry().map(|e| &e.task_id);

        let time_for_tasks: Vec<u32> = model
            .visible_tasks
            .iter()
            .map(|id| model.total_time_for_task(id))
            .collect();

        Self {
            tasks,
            selected: model.selected_index,
            active_tracking,
            time_for_tasks,
        }
    }
}

impl Widget for TaskList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let is_selected = i == self.selected;
                let is_tracking = self.active_tracking == Some(&task.id);
                let time_spent = self.time_for_tasks.get(i).copied().unwrap_or(0);
                task_to_list_item(task, is_selected, is_tracking, time_spent)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Tasks ")
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(self.selected));

        StatefulWidget::render(list, area, buf, &mut state);
    }
}

fn task_to_list_item(
    task: &Task,
    is_selected: bool,
    is_tracking: bool,
    time_spent: u32,
) -> ListItem<'static> {
    let status_style = match task.status {
        TaskStatus::Done => Style::default().fg(Color::Green),
        TaskStatus::InProgress => Style::default().fg(Color::Yellow),
        TaskStatus::Blocked => Style::default().fg(Color::Red),
        TaskStatus::Cancelled => Style::default().fg(Color::DarkGray),
        TaskStatus::Todo => Style::default().fg(Color::White),
    };

    let priority_span = match task.priority {
        Priority::Urgent => Span::styled("!!!! ", Style::default().fg(Color::Red)),
        Priority::High => Span::styled("!!!  ", Style::default().fg(Color::LightRed)),
        Priority::Medium => Span::styled("!!   ", Style::default().fg(Color::Yellow)),
        Priority::Low => Span::styled("!    ", Style::default().fg(Color::Green)),
        Priority::None => Span::raw("     "),
    };

    // Time tracking indicator
    let tracking_span = if is_tracking {
        Span::styled(
            "● ",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::SLOW_BLINK),
        )
    } else {
        Span::raw("  ")
    };

    let status_span = Span::styled(format!("{} ", task.status.symbol()), status_style);

    let title_style = if task.status.is_complete() {
        Style::default()
            .fg(Color::DarkGray)
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
            Style::default().fg(Color::Red) // Overdue
        } else if due == today {
            Style::default().fg(Color::Yellow) // Due today
        } else {
            Style::default().fg(Color::DarkGray)
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
        Span::styled(time_str, Style::default().fg(Color::Cyan))
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        tracking_span,
        priority_span,
        status_span,
        title_span,
        due_span,
        time_span,
    ]);

    ListItem::new(line)
}
