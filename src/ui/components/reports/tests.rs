//! Reports component tests

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
