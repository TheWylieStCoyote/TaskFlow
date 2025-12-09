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
}

impl<'a> Network<'a> {
    /// Create a new network widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
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

        let title_style = if task.status.is_complete() {
            Style::default()
                .fg(self.theme.colors.muted.to_color())
                .add_modifier(Modifier::CROSSED_OUT)
        } else {
            Style::default().fg(self.theme.colors.foreground.to_color())
        };

        lines.push(Line::from(vec![
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

    #[test]
    fn test_network_empty_model() {
        let model = Model::new();
        let theme = Theme::default();
        let network = Network::new(&model, &theme);
        assert!(network.get_root_tasks().is_empty());
    }
}
