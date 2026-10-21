//! Calendar-date helpers that agree on what "today" means.
//!
//! Task fields such as `due_date`, `scheduled_date` and `snooze_until` are
//! `NaiveDate`s in the user's local calendar. Timestamps (`created_at`,
//! `completed_at`, time-entry `started_at`, ...) are stored in UTC. Mixing the
//! two naively, by comparing a local due date with `Utc::now().date_naive()` or
//! with `completed_at.date_naive()`, makes "today", "overdue" and "completed
//! today" flip at the wrong hour for anyone outside UTC.
//!
//! Every "today" computation and every timestamp-to-date conversion in the
//! codebase should go through this module:
//!
//! - [`today`] is the current date in the local timezone.
//! - [`LocalDate::local_date`] converts a timestamp to its local calendar date.

use chrono::{DateTime, Local, NaiveDate, TimeZone};

/// The current date in the local timezone.
///
/// Use this instead of `Utc::now().date_naive()` or `Local::now().date_naive()`
/// so the whole application shares one definition of "today".
#[must_use]
pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// Convert a timestamp to the calendar date it falls on in the local timezone.
pub trait LocalDate {
    /// The local calendar date of this instant.
    fn local_date(&self) -> NaiveDate;
}

impl<Tz: TimeZone> LocalDate for DateTime<Tz> {
    fn local_date(&self) -> NaiveDate {
        self.with_timezone(&Local).date_naive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn today_matches_local_clock() {
        assert_eq!(today(), Local::now().date_naive());
    }

    #[test]
    fn local_date_of_now_is_today() {
        assert_eq!(Utc::now().local_date(), today());
        assert_eq!(Local::now().local_date(), today());
    }

    #[test]
    fn local_date_respects_timezone_offset() {
        // An instant and the same instant expressed in another zone must map to
        // the same local date, whatever the host timezone is.
        let utc = Utc::now();
        let local = utc.with_timezone(&Local);
        assert_eq!(utc.local_date(), local.local_date());

        // 24h apart always differ by exactly one calendar day.
        let later = utc + Duration::hours(24);
        assert_eq!(later.local_date(), utc.local_date() + Duration::days(1));
    }

    #[test]
    fn local_date_works_through_references() {
        let ts = Utc::now();
        let by_ref: &DateTime<Utc> = &ts;
        assert_eq!(by_ref.local_date(), ts.local_date());
    }
}
