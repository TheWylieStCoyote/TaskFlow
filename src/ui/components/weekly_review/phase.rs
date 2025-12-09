//! Weekly review phases.

/// Phases of the weekly review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeeklyReviewPhase {
    #[default]
    Welcome,
    CompletedTasks,
    OverdueTasks,
    UpcomingWeek,
    StaleProjects,
    Summary,
}

impl WeeklyReviewPhase {
    /// Get the next phase in the review
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Welcome => Self::CompletedTasks,
            Self::CompletedTasks => Self::OverdueTasks,
            Self::OverdueTasks => Self::UpcomingWeek,
            Self::UpcomingWeek => Self::StaleProjects,
            Self::StaleProjects => Self::Summary,
            Self::Summary => Self::Summary, // Stay at end
        }
    }

    /// Get the previous phase
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Welcome => Self::Welcome, // Stay at start
            Self::CompletedTasks => Self::Welcome,
            Self::OverdueTasks => Self::CompletedTasks,
            Self::UpcomingWeek => Self::OverdueTasks,
            Self::StaleProjects => Self::UpcomingWeek,
            Self::Summary => Self::StaleProjects,
        }
    }

    /// Get phase number (1-6)
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Welcome => 1,
            Self::CompletedTasks => 2,
            Self::OverdueTasks => 3,
            Self::UpcomingWeek => 4,
            Self::StaleProjects => 5,
            Self::Summary => 6,
        }
    }

    /// Get phase title
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Weekly Review",
            Self::CompletedTasks => "Completed This Week",
            Self::OverdueTasks => "Overdue Tasks",
            Self::UpcomingWeek => "Next 7 Days",
            Self::StaleProjects => "Project Check",
            Self::Summary => "Weekly Summary",
        }
    }
}
