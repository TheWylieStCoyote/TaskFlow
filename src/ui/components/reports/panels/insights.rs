//! Insights panel rendering

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use crate::app::analytics::AnalyticsEngine;
use crate::domain::analytics::ReportConfig;
use crate::ui::components::charts::StatBox;

use super::super::ReportsView;

impl ReportsView<'_> {
    pub(crate) fn render_insights(&self, area: Rect, buf: &mut Buffer) {
        // Use cached 90-day report if available, otherwise generate on-the-fly
        let fallback_report;
        let report = if let Some(ref cached) = self.model.report_cache.report_90d {
            cached
        } else {
            let engine = AnalyticsEngine::new(self.model);
            fallback_report = engine.generate_report(&ReportConfig::last_n_days(90));
            &fallback_report
        };

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
}
