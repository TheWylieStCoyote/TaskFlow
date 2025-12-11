//! Duplicates view component - shows potential duplicate task pairs.
//!
//! Displays tasks that may be duplicates based on fuzzy title matching,
//! allowing users to review and merge or dismiss duplicate pairs.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Duplicates view widget showing potential duplicate task pairs
pub struct Duplicates<'a> {
    model: &'a Model,
    theme: &'a Theme,
}

impl<'a> Duplicates<'a> {
    /// Create a new duplicates widget
    #[must_use]
    pub const fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self { model, theme }
    }

    /// Render the header with statistics
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let pair_count = self.model.duplicates_view.pairs.len();
        let threshold = self.model.duplicates_view.threshold * 100.0;

        let header = Line::from(vec![
            Span::styled(
                format!("{pair_count} "),
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "potential duplicate pair{} found (threshold: {threshold:.0}%)",
                    if pair_count == 1 { "" } else { "s" }
                ),
                Style::default().fg(self.theme.colors.foreground.to_color()),
            ),
        ]);

        Paragraph::new(header).render(area, buf);
    }

    /// Render the list of duplicate pairs
    fn render_list(&self, area: Rect, buf: &mut Buffer) {
        let pairs = &self.model.duplicates_view.pairs;
        let selected = self.model.duplicates_view.selected;

        if pairs.is_empty() {
            let empty_msg = Paragraph::new(Line::from(vec![Span::styled(
                "No duplicate tasks found!",
                Style::default()
                    .fg(self.theme.colors.success.to_color())
                    .add_modifier(Modifier::BOLD),
            )]))
            .style(Style::default().fg(self.theme.colors.muted.to_color()));
            empty_msg.render(area, buf);
            return;
        }

        let items: Vec<ListItem<'_>> = pairs
            .iter()
            .enumerate()
            .map(|(i, pair)| {
                let task1 = self.model.tasks.get(&pair.task1_id);
                let task2 = self.model.tasks.get(&pair.task2_id);

                let (title1, title2, project_name) = match (task1, task2) {
                    (Some(t1), Some(t2)) => {
                        let project = t1
                            .project_id
                            .and_then(|pid| self.model.projects.get(&pid))
                            .map_or("Inbox".to_string(), |p| p.name.clone());
                        (t1.title.clone(), t2.title.clone(), project)
                    }
                    _ => (
                        "[deleted]".to_string(),
                        "[deleted]".to_string(),
                        "Unknown".to_string(),
                    ),
                };

                let similarity_pct = (pair.similarity * 100.0) as u32;
                let is_selected = i == selected;

                // Color based on similarity level
                let similarity_color = if pair.similarity >= 0.95 {
                    Color::Red
                } else if pair.similarity >= 0.90 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{similarity_pct:>3}% "),
                        Style::default()
                            .fg(similarity_color)
                            .add_modifier(if is_selected {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                    Span::styled(
                        format!("[{project_name}] "),
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        truncate_title(&title1, 30),
                        Style::default().fg(self.theme.colors.foreground.to_color()),
                    ),
                    Span::styled(
                        " <-> ",
                        Style::default().fg(self.theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        truncate_title(&title2, 30),
                        Style::default().fg(self.theme.colors.foreground.to_color()),
                    ),
                ]);

                let style = if is_selected {
                    Style::default()
                        .bg(self.theme.colors.accent_secondary.to_color())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items);
        Widget::render(list, area, buf);
    }

    /// Render the detail panel for selected pair
    fn render_detail(&self, area: Rect, buf: &mut Buffer) {
        let pairs = &self.model.duplicates_view.pairs;
        let selected = self.model.duplicates_view.selected;

        let Some(pair) = pairs.get(selected) else {
            return;
        };

        let task1 = self.model.tasks.get(&pair.task1_id);
        let task2 = self.model.tasks.get(&pair.task2_id);

        let (Some(t1), Some(t2)) = (task1, task2) else {
            return;
        };

        let lines = vec![
            Line::from(vec![
                Span::styled(
                    "Task 1: ",
                    Style::default()
                        .fg(self.theme.colors.muted.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&t1.title, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Status: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{:?}", t1.status),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
                Span::styled(
                    "  Priority: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{:?}", t1.priority),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Task 2: ",
                    Style::default()
                        .fg(self.theme.colors.muted.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&t2.title, Style::default().fg(Color::Magenta)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Status: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{:?}", t2.status),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
                Span::styled(
                    "  Priority: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{:?}", t2.priority),
                    Style::default().fg(self.theme.colors.foreground.to_color()),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Similarity: ",
                    Style::default().fg(self.theme.colors.muted.to_color()),
                ),
                Span::styled(
                    format!("{:.1}%", pair.similarity * 100.0),
                    Style::default()
                        .fg(self.theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];

        let block = Block::default()
            .title(" Selected Pair Details ")
            .title_style(Style::default().fg(self.theme.colors.accent.to_color()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);
        Paragraph::new(lines).render(inner, buf);
    }

    /// Render the help footer
    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let help = Line::from(vec![
            Span::styled(
                "j/k",
                Style::default().fg(self.theme.colors.accent.to_color()),
            ),
            Span::styled(
                " navigate  ",
                Style::default().fg(self.theme.colors.muted.to_color()),
            ),
            Span::styled(
                "D",
                Style::default().fg(self.theme.colors.accent.to_color()),
            ),
            Span::styled(
                " dismiss  ",
                Style::default().fg(self.theme.colors.muted.to_color()),
            ),
            Span::styled(
                "M",
                Style::default().fg(self.theme.colors.accent.to_color()),
            ),
            Span::styled(
                " merge (keep first)  ",
                Style::default().fg(self.theme.colors.muted.to_color()),
            ),
            Span::styled(
                "r",
                Style::default().fg(self.theme.colors.accent.to_color()),
            ),
            Span::styled(
                " refresh",
                Style::default().fg(self.theme.colors.muted.to_color()),
            ),
        ]);

        Paragraph::new(help).render(area, buf);
    }
}

impl Widget for Duplicates<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Duplicates - Potential Duplicate Tasks ")
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 20 || inner.height < 10 {
            return;
        }

        // Layout: header, list, detail panel, help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header
                Constraint::Min(5),    // List
                Constraint::Length(9), // Detail panel
                Constraint::Length(1), // Help
            ])
            .split(inner);

        self.render_header(chunks[0], buf);
        self.render_list(chunks[1], buf);
        self.render_detail(chunks[2], buf);
        self.render_help(chunks[3], buf);
    }
}

/// Truncate a title to a maximum length, adding ellipsis if needed
fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_string()
    } else {
        format!("{}...", &title[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Task;
    use ratatui::buffer::Buffer;

    #[test]
    fn test_duplicates_empty_model() {
        let model = Model::new();
        let theme = Theme::default();
        let duplicates = Duplicates::new(&model, &theme);

        let area = Rect::new(0, 0, 80, 20);
        let mut buffer = Buffer::empty(area);
        duplicates.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_duplicates_renders_without_panic() {
        let mut model = Model::new();

        // Add some tasks
        let task1 = Task::new("Buy groceries");
        let task2 = Task::new("Buy groceries at store");
        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);

        let theme = Theme::default();
        let duplicates = Duplicates::new(&model, &theme);

        let area = Rect::new(0, 0, 100, 30);
        let mut buffer = Buffer::empty(area);
        duplicates.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_duplicates_small_area_does_not_panic() {
        let model = Model::new();
        let theme = Theme::default();
        let duplicates = Duplicates::new(&model, &theme);

        // Very small area - should early return without panic
        let area = Rect::new(0, 0, 10, 5);
        let mut buffer = Buffer::empty(area);
        duplicates.render(area, &mut buffer);

        assert!(buffer.area.width > 0);
    }

    #[test]
    fn test_truncate_title_short() {
        let result = truncate_title("Short title", 20);
        assert_eq!(result, "Short title");
    }

    #[test]
    fn test_truncate_title_long() {
        let result = truncate_title("This is a very long title that needs truncation", 20);
        assert_eq!(result, "This is a very lo...");
    }

    #[test]
    fn test_truncate_title_exact() {
        let result = truncate_title("Exactly twenty chars", 20);
        assert_eq!(result, "Exactly twenty chars");
    }
}
