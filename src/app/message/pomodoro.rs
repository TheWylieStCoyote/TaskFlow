//! Pomodoro timer messages.

/// Pomodoro timer messages.
///
/// These messages control the Pomodoro timer in focus mode.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, PomodoroMessage, update};
///
/// let mut model = Model::new().with_sample_data();
///
/// // Start a Pomodoro session with a goal of 4 cycles
/// update(&mut model, PomodoroMessage::Start { goal_cycles: 4 }.into());
///
/// // Pause/resume the timer
/// update(&mut model, PomodoroMessage::TogglePause.into());
///
/// // Skip current phase
/// update(&mut model, PomodoroMessage::Skip.into());
/// ```
#[derive(Debug, Clone)]
pub enum PomodoroMessage {
    /// Start a new Pomodoro session
    Start {
        /// Target number of work cycles to complete
        goal_cycles: u32,
    },
    /// Pause the current timer
    Pause,
    /// Resume a paused timer
    Resume,
    /// Toggle between paused and running
    TogglePause,
    /// Skip the current phase (work/break)
    Skip,
    /// Stop the Pomodoro session entirely
    Stop,
    /// Timer tick (called every second when running)
    Tick,
    /// Configure work duration (in minutes)
    SetWorkDuration(u32),
    /// Configure short break duration (in minutes)
    SetShortBreak(u32),
    /// Configure long break duration (in minutes)
    SetLongBreak(u32),
    /// Configure cycles before long break
    SetCyclesBeforeLongBreak(u32),
    /// Increment session goal
    IncrementGoal,
    /// Decrement session goal
    DecrementGoal,
}
