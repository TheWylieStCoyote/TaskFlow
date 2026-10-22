//! Data shared by the "Time Tracking" section of the Markdown and HTML
//! reports: project name resolution, table rows and a text bar.

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::domain::analytics::AnalyticsReport;
use crate::domain::ProjectId;

/// Cells in the longest text bar.
const BAR_WIDTH: usize = 20;

/// Project names as the report knows them: every per-project burndown chart
/// carries its project's id and name (the global chart has no id).
fn project_names(report: &AnalyticsReport) -> HashMap<ProjectId, &str> {
    report
        .burn_charts
        .iter()
        .filter_map(|c| c.project_id.map(|id| (id, c.project_name.as_str())))
        .collect()
}

/// Percentage `part` is of `whole`; 0 when `whole` is 0.
pub(super) fn share(part: u32, whole: u32) -> f64 {
    if whole == 0 {
        0.0
    } else {
        f64::from(part) / f64::from(whole) * 100.0
    }
}

/// Per-project rows, most time first: `(name, minutes, share of total %)`.
pub(super) fn project_rows(report: &AnalyticsReport) -> Vec<(String, u32, f64)> {
    let names = project_names(report);
    let total = report.time_analytics.total_minutes;
    report
        .time_analytics
        .projects_by_minutes()
        .into_iter()
        .map(|(id, minutes)| {
            let name = match id {
                None => "(no project)".to_string(),
                Some(id) => names
                    .get(&id)
                    .map_or_else(|| "(unknown project)".to_string(), |n| (*n).to_string()),
            };
            (name, minutes, share(minutes, total))
        })
        .collect()
}

/// Minutes for every day of the report window, zero-filled.
pub(super) fn day_rows(report: &AnalyticsReport) -> Vec<(NaiveDate, u32)> {
    report
        .time_analytics
        .daily_series(report.config.start_date, report.config.end_date)
}

/// A bar proportional to `value / max`, at most `BAR_WIDTH` cells, with an
/// eighth-block character for the fractional cell.
pub(super) fn text_bar(value: u32, max: u32) -> String {
    const EIGHTHS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    if max == 0 || value == 0 {
        return String::new();
    }
    let units = u64::from(value) * (BAR_WIDTH as u64) * 8 / u64::from(max);
    let full = usize::try_from(units / 8).unwrap_or(BAR_WIDTH);
    let rem = usize::try_from(units % 8).unwrap_or(0);
    let mut out = "█".repeat(full);
    if rem > 0 {
        out.push(EIGHTHS[rem]);
    }
    out
}
