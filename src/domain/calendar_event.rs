//! Calendar event entity for imported ICS VEVENT components.
//!
//! Calendar events represent scheduled time blocks imported from external
//! calendar applications via ICS files. Unlike tasks, events are display-only
//! and represent fixed appointments rather than actionable items.
//!
//! # Examples
//!
//! ```
//! use taskflow::domain::{CalendarEvent, CalendarEventStatus};
//! use chrono::Utc;
//!
//! let event = CalendarEvent::new("Team Meeting")
//!     .with_location("Conference Room A")
//!     .with_description("Weekly standup");
//!
//! assert_eq!(event.title, "Team Meeting");
//! assert_eq!(event.status, CalendarEventStatus::Confirmed);
//! ```

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a calendar event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CalendarEventId(pub Uuid);

impl CalendarEventId {
    /// Creates a new unique event ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CalendarEventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CalendarEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a calendar event (from ICS STATUS property).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalendarEventStatus {
    /// Event is tentative/unconfirmed.
    Tentative,
    /// Event is confirmed (default).
    #[default]
    Confirmed,
    /// Event was cancelled.
    Cancelled,
}

impl std::fmt::Display for CalendarEventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tentative => write!(f, "Tentative"),
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// A calendar event imported from an ICS file.
///
/// Events represent scheduled time blocks from external calendars.
/// They are displayed in the Calendar view alongside tasks but are
/// not editable within TaskFlow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// Unique identifier within TaskFlow.
    pub id: CalendarEventId,
    /// Original UID from the ICS file (for deduplication).
    pub uid: String,
    /// Event title (from SUMMARY).
    pub title: String,
    /// Optional description (from DESCRIPTION).
    pub description: Option<String>,
    /// Optional location (from LOCATION).
    pub location: Option<String>,
    /// Start date/time (from DTSTART).
    pub start: DateTime<Utc>,
    /// Optional end date/time (from DTEND).
    pub end: Option<DateTime<Utc>>,
    /// True if this is an all-day event (date-only, no time).
    pub all_day: bool,
    /// Event status (from STATUS).
    pub status: CalendarEventStatus,
    /// When this event was imported.
    pub created_at: DateTime<Utc>,
}

impl CalendarEvent {
    /// Creates a new calendar event with the given title.
    ///
    /// The event starts at the current time by default.
    /// Use builder methods to customize.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: CalendarEventId::new(),
            uid: Uuid::new_v4().to_string(),
            title: title.into(),
            description: None,
            location: None,
            start: now,
            end: None,
            all_day: false,
            status: CalendarEventStatus::default(),
            created_at: now,
        }
    }

    /// Sets the event description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the event location.
    #[must_use]
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Sets the start time.
    #[must_use]
    pub fn with_start(mut self, start: DateTime<Utc>) -> Self {
        self.start = start;
        self
    }

    /// Sets the end time.
    #[must_use]
    pub fn with_end(mut self, end: DateTime<Utc>) -> Self {
        self.end = Some(end);
        self
    }

    /// Sets whether this is an all-day event.
    #[must_use]
    pub fn with_all_day(mut self, all_day: bool) -> Self {
        self.all_day = all_day;
        self
    }

    /// Sets the event status.
    #[must_use]
    pub fn with_status(mut self, status: CalendarEventStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets the original ICS UID.
    #[must_use]
    pub fn with_uid(mut self, uid: impl Into<String>) -> Self {
        self.uid = uid.into();
        self
    }

    /// Returns the date of this event.
    #[must_use]
    pub fn date(&self) -> NaiveDate {
        self.start.date_naive()
    }

    /// Returns the start time (if not an all-day event).
    #[must_use]
    pub fn start_time(&self) -> Option<NaiveTime> {
        if self.all_day {
            None
        } else {
            Some(self.start.time())
        }
    }

    /// Returns the end time (if available and not an all-day event).
    #[must_use]
    pub fn end_time(&self) -> Option<NaiveTime> {
        if self.all_day {
            None
        } else {
            self.end.map(|e| e.time())
        }
    }

    /// Returns true if this event spans multiple days.
    #[must_use]
    pub fn is_multi_day(&self) -> bool {
        self.end
            .is_some_and(|end| end.date_naive() > self.start.date_naive())
    }

    /// Returns true if this event occurs on the given date.
    #[must_use]
    pub fn occurs_on(&self, date: NaiveDate) -> bool {
        let start_date = self.start.date_naive();
        if let Some(end) = self.end {
            let end_date = end.date_naive();
            date >= start_date && date <= end_date
        } else {
            date == start_date
        }
    }

    /// Returns a formatted time range string (e.g., "10:00 - 11:00").
    #[must_use]
    pub fn formatted_time_range(&self) -> String {
        if self.all_day {
            "All day".to_string()
        } else {
            let start = self.start.format("%H:%M").to_string();
            if let Some(end) = self.end {
                format!("{} - {}", start, end.format("%H:%M"))
            } else {
                start
            }
        }
    }
}

