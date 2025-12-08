//! Reports view component for analytics display.
//!
//! This module provides the reports view widget that displays analytics
//! and statistics about tasks using chart widgets.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Widget},
};

use crate::app::{analytics::AnalyticsEngine, Model};
use crate::domain::analytics::ReportConfig;

use super::charts::{BarChart, ProgressGauge, Sparkline, StatBox};

/// The currently selected report panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportPanel {
    #[default]
    Overview,
    Velocity,
    Tags,
    Time,
    Focus,
    Insights,
    Estimation,
}

impl ReportPanel {
    /// Get the next panel (wrapping).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Overview => Self::Velocity,
            Self::Velocity => Self::Tags,
            Self::Tags => Self::Time,
            Self::Time => Self::Focus,
            Self::Focus => Self::Insights,
            Self::Insights => Self::Estimation,
            Self::Estimation => Self::Overview,
        }
    }

    /// Get the previous panel (wrapping).
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Overview => Self::Estimation,
            Self::Velocity => Self::Overview,
            Self::Tags => Self::Velocity,
            Self::Time => Self::Tags,
            Self::Focus => Self::Time,
            Self::Insights => Self::Focus,
            Self::Estimation => Self::Insights,
        }
    }

    /// Get the panel index.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Overview => 0,
            Self::Velocity => 1,
            Self::Tags => 2,
            Self::Time => 3,
            Self::Focus => 4,
            Self::Insights => 5,
            Self::Estimation => 6,
        }
    }

    /// Get panel names for tabs.
    #[must_use]
    pub const fn names() -> [&'static str; 7] {
        [
            "Overview",
            "Velocity",
            "Tags",
            "Time",
            "Focus",
            "Insights",
            "Estimation",
        ]
    }
}

/// Reports view widget.
pub struct ReportsView<'a> {
    model: &'a Model,
    selected_panel: ReportPanel,
}

impl<'a> ReportsView<'a> {
    /// Create a new reports view.
    #[must_use]
    pub const fn new(model: &'a Model, selected_panel: ReportPanel) -> Self {
        Self {
            model,
            selected_panel,
        }
    }
}

impl Widget for ReportsView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render outer border
        let block = Block::default()
            .title(" Reports ")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 20 || inner.height < 10 {
            return;
        }

        // Split into tabs area and content area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner);

        // Render tabs
        let tab_titles: Vec<Line<'_>> = ReportPanel::names()
            .iter()
            .map(|t| Line::from(*t))
            .collect();
        let tabs = Tabs::new(tab_titles)
            .select(self.selected_panel.index())
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .divider(" | ");
        tabs.render(chunks[0], buf);

        // Render selected panel
        match self.selected_panel {
            ReportPanel::Overview => self.render_overview(chunks[1], buf),
            ReportPanel::Velocity => self.render_velocity(chunks[1], buf),
            ReportPanel::Tags => self.render_tags(chunks[1], buf),
            ReportPanel::Time => self.render_time(chunks[1], buf),
            ReportPanel::Focus => self.render_focus(chunks[1], buf),
            ReportPanel::Insights => self.render_insights(chunks[1], buf),
            ReportPanel::Estimation => self.render_estimation(chunks[1], buf),
        }
    }
}

