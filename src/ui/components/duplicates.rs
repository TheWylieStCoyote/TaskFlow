//! Duplicates view component - shows potential duplicate task pairs.
//!
//! Displays tasks that may be duplicates based on fuzzy title matching,
//! allowing users to review and merge or dismiss duplicate pairs.

use crate::domain::text::truncate_chars;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget,
    },
};

use crate::app::Model;
use crate::config::Theme;
use crate::domain::plural_s;

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
                    plural_s(pair_count)
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
        let scroll_offset = self.model.duplicates_view.scroll_offset;

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

        let total_pairs = pairs.len();
        let viewport_height = area.height as usize;
        let scroll_offset = scroll_offset.min(total_pairs.saturating_sub(viewport_height));

        let items: Vec<ListItem<'_>> = pairs
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(viewport_height)
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

        // Render scrollbar if content exceeds viewport
        if total_pairs > viewport_height {
            let mut scrollbar_state =
                ScrollbarState::new(total_pairs.saturating_sub(1)).position(selected);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .track_style(Style::default().fg(self.theme.colors.muted.to_color()))
                .thumb_style(Style::default().fg(self.theme.colors.accent.to_color()));

            StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
        }
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
    truncate_chars(title, max_len, "...")
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

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::domain::duplicate_detector::DuplicatePair;
    use crate::domain::{Priority, Project, Task, TaskId, TaskStatus};
    use crate::ui::test_utils::{buffer_content, render_widget};

    fn render(model: &Model, width: u16, height: u16) -> String {
        let theme = Theme::default();
        buffer_content(&render_widget(
            Duplicates::new(model, &theme),
            width,
            height,
        ))
    }

    fn pair(a: TaskId, b: TaskId, similarity: f64) -> DuplicatePair {
        DuplicatePair {
            task1_id: a,
            task2_id: b,
            similarity,
        }
    }

    fn model_with_pairs() -> (Model, Vec<TaskId>) {
        let mut model = Model::new();
        let project = Project::new("Chores");
        let project_id = project.id;
        model.projects.insert(project_id, project);

        let mut a = Task::new("Buy groceries");
        a.project_id = Some(project_id);
        a.priority = Priority::High;
        let mut b = Task::new("Buy the groceries");
        b.status = TaskStatus::InProgress;
        let c = Task::new("Call mom");
        let d = Task::new("Call mum");
        let ids = vec![a.id, b.id, c.id, d.id];
        for task in [a, b, c, d] {
            model.tasks.insert(task.id, task);
        }
        model.duplicates_view.pairs = vec![pair(ids[0], ids[1], 0.96), pair(ids[2], ids[3], 0.91)];
        model.duplicates_view.threshold = 0.85;
        (model, ids)
    }

    #[test]
    fn test_header_counts_pairs_and_threshold() {
        let (model, _) = model_with_pairs();
        let content = render(&model, 120, 30);
        assert!(content.contains("2 potential duplicate pairs found (threshold: 85%)"));
    }

    #[test]
    fn test_header_singular_pair() {
        let (mut model, _) = model_with_pairs();
        model.duplicates_view.pairs.truncate(1);
        let content = render(&model, 120, 30);
        assert!(content.contains("1 potential duplicate pair found"));
    }

    #[test]
    fn test_empty_list_message() {
        let model = Model::new();
        let content = render(&model, 100, 30);
        assert!(content.contains("No duplicate tasks found!"));
        assert!(!content.contains("Selected Pair Details"));
    }

    #[test]
    fn test_list_rows_show_similarity_project_and_titles() {
        let (model, _) = model_with_pairs();
        let content = render(&model, 120, 30);
        assert!(content.contains(" 96% [Chores] Buy groceries <-> Buy the groceries"));
        assert!(content.contains(" 91% [Inbox] Call mom <-> Call mum"));
    }

    #[test]
    fn test_detail_panel_for_selected_pair() {
        let (mut model, _) = model_with_pairs();
        model.duplicates_view.selected = 1;
        let content = render(&model, 120, 30);
        assert!(content.contains("Selected Pair Details"));
        assert!(content.contains("Task 1: Call mom"));
        assert!(content.contains("Task 2: Call mum"));
        assert!(content.contains("Similarity: 91.0%"));
        assert!(content.contains("Status: Todo"));
        assert!(content.contains("Priority: None"));
    }

    #[test]
    fn test_detail_panel_shows_status_and_priority_of_first_pair() {
        let (model, _) = model_with_pairs();
        let content = render(&model, 120, 30);
        assert!(content.contains("Task 1: Buy groceries"));
        assert!(content.contains("Priority: High"));
        assert!(content.contains("Status: InProgress"));
        assert!(content.contains("Similarity: 96.0%"));
    }

    #[test]
    fn test_pair_with_deleted_task_renders_placeholder_and_no_detail() {
        let (mut model, ids) = model_with_pairs();
        model.tasks.remove(&ids[1]);
        model.duplicates_view.selected = 0;
        let content = render(&model, 120, 30);
        assert!(content.contains("[Unknown] [deleted] <-> [deleted]"));
        assert!(!content.contains("Selected Pair Details"));
    }

    #[test]
    fn test_selected_index_past_end_skips_detail() {
        let (mut model, _) = model_with_pairs();
        model.duplicates_view.selected = 10;
        let content = render(&model, 120, 30);
        assert!(!content.contains("Selected Pair Details"));
        assert!(content.contains("Buy groceries"));
    }

    #[test]
    fn test_long_titles_are_truncated_in_list() {
        let mut model = Model::new();
        let a = Task::new("This is an extremely long task title that goes on and on");
        let b = Task::new("This is an extremely long task title that goes on and onward");
        let ids = [a.id, b.id];
        model.tasks.insert(a.id, a);
        model.tasks.insert(b.id, b);
        model.duplicates_view.pairs = vec![pair(ids[0], ids[1], 0.88)];
        let content = render(&model, 140, 30);
        assert!(
            content.contains("This is an extremely long t... <-> This is an extremely long t...")
        );
    }

    #[test]
    fn test_scroll_offset_hides_earlier_rows_and_shows_scrollbar() {
        let mut model = Model::new();
        let mut ids = Vec::new();
        for i in 0..30 {
            let a = Task::new(format!("Row {i:02} left"));
            let b = Task::new(format!("Row {i:02} right"));
            ids.push((a.id, b.id));
            model.tasks.insert(a.id, a);
            model.tasks.insert(b.id, b);
        }
        model.duplicates_view.pairs = ids.iter().map(|(a, b)| pair(*a, *b, 0.9)).collect();
        model.duplicates_view.selected = 20;
        model.duplicates_view.scroll_offset = 15;

        let content = render(&model, 120, 30);

        assert!(!content.contains("Row 00 left"));
        assert!(content.contains("Row 15 left"));
        assert!(
            content.contains("▲") && content.contains("▼"),
            "scrollbar rendered"
        );
    }

    #[test]
    fn test_help_footer_lists_keys() {
        let (model, _) = model_with_pairs();
        let content = render(&model, 120, 30);
        assert!(content.contains("j/k navigate  D dismiss  M merge (keep first)  r refresh"));
    }
}
