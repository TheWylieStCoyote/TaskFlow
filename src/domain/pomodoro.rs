//! Pomodoro timer types.
//!
//! This module provides types for implementing the Pomodoro Technique:
//! - Work sessions of configurable duration (default 25 minutes)
//! - Short breaks (default 5 minutes)
//! - Long breaks after a configurable number of cycles (default 4)
//!
//! ## Example
//!
//! ```
//! use taskflow::domain::{PomodoroConfig, PomodoroSession, PomodoroPhase, TaskId};
//!
//! let config = PomodoroConfig::default();
//! assert_eq!(config.work_duration_mins, 25);
//! assert_eq!(config.short_break_mins, 5);
//! assert_eq!(config.long_break_mins, 15);
//! assert_eq!(config.cycles_before_long_break, 4);
//! ```

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::TaskId;

/// Configuration for Pomodoro timer durations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PomodoroConfig {
    /// Duration of work sessions in minutes
    pub work_duration_mins: u32,
    /// Duration of short breaks in minutes
    pub short_break_mins: u32,
    /// Duration of long breaks in minutes
    pub long_break_mins: u32,
    /// Number of work cycles before a long break
    pub cycles_before_long_break: u32,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            work_duration_mins: 25,
            short_break_mins: 5,
            long_break_mins: 15,
            cycles_before_long_break: 4,
        }
    }
}

impl PomodoroConfig {
    /// Creates a new config with custom work duration.
    #[must_use]
    pub const fn with_work_duration(mut self, mins: u32) -> Self {
        self.work_duration_mins = mins;
        self
    }

    /// Creates a new config with custom short break duration.
    #[must_use]
    pub const fn with_short_break(mut self, mins: u32) -> Self {
        self.short_break_mins = mins;
        self
    }

    /// Creates a new config with custom long break duration.
    #[must_use]
    pub const fn with_long_break(mut self, mins: u32) -> Self {
        self.long_break_mins = mins;
        self
    }

    /// Creates a new config with custom cycles before long break.
    #[must_use]
    pub const fn with_cycles_before_long_break(mut self, cycles: u32) -> Self {
        self.cycles_before_long_break = cycles;
        self
    }
}

/// Current phase of the Pomodoro session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PomodoroPhase {
    /// Working phase
    Work,
    /// Short break between work sessions
    ShortBreak,
    /// Long break after completing a set of cycles
    LongBreak,
}

impl PomodoroPhase {
    /// Returns a human-readable name for the phase.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Work => "Work",
            Self::ShortBreak => "Short Break",
            Self::LongBreak => "Long Break",
        }
    }

    /// Returns an emoji icon for the phase.
    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Work => "🍅",
            Self::ShortBreak => "☕",
            Self::LongBreak => "🌴",
        }
    }

    /// Returns true if this is a break phase.
    #[must_use]
    pub const fn is_break(&self) -> bool {
        matches!(self, Self::ShortBreak | Self::LongBreak)
    }
}

/// An active Pomodoro session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroSession {
    /// The task being worked on
    pub task_id: TaskId,
    /// Current phase of the session
    pub phase: PomodoroPhase,
    /// Remaining time in the current phase (in seconds)
    pub remaining_secs: u32,
    /// Number of work cycles completed in this session
    pub cycles_completed: u32,
    /// Target number of cycles for this session
    pub session_goal: u32,
    /// When the session was started
    pub started_at: DateTime<Utc>,
    /// Whether the timer is paused
    pub paused: bool,
    /// When the current phase started (for time recalculation on load)
    #[serde(default = "Utc::now")]
    pub phase_started_at: DateTime<Utc>,
    /// Total time spent paused in the current phase (in seconds)
    #[serde(default)]
    pub paused_duration_secs: u32,
    /// When the current pause started (None if not paused)
    #[serde(default)]
    pub paused_at: Option<DateTime<Utc>>,
}

