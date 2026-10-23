//! Desktop notification support.
//!
//! Sends OS-level notifications for key events when the `desktop-notifications`
//! feature is enabled (it is part of the default feature set).  Falls back to a
//! no-op when the feature is disabled or when the notification daemon is
//! unavailable (e.g., in headless CI).
//!
//! # Usage
//!
//! The feature is on by default; build without it with
//! `cargo build --no-default-features`.  At runtime the user's
//! `notifications_enabled` setting gates every notification through
//! [`set_enabled`], which the TUI calls once at startup before anything fires:
//!
//! ```rust,ignore
//! use taskflow::notifications;
//!
//! notifications::set_enabled(true);
//! notifications::notify_pomodoro_phase("Work session complete! Take a break.");
//! notifications::notify_overdue_tasks(3);
//! notifications::notify_due_soon(2, 1);
//! ```

use std::sync::atomic::{AtomicBool, Ordering};

/// Runtime master switch; `true` until [`set_enabled`] says otherwise.
static ENABLED: AtomicBool = AtomicBool::new(true);

/// Set whether desktop notifications are active at runtime.
///
/// Call once at startup based on the user's `notifications_enabled` setting.
/// This gate applies to all notification functions regardless of Cargo feature state.
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// Whether notifications are currently allowed to fire (see [`set_enabled`]).
#[must_use]
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Send a desktop notification for a Pomodoro phase transition.
///
/// Silently ignored if the `desktop-notifications` feature is not enabled
/// or if the notification daemon is unavailable.
pub fn notify_pomodoro_phase(message: &str) {
    dispatch("TaskFlow – Pomodoro", message);
}

