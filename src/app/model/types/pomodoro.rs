//! Pomodoro timer state types.

/// State for the Pomodoro timer.
///
/// Groups all Pomodoro-related fields including the active session,
/// configuration, and statistics.
#[derive(Debug, Clone, Default)]
pub struct PomodoroState {
    /// Active Pomodoro session (if any)
    pub session: Option<crate::domain::PomodoroSession>,
    /// Pomodoro timer configuration (work/break durations)
    pub config: crate::domain::PomodoroConfig,
    /// Pomodoro statistics (completed sessions, total time)
    pub stats: crate::domain::PomodoroStats,
}