impl ReportsView<'_> {
    fn render_overview(&self, area: Rect, buf: &mut Buffer) {
        let engine = AnalyticsEngine::new(self.model);
        let config = ReportConfig::last_n_days(30);
        let report = engine.generate_report(&config);

        // Split into stat boxes row and charts
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stat boxes
                Constraint::Min(0),    // Charts
            ])
            .split(area);

        // Render stat boxes
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(chunks[0]);

        // Total tasks
        let total_str = report.status_breakdown.total().to_string();
        let total_stat = StatBox::new("Total", &total_str);
        total_stat.render(stat_chunks[0], buf);

        // Done tasks
        let done_str = report.status_breakdown.done.to_string();
        let done_stat = StatBox::new("Done", &done_str).trend(report.velocity.trend);
        done_stat.render(stat_chunks[1], buf);

        // In Progress
        let progress_str = report.status_breakdown.in_progress.to_string();
        let progress_stat = StatBox::new("In Progress", &progress_str);
        progress_stat.render(stat_chunks[2], buf);

        // Completion rate
        let rate = (report.status_breakdown.completion_rate() * 100.0).round() as u32;
        let rate_str = format!("{rate}%");
        let rate_stat = StatBox::new("Complete", &rate_str);
        rate_stat.render(stat_chunks[3], buf);

        // Render progress gauge
        if chunks[1].height > 1 {
            let gauge_area = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, 1);
            let gauge = ProgressGauge::new(
                "Overall Progress",
                report.status_breakdown.completion_rate(),
            );
            gauge.render(gauge_area, buf);
        }

        // Render priority breakdown as bar chart
        if chunks[1].height > 3 {
            let chart_area = Rect::new(
                chunks[1].x,
                chunks[1].y + 2,
                chunks[1].width,
                chunks[1].height.saturating_sub(2),
            );

            let priority_data = vec![
                ("Urgent".to_string(), report.priority_breakdown.urgent),
                ("High".to_string(), report.priority_breakdown.high),
                ("Medium".to_string(), report.priority_breakdown.medium),
                ("Low".to_string(), report.priority_breakdown.low),
                ("None".to_string(), report.priority_breakdown.none),
            ];

            let chart = BarChart::new("Priority Distribution", &priority_data);
            chart.render(chart_area, buf);
        }
    }

    fn render_velocity(&self, area: Rect, buf: &mut Buffer) {
        let engine = AnalyticsEngine::new(self.model);
        let config = ReportConfig::last_n_days(60);
        let report = engine.generate_report(&config);

        // Split vertically
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Sparkline
                Constraint::Length(4), // Stats
                Constraint::Min(0),    // Weekly bar chart
            ])
            .split(area);

        // Velocity sparkline
        let velocity_values: Vec<f64> = report
            .velocity
            .weekly_velocity
            .iter()
            .map(|(_, v)| *v as f64)
            .collect();

        if !velocity_values.is_empty() {
            let spark = Sparkline::new("Weekly Velocity Trend", &velocity_values).line_color(
                if report.velocity.is_improving() {
                    Color::Green
                } else {
                    Color::Red
                },
            );
            spark.render(chunks[0], buf);
        }

        // Velocity stats
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[1]);

        let avg_str = format!("{:.1}", report.velocity.avg_weekly);
        let avg_stat = StatBox::new("Avg/Week", &avg_str).trend(report.velocity.trend);
        avg_stat.render(stat_chunks[0], buf);

        if let Some((_, best)) = report.velocity.best_week() {
            let best_str = best.to_string();
            let best_stat = StatBox::new("Best Week", &best_str);
            best_stat.render(stat_chunks[1], buf);
        }

        let trend_str = if report.velocity.trend > 0.0 {
            "Improving"
        } else if report.velocity.trend < 0.0 {
            "Declining"
        } else {
            "Stable"
        };
        let trend_stat = StatBox::new("Trend", trend_str);
        trend_stat.render(stat_chunks[2], buf);

        // Weekly bar chart
        let weekly_data: Vec<(String, u32)> = report
            .velocity
            .weekly_velocity
            .iter()
            .rev()
            .take(8)
            .rev()
            .map(|(date, v)| (date.format("W%U").to_string(), *v))
            .collect();

        if !weekly_data.is_empty() && chunks[2].height > 3 {
            let chart = BarChart::new("Weekly Completions", &weekly_data).bar_color(Color::Blue);
            chart.render(chunks[2], buf);
        }
    }

    fn render_tags(&self, area: Rect, buf: &mut Buffer) {
        let engine = AnalyticsEngine::new(self.model);
        let config = ReportConfig::last_n_days(30);
        let report = engine.generate_report(&config);

        // Split into header and chart
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area);

        // Header
        let header = Paragraph::new(Line::from(vec![
            Span::styled("Tag Statistics ", Style::default().fg(Color::White)),
            Span::styled(
                format!("({} unique tags)", report.tag_stats.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        header.render(chunks[0], buf);

        // Tag bar chart
        let tag_data: Vec<(String, u32)> = report
            .tag_stats
            .iter()
            .take(10)
            .map(|t| (t.tag.clone(), t.count))
            .collect();

        if !tag_data.is_empty() {
            let chart =
                BarChart::new("Top Tags by Task Count", &tag_data).bar_color(Color::Magenta);
            chart.render(chunks[1], buf);
        } else {
            let msg =
                Paragraph::new("No tags found. Add tags to your tasks to see statistics here.");
            msg.render(chunks[1], buf);
        }
    }

    fn render_time(&self, area: Rect, buf: &mut Buffer) {
        let engine = AnalyticsEngine::new(self.model);
        let config = ReportConfig::last_n_days(30);
        let report = engine.generate_report(&config);

        // Split vertically
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stats
                Constraint::Min(0),    // Charts
            ])
            .split(area);

        // Time stats
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[0]);

        let total_hours = report.time_analytics.total_hours();
        let hours_str = format!("{total_hours:.1}");
        let hours_stat = StatBox::new("Total Hours", &hours_str);
        hours_stat.render(stat_chunks[0], buf);

        if let Some(day) = report.time_analytics.most_productive_day() {
            let day_str = format!("{day:?}");
            let day_stat = StatBox::new("Best Day", &day_str);
            day_stat.render(stat_chunks[1], buf);
        }

        if let Some(hour) = report.time_analytics.peak_hour() {
            let hour_str = format!("{hour}:00");
            let hour_stat = StatBox::new("Peak Hour", &hour_str);
            hour_stat.render(stat_chunks[2], buf);
        }

        // Day of week chart
        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let day_data: Vec<(String, u32)> = report
            .time_analytics
            .by_day_of_week
            .iter()
            .enumerate()
            .map(|(i, &v)| (day_names[i].to_string(), v))
            .collect();

        if chunks[1].height > 3 {
            let chart = BarChart::new("Minutes by Day of Week", &day_data).bar_color(Color::Yellow);
            chart.render(chunks[1], buf);
        }
    }

    fn render_focus(&self, area: Rect, buf: &mut Buffer) {
        let stats = &self.model.pomodoro_stats;

        // Split vertically
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stats row
                Constraint::Length(5), // Streak info
                Constraint::Min(0),    // Chart
            ])
            .split(area);

        // Stats row
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(chunks[0]);

        // Today's cycles
        let today_str = stats.cycles_today().to_string();
        let today_stat = StatBox::new("Today 🍅", &today_str);
        today_stat.render(stat_chunks[0], buf);

        // Total cycles
        let total_str = stats.total_cycles.to_string();
        let total_stat = StatBox::new("Total Cycles", &total_str);
        total_stat.render(stat_chunks[1], buf);

        // Total hours
        let total_hours = stats.total_work_mins / 60;
        let hours_str = format!("{}h", total_hours);
        let hours_stat = StatBox::new("Focus Time", &hours_str);
        hours_stat.render(stat_chunks[2], buf);

        // Current streak
        let streak_str = format!("{} days", stats.current_streak());
        let streak_stat = StatBox::new("Streak", &streak_str);
        streak_stat.render(stat_chunks[3], buf);

        // Streak info section
        let streak_info = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Current Streak: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} days", stats.current_streak()),
                    Style::default()
                        .fg(if stats.current_streak() > 0 {
                            Color::Green
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Longest Streak: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} days", stats.longest_streak),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Avg Minutes/Cycle: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if stats.total_cycles > 0 {
                        format!(
                            "{:.0}",
                            stats.total_work_mins as f32 / stats.total_cycles as f32
                        )
                    } else {
                        "N/A".to_string()
                    },
                    Style::default().fg(Color::White),
                ),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Streak Stats ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        streak_info.render(chunks[1], buf);

        // Weekly activity chart (last 7 days)
        let today = chrono::Utc::now().date_naive();
        let weekly_data: Vec<(String, u32)> = (0..7)
            .rev()
            .map(|i| {
                let date = today - chrono::Duration::days(i);
                let cycles = stats.cycles_by_date.get(&date).copied().unwrap_or(0);
                let day_name = date.format("%a").to_string();
                (day_name, cycles)
            })
            .collect();

        if chunks[2].height > 3 {
            let chart =
                BarChart::new("Last 7 Days (Pomodoro Cycles)", &weekly_data).bar_color(Color::Red);
            chart.render(chunks[2], buf);
        }
    }

    fn render_insights(&self, area: Rect, buf: &mut Buffer) {
        let engine = AnalyticsEngine::new(self.model);
        let report = engine.generate_report(&ReportConfig::last_n_days(90));

        // Split into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stats row
                Constraint::Length(5), // Streak info
                Constraint::Min(0),    // Additional insights
            ])
            .split(area);

        // Main stats
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(chunks[0]);

        let completed_str = report.insights.total_completed.to_string();
        let completed_stat = StatBox::new("All Time", &completed_str);
        completed_stat.render(stat_chunks[0], buf);

        let avg_str = format!("{:.1}", report.insights.avg_tasks_per_day);
        let avg_stat = StatBox::new("Avg/Day", &avg_str);
        avg_stat.render(stat_chunks[1], buf);

        let current_str = format!("{} days", report.insights.current_streak);
        let current_streak = StatBox::new("Current", &current_str);
        current_streak.render(stat_chunks[2], buf);

        let longest_str = format!("{} days", report.insights.longest_streak);
        let longest_streak = StatBox::new("Longest", &longest_str);
        longest_streak.render(stat_chunks[3], buf);

        // Streak visualization
        let streak_block = Block::default()
            .title(" Streak Status ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let streak_inner = streak_block.inner(chunks[1]);
        streak_block.render(chunks[1], buf);

        let streak_msg = if report.insights.is_on_streak() {
            if report.insights.is_best_streak() {
                format!(
                    "You're on a {} day streak - your best ever!",
                    report.insights.current_streak
                )
            } else {
                format!(
                    "You're on a {} day streak! Keep it going!",
                    report.insights.current_streak
                )
            }
        } else {
            "Complete a task today to start a new streak!".to_string()
        };

        let streak_color = if report.insights.is_best_streak() {
            Color::Green
        } else if report.insights.is_on_streak() {
            Color::Yellow
        } else {
            Color::DarkGray
        };

        buf.set_string(
            streak_inner.x + 1,
            streak_inner.y,
            &streak_msg,
            Style::default().fg(streak_color),
        );

        // Additional insights
        let insights_block = Block::default()
            .title(" Productivity Tips ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let insights_inner = insights_block.inner(chunks[2]);
        insights_block.render(chunks[2], buf);

        let mut tips = Vec::new();

        if let Some(day) = report.insights.best_day {
            tips.push(format!("- Your most productive day is {day:?}"));
        }

        if let Some(hour) = report.insights.peak_hour {
            tips.push(format!("- You tend to complete tasks around {hour}:00"));
        }

        if report.velocity.is_improving() {
            tips.push("- Your velocity is trending upward!".to_string());
        } else if report.velocity.trend < -0.5 {
            tips.push("- Consider breaking tasks into smaller pieces".to_string());
        }

        if report.status_breakdown.blocked > 0 {
            tips.push(format!(
                "- You have {} blocked tasks to unblock",
                report.status_breakdown.blocked
            ));
        }

        for (i, tip) in tips.iter().enumerate() {
            if i as u16 >= insights_inner.height {
                break;
            }
            buf.set_string(
                insights_inner.x + 1,
                insights_inner.y + i as u16,
                tip,
                Style::default().fg(Color::Cyan),
            );
        }
    }

    fn render_estimation(&self, area: Rect, buf: &mut Buffer) {
        // Calculate estimation statistics
        let mut total_estimated: u32 = 0;
        let mut total_actual: u32 = 0;
        let mut over_count = 0;
        let mut under_count = 0;
        let mut on_target_count = 0;
        let mut accuracies: Vec<f64> = Vec::new();

        for task in self.model.tasks.values() {
            if let Some(est) = task.estimated_minutes {
                total_estimated = total_estimated.saturating_add(est);
                total_actual = total_actual.saturating_add(task.actual_minutes);

                if let Some(variance) = task.time_variance() {
                    match variance.cmp(&0) {
                        std::cmp::Ordering::Greater => over_count += 1,
                        std::cmp::Ordering::Less => under_count += 1,
                        std::cmp::Ordering::Equal => on_target_count += 1,
                    }
                }

                if let Some(accuracy) = task.estimation_accuracy() {
                    accuracies.push(accuracy);
                }
            }
        }

        let avg_accuracy = if accuracies.is_empty() {
            None
        } else {
            Some(accuracies.iter().sum::<f64>() / accuracies.len() as f64)
        };

        let tasks_with_estimates = over_count + under_count + on_target_count;

        // Split into stat boxes, gauge, and bar chart
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Stat boxes
                Constraint::Length(3), // Accuracy gauge
                Constraint::Min(0),    // Breakdown chart
            ])
            .split(area);

        // Render stat boxes
        let stat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(chunks[0]);

        // Estimated total
        let est_str = Self::format_duration(total_estimated);
        let est_stat = StatBox::new("Estimated", &est_str);
        est_stat.render(stat_chunks[0], buf);

        // Actual total
        let actual_str = Self::format_duration(total_actual);
        let actual_stat = StatBox::new("Actual", &actual_str);
        actual_stat.render(stat_chunks[1], buf);

        // Variance
        let variance = total_actual as i64 - total_estimated as i64;
        let variance_str = if variance > 0 {
            format!("+{}", Self::format_duration(variance as u32))
        } else if variance < 0 {
            format!("-{}", Self::format_duration((-variance) as u32))
        } else {
            "0m".to_string()
        };
        let variance_stat = StatBox::new("Variance", &variance_str);
        variance_stat.render(stat_chunks[2], buf);

        // Accuracy
        let accuracy_str = avg_accuracy.map_or("N/A".to_string(), |a| format!("{:.0}%", a));
        let accuracy_stat = StatBox::new("Accuracy", &accuracy_str);
        accuracy_stat.render(stat_chunks[3], buf);

        // Render accuracy gauge
        if chunks[1].height > 0 {
            let accuracy_ratio = avg_accuracy.map_or(0.0, |a| {
                // Normalize: 100% accuracy = 1.0, 200% = 0.5, 50% = 0.5
                if a <= 100.0 {
                    a / 100.0
                } else {
                    100.0 / a
                }
            });
            let gauge = ProgressGauge::new("Estimation Accuracy", accuracy_ratio);
            gauge.render(chunks[1], buf);
        }

        // Render breakdown bar chart
        if chunks[2].height > 2 {
            let chart_block = Block::default()
                .title(" Estimation Breakdown ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            let chart_inner = chart_block.inner(chunks[2]);
            chart_block.render(chunks[2], buf);

            if tasks_with_estimates > 0 {
                let data = vec![
                    ("Over".to_string(), over_count as u32),
                    ("Under".to_string(), under_count as u32),
                    ("On Target".to_string(), on_target_count as u32),
                ];

                let chart = BarChart::new("Breakdown", &data);
                chart.render(chart_inner, buf);
            } else {
                let msg = "No tasks with time estimates";
                buf.set_string(
                    chart_inner.x + 1,
                    chart_inner.y,
                    msg,
                    Style::default().fg(Color::DarkGray),
                );
            }
        }
    }

    /// Format minutes as hours and minutes
    fn format_duration(minutes: u32) -> String {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if hours > 0 {
            format!("{hours}h {mins}m")
        } else {
            format!("{mins}m")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_panel_navigation() {
        let panel = ReportPanel::Overview;
        assert_eq!(panel.next(), ReportPanel::Velocity);
        assert_eq!(panel.prev(), ReportPanel::Estimation);
    }

    #[test]
    fn test_report_panel_cycle() {
        let mut panel = ReportPanel::Overview;
        for _ in 0..7 {
            panel = panel.next();
        }
        assert_eq!(panel, ReportPanel::Overview);
    }

    #[test]
    fn test_report_panel_index() {
        assert_eq!(ReportPanel::Overview.index(), 0);
        assert_eq!(ReportPanel::Velocity.index(), 1);
        assert_eq!(ReportPanel::Tags.index(), 2);
        assert_eq!(ReportPanel::Time.index(), 3);
        assert_eq!(ReportPanel::Focus.index(), 4);
        assert_eq!(ReportPanel::Insights.index(), 5);
        assert_eq!(ReportPanel::Estimation.index(), 6);
    }

    #[test]
    fn test_report_panel_names() {
        let names = ReportPanel::names();
        assert_eq!(names.len(), 7);
        assert_eq!(names[0], "Overview");
        assert_eq!(names[4], "Focus");
        assert_eq!(names[6], "Estimation");
    }
}
