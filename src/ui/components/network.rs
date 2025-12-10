//! Network graph view component - dependency visualization.
//!
//! Displays task dependencies as an interactive ASCII graph,
//! showing relationships between blocked and blocking tasks.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::TaskId;

/// Network graph view widget showing task dependencies
pub struct Network<'a> {
    model: &'a Model,
    theme: &'a Theme,
    selected_task_index: usize,
}

impl<'a> Network<'a> {
    /// Create a new network widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme, selected_task_index: usize) -> Self {
        Self {
            model,
            theme,
            selected_task_index,
        }
    }

    /// Get the selected task ID based on the current selection index
    fn get_selected_task_id(&self) -> Option<TaskId> {
        let network_tasks = self.model.network_tasks();
        network_tasks.get(self.selected_task_index).copied()
    }

    /// Get tasks with no dependencies (roots)
    fn get_root_tasks(&self) -> Vec<TaskId> {
        self.model
            .tasks
            .values()
            .filter(|t| {
                t.dependencies.is_empty()
                    && (self
                        .model
                        .tasks
                        .values()
                        .any(|other| other.dependencies.contains(&t.id))
                        || t.next_task_id.is_some())
            })
            .map(|t| t.id)
            .collect()
    }

    /// Render a task node with its connections
    fn render_task_tree(&self, area: Rect, buf: &mut Buffer) {
        let roots = self.get_root_tasks();
        let mut lines: Vec<Line<'_>> = Vec::new();
        let mut rendered = std::collections::HashSet::new();

        for root_id in roots {
            self.render_node(&mut lines, &mut rendered, root_id, 0);
        }

        // Also show orphaned dependency chains
        for task in self.model.tasks.values() {
            if !rendered.contains(&task.id) && !task.dependencies.is_empty() {
                self.render_node(&mut lines, &mut rendered, task.id, 0);
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No task dependencies found.",
                Style::default().fg(self.theme.colors.muted.to_color()),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Create dependencies with 'D' on a task,",
                Style::default().fg(self.theme.colors.muted.to_color()),
            )));
            lines.push(Line::from(Span::styled(
                "or link tasks in a chain with 'L'.",
                Style::default().fg(self.theme.colors.muted.to_color()),
            )));
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" Dependency Graph ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.colors.border.to_color())),
        );
        paragraph.render(area, buf);
    }

    /// Recursively render a node and its dependents
    fn render_node(
        &self,
        lines: &mut Vec<Line<'_>>,
        rendered: &mut std::collections::HashSet<TaskId>,
        task_id: TaskId,
        depth: usize,
    ) {
        if rendered.contains(&task_id) {
            return;
        }
        rendered.insert(task_id);

        let Some(task) = self.model.tasks.get(&task_id) else {
            return;
        };

        // Build the prefix
        let indent = "  ".repeat(depth);
        let connector = if depth > 0 { "├─ " } else { "" };

        // Task status indicator
        let status_icon = if task.status.is_complete() {
            Span::styled("✓ ", Style::default().fg(Color::Green))
        } else if self
            .model
            .tasks
            .values()
            .any(|t| t.dependencies.contains(&task_id) && !t.status.is_complete())
        {
            Span::styled("● ", Style::default().fg(Color::Yellow)) // Blocking others
        } else {
            Span::styled(
                "○ ",
                Style::default().fg(self.theme.colors.muted.to_color()),
            )
        };

        // Check if this task is selected
        let is_selected = self.get_selected_task_id() == Some(task_id);

        let title_style = if is_selected {
            Style::default()
                .fg(self.theme.colors.foreground.to_color())
                .bg(self.theme.colors.accent_secondary.to_color())
                .add_modifier(Modifier::BOLD)
        } else if task.status.is_complete() {
            Style::default()
                .fg(self.theme.colors.muted.to_color())
                .add_modifier(Modifier::CROSSED_OUT)
        } else {
            Style::default().fg(self.theme.colors.foreground.to_color())
        };

        // Build the selection indicator
        let selection_marker = if is_selected { "▶ " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(
                selection_marker,
                Style::default().fg(self.theme.colors.accent.to_color()),
            ),
            Span::raw(format!("{indent}{connector}")),
            status_icon,
            Span::styled(task.title.chars().take(40).collect::<String>(), title_style),
        ]));

        // Find tasks that depend on this one
        let dependents: Vec<TaskId> = self
            .model
            .tasks
            .values()
            .filter(|t| t.dependencies.contains(&task_id))
            .map(|t| t.id)
            .collect();

        for dep_id in dependents {
            self.render_node(lines, rendered, dep_id, depth + 1);
        }

        // Follow chain link
        if let Some(next_id) = task.next_task_id {
            if !rendered.contains(&next_id) {
                let chain_indent = "  ".repeat(depth);
                lines.push(Line::from(vec![
                    Span::raw(format!("{chain_indent}  ")),
                    Span::styled("↓ chain", Style::default().fg(Color::Cyan)),
                ]));
                self.render_node(lines, rendered, next_id, depth);
            }
        }
    }

    /// Render statistics about dependencies
    fn render_stats(&self, area: Rect, buf: &mut Buffer) {
        let total_with_deps = self
            .model
            .tasks
            .values()
            .filter(|t| !t.dependencies.is_empty())
            .count();

        let blocked_tasks = self
            .model
            .tasks
            .values()
            .filter(|t| {
                !t.status.is_complete()
                    && t.dependencies.iter().any(|dep_id| {
                        self.model
                            .tasks
                            .get(dep_id)
                            .is_some_and(|dep| !dep.status.is_complete())
                    })
            })
            .count();

        let chain_count = self
            .model
            .tasks
            .values()
            .filter(|t| t.next_task_id.is_some())
            .count();

        let lines = vec![
            Line::from(vec![
                Span::styled(
                    "Tasks with dependencies: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{total_with_deps}"),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Currently blocked: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{blocked_tasks}"),
                    Style::default().fg(if blocked_tasks > 0 {
                        Color::Yellow
                    } else {
                        self.theme.colors.foreground.to_color()
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Task chains: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{chain_count}"),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" Statistics ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.colors.border.to_color())),
        );
        paragraph.render(area, buf);
    }

    /// Render the legend
    fn render_legend(&self, area: Rect, buf: &mut Buffer) {
        let lines = vec![
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::styled(
                    "Completed",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled("● ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "Blocking others",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "○ ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    "Pending",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled("↓ ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Chain link",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" Legend ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.colors.border.to_color())),
        );
        paragraph.render(area, buf);
    }
}

impl Widget for Network<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Network - Dependency Visualization ")
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 30 || inner.height < 10 {
            return;
        }

        // Layout: graph on left, stats and legend on right
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(inner);

        let right_panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(5)])
            .split(chunks[1]);

        self.render_task_tree(chunks[0], buf);
        self.render_stats(right_panel[0], buf);
        self.render_legend(right_panel[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;
    use crate::domain::Task;

    #[test]
    fn test_network_empty_model() {
        let model = Model::new();
        let theme = Theme::default();
        let network = Network::new(&model, &theme, 0);
        assert!(network.get_root_tasks().is_empty());
    }

    #[test]
    fn test_network_renders_without_panic() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let network = Network::new(&model, &theme, 0);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_network_with_dependencies() {
        let mut model = Model::new();

        // Create parent task
        let parent = Task::new("Parent Task");
        let parent_id = parent.id;
        model.tasks.insert(parent_id, parent);

        // Create child task with dependency on parent
        let mut child = Task::new("Child Task");
        child.dependencies.push(parent_id);
        model.tasks.insert(child.id, child);

        model.refresh_visible_tasks();

        let theme = Theme::default();

        // Check roots first (before render consumes the widget)
        let network_for_roots = Network::new(&model, &theme, 0);
        let roots = network_for_roots.get_root_tasks();
        assert!(roots.contains(&parent_id));

        // Then test rendering
        let network = Network::new(&model, &theme, 0);
        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_network_with_task_chain() {
        let mut model = Model::new();

        // Create first task
        let mut task1 = Task::new("First in chain");
        let task1_id = task1.id;

        // Create second task
        let task2 = Task::new("Second in chain");
        let task2_id = task2.id;

        // Link them in a chain
        task1.next_task_id = Some(task2_id);

        model.tasks.insert(task1_id, task1);
        model.tasks.insert(task2_id, task2);
        model.refresh_visible_tasks();

        let theme = Theme::default();

        // Check roots first (before render consumes the widget)
        let network_for_roots = Network::new(&model, &theme, 0);
        let roots = network_for_roots.get_root_tasks();
        assert!(roots.contains(&task1_id));

        // Then test rendering
        let network = Network::new(&model, &theme, 0);
        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);
        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_network_with_completed_task() {
        let mut model = Model::new();

        let parent = Task::new("Parent Task");
        let parent_id = parent.id;
        model.tasks.insert(parent_id, parent);

        let mut child = Task::new("Child Task");
        child.dependencies.push(parent_id);
        child.toggle_complete();
        model.tasks.insert(child.id, child);

        model.refresh_visible_tasks();

        let theme = Theme::default();
        let network = Network::new(&model, &theme, 0);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_network_narrow_area() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let network = Network::new(&model, &theme, 0);

        // Too narrow - should return early
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_network_get_selected_task_id() {
        let mut model = Model::new();

        let task1 = Task::new("Task 1");
        let task1_id = task1.id;
        let task2 = Task::new("Task 2");
        let task2_id = task2.id;

        // Link tasks to make them appear in network_tasks
        let mut task1 = task1;
        task1.next_task_id = Some(task2_id);
        model.tasks.insert(task1_id, task1);
        model.tasks.insert(task2_id, task2);
        model.refresh_visible_tasks();

        let theme = Theme::default();

        // Select first task
        let network = Network::new(&model, &theme, 0);
        let selected = network.get_selected_task_id();
        // The selected task depends on network_tasks order
        assert!(selected.is_some() || model.network_tasks().is_empty());
    }

    #[test]
    fn test_network_stats_rendering() {
        let mut model = Model::new();

        // Create tasks with dependencies
        let parent = Task::new("Blocking Task");
        let parent_id = parent.id;
        model.tasks.insert(parent_id, parent);

        let mut blocked = Task::new("Blocked Task");
        blocked.dependencies.push(parent_id);
        model.tasks.insert(blocked.id, blocked);

        model.refresh_visible_tasks();

        let theme = Theme::default();
        let network = Network::new(&model, &theme, 0);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);

        // Check for stats content
        let mut found_stats = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .filter_map(|x| buffer.cell((x, y)).map(ratatui::buffer::Cell::symbol))
                .collect();
            if line.contains("Statistics") {
                found_stats = true;
                break;
            }
        }
        assert!(found_stats);
    }

    #[test]
    fn test_network_legend_rendering() {
        let model = Model::new().with_sample_data();
        let theme = Theme::default();
        let network = Network::new(&model, &theme, 0);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);

        // Check for legend content
        let mut found_legend = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .filter_map(|x| buffer.cell((x, y)).map(ratatui::buffer::Cell::symbol))
                .collect();
            if line.contains("Legend") {
                found_legend = true;
                break;
            }
        }
        assert!(found_legend);
    }

    #[test]
    fn test_network_no_dependencies_message() {
        let mut model = Model::new();
        // Add tasks without any dependencies
        let task = Task::new("Standalone Task");
        model.tasks.insert(task.id, task);
        model.refresh_visible_tasks();

        let theme = Theme::default();
        let network = Network::new(&model, &theme, 0);

        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        network.render(area, &mut buffer);

        // Check for "No task dependencies" message
        let mut found_message = false;
        for y in 0..buffer.area.height {
            let line: String = (0..buffer.area.width)
                .filter_map(|x| buffer.cell((x, y)).map(ratatui::buffer::Cell::symbol))
                .collect();
            if line.contains("No task dependencies") {
                found_message = true;
                break;
            }
        }
        assert!(found_message);
    }

    #[test]
    fn test_network_selection_index() {
        let mut model = Model::new();

        // Create chain of tasks
        let mut task1 = Task::new("Task 1");
        let task1_id = task1.id;
        let mut task2 = Task::new("Task 2");
        let task2_id = task2.id;
        let task3 = Task::new("Task 3");
        let task3_id = task3.id;

        task1.next_task_id = Some(task2_id);
        task2.next_task_id = Some(task3_id);

        model.tasks.insert(task1_id, task1);
        model.tasks.insert(task2_id, task2);
        model.tasks.insert(task3_id, task3);
        model.refresh_visible_tasks();

        let theme = Theme::default();

        // Different selection indices
        for idx in 0..3 {
            let network = Network::new(&model, &theme, idx);
            let area = Rect::new(0, 0, 120, 30);
            let mut buffer = Buffer::empty(area);
            network.render(area, &mut buffer);
            assert!(buffer.area.width > 0);
        }
    }
}
