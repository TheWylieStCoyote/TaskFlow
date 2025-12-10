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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_next_transitions() {
        assert_eq!(
            WeeklyReviewPhase::Welcome.next(),
            WeeklyReviewPhase::CompletedTasks
        );
        assert_eq!(
            WeeklyReviewPhase::CompletedTasks.next(),
            WeeklyReviewPhase::OverdueTasks
        );
        assert_eq!(
            WeeklyReviewPhase::OverdueTasks.next(),
            WeeklyReviewPhase::UpcomingWeek
        );
        assert_eq!(
            WeeklyReviewPhase::UpcomingWeek.next(),
            WeeklyReviewPhase::StaleProjects
        );
        assert_eq!(
            WeeklyReviewPhase::StaleProjects.next(),
            WeeklyReviewPhase::Summary
        );
        // Summary stays at end
        assert_eq!(
            WeeklyReviewPhase::Summary.next(),
            WeeklyReviewPhase::Summary
        );
    }

    #[test]
    fn test_phase_prev_transitions() {
        // Welcome stays at start
        assert_eq!(
            WeeklyReviewPhase::Welcome.prev(),
            WeeklyReviewPhase::Welcome
        );
        assert_eq!(
            WeeklyReviewPhase::CompletedTasks.prev(),
            WeeklyReviewPhase::Welcome
        );
        assert_eq!(
            WeeklyReviewPhase::OverdueTasks.prev(),
            WeeklyReviewPhase::CompletedTasks
        );
        assert_eq!(
            WeeklyReviewPhase::UpcomingWeek.prev(),
            WeeklyReviewPhase::OverdueTasks
        );
        assert_eq!(
            WeeklyReviewPhase::StaleProjects.prev(),
            WeeklyReviewPhase::UpcomingWeek
        );
        assert_eq!(
            WeeklyReviewPhase::Summary.prev(),
            WeeklyReviewPhase::StaleProjects
        );
    }

    #[test]
    fn test_phase_numbers() {
        assert_eq!(WeeklyReviewPhase::Welcome.number(), 1);
        assert_eq!(WeeklyReviewPhase::CompletedTasks.number(), 2);
        assert_eq!(WeeklyReviewPhase::OverdueTasks.number(), 3);
        assert_eq!(WeeklyReviewPhase::UpcomingWeek.number(), 4);
        assert_eq!(WeeklyReviewPhase::StaleProjects.number(), 5);
        assert_eq!(WeeklyReviewPhase::Summary.number(), 6);
    }

    #[test]
    fn test_phase_titles() {
        assert_eq!(WeeklyReviewPhase::Welcome.title(), "Weekly Review");
        assert_eq!(
            WeeklyReviewPhase::CompletedTasks.title(),
            "Completed This Week"
        );
        assert_eq!(WeeklyReviewPhase::OverdueTasks.title(), "Overdue Tasks");
        assert_eq!(WeeklyReviewPhase::UpcomingWeek.title(), "Next 7 Days");
        assert_eq!(WeeklyReviewPhase::StaleProjects.title(), "Project Check");
        assert_eq!(WeeklyReviewPhase::Summary.title(), "Weekly Summary");
    }

    #[test]
    fn test_phase_default() {
        assert_eq!(WeeklyReviewPhase::default(), WeeklyReviewPhase::Welcome);
    }
}
