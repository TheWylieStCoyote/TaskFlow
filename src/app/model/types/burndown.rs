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
