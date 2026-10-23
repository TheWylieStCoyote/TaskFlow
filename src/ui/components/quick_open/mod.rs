//! Fuzzy quick-open task picker component.
//!
//! A floating overlay that lets users type to fuzzy-filter all tasks and
//! jump to the selected task. Modeled on Telescope (Neovim) / VS Code Ctrl+P.
//!
//! # Usage
//!
//! Open with `Alt+Space`. Type to narrow results, `↑`/`↓` (or `Ctrl+k`/`Ctrl+j`)
//! to navigate, `Enter` to jump, `Esc` to dismiss.

mod scoring;

pub use scoring::{filter_tasks, score_task, QuickOpenEntry, QUICK_OPEN_THRESHOLD};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget},
};

use crate::app::Model;
use crate::config::Theme;

/// Fuzzy quick-open task picker widget.
///
/// Renders a floating overlay with a search input and a scrollable ranked
/// list of matching tasks. Built from model state — no mutable access needed.
pub struct QuickOpen<'a> {
    query: &'a str,
    cursor: usize,
    selected: usize,
    results: Vec<QuickOpenEntry>,
    theme: &'a Theme,
}

impl<'a> QuickOpen<'a> {
    /// Build the widget from current model state.
    ///
    /// Computes the filtered and ranked result list immediately so that
    /// `result_count()` and `selected_task_id()` are available before render.
    #[must_use]
    pub fn new(model: &'a Model, theme: &'a Theme) -> Self {
        Self {
            query: &model.quick_open.query,
            cursor: model.quick_open.cursor,
            selected: model.quick_open.selected,
            results: ranked_entries(model),
            theme,
        }
    }

    /// Number of results after filtering.
    #[must_use]
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Task ID of the currently highlighted result, if any.
    #[must_use]
    pub fn selected_task_id(&self) -> Option<crate::domain::TaskId> {
        self.results.get(self.selected).map(|e| e.task_id)
    }
}

