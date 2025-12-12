//! Input handlers for views and modals.

mod modals;
mod views;

pub use modals::{
    handle_daily_review, handle_description_editor, handle_evening_review,
    handle_keybindings_editor, handle_macro_slot, handle_task_detail, handle_template_picker,
    handle_time_log, handle_weekly_review, handle_work_log,
};
pub use views::{
    handle_calendar_view, handle_eisenhower_view, handle_goals_view, handle_habits_view,
    handle_kanban_view, handle_network_view, handle_reports_view, handle_timeline_view,
    handle_weekly_planner_view,
};
