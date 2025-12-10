//! Time tracking messages.

/// Time tracking messages.
///
/// Control time tracking for the currently selected task.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, TimeMessage, TaskMessage, update};
///
/// let mut model = Model::new();
/// update(&mut model, TaskMessage::Create("Work on project".to_string()).into());
///
/// // Start tracking time
/// update(&mut model, TimeMessage::StartTracking.into());
///
/// // Stop tracking
/// update(&mut model, TimeMessage::StopTracking.into());
/// ```
#[derive(Debug, Clone)]
pub enum TimeMessage {
    /// Start time tracking for selected task
    StartTracking,
    /// Stop the current time tracking session
    StopTracking,
    /// Toggle time tracking (start if stopped, stop if running)
    ToggleTracking,
}
