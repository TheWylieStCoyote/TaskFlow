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
            if let Some(panel) = crate::ui::ReportPanel::from_index(panel_idx) {
                model.report_panel = panel;
            }
        }
        _ => {}
    }
}