impl Widget for QuickOpen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Wipe the background so we draw over whatever is behind the overlay.
        Clear.render(area, buf);

        let block = Block::default()
            .title(" Quick Open ")
            .title_style(
                Style::default()
                    .fg(self.theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 3 || inner.width < 20 {
            return;
        }

        // Layout: search input | separator | result list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Search input
                Constraint::Length(1), // Separator
                Constraint::Min(1),    // Result list
            ])
            .split(inner);

        // Render search bar with inline cursor indicator.
        let display_query = if self.cursor >= self.query.len() {
            format!("{}|", self.query)
        } else {
            let (before, after) = self.query.split_at(self.cursor);
            format!("{before}|{after}")
        };
        let search_line = Line::from(vec![
            Span::styled(
                "> ",
                Style::default().fg(self.theme.colors.accent.to_color()),
            ),
            Span::styled(
                display_query,
                Style::default().fg(self.theme.colors.foreground.to_color()),
            ),
        ]);
        Paragraph::new(search_line).render(chunks[0], buf);

        // Separator rule.
        let sep = "\u{2500}".repeat(inner.width as usize);
        Paragraph::new(sep)
            .style(Style::default().fg(self.theme.colors.border.to_color()))
            .render(chunks[1], buf);

        // Empty-state message.
        if self.results.is_empty() {
            let msg = if self.query.is_empty() {
                "No tasks"
            } else {
                "No matching tasks"
            };
            let no_match = Line::from(Span::styled(
                msg,
                Style::default()
                    .fg(self.theme.colors.muted.to_color())
                    .add_modifier(Modifier::ITALIC),
            ));
            Paragraph::new(no_match).render(chunks[2], buf);
            return;
        }

        // Scrollable result list.
        let visible_height = chunks[2].height as usize;
        let scroll_offset = if self.selected >= visible_height {
            self.selected - visible_height + 1
        } else {
            0
        };

        let available_width = inner.width as usize;
        let items: Vec<ListItem<'_>> = self
            .results
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|(i, entry)| {
                let is_selected = i == self.selected;

                // Right-align project name; truncate title to fill remaining width.
                let proj_str = entry.project_name.as_deref().unwrap_or("");
                let proj_display_width = proj_str.chars().count().min(20);
                let title_max = available_width.saturating_sub(proj_display_width + 4);

                let title_display = truncate_chars(&entry.title, title_max);

                let line = Line::from(vec![
                    Span::styled(
                        format!("{title_display:<title_max$}"),
                        Style::default().fg(self.theme.colors.foreground.to_color()),
                    ),
                    Span::styled(
                        format!("  {proj_str:>proj_display_width$}"),
                        Style::default().fg(self.theme.colors.muted.to_color()),
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

        List::new(items).render(chunks[2], buf);
    }
}

/// Shorten `text` to at most `max_chars` characters, ending in an ellipsis.
///
/// Counts characters rather than bytes so multi-byte titles never split
/// inside a code point.
fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// Rank every task against the picker's current query.
///
/// An empty query lists all tasks (threshold `0.0`); otherwise tasks below
/// [`QUICK_OPEN_THRESHOLD`] are dropped.
fn ranked_entries(model: &Model) -> Vec<QuickOpenEntry> {
    let query = &model.quick_open.query;
    let project_name_fn = |task: &crate::domain::Task| {
        task.project_id
            .as_ref()
            .and_then(|id| model.projects.get(id))
            .map(|p| p.name.clone())
    };
    let threshold = if query.is_empty() {
        0.0
    } else {
        QUICK_OPEN_THRESHOLD
    };
    filter_tasks(query, model.tasks.values(), project_name_fn, threshold)
}

/// Count filtered results for a given query without constructing the full widget.
///
/// Used by the update handler to clamp `selected` on Down.
#[must_use]
pub fn get_quick_open_result_count(model: &Model) -> usize {
    ranked_entries(model).len()
}

/// Resolve the task ID currently highlighted in the quick-open picker.
///
/// Called by the update handler on Enter so it can navigate to the task
/// before closing the picker.
#[must_use]
pub fn get_quick_open_task_id(model: &Model) -> Option<crate::domain::TaskId> {
    ranked_entries(model)
        .get(model.quick_open.selected)
        .map(|e| e.task_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Project, Task};

    fn render_to_text(model: &Model, width: u16, height: u16) -> String {
        let theme = Theme::default();
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        QuickOpen::new(model, &theme).render(area, &mut buf);
        let mut text = String::new();
        for y in 0..height {
            for x in 0..width {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    fn model_with(titles: &[&str]) -> Model {
        let mut model = Model::new();
        for title in titles {
            let task = Task::new(*title);
            model.tasks.insert(task.id, task);
        }
        model.quick_open.visible = true;
        model
    }

    #[test]
    fn truncate_chars_cuts_on_char_boundaries() {
        assert_eq!(truncate_chars("short", 10), "short");
        assert_eq!(truncate_chars("exact", 5), "exact");
        let cut = truncate_chars("Café au lait très long", 6);
        assert_eq!(cut, "Café \u{2026}");
        assert_eq!(cut.chars().count(), 6);
        assert_eq!(truncate_chars("abc", 0), "\u{2026}");
    }

    #[test]
    fn renders_title_query_cursor_and_results() {
        let mut model = model_with(&["Alpha", "Beta"]);
        let project = Project::new("Work");
        let project_id = project.id;
        model.projects.insert(project_id, project);
        let beta = model
            .tasks
            .values_mut()
            .find(|t| t.title == "Beta")
            .unwrap();
        beta.project_id = Some(project_id);

        let text = render_to_text(&model, 60, 17);
        assert!(text.contains("Quick Open"), "{text}");
        assert!(text.contains("> |"), "{text}");
        assert!(text.contains("Alpha"), "{text}");
        assert!(text.contains("Beta"), "{text}");
        assert!(text.contains("Work"), "{text}");
    }

    #[test]
    fn renders_cursor_inside_query() {
        let mut model = model_with(&["Alpha"]);
        model.quick_open.query = "alp".to_string();
        model.quick_open.cursor = 1;
        let text = render_to_text(&model, 60, 17);
        assert!(text.contains("> a|lp"), "{text}");
    }

    #[test]
    fn renders_empty_states() {
        let model = model_with(&[]);
        assert!(render_to_text(&model, 60, 17).contains("No tasks"));

        let mut model = model_with(&["Alpha"]);
        model.quick_open.query = "zzqx".to_string();
        model.quick_open.cursor = 4;
        assert!(render_to_text(&model, 60, 17).contains("No matching tasks"));
    }

    #[test]
    fn long_multibyte_title_is_truncated_without_panicking() {
        let title: String = "é".repeat(80);
        let model = model_with(&[title.as_str()]);
        let text = render_to_text(&model, 40, 17);
        assert!(text.contains('\u{2026}'), "{text}");
    }

    #[test]
    fn new_exposes_result_count_and_selected_task_id() {
        let mut model = model_with(&["Beta", "Alpha"]);
        let theme = Theme::default();
        let widget = QuickOpen::new(&model, &theme);
        assert_eq!(widget.result_count(), 2);
        let alpha = model
            .tasks
            .values()
            .find(|t| t.title == "Alpha")
            .unwrap()
            .id;
        assert_eq!(widget.selected_task_id(), Some(alpha));

        model.quick_open.selected = 5;
        assert_eq!(QuickOpen::new(&model, &theme).selected_task_id(), None);
    }

    #[test]
    fn scrolls_to_keep_selection_visible() {
        let titles: Vec<String> = (0..20).map(|i| format!("Task {i:02}")).collect();
        let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
        let mut model = model_with(&refs);
        model.quick_open.selected = 19;
        let text = render_to_text(&model, 60, 17);
        assert!(text.contains("Task 19"), "{text}");
        assert!(!text.contains("Task 00"), "{text}");
    }

    #[test]
    fn tiny_area_renders_only_the_frame() {
        let model = model_with(&["Alpha"]);
        let text = render_to_text(&model, 12, 3);
        assert!(!text.contains("Alpha"));
    }
}
