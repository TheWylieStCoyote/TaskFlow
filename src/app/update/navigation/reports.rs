//! Reports view navigation handlers.

use crate::app::{Model, NavigationMessage, ViewId};

/// Handle reports-specific navigation messages.
pub fn handle_reports_navigation(model: &mut Model, msg: NavigationMessage) {
    if model.current_view != ViewId::Reports {
        return;
    }

    match msg {
        NavigationMessage::ReportsNextPanel => {
            model.report_panel = model.report_panel.next();
        }
        NavigationMessage::ReportsPrevPanel => {
            model.report_panel = model.report_panel.prev();
        }
        NavigationMessage::ReportsSelectPanel(panel_idx) => {
            if panel_idx < 7 {
                model.report_panel = match panel_idx {
                    0 => crate::ui::ReportPanel::Overview,
                    1 => crate::ui::ReportPanel::Velocity,
                    2 => crate::ui::ReportPanel::Tags,
                    3 => crate::ui::ReportPanel::Time,
                    4 => crate::ui::ReportPanel::Focus,
                    5 => crate::ui::ReportPanel::Insights,
                    6 => crate::ui::ReportPanel::Estimation,
                    _ => return,
                };
            }
        }
        _ => {}
    }
}
