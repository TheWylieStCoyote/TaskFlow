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
        let results = filter_tasks(query, model.tasks.values(), project_name_fn, threshold);
        Self {
            query,
            cursor: model.quick_open.cursor,
            selected: model.quick_open.selected,
            results,
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
                let proj_display_width = proj_str.len().min(20);
                let title_max = available_width.saturating_sub(proj_display_width + 4);

                let title_display = if entry.title.len() > title_max {
                    format!("{}…", &entry.title[..title_max.saturating_sub(1)])
                } else {
                    entry.title.clone()
                };

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

/// Count filtered results for a given query without constructing the full widget.
///
/// Used by the update handler to clamp `selected` on Down.
#[must_use]
pub fn get_quick_open_result_count(model: &Model) -> usize {
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
    filter_tasks(query, model.tasks.values(), project_name_fn, threshold).len()
}

/// Resolve the task ID currently highlighted in the quick-open picker.
///
/// Called by the input handler before closing the picker so it can
/// navigate to the correct task.
#[must_use]
pub fn get_quick_open_task_id(model: &Model) -> Option<crate::domain::TaskId> {
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
        .get(model.quick_open.selected)
        .map(|e| e.task_id)
}
