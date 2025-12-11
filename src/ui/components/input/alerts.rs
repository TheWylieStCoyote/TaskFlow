//! Alert popup widgets for overdue tasks and storage errors.

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Clear, Paragraph, Widget},
};

use crate::config::Theme;
use crate::ui::primitives::{danger_block, warning_block};

/// Overdue tasks alert popup shown at startup
pub struct OverdueAlert<'a> {
    count: usize,
    task_titles: Vec<String>,
    theme: &'a Theme,
}

impl<'a> OverdueAlert<'a> {
    #[must_use]
    pub fn new(count: usize, task_titles: Vec<String>, theme: &'a Theme) -> Self {
        Self {
            count,
            task_titles,
            theme,
        }
    }
}

impl Widget for OverdueAlert<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        Clear.render(area, buf);

        let mut lines = vec![format!(
            "You have {} overdue task{}!\n",
            self.count,
            if self.count == 1 { "" } else { "s" }
        )];

        // Show up to 5 task titles
        for (i, title) in self.task_titles.iter().take(5).enumerate() {
            lines.push(format!("  {}. {}", i + 1, title));
        }
        if self.count > 5 {
            lines.push(format!("  ... and {} more", self.count - 5));
        }

        lines.push(String::new());
        lines.push("Press any key to dismiss".to_string());

        let text = lines.join("\n");

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(danger_block("⚠ Overdue Tasks", self.theme));

        paragraph.render(area, buf);
    }
}

/// Storage error alert popup shown when data cannot be loaded
pub struct StorageErrorAlert<'a> {
    error_message: &'a str,
    theme: &'a Theme,
}

impl<'a> StorageErrorAlert<'a> {
    #[must_use]
    pub fn new(error_message: &'a str, theme: &'a Theme) -> Self {
        Self {
            error_message,
            theme,
        }
    }
}

impl Widget for StorageErrorAlert<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        Clear.render(area, buf);

        let text = format!(
            "Could not load your task data:\n\n  {}\n\nStarting with sample data instead.\nYour existing data has not been modified.\n\nPress any key to continue",
            self.error_message
        );

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.colors.foreground.to_color()))
            .block(warning_block("⚠ Storage Error", self.theme));

        paragraph.render(area, buf);
    }
}
