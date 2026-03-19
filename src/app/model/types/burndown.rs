//! Burndown chart configuration state.

/// Time window for burndown chart display.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BurndownTimeWindow {
    /// Last 7 days
    Days7,
    /// Last 14 days (default)
    #[default]
    Days14,
    /// Last 30 days
    Days30,
    /// Last 90 days
    Days90,
}

impl BurndownTimeWindow {
    /// Get the number of days in this window.
    #[must_use]
    pub const fn days(&self) -> i64 {
        match self {
            Self::Days7 => 7,
            Self::Days14 => 14,
            Self::Days30 => 30,
            Self::Days90 => 90,
        }
    }

    /// Get display label for this window.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Days7 => "7 Days",
            Self::Days14 => "14 Days",
            Self::Days30 => "30 Days",
            Self::Days90 => "90 Days",
        }
    }

    /// Cycle to next window size.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Days7 => Self::Days14,
            Self::Days14 => Self::Days30,
            Self::Days30 => Self::Days90,
            Self::Days90 => Self::Days7,
        }
    }
}

/// Burndown chart display mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BurndownMode {
    /// Show task count remaining (default)
    #[default]
    TaskCount,
    /// Show estimated hours remaining
    TimeHours,
}

impl BurndownMode {
    /// Get display label for this mode.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::TaskCount => "Tasks",
            Self::TimeHours => "Hours",
        }
    }

    /// Toggle between modes.
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::TaskCount => Self::TimeHours,
            Self::TimeHours => Self::TaskCount,
        }
    }
}

/// State for the burndown chart view.
#[derive(Debug, Clone, Default)]
pub struct BurndownState {
    /// Time window to display
    pub time_window: BurndownTimeWindow,
    /// Display mode (task count vs time)
    pub mode: BurndownMode,
    /// Whether to show scope changes (tasks added during period)
    pub show_scope_creep: bool,
    /// Selected project filter (None = all tasks)
    pub selected_project_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_window_days() {
        assert_eq!(BurndownTimeWindow::Days7.days(), 7);
        assert_eq!(BurndownTimeWindow::Days14.days(), 14);
        assert_eq!(BurndownTimeWindow::Days30.days(), 30);
        assert_eq!(BurndownTimeWindow::Days90.days(), 90);
    }

    #[test]
    fn test_time_window_label() {
        assert_eq!(BurndownTimeWindow::Days7.label(), "7 Days");
        assert_eq!(BurndownTimeWindow::Days14.label(), "14 Days");
        assert_eq!(BurndownTimeWindow::Days30.label(), "30 Days");
        assert_eq!(BurndownTimeWindow::Days90.label(), "90 Days");
    }

    #[test]
    fn test_time_window_next_cycles() {
        assert_eq!(BurndownTimeWindow::Days7.next(), BurndownTimeWindow::Days14);
        assert_eq!(
            BurndownTimeWindow::Days14.next(),
            BurndownTimeWindow::Days30
        );
        assert_eq!(
            BurndownTimeWindow::Days30.next(),
            BurndownTimeWindow::Days90
        );
        assert_eq!(BurndownTimeWindow::Days90.next(), BurndownTimeWindow::Days7);
    }

    #[test]
    fn test_burndown_mode_label() {
        assert_eq!(BurndownMode::TaskCount.label(), "Tasks");
        assert_eq!(BurndownMode::TimeHours.label(), "Hours");
    }

    #[test]
    fn test_burndown_mode_toggle() {
        assert_eq!(BurndownMode::TaskCount.toggle(), BurndownMode::TimeHours);
        assert_eq!(BurndownMode::TimeHours.toggle(), BurndownMode::TaskCount);
    }

    #[test]
    fn test_burndown_state_default() {
        let state = BurndownState::default();
        assert_eq!(state.time_window, BurndownTimeWindow::Days14);
        assert_eq!(state.mode, BurndownMode::TaskCount);
        assert!(!state.show_scope_creep);
        assert_eq!(state.selected_project_index, 0);
    }
}