impl PomodoroSession {
    /// Creates a new Pomodoro session for the given task.
    #[must_use]
    pub fn new(task_id: TaskId, config: &PomodoroConfig, session_goal: u32) -> Self {
        let now = Utc::now();
        Self {
            task_id,
            phase: PomodoroPhase::Work,
            remaining_secs: config.work_duration_mins * 60,
            cycles_completed: 0,
            session_goal,
            started_at: now,
            paused: false,
            phase_started_at: now,
            paused_duration_secs: 0,
            paused_at: None,
        }
    }

    /// Returns the total duration of the current phase (in seconds).
    #[must_use]
    pub fn phase_duration(&self, config: &PomodoroConfig) -> u32 {
        match self.phase {
            PomodoroPhase::Work => config.work_duration_mins * 60,
            PomodoroPhase::ShortBreak => config.short_break_mins * 60,
            PomodoroPhase::LongBreak => config.long_break_mins * 60,
        }
    }

    /// Returns the progress through the current phase as a percentage (0.0 to 1.0).
    #[must_use]
    pub fn progress(&self, config: &PomodoroConfig) -> f64 {
        let total = self.phase_duration(config);
        if total == 0 {
            return 1.0;
        }
        1.0 - (f64::from(self.remaining_secs) / f64::from(total))
    }

    /// Returns a formatted string of the remaining time (MM:SS).
    #[must_use]
    pub fn formatted_remaining(&self) -> String {
        let mins = self.remaining_secs / 60;
        let secs = self.remaining_secs % 60;
        format!("{mins:02}:{secs:02}")
    }

    /// Returns true if the session goal has been reached.
    #[must_use]
    pub fn goal_reached(&self) -> bool {
        self.cycles_completed >= self.session_goal
    }

    /// Recalculates remaining_secs based on elapsed time since the phase started.
    /// Call this after deserializing a session to account for time that passed
    /// while the app was closed.
    pub fn recalculate_remaining_time(&mut self, config: &PomodoroConfig) {
        // If currently paused, first account for time spent in current pause
        if self.paused {
            if let Some(pause_start) = self.paused_at {
                let pause_elapsed = (Utc::now() - pause_start).num_seconds().max(0) as u32;
                self.paused_duration_secs += pause_elapsed;
                // Reset paused_at to now so we don't double-count
                self.paused_at = Some(Utc::now());
            }
            // When paused, remaining_secs doesn't change (time was paused)
            return;
        }

        // Calculate how much time has elapsed in this phase
        let now = Utc::now();
        let elapsed_since_phase_start = (now - self.phase_started_at).num_seconds().max(0) as u32;
        let running_time = elapsed_since_phase_start.saturating_sub(self.paused_duration_secs);

        let phase_duration = self.phase_duration(config);

        // Calculate what remaining_secs should be
        self.remaining_secs = phase_duration.saturating_sub(running_time);
    }

    /// Resets phase timing when transitioning to a new phase.
    pub fn reset_phase_timing(&mut self, new_remaining: u32) {
        self.remaining_secs = new_remaining;
        self.phase_started_at = Utc::now();
        self.paused_duration_secs = 0;
        self.paused_at = None;
    }
}

/// Statistics for Pomodoro sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PomodoroStats {
    /// Total work time in minutes (across all sessions)
    pub total_work_mins: u32,
    /// Total number of completed work cycles
    pub total_cycles: u32,
    /// Cycles completed by date
    pub cycles_by_date: HashMap<NaiveDate, u32>,
    /// Longest streak of consecutive days with at least one cycle
    pub longest_streak: u32,
}

impl PomodoroStats {
    /// Creates new empty stats.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a completed work cycle.
    pub fn record_cycle(&mut self, work_duration_mins: u32) {
        self.total_work_mins += work_duration_mins;
        self.total_cycles += 1;

        let today = Utc::now().date_naive();
        *self.cycles_by_date.entry(today).or_insert(0) += 1;

        self.update_streak();
    }

