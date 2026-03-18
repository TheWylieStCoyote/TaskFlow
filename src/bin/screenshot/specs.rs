//! Screenshot specifications: one entry per view to capture.

use taskflow::{
    app::{Model, ViewId},
    domain::{Goal, KeyResult, Quarter},
};

/// A single screenshot specification.
pub struct Spec {
    pub filename: &'static str,
    pub view_id: ViewId,
    pub width: u16,
    pub height: u16,
    /// Called after model is initialised; use for view-specific setup.
    pub setup: fn(&mut Model),
}

/// All screenshot specs in capture order.
pub fn all_specs() -> Vec<Spec> {
    vec![
        Spec {
            filename: "task_list.svg",
            view_id: ViewId::TaskList,
            width: 160,
            height: 45,
            setup: |_| {},
        },
        Spec {
            filename: "kanban.svg",
            view_id: ViewId::Kanban,
            width: 200,
            height: 50,
            setup: |_| {},
        },
        Spec {
            filename: "calendar.svg",
            view_id: ViewId::Calendar,
            width: 160,
            height: 45,
            setup: |_| {},
        },
        Spec {
            filename: "dashboard.svg",
            view_id: ViewId::Dashboard,
            width: 160,
            height: 45,
            setup: |_| {},
        },
        Spec {
            filename: "weekly_planner.svg",
            view_id: ViewId::WeeklyPlanner,
            width: 200,
            height: 50,
            setup: |_| {},
        },
        Spec {
            filename: "eisenhower.svg",
            view_id: ViewId::Eisenhower,
            width: 160,
            height: 50,
            setup: |_| {},
        },
        Spec {
            filename: "timeline.svg",
            view_id: ViewId::Timeline,
            width: 200,
            height: 45,
            setup: |_| {},
        },
        Spec {
            filename: "reports.svg",
            view_id: ViewId::Reports,
            width: 160,
            height: 45,
            setup: |model| model.ensure_report_cache_populated(),
        },
        Spec {
            filename: "heatmap.svg",
            view_id: ViewId::Heatmap,
            width: 160,
            height: 45,
            setup: |_| {},
        },
        Spec {
            filename: "network.svg",
            view_id: ViewId::Network,
            width: 160,
            height: 45,
            setup: |_| {},
        },
        Spec {
            filename: "habits.svg",
            view_id: ViewId::Habits,
            width: 160,
            height: 45,
            setup: |model| model.refresh_visible_habits(),
        },
        Spec {
            filename: "goals.svg",
            view_id: ViewId::Goals,
            width: 160,
            height: 45,
            setup: |model| {
                let goal1 = Goal::new("Launch v1.0").with_quarter(2026, Quarter::Q2);
                let kr1 = KeyResult::new(goal1.id, "Ship core features");
                let kr2 = KeyResult::new(goal1.id, "Write documentation");
                model.key_results.insert(kr1.id, kr1);
                model.key_results.insert(kr2.id, kr2);
                model.goals.insert(goal1.id, goal1);

                let goal2 = Goal::new("Grow user base").with_quarter(2026, Quarter::Q3);
                let kr3 = KeyResult::new(goal2.id, "Reach 1 000 active users");
                model.key_results.insert(kr3.id, kr3);
                model.goals.insert(goal2.id, goal2);

                let goal3 = Goal::new("Improve performance").with_quarter(2026, Quarter::Q4);
                model.goals.insert(goal3.id, goal3);

                model.refresh_visible_goals();
            },
        },
        Spec {
            filename: "focus.svg",
            view_id: ViewId::TaskList,
            width: 160,
            height: 45,
            setup: |model| {
                model.focus_mode = true;
                model.selected_index = 0;
            },
        },
        Spec {
            filename: "forecast.svg",
            view_id: ViewId::Forecast,
            width: 160,
            height: 45,
            setup: |_| {},
        },
        Spec {
            filename: "burndown.svg",
            view_id: ViewId::Burndown,
            width: 160,
            height: 45,
            setup: |_| {},
        },
    ]
}