impl std::fmt::Display for CalendarEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_calendar_event_new() {
        let event = CalendarEvent::new("Team Meeting");
        assert_eq!(event.title, "Team Meeting");
        assert_eq!(event.status, CalendarEventStatus::Confirmed);
        assert!(!event.all_day);
        assert!(event.description.is_none());
        assert!(event.location.is_none());
    }

    #[test]
    fn test_calendar_event_builder() {
        let event = CalendarEvent::new("Conference")
            .with_description("Annual conference")
            .with_location("Main Hall")
            .with_all_day(true)
            .with_status(CalendarEventStatus::Tentative);

        assert_eq!(event.title, "Conference");
        assert_eq!(event.description, Some("Annual conference".to_string()));
        assert_eq!(event.location, Some("Main Hall".to_string()));
        assert!(event.all_day);
        assert_eq!(event.status, CalendarEventStatus::Tentative);
    }

    #[test]
    fn test_calendar_event_date() {
        let start = Utc.with_ymd_and_hms(2024, 12, 15, 10, 0, 0).unwrap();
        let event = CalendarEvent::new("Meeting").with_start(start);
        assert_eq!(event.date(), NaiveDate::from_ymd_opt(2024, 12, 15).unwrap());
    }

    #[test]
    fn test_calendar_event_occurs_on() {
        let start = Utc.with_ymd_and_hms(2024, 12, 15, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 17, 18, 0, 0).unwrap();
        let event = CalendarEvent::new("Conference")
            .with_start(start)
            .with_end(end);

        assert!(event.occurs_on(NaiveDate::from_ymd_opt(2024, 12, 15).unwrap()));
        assert!(event.occurs_on(NaiveDate::from_ymd_opt(2024, 12, 16).unwrap()));
        assert!(event.occurs_on(NaiveDate::from_ymd_opt(2024, 12, 17).unwrap()));
        assert!(!event.occurs_on(NaiveDate::from_ymd_opt(2024, 12, 14).unwrap()));
        assert!(!event.occurs_on(NaiveDate::from_ymd_opt(2024, 12, 18).unwrap()));
    }

    #[test]
    fn test_calendar_event_is_multi_day() {
        let start = Utc.with_ymd_and_hms(2024, 12, 15, 10, 0, 0).unwrap();
        let end_same_day = Utc.with_ymd_and_hms(2024, 12, 15, 18, 0, 0).unwrap();
        let end_next_day = Utc.with_ymd_and_hms(2024, 12, 16, 10, 0, 0).unwrap();

        let single_day = CalendarEvent::new("Meeting")
            .with_start(start)
            .with_end(end_same_day);
        assert!(!single_day.is_multi_day());

        let multi_day = CalendarEvent::new("Conference")
            .with_start(start)
            .with_end(end_next_day);
        assert!(multi_day.is_multi_day());
    }

    #[test]
    fn test_calendar_event_formatted_time_range() {
        let start = Utc.with_ymd_and_hms(2024, 12, 15, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 15, 11, 30, 0).unwrap();

        let event = CalendarEvent::new("Meeting")
            .with_start(start)
            .with_end(end);
        assert_eq!(event.formatted_time_range(), "10:00 - 11:30");

        let all_day = CalendarEvent::new("Holiday").with_all_day(true);
        assert_eq!(all_day.formatted_time_range(), "All day");
    }

    #[test]
    fn test_calendar_event_status_display() {
        assert_eq!(CalendarEventStatus::Tentative.to_string(), "Tentative");
        assert_eq!(CalendarEventStatus::Confirmed.to_string(), "Confirmed");
        assert_eq!(CalendarEventStatus::Cancelled.to_string(), "Cancelled");
    }

    #[test]
    fn test_calendar_event_id() {
        let id1 = CalendarEventId::new();
        let id2 = CalendarEventId::new();
        assert_ne!(id1, id2);
    }
}