    /// Updates the longest streak calculation.
    fn update_streak(&mut self) {
        let today = Utc::now().date_naive();
        let mut current_streak = 0;
        let mut date = today;

        // Count backwards from today
        while self.cycles_by_date.contains_key(&date) {
            current_streak += 1;
            date -= chrono::Duration::days(1);
        }

        if current_streak > self.longest_streak {
            self.longest_streak = current_streak;
        }
    }

    /// Returns the number of cycles completed today.
    #[must_use]
    pub fn cycles_today(&self) -> u32 {
        let today = Utc::now().date_naive();
        self.cycles_by_date.get(&today).copied().unwrap_or(0)
    }

    /// Returns the current streak (consecutive days with cycles).
    #[must_use]
    pub fn current_streak(&self) -> u32 {
        let today = Utc::now().date_naive();
        let mut streak = 0;
        let mut date = today;

        while self.cycles_by_date.contains_key(&date) {
            streak += 1;
            date -= chrono::Duration::days(1);
        }

        streak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pomodoro_config_defaults() {
        let config = PomodoroConfig::default();
        assert_eq!(config.work_duration_mins, 25);
        assert_eq!(config.short_break_mins, 5);
        assert_eq!(config.long_break_mins, 15);
        assert_eq!(config.cycles_before_long_break, 4);
    }

    #[test]
    fn test_pomodoro_config_builder() {
        let config = PomodoroConfig::default()
            .with_work_duration(30)
            .with_short_break(10)
            .with_long_break(20)
            .with_cycles_before_long_break(3);

        assert_eq!(config.work_duration_mins, 30);
        assert_eq!(config.short_break_mins, 10);
        assert_eq!(config.long_break_mins, 20);
        assert_eq!(config.cycles_before_long_break, 3);
    }

    #[test]
    fn test_pomodoro_phase_names() {
        assert_eq!(PomodoroPhase::Work.name(), "Work");
        assert_eq!(PomodoroPhase::ShortBreak.name(), "Short Break");
        assert_eq!(PomodoroPhase::LongBreak.name(), "Long Break");
    }

    #[test]
    fn test_pomodoro_phase_icons() {
        assert_eq!(PomodoroPhase::Work.icon(), "🍅");
        assert_eq!(PomodoroPhase::ShortBreak.icon(), "☕");
        assert_eq!(PomodoroPhase::LongBreak.icon(), "🌴");
    }

    #[test]
    fn test_pomodoro_phase_is_break() {
        assert!(!PomodoroPhase::Work.is_break());
        assert!(PomodoroPhase::ShortBreak.is_break());
        assert!(PomodoroPhase::LongBreak.is_break());
    }

    #[test]
    fn test_pomodoro_session_new() {
        let task_id = TaskId::new();
        let config = PomodoroConfig::default();
        let session = PomodoroSession::new(task_id.clone(), &config, 4);

        assert_eq!(session.task_id, task_id);
        assert_eq!(session.phase, PomodoroPhase::Work);
        assert_eq!(session.remaining_secs, 25 * 60);
        assert_eq!(session.cycles_completed, 0);
        assert_eq!(session.session_goal, 4);
        assert!(!session.paused);
    }

    #[test]
    fn test_pomodoro_session_formatted_remaining() {
        let task_id = TaskId::new();
        let config = PomodoroConfig::default();
        let mut session = PomodoroSession::new(task_id, &config, 4);

        assert_eq!(session.formatted_remaining(), "25:00");

        session.remaining_secs = 90;
        assert_eq!(session.formatted_remaining(), "01:30");

        session.remaining_secs = 5;
        assert_eq!(session.formatted_remaining(), "00:05");
    }

    #[test]
    fn test_pomodoro_session_progress() {
        let task_id = TaskId::new();
        let config = PomodoroConfig::default();
        let mut session = PomodoroSession::new(task_id, &config, 4);

        // Full time remaining = 0% progress
        let progress = session.progress(&config);
        assert!((progress - 0.0).abs() < 0.01);

        // Half time remaining = 50% progress
        session.remaining_secs = 25 * 60 / 2;
        let progress = session.progress(&config);
        assert!((progress - 0.5).abs() < 0.01);

        // No time remaining = 100% progress
        session.remaining_secs = 0;
        let progress = session.progress(&config);
        assert!((progress - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_pomodoro_session_goal_reached() {
        let task_id = TaskId::new();
        let config = PomodoroConfig::default();
        let mut session = PomodoroSession::new(task_id, &config, 4);

        assert!(!session.goal_reached());

        session.cycles_completed = 3;
        assert!(!session.goal_reached());

        session.cycles_completed = 4;
        assert!(session.goal_reached());

        session.cycles_completed = 5;
        assert!(session.goal_reached());
    }

    #[test]
    fn test_pomodoro_stats_record_cycle() {
        let mut stats = PomodoroStats::new();

        assert_eq!(stats.total_cycles, 0);
        assert_eq!(stats.total_work_mins, 0);

        stats.record_cycle(25);
        assert_eq!(stats.total_cycles, 1);
        assert_eq!(stats.total_work_mins, 25);
        assert_eq!(stats.cycles_today(), 1);

        stats.record_cycle(25);
        assert_eq!(stats.total_cycles, 2);
        assert_eq!(stats.total_work_mins, 50);
        assert_eq!(stats.cycles_today(), 2);
    }

    #[test]
    fn test_pomodoro_stats_current_streak() {
        let mut stats = PomodoroStats::new();
        assert_eq!(stats.current_streak(), 0);

        stats.record_cycle(25);
        assert_eq!(stats.current_streak(), 1);
    }

    #[test]
    fn test_pomodoro_session_serialization_roundtrip() {
        let task_id = TaskId::new();
        let config = PomodoroConfig::default();
        let session = PomodoroSession::new(task_id.clone(), &config, 4);

        // Serialize to JSON
        let json = serde_json::to_string(&session).expect("Failed to serialize");

        // Deserialize back
        let restored: PomodoroSession = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.task_id, task_id);
        assert_eq!(restored.phase, PomodoroPhase::Work);
        assert_eq!(restored.remaining_secs, 25 * 60);
        assert_eq!(restored.cycles_completed, 0);
        assert_eq!(restored.session_goal, 4);
        assert!(!restored.paused);
    }

    #[test]
    fn test_recalculate_remaining_time_when_paused() {
        let task_id = TaskId::new();
        let config = PomodoroConfig::default();
        let mut session = PomodoroSession::new(task_id, &config, 4);

        // Pause the session
        session.paused = true;
        session.paused_at = Some(Utc::now());
        let original_remaining = session.remaining_secs;

        // Recalculate - should not change remaining time when paused
        session.recalculate_remaining_time(&config);

        assert_eq!(session.remaining_secs, original_remaining);
    }

    #[test]
    fn test_reset_phase_timing() {
        let task_id = TaskId::new();
        let config = PomodoroConfig::default();
        let mut session = PomodoroSession::new(task_id, &config, 4);

        // Simulate some time passing and pausing
        session.remaining_secs = 100;
        session.paused_duration_secs = 50;
        session.paused_at = Some(Utc::now());

        // Reset phase timing
        let new_remaining = config.short_break_mins * 60;
        session.reset_phase_timing(new_remaining);

        assert_eq!(session.remaining_secs, new_remaining);
        assert_eq!(session.paused_duration_secs, 0);
        assert!(session.paused_at.is_none());
    }

    #[test]
    fn test_pomodoro_config_serialization_roundtrip() {
        let config = PomodoroConfig::default()
            .with_work_duration(30)
            .with_short_break(10);

        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let restored: PomodoroConfig = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.work_duration_mins, 30);
        assert_eq!(restored.short_break_mins, 10);
    }

    #[test]
    fn test_pomodoro_stats_serialization_roundtrip() {
        let mut stats = PomodoroStats::new();
        stats.record_cycle(25);
        stats.record_cycle(25);

        let json = serde_json::to_string(&stats).expect("Failed to serialize");
        let restored: PomodoroStats = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.total_cycles, 2);
        assert_eq!(restored.total_work_mins, 50);
    }
}
