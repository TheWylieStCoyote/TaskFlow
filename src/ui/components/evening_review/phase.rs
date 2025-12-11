//! Evening review phase definitions.
//!
//! The evening review guides users through a structured end-of-day workflow
//! to celebrate accomplishments, address incomplete work, and prepare for tomorrow.

/// Phases of the evening review.
///
/// The evening review complements the Daily Review (morning planning) and Weekly Review
/// by focusing on "what happened today and what's next tomorrow."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EveningReviewPhase {
    /// Welcome phase with day summary and completion stats
    #[default]
    Welcome,
    /// Celebrate tasks completed today
    CompletedToday,
    /// Review incomplete tasks that were due/scheduled today
    IncompleteTasks,
    /// Preview tomorrow's tasks (due + scheduled)
    TomorrowPreview,
    /// Time tracking summary for today (auto-skips if empty)
    TimeReview,
    /// Final stats, streak info, and encouraging close
    Summary,
}

impl EveningReviewPhase {
    /// Total number of phases in the evening review.
    pub const TOTAL_PHASES: u8 = 6;

    /// Get the next phase in the review.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Welcome => Self::CompletedToday,
            Self::CompletedToday => Self::IncompleteTasks,
            Self::IncompleteTasks => Self::TomorrowPreview,
            Self::TomorrowPreview => Self::TimeReview,
            Self::TimeReview => Self::Summary,
            Self::Summary => Self::Summary, // Stay at end
        }
    }

    /// Get the previous phase.
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Welcome => Self::Welcome, // Stay at start
            Self::CompletedToday => Self::Welcome,
            Self::IncompleteTasks => Self::CompletedToday,
            Self::TomorrowPreview => Self::IncompleteTasks,
            Self::TimeReview => Self::TomorrowPreview,
            Self::Summary => Self::TimeReview,
        }
    }

    /// Get phase number (1-6).
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Welcome => 1,
            Self::CompletedToday => 2,
            Self::IncompleteTasks => 3,
            Self::TomorrowPreview => 4,
            Self::TimeReview => 5,
            Self::Summary => 6,
        }
    }

    /// Get phase title for display.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Evening Review",
            Self::CompletedToday => "Today's Wins",
            Self::IncompleteTasks => "Unfinished Business",
            Self::TomorrowPreview => "Tomorrow's Plan",
            Self::TimeReview => "Time Spent",
            Self::Summary => "Day Complete",
        }
    }

    /// Check if this is the first phase.
    #[must_use]
    pub const fn is_first(self) -> bool {
        matches!(self, Self::Welcome)
    }

    /// Check if this is the last phase.
    #[must_use]
    pub const fn is_last(self) -> bool {
        matches!(self, Self::Summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_transitions() {
        assert_eq!(
            EveningReviewPhase::Welcome.next(),
            EveningReviewPhase::CompletedToday
        );
        assert_eq!(
            EveningReviewPhase::CompletedToday.next(),
            EveningReviewPhase::IncompleteTasks
        );
        assert_eq!(
            EveningReviewPhase::IncompleteTasks.next(),
            EveningReviewPhase::TomorrowPreview
        );
        assert_eq!(
            EveningReviewPhase::TomorrowPreview.next(),
            EveningReviewPhase::TimeReview
        );
        assert_eq!(
            EveningReviewPhase::TimeReview.next(),
            EveningReviewPhase::Summary
        );
        assert_eq!(
            EveningReviewPhase::Summary.next(),
            EveningReviewPhase::Summary
        );
    }

    #[test]
    fn test_phase_prev_transitions() {
        assert_eq!(
            EveningReviewPhase::Welcome.prev(),
            EveningReviewPhase::Welcome
        );
        assert_eq!(
            EveningReviewPhase::CompletedToday.prev(),
            EveningReviewPhase::Welcome
        );
        assert_eq!(
            EveningReviewPhase::IncompleteTasks.prev(),
            EveningReviewPhase::CompletedToday
        );
        assert_eq!(
            EveningReviewPhase::TomorrowPreview.prev(),
            EveningReviewPhase::IncompleteTasks
        );
        assert_eq!(
            EveningReviewPhase::TimeReview.prev(),
            EveningReviewPhase::TomorrowPreview
        );
        assert_eq!(
            EveningReviewPhase::Summary.prev(),
            EveningReviewPhase::TimeReview
        );
    }

    #[test]
    fn test_phase_numbers() {
        assert_eq!(EveningReviewPhase::Welcome.number(), 1);
        assert_eq!(EveningReviewPhase::CompletedToday.number(), 2);
        assert_eq!(EveningReviewPhase::IncompleteTasks.number(), 3);
        assert_eq!(EveningReviewPhase::TomorrowPreview.number(), 4);
        assert_eq!(EveningReviewPhase::TimeReview.number(), 5);
        assert_eq!(EveningReviewPhase::Summary.number(), 6);
    }

    #[test]
    fn test_phase_titles() {
        assert_eq!(EveningReviewPhase::Welcome.title(), "Evening Review");
        assert_eq!(EveningReviewPhase::CompletedToday.title(), "Today's Wins");
        assert_eq!(
            EveningReviewPhase::IncompleteTasks.title(),
            "Unfinished Business"
        );
        assert_eq!(
            EveningReviewPhase::TomorrowPreview.title(),
            "Tomorrow's Plan"
        );
        assert_eq!(EveningReviewPhase::TimeReview.title(), "Time Spent");
        assert_eq!(EveningReviewPhase::Summary.title(), "Day Complete");
    }

    #[test]
    fn test_is_first_last() {
        assert!(EveningReviewPhase::Welcome.is_first());
        assert!(!EveningReviewPhase::Welcome.is_last());
        assert!(!EveningReviewPhase::Summary.is_first());
        assert!(EveningReviewPhase::Summary.is_last());
        assert!(!EveningReviewPhase::IncompleteTasks.is_first());
        assert!(!EveningReviewPhase::IncompleteTasks.is_last());
    }

    #[test]
    fn test_default_is_welcome() {
        assert_eq!(EveningReviewPhase::default(), EveningReviewPhase::Welcome);
    }
}
