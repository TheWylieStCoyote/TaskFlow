//! Goal list and detail panel rendering.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use super::GoalsView;

impl GoalsView<'_> {
    pub(crate) fn render_goal_list(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let title = if self.model.goal_view.show_archived {
            " Goals (showing completed) "
        } else {
            " Goals "
        };

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.model.visible_goals.is_empty() {
            let empty_msg = Paragraph::new("No goals yet. Press 'n' to create one.")
                .style(Style::default().fg(theme.colors.muted.to_color()));
            empty_msg.render(inner, buf);
            return;
        }

        // Create list items for each goal
        let items: Vec<ListItem<'_>> = self
            .model
            .visible_goals
            .iter()
            .enumerate()
            .filter_map(|(idx, id)| {
                let goal = self.model.goals.get(id)?;
                let is_selected = idx == self.model.goal_view.selected_goal
                    && self.model.goal_view.expanded_goal.is_none();
                let is_expanded = self.model.goal_view.expanded_goal == Some(*id);

                // Status symbol
                let status_symbol = goal.status.symbol();
                let status_style = if goal.is_complete() {
                    Style::default().fg(theme.colors.success.to_color())
                } else if goal.is_active() {
                    Style::default().fg(theme.colors.accent.to_color())
                } else {
                    Style::default().fg(theme.colors.muted.to_color())
                };

                // Goal name
                let name_style = if is_selected || is_expanded {
                    Style::default()
                        .fg(theme.colors.accent.to_color())
                        .add_modifier(Modifier::BOLD)
                } else if goal.is_complete() {
                    Style::default()
                        .fg(theme.colors.muted.to_color())
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default().fg(theme.colors.foreground.to_color())
                };

                // Progress bar
                let progress = self.model.goal_progress(*id);
                let progress_bar = self.render_progress_bar(progress, 10);

                // Quarter/timeframe
                let timeframe = goal.formatted_timeframe();
                let timeframe_style = Style::default().fg(theme.colors.muted.to_color());

                // Expand/collapse indicator
                let expand_indicator = if is_expanded { "▼ " } else { "▸ " };

                let line = Line::from(vec![
                    Span::styled(
                        expand_indicator,
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                    Span::styled(format!("{status_symbol} "), status_style),
                    Span::styled(&goal.name, name_style),
                    Span::raw(" "),
                    progress_bar,
                    Span::styled(
                        format!(" {progress}%"),
                        Style::default().fg(self.progress_color(progress)),
                    ),
                    Span::raw("  "),
                    Span::styled(timeframe, timeframe_style),
                ]);

                let item = if is_selected || is_expanded {
                    ListItem::new(line).style(
                        Style::default()
                            .bg(theme.colors.accent_secondary.to_color())
                            .fg(theme.colors.foreground.to_color()),
                    )
                } else {
                    ListItem::new(line)
                };

                // If expanded, add key results as nested items
                if is_expanded {
                    // We need to return multiple items for expanded goals
                    // For now, just return the goal item
                }

                Some(item)
            })
            .collect();

        // If a goal is expanded, we need to inject key result items
        let final_items = self.inject_key_results(items);

        let list = List::new(final_items);
        list.render(inner, buf);
    }

    fn inject_key_results<'a>(&'a self, mut items: Vec<ListItem<'a>>) -> Vec<ListItem<'a>> {
        let theme = self.theme;

        if let Some(expanded_id) = self.model.goal_view.expanded_goal {
            // Find the index of the expanded goal in visible_goals
            if let Some(goal_idx) = self
                .model
                .visible_goals
                .iter()
                .position(|id| *id == expanded_id)
            {
                let krs = self.model.key_results_for_goal(expanded_id);

                // Insert key results after the goal item
                let insert_pos = goal_idx + 1;
                let kr_items: Vec<ListItem<'a>> = krs
                    .iter()
                    .enumerate()
                    .map(|(idx, kr)| {
                        let is_selected = idx == self.model.goal_view.selected_kr;

                        // Status symbol
                        let status_symbol = kr.status.symbol();
                        let status_style = if kr.is_complete() {
                            Style::default().fg(theme.colors.success.to_color())
                        } else if kr.status.is_in_progress() {
                            Style::default().fg(theme.colors.accent.to_color())
                        } else {
                            Style::default().fg(theme.colors.muted.to_color())
                        };

                        // Key result name
                        let name_style = if is_selected {
                            Style::default()
                                .fg(theme.colors.accent.to_color())
                                .add_modifier(Modifier::BOLD)
                        } else if kr.is_complete() {
                            Style::default()
                                .fg(theme.colors.muted.to_color())
                                .add_modifier(Modifier::CROSSED_OUT)
                        } else {
                            Style::default().fg(theme.colors.foreground.to_color())
                        };

                        // Progress
                        let progress = self.model.key_result_progress(kr.id);
                        let progress_bar = self.render_progress_bar(progress, 8);

                        // Formatted progress (e.g., "45/100 users")
                        let formatted = kr.formatted_progress();

                        let line = Line::from(vec![
                            Span::raw("    "), // Indentation
                            Span::styled(format!("{status_symbol} "), status_style),
                            Span::styled(kr.name.clone(), name_style),
                            Span::raw(" "),
                            progress_bar,
                            Span::styled(
                                format!(" {progress}%"),
                                Style::default().fg(self.progress_color(progress)),
                            ),
                            Span::raw("  "),
                            Span::styled(
                                formatted,
                                Style::default().fg(theme.colors.muted.to_color()),
                            ),
                        ]);

                        if is_selected {
                            ListItem::new(line).style(
                                Style::default()
                                    .bg(theme.colors.accent_secondary.to_color())
                                    .fg(theme.colors.foreground.to_color()),
                            )
                        } else {
                            ListItem::new(line)
                        }
                    })
                    .collect();

                // Insert at the correct position
                for (i, item) in kr_items.into_iter().enumerate() {
                    if insert_pos + i <= items.len() {
                        items.insert(insert_pos + i, item);
                    } else {
                        items.push(item);
                    }
                }
            }
        }

        items
    }

    fn render_progress_bar(&self, progress: u8, width: usize) -> Span<'_> {
        let filled = (progress as usize * width / 100).min(width);
        let empty = width - filled;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
        Span::styled(bar, Style::default().fg(self.progress_color(progress)))
    }

    pub(crate) fn render_detail_panel(&self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let block = Block::default()
            .title(" Details ")
            .title_style(
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border.to_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Get selected goal
        let Some(goal) = self.model.selected_goal() else {
            let empty_msg = Paragraph::new("Select a goal to view details")
                .style(Style::default().fg(theme.colors.muted.to_color()));
            empty_msg.render(inner, buf);
            return;
        };

        // Goal name (header), empty line, status, and timeframe
        let mut lines: Vec<Line<'_>> = vec![
            Line::from(vec![Span::styled(
                &goal.name,
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            // Status
            Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    goal.status.to_string(),
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
            ]),
            // Timeframe
            Line::from(vec![
                Span::styled(
                    "Timeframe: ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(
                    goal.formatted_timeframe(),
                    Style::default().fg(theme.colors.foreground.to_color()),
                ),
            ]),
        ];

        // Progress
        let progress = self.model.goal_progress(goal.id);
        let progress_bar = self.render_progress_bar(progress, 15);
        lines.push(Line::from(vec![
            Span::styled(
                "Progress: ",
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            progress_bar,
            Span::styled(
                format!(" {progress}%"),
                Style::default().fg(self.progress_color(progress)),
            ),
        ]));

        // Description
        if let Some(desc) = &goal.description {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Description:",
                Style::default()
                    .fg(theme.colors.muted.to_color())
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(theme.colors.foreground.to_color()),
            )));
        }

        // Key Results section
        let krs = self.model.key_results_for_goal(goal.id);
        if !krs.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Key Results ({}):", krs.len()),
                Style::default()
                    .fg(theme.colors.muted.to_color())
                    .add_modifier(Modifier::BOLD),
            )));

            for kr in krs.iter().take(5) {
                // Show first 5 KRs
                let kr_progress = self.model.key_result_progress(kr.id);
                let status_symbol = kr.status.symbol();

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {status_symbol} "),
                        Style::default().fg(theme.colors.muted.to_color()),
                    ),
                    Span::styled(
                        &kr.name,
                        Style::default().fg(theme.colors.foreground.to_color()),
                    ),
                    Span::styled(
                        format!(" ({kr_progress}%)"),
                        Style::default().fg(self.progress_color(kr_progress)),
                    ),
                ]));
            }

            if krs.len() > 5 {
                lines.push(Line::from(Span::styled(
                    format!("  ... and {} more", krs.len() - 5),
                    Style::default().fg(theme.colors.muted.to_color()),
                )));
            }
        }

        // Help text at bottom
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "n: new goal | N: new KR | Enter: expand | d: delete",
            Style::default().fg(theme.colors.muted.to_color()),
        )));

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}
