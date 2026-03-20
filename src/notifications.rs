//! Desktop notification support.
//!
//! Sends OS-level notifications for key events when the `desktop-notifications`
//! feature is enabled.  Falls back to a no-op when the feature is disabled or
//! when the notification daemon is unavailable (e.g., in headless CI).
//!
//! # Usage
//!
//! Enable via Cargo feature:
//! ```toml
//! taskflow = { features = ["desktop-notifications"] }
//! ```
//!
//! Then call the helper functions at the appropriate event points:
//!
//! ```rust,ignore
//! use taskflow::notifications;
//!
//! notifications::set_enabled(true);
//! notifications::notify_pomodoro_phase("Work session complete! Take a break.");
//! notifications::notify_overdue_tasks(3);
//! ```

use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);

/// Set whether desktop notifications are active at runtime.
///
/// Call once at startup based on the user's `notifications_enabled` setting.
/// This gate applies to all notification functions regardless of Cargo feature state.
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// Send a desktop notification for a Pomodoro phase transition.
///
/// Silently ignored if the `desktop-notifications` feature is not enabled
/// or if the notification daemon is unavailable.
pub fn notify_pomodoro_phase(message: &str) {
    send("TaskFlow – Pomodoro", message);
}

/// Send a desktop notification when overdue tasks are found at startup.
pub fn notify_overdue_tasks(count: usize) {
    if count == 0 {
        return;
    }
    let noun = if count == 1 { "task is" } else { "tasks are" };
    send(
        "TaskFlow – Overdue Tasks",
        &format!("{count} {noun} overdue"),
    );
}

/// Send a desktop notification when tasks are due today.
pub fn notify_due_today(count: usize) {
    if count == 0 {
        return;
    }
    let noun = if count == 1 { "task" } else { "tasks" };
    send("TaskFlow – Due Today", &format!("{count} {noun} due today"));
}

/// Send a desktop notification when a new recurring task occurrence is spawned.
pub fn notify_recurring_spawned(title: &str) {
    send(
        "TaskFlow – Recurring Task",
        &format!("New occurrence created: {title}"),
    );
}

/// Send a desktop notification for tasks due within the next N days.
pub fn notify_due_soon(count: usize, days: u32) {
    if count == 0 || days == 0 {
        return;
    }
    let noun = if count == 1 { "task" } else { "tasks" };
    let period = if days == 1 {
        "tomorrow".to_string()
    } else {
        format!("in {days} days")
    };
    send(
        "TaskFlow – Due Soon",
        &format!("{count} {noun} due {period}"),
    );
}

/// Send a desktop notification when snoozed tasks have become active again.
pub fn notify_snooze_expired(count: usize) {
    if count == 0 {
        return;
    }
    let noun = if count == 1 { "task" } else { "tasks" };
    send(
        "TaskFlow – Snooze Expired",
        &format!("{count} snoozed {noun} now due"),
    );
}

// ============================================================================
// Internal dispatch
// ============================================================================

#[cfg(feature = "desktop-notifications")]
fn send(summary: &str, body: &str) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    use notify_rust::Notification;
    // Fire-and-forget: ignore errors (daemon may be unavailable in SSH sessions etc.)
    let _ = Notification::new()
        .summary(summary)
        .body(body)
        .appname("taskflow")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
}

#[cfg(not(feature = "desktop-notifications"))]
fn send(_summary: &str, _body: &str) {
    // No-op when feature is disabled
}
