//! Navigation message handlers.
//!
//! Handles all navigation-related messages including:
//! - Task list navigation (up, down, page up, page down)
//! - Sidebar navigation
//! - Calendar navigation
//! - View switching
//! - View-specific navigation (Kanban, Eisenhower, Timeline, etc.)

mod calendar;
mod eisenhower;
mod kanban;
mod network;
mod reports;
mod sidebar;
mod task_list;
mod timeline;
mod view;
mod weekly_planner;

pub use calendar::days_in_month;

use crate::app::{Model, NavigationMessage};

use calendar::handle_calendar_navigation;
use eisenhower::handle_eisenhower_navigation;
use kanban::handle_kanban_navigation;
use network::handle_network_navigation;
use reports::handle_reports_navigation;
use sidebar::handle_sidebar_navigation;
use task_list::handle_task_list_navigation;
use timeline::handle_timeline_navigation;
use view::handle_view_navigation;
use weekly_planner::handle_weekly_planner_navigation;

/// Handle navigation messages by dispatching to the appropriate view-specific handler.
pub fn handle_navigation(model: &mut Model, msg: NavigationMessage) {
    match &msg {
        // Basic task list navigation
        NavigationMessage::Up
        | NavigationMessage::Down
        | NavigationMessage::First
        | NavigationMessage::Last
        | NavigationMessage::PageUp
        | NavigationMessage::PageDown
        | NavigationMessage::Select(_) => {
            handle_task_list_navigation(model, msg);
        }

        // View switching
        NavigationMessage::GoToView(_) => {
            handle_view_navigation(model, msg);
        }

        // Sidebar navigation
        NavigationMessage::FocusSidebar
        | NavigationMessage::FocusTaskList
        | NavigationMessage::SelectSidebarItem
        | NavigationMessage::SidebarSelectIndex(_) => {
            handle_sidebar_navigation(model, msg);
        }

        // Calendar navigation
        NavigationMessage::CalendarPrevMonth
        | NavigationMessage::CalendarNextMonth
        | NavigationMessage::CalendarSelectDay(_)
        | NavigationMessage::CalendarFocusTaskList
        | NavigationMessage::CalendarFocusGrid => {
            handle_calendar_navigation(model, msg);
        }

        // Timeline navigation
        NavigationMessage::TimelineScrollLeft
        | NavigationMessage::TimelineScrollRight
        | NavigationMessage::TimelineZoomIn
        | NavigationMessage::TimelineZoomOut
        | NavigationMessage::TimelineGoToday
        | NavigationMessage::TimelineUp
        | NavigationMessage::TimelineDown => {
            handle_timeline_navigation(model, msg);
        }

        // Kanban navigation
        NavigationMessage::KanbanLeft
        | NavigationMessage::KanbanRight
        | NavigationMessage::KanbanUp
        | NavigationMessage::KanbanDown
        | NavigationMessage::KanbanSelectColumn(_) => {
            handle_kanban_navigation(model, msg);
        }

        // Eisenhower navigation
        NavigationMessage::EisenhowerUp
        | NavigationMessage::EisenhowerDown
        | NavigationMessage::EisenhowerLeft
        | NavigationMessage::EisenhowerRight
        | NavigationMessage::EisenhowerSelectQuadrant(_) => {
            handle_eisenhower_navigation(model, msg);
        }

        // Weekly planner navigation
        NavigationMessage::WeeklyPlannerLeft
        | NavigationMessage::WeeklyPlannerRight
        | NavigationMessage::WeeklyPlannerUp
        | NavigationMessage::WeeklyPlannerDown
        | NavigationMessage::WeeklyPlannerSelectDay(_) => {
            handle_weekly_planner_navigation(model, msg);
        }

        // Network navigation
        NavigationMessage::NetworkUp | NavigationMessage::NetworkDown => {
            handle_network_navigation(model, msg);
        }

        // Reports navigation
        NavigationMessage::ReportsNextPanel
        | NavigationMessage::ReportsPrevPanel
        | NavigationMessage::ReportsSelectPanel(_) => {
            handle_reports_navigation(model, msg);
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests are in src/app/update/tests/navigation.rs
}
