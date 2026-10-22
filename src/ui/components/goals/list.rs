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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Model;
    use crate::config::Theme;
    use crate::domain::{Goal, GoalStatus, KeyResult, KeyResultStatus, Quarter};
    use crate::ui::test_utils::{buffer_content, render_widget};

    fn render(model: &Model) -> String {
        let theme = Theme::default();
        buffer_content(&render_widget(GoalsView::new(model, &theme), 140, 30))
    }

    fn model_with_goal() -> (Model, crate::domain::GoalId) {
        let mut model = Model::new();
        let mut goal = Goal::new("Grow the user base");
        goal.quarter = Some((2030, Quarter::Q2));
        let goal_id = goal.id;
        model.goals.insert(goal_id, goal);
        model.refresh_visible_goals();
        (model, goal_id)
    }

    #[test]
    fn test_empty_goal_list_and_detail_messages() {
        let model = Model::new();
        let content = render(&model);
        assert!(content.contains(" Goals "));
        assert!(content.contains("No goals yet. Press 'n' to create one."));
        assert!(content.contains("Select a goal to view details"));
    }

    #[test]
    fn test_show_archived_changes_title() {
        let (mut model, _) = model_with_goal();
        model.goal_view.show_archived = true;
        let content = render(&model);
        assert!(content.contains("Goals (showing completed)"));
    }

    #[test]
    fn test_goal_list_shows_progress_and_timeframe() {
        let (mut model, goal_id) = model_with_goal();
        model.goals.get_mut(&goal_id).unwrap().manual_progress = Some(60);

        let content = render(&model);

        assert!(content.contains("▸ "));
        assert!(content.contains("Grow the user base"));
        assert!(content.contains(" 60%"));
        assert!(content.contains(&model.goals[&goal_id].formatted_timeframe()));
        assert!(content.contains("Status: "));
        assert!(content.contains("Timeframe: "));
        assert!(content.contains("Progress: "));
        assert!(content.contains("n: new goal | N: new KR | Enter: expand | d: delete"));
    }

    #[test]
    fn test_completed_goal_renders_with_status_symbol() {
        let (mut model, goal_id) = model_with_goal();
        model.goals.get_mut(&goal_id).unwrap().status = GoalStatus::Completed;
        model.goal_view.show_archived = true;
        model.refresh_visible_goals();

        let content = render(&model);
        assert!(content.contains("Grow the user base"));
        assert!(content.contains(&format!("Status: {}", GoalStatus::Completed)));
    }

    #[test]
    fn test_expanded_goal_injects_key_results() {
        let (mut model, goal_id) = model_with_goal();
        let mut active = KeyResult::new(goal_id, "Active KR").with_target(100.0, Some("users"));
        active.set_value(45.0);
        active.status = KeyResultStatus::InProgress;
        let mut done = KeyResult::new(goal_id, "Done KR");
        done.status = KeyResultStatus::Completed;
        done.manual_progress = Some(100);
        let pending = KeyResult::new(goal_id, "Pending KR");
        model.key_results.insert(active.id, active);
        model.key_results.insert(done.id, done);
        model.key_results.insert(pending.id, pending);
        model.goal_view.expanded_goal = Some(goal_id);
        model.goal_view.selected_kr = 1;

        let content = render(&model);

        assert!(content.contains("▼ "), "expanded indicator");
        assert!(content.contains("    "), "key results are indented");
        assert!(content.contains("Active KR"));
        assert!(content.contains("Done KR"));
        assert!(content.contains("Pending KR"));
        assert!(content.contains(" 45%"));
        assert!(content.contains("45/100 users"));
        assert!(content.contains("Key Results (3):"));
        assert!(content.contains("Active KR (45%)"));
        assert!(content.contains("Done KR (100%)"));
        assert!(content.contains("Pending KR (0%)"));
    }

    #[test]
    fn test_expanded_goal_with_many_key_results_appends_overflow() {
        let (mut model, goal_id) = model_with_goal();
        for i in 0..7 {
            let kr = KeyResult::new(goal_id, format!("KR number {i}"));
            model.key_results.insert(kr.id, kr);
        }
        model.goal_view.expanded_goal = Some(goal_id);

        let content = render(&model);

        // All seven appear in the list on the left...
        for i in 0..7 {
            assert!(content.contains(&format!("KR number {i}")), "KR {i} listed");
        }
        // ...while the detail panel caps at five and reports the rest
        assert!(content.contains("Key Results (7):"));
        assert!(content.contains("... and 2 more"));
    }

    #[test]
    fn test_expanded_goal_not_visible_does_not_inject() {
        let (mut model, goal_id) = model_with_goal();
        let kr = KeyResult::new(goal_id, "Hidden KR");
        model.key_results.insert(kr.id, kr);
        model.goal_view.expanded_goal = Some(crate::domain::GoalId::new());

        let content = render(&model);

        // The detail panel still lists the selected goal's key results, but no
        // goal row is expanded in the list on the left.
        assert!(content.contains("Key Results (1):"));
        assert!(!content.contains("▼ "), "no expanded indicator");
        assert!(content.contains("▸ "));
    }

    #[test]
    fn test_second_goal_expanded_inserts_after_it() {
        let (mut model, first_id) = model_with_goal();
        let second = Goal::new("Zebra goal");
        let second_id = second.id;
        model.goals.insert(second_id, second);
        model.refresh_visible_goals();
        let kr = KeyResult::new(second_id, "Zebra KR");
        model.key_results.insert(kr.id, kr);
        model.goal_view.expanded_goal = Some(second_id);
        model.goal_view.selected_goal = model
            .visible_goals
            .iter()
            .position(|id| *id == second_id)
            .unwrap();

        let content = render(&model);

        let zebra_pos = content.find("Zebra goal").expect("goal rendered");
        let kr_pos = content.find("Zebra KR").expect("key result rendered");
        assert!(kr_pos > zebra_pos, "key result follows its goal");
        assert!(content.contains("Grow the user base"));
        let _ = first_id;
    }

    #[test]
    fn test_detail_panel_shows_description() {
        let (mut model, goal_id) = model_with_goal();
        model.goals.get_mut(&goal_id).unwrap().description =
            Some("Reach ten thousand weekly actives".to_string());

        let content = render(&model);

        assert!(content.contains("Description:"));
        assert!(content.contains("Reach ten thousand weekly actives"));
    }

    #[test]
    fn test_progress_colors_by_band() {
        let model = Model::new();
        let theme = Theme::default();
        let view = GoalsView::new(&model, &theme);
        assert_eq!(view.progress_color(90), theme.colors.success.to_color());
        assert_eq!(view.progress_color(50), theme.colors.accent.to_color());
        assert_eq!(view.progress_color(30), theme.colors.warning.to_color());
        assert_eq!(view.progress_color(5), theme.colors.danger.to_color());
    }
}