/// Send a desktop notification when overdue tasks are found at startup.
pub fn notify_overdue_tasks(count: usize) {
    if count == 0 {
        return;
    }
    let noun = if count == 1 { "task is" } else { "tasks are" };
    dispatch(
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
    dispatch("TaskFlow – Due Today", &format!("{count} {noun} due today"));
}

/// Send a desktop notification when a new recurring task occurrence is spawned.
pub fn notify_recurring_spawned(title: &str) {
    dispatch(
        "TaskFlow – Recurring Task",
        &format!("New occurrence created: {title}"),
    );
}

/// Send a desktop notification for tasks due within the next `days` days
/// (after today, up to and including today + `days`).
///
/// Nothing is sent when `count` or `days` is zero; `days == 0` is how the
/// `due_soon_days` setting disables this notification.
pub fn notify_due_soon(count: usize, days: u32) {
    if count == 0 || days == 0 {
        return;
    }
    let noun = if count == 1 { "task" } else { "tasks" };
    let period = if days == 1 {
        "tomorrow".to_string()
    } else {
        format!("within {days} days")
    };
    dispatch(
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
    dispatch(
        "TaskFlow – Snooze Expired",
        &format!("{count} snoozed {noun} now due"),
    );
}

// ============================================================================
// Internal dispatch
// ============================================================================

/// Apply the runtime gate, then hand the notification to the platform backend.
fn dispatch(summary: &str, body: &str) {
    if !is_enabled() {
        return;
    }
    send(summary, body);
}

#[cfg(all(feature = "desktop-notifications", not(test)))]
fn send(summary: &str, body: &str) {
    use notify_rust::Notification;
    // Fire-and-forget: ignore errors (daemon may be unavailable in SSH sessions etc.)
    let _ = Notification::new()
        .summary(summary)
        .body(body)
        .appname("taskflow")
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
}

#[cfg(all(not(feature = "desktop-notifications"), not(test)))]
fn send(_summary: &str, _body: &str) {
    // No-op when feature is disabled
}

/// Unit tests never talk to a notification daemon: every notification that
/// passes the gate is recorded on the calling thread instead.
#[cfg(test)]
fn send(summary: &str, body: &str) {
    tests::record(summary, body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::{Mutex, MutexGuard};

    thread_local! {
        static SENT: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(summary: &str, body: &str) {
        SENT.with(|s| s.borrow_mut().push((summary.to_string(), body.to_string())));
    }

    fn take_sent() -> Vec<(String, String)> {
        SENT.with(|s| std::mem::take(&mut *s.borrow_mut()))
    }

    /// `ENABLED` is process-global, so tests that flip it run one at a time
    /// and restore the default before releasing the lock.
    static GATE: Mutex<()> = Mutex::new(());

    struct Gate(#[allow(dead_code)] MutexGuard<'static, ()>);

    impl Gate {
        fn lock() -> Self {
            let guard = GATE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            set_enabled(true);
            take_sent();
            Self(guard)
        }
    }

    impl Drop for Gate {
        fn drop(&mut self) {
            set_enabled(true);
        }
    }

    #[test]
    fn enabled_by_default() {
        let _gate = Gate::lock();
        assert!(is_enabled());
    }

    #[test]
    fn set_enabled_toggles_the_gate() {
        let _gate = Gate::lock();
        set_enabled(false);
        assert!(!is_enabled());
        set_enabled(true);
        assert!(is_enabled());
    }

    #[test]
    fn disabled_gate_makes_every_notification_a_noop() {
        let _gate = Gate::lock();
        set_enabled(false);

        notify_pomodoro_phase("Break over");
        notify_overdue_tasks(3);
        notify_due_today(2);
        notify_recurring_spawned("Water plants");
        notify_due_soon(4, 1);
        notify_snooze_expired(1);

        assert!(take_sent().is_empty());
    }

    #[test]
    fn enabled_gate_dispatches_notifications() {
        let _gate = Gate::lock();

        notify_pomodoro_phase("Work session complete");
        notify_recurring_spawned("Water plants");

        assert_eq!(
            take_sent(),
            vec![
                (
                    "TaskFlow – Pomodoro".to_string(),
                    "Work session complete".to_string()
                ),
                (
                    "TaskFlow – Recurring Task".to_string(),
                    "New occurrence created: Water plants".to_string()
                ),
            ]
        );
    }

    #[test]
    fn re_enabling_resumes_delivery() {
        let _gate = Gate::lock();
        set_enabled(false);
        notify_due_today(1);
        set_enabled(true);
        notify_due_today(1);

        let sent = take_sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].1, "1 task due today");
    }

    #[test]
    fn zero_counts_send_nothing() {
        let _gate = Gate::lock();
        notify_overdue_tasks(0);
        notify_due_today(0);
        notify_due_soon(0, 1);
        notify_due_soon(5, 0);
        notify_snooze_expired(0);
        assert!(take_sent().is_empty());
    }

    #[test]
    fn overdue_and_due_today_wording() {
        let _gate = Gate::lock();
        notify_overdue_tasks(1);
        notify_overdue_tasks(2);
        notify_due_today(1);
        notify_due_today(3);

        let bodies: Vec<String> = take_sent().into_iter().map(|(_, b)| b).collect();
        assert_eq!(
            bodies,
            [
                "1 task is overdue",
                "2 tasks are overdue",
                "1 task due today",
                "3 tasks due today",
            ]
        );
    }

    #[test]
    fn due_soon_wording_for_one_day_and_many() {
        let _gate = Gate::lock();
        notify_due_soon(1, 1);
        notify_due_soon(2, 1);
        notify_due_soon(1, 3);
        notify_due_soon(5, 7);

        let sent = take_sent();
        assert!(sent.iter().all(|(s, _)| s == "TaskFlow – Due Soon"));
        let bodies: Vec<&str> = sent.iter().map(|(_, b)| b.as_str()).collect();
        assert_eq!(
            bodies,
            [
                "1 task due tomorrow",
                "2 tasks due tomorrow",
                "1 task due within 3 days",
                "5 tasks due within 7 days",
            ]
        );
    }

    #[test]
    fn snooze_expired_wording() {
        let _gate = Gate::lock();
        notify_snooze_expired(1);
        notify_snooze_expired(4);

        let sent = take_sent();
        assert_eq!(sent[0].0, "TaskFlow – Snooze Expired");
        assert_eq!(sent[0].1, "1 snoozed task now due");
        assert_eq!(sent[1].1, "4 snoozed tasks now due");
    }
}
