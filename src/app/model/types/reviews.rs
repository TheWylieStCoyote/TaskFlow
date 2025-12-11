//! Review state types.

/// State for daily review mode.
#[derive(Debug, Clone, Default)]
pub struct DailyReviewState {
    /// Whether daily review mode is active
    pub visible: bool,
    /// Current phase of the daily review
    pub phase: crate::ui::DailyReviewPhase,
    /// Selected index within current review phase
    pub selected: usize,
}

/// State for weekly review mode.
#[derive(Debug, Clone, Default)]
pub struct WeeklyReviewState {
    /// Whether weekly review mode is active
    pub visible: bool,
    /// Current phase of the weekly review
    pub phase: crate::ui::WeeklyReviewPhase,
    /// Selected index within current review phase
    pub selected: usize,
}
