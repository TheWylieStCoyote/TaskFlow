//! Daily review phase definitions.

/// Phases of the daily review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DailyReviewPhase {
    #[default]
    Welcome,
    OverdueTasks,
    TodayTasks,
    ScheduledTasks,
    Summary,
}

impl DailyReviewPhase {
    /// Get the next phase in the review
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Welcome => Self::OverdueTasks,
            Self::OverdueTasks => Self::TodayTasks,
            Self::TodayTasks => Self::ScheduledTasks,
            Self::ScheduledTasks => Self::Summary,
            Self::Summary => Self::Summary, // Stay at end
        }
    }

    /// Get the previous phase
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Welcome => Self::Welcome, // Stay at start
            Self::OverdueTasks => Self::Welcome,
            Self::TodayTasks => Self::OverdueTasks,
            Self::ScheduledTasks => Self::TodayTasks,
            Self::Summary => Self::ScheduledTasks,
        }
    }

    /// Get phase number (1-5)
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Welcome => 1,
            Self::OverdueTasks => 2,
            Self::TodayTasks => 3,
            Self::ScheduledTasks => 4,
            Self::Summary => 5,
        }
    }

    /// Get phase title
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Good Morning!",
            Self::OverdueTasks => "Overdue Tasks",
            Self::TodayTasks => "Today's Tasks",
            Self::ScheduledTasks => "Scheduled for Today",
            Self::Summary => "Daily Summary",
        }
    }
}
